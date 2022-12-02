//! A background process which tries to keep an eye on running jobs.
//!
//! We only store state in Postgres, and we assume that:
//!
//! 1. Any process can fail at any time, and
//! 2. **More than one copy of the babysitter will normally be running.**
//!
//! Using PostgreSQL to store state is one of the simplest ways to build a
//! medium-reliability, small-scale distributed job system.

use std::{panic::catch_unwind, process, thread, time::Duration};

use falconeri_common::{
    chrono, db, kubernetes::get_all_job_names, prelude::*, tracing,
};

/// Spawn a thread and run the babysitter in it. This should run indefinitely.
#[tracing::instrument(level = "trace")]
pub fn start_babysitter() -> Result<thread::JoinHandle<()>> {
    let builder = thread::Builder::new().name("babysitter".to_owned());
    builder
        .spawn(run_babysitter_wrapper)
        .context("could not create babysitter thread")
}

/// Run the babysitter, and abort if we catch any panics.
#[tracing::instrument(level = "trace")]
fn run_babysitter_wrapper() {
    // If this thread panics, attempt to shut down the entire process, forcing
    // Kubernetes to make noise and restart this `falconerid`. The last thing we
    // want is for the babysitter to silently fail.
    //
    // And no, Rust does not make it nice to catch panics. We're supposed to
    // used `Result` for any kind of ordinary error handling, and reserve
    // `panic!` for assertion-failure-like errors.
    if let Err(err) = catch_unwind(run_babysitter) {
        // Extract information about the panic, if it's one of the common types.
        let msg = if let Some(msg) = err.downcast_ref::<&str>() {
            // Created by `panic!("fixed string")`.
            *msg
        } else if let Some(msg) = err.downcast_ref::<String>() {
            // Created by `panic!("format string: {}", "with arguments")`.
            msg
        } else {
            // There's really nothing better we can do here.
            "an unknown panic occurred"
        };

        // Log and print this just in case, so everyone knows what's happening,
        // regardless of whether logs are enabled or where they are sent.
        error!("BABYSITTER PANIC, aborting: {}", msg);
        eprintln!("BABYSITTER PANIC, aborting: {}", msg);
        process::abort();
    }
}

/// Actually run the babysitter.
#[tracing::instrument(level = "trace")]
fn run_babysitter() {
    loop {
        // We always want to retry all errors. This way, if PostgreSQL is still
        // starting up, or if someone retarted it, we'll eventually recover.
        if let Err(err) = check_running_jobs() {
            error!(
                "error checking running jobs (will retry later): {}",
                err.display_causes_and_backtrace()
            );
        }
        thread::sleep(Duration::from_secs(2 * 60));
    }
}

/// Check our running jobs for various situations we might might need to deal
/// with.
#[tracing::instrument(level = "debug")]
fn check_running_jobs() -> Result<()> {
    let conn = db::connect(ConnectVia::Cluster)?;
    check_for_finished_and_vanished_jobs(&conn)?;
    check_for_zombie_datums(&conn)?;
    // Note that any datums marked as `Status::Error` by
    // `check_for_zombie_datums` above may then be retried normally by
    // `check_for_datums_which_can_be_rerun` (if they're eligible).
    check_for_datums_which_can_be_rerun(&conn)
}

/// Check for jobs which should already be marked as finished, or which have
/// vanished off the cluster.
#[tracing::instrument(skip(conn), level = "debug")]
fn check_for_finished_and_vanished_jobs(conn: &PgConnection) -> Result<()> {
    let jobs = Job::find_by_status(Status::Running, conn)?;
    let all_job_names = get_all_job_names()?;
    for mut job in jobs {
        conn.transaction(|| -> Result<()> {
            // We may be racing a second copy of the babysitter here, or a
            // request from a worker, so start a transaction, take a lock, and
            // double-check everything before we act on it.
            job.lock_for_update(conn)?;

            // Check to see if we should have already marked this job as
            // finished. This should normally happen automatically, but if it
            // doesn't, we'll catch it here.
            //
            // This will internally retake the lock and open a nested a
            // transaction, but that should be fine.
            job.update_status_if_done(conn)?;

            // If the job has been running for a while, but it has no associated
            // Kubernetes job, assume that either the job has exceeded
            // `ttlAfterSecondsFinished`, or was manually deleted by someone.
            let cutoff = Utc::now().naive_utc() - chrono::Duration::minutes(15);
            if job.status == Status::Running
                && job.created_at < cutoff
                && !all_job_names.contains(&job.job_name)
            {
                warn!("job {} is running but has no corresponding Kubernetes job, setting status to 'error'", job.job_name);
                job.mark_as_error(conn)?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Check for datums which claim to be running in a pod that no longer exists.
#[tracing::instrument(skip(conn), level = "debug")]
fn check_for_zombie_datums(conn: &PgConnection) -> Result<()> {
    let zombies = Datum::zombies(conn)?;
    for mut zombie in zombies {
        // We may be racing a second copy of the babysitter here, so start a
        // transaction, take a lock, and double-check that our status is still
        // `Status::Running`.
        conn.transaction(|| -> Result<()> {
            zombie.lock_for_update(conn)?;
            if zombie.status == Status::Running {
                warn!(
                    "found zombie datum {}, which was supposed to be running on pod {:?}",
                    zombie.id, zombie.pod_name
                );
                zombie.mark_as_error(
                    "(did not capture output)",
                    "worker pod disappeared while working on datum",
                    "(no backtrace available)",
                    conn,
                )?;
            } else {
                warn!("someone beat us to zombie datum {}", zombie.id);
            }
            Ok(())
        })?;
        // If there are no more datums, mark the job as finished (either
        // done or error).
        zombie.update_job_status_if_done(conn)?;
    }
    Ok(())
}

/// Check for datums which are in the error state but which are eligible for
/// retries.
#[tracing::instrument(skip(conn), level = "debug")]
fn check_for_datums_which_can_be_rerun(conn: &PgConnection) -> Result<()> {
    let rerunable_datums = Datum::rerunable(conn)?;
    for mut datum in rerunable_datums {
        // We may be racing a second copy of the babysitter here, so start a
        // transaction, take a lock, and double-check that we're still eligible
        // for a re-run.
        conn.transaction(|| -> Result<()> {
            // Mark our datum as re-runnable.
            datum.lock_for_update(conn)?;
            if datum.is_rerunable() {
                warn!(
                    "rescheduling errored datum {} (previously on try {}/{})",
                    datum.id,
                    datum.attempted_run_count,
                    datum.maximum_allowed_run_count
                );
                datum.mark_as_eligible_for_rerun(conn)?;
            } else {
                warn!("someone beat us to rerunable datum {}", datum.id);
            }

            // Remove `OutputFile` records for this datum, so we can upload the
            // same output files again.
            //
            // TODO: Unfortunately, there's an issue here. It takes one of two
            // forms:
            //
            // 1. Workers use deterministic file names. In this case, we
            //    _should_ be fine, because we'll just overwrite any files we
            //    did manage to upload.
            // 2. Workers use random filenames. Here, there are two subcases: a.
            //    We have successfully created an `OutputFile` record. b. We
            //    have yet to create an `OutputFile` record.
            //
            // We need to fix (2b) by pre-creating all our `OutputFile` records
            // _before_ uploading, and then updating them later to show that the
            // output succeeded. Which them into case (2a). And then we can fix (2a)
            // by deleting any S3/GCS files corresponding to `OutputFile::uri`.
            OutputFile::delete_for_datum(&datum, conn)?;
            Ok(())
        })?;
    }
    Ok(())
}
