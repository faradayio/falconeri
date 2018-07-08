extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate falconeri_common;
extern crate glob;
#[macro_use]
extern crate log;
extern crate uuid;

use failure::ResultExt;
use falconeri_common::{db, models::*, Result};
use std::{env, fs, process};
use uuid::Uuid;

fn main() -> Result<()> {
    env_logger::init();

    // Parse our arguments (manually, so we don't need to drag in a ton of
    // libraries).
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: falconeri-worker <job id> <cmd> <args...>");
        process::exit(1);
    }
    let job_id: Uuid = args[0].parse()?;
    let cmd = &args[1..];
    debug!("Job ID: {} cmd: {:?}", job_id, cmd);

    // Connect to the database.
    let mut conn = db::connect()?;

    // Loop until there are no more datums.
    loop {
        // Fetch our job, and make sure that it's still running.
        let job = Job::find(job_id, &conn)?;
        trace!("Job: {:?}", job);
        if job.status != Status::Running {
            break;
        }

        // Get the next datum and process it.
        if let Some((mut datum, files)) = job.reserve_next_datum(&conn)? {
            let result = process_datum(&job, &datum, &files, cmd);

            // Reconnect to the database after processing the datum, in case our DB
            // connection has timed out or something horrible like that.
            conn = db::connect()?;

            // Handle the processing results.
            match result {
                Ok(()) => datum.mark_as_done(&conn)?,
                Err(err) => datum.mark_as_error(&err, &conn)?,
            }
        } else {
            debug!("no more datums to process");
            break;
        }
    }

    Ok(())
}

/// Process a single datum.
fn process_datum(
    job: &Job,
    datum: &Datum,
    files: &[File],
    cmd: &[String],
) -> Result<()> {
    debug!("processing datum {}", datum.id);

    // Download each file.
    reset_work_dir()?;
    for file in files {
        let status = process::Command::new("gsutil")
            .arg("cp")
            .args(&[&file.uri, &file.local_path])
            .status()?;
        if !status.success() {
            return Err(format_err!("could not download {:?}", file.uri));
        }
    }

    // Run our command.
    let status = process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()?;
    if !status.success() {
        return Err(format_err!("could not run {:?}", cmd));
    }

    // Finish up.
    upload_outputs(job, datum)?;
    reset_work_dir()?;
    Ok(())
}

/// Restore our `/pfs` directory to its default, clean state.
fn reset_work_dir() -> Result<()> {
    let paths = glob::glob("/pfs/*").context("error listing /pfs")?;
    for path in paths {
        let path = path.context("error listing /pfs")?;
        trace!("deleting: {}", path.display());
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        }
    }
    fs::create_dir("/pfs/out").context("cannot create /pfs/out")?;
    Ok(())
}

/// Upload `/pfs/out` to our output bucket.
fn upload_outputs(job: &Job, _datum: &Datum) -> Result<()> {
    debug!("uploading outputs");
    let local_paths = glob::glob("/pfs/out/**/*").context("error listing /pfs/out")?;
    for local_path in local_paths {
        let local_path = local_path.context("error listing /pfs/out")?;

        // Skip anything we can't upload.
        if local_path.is_dir() {
            continue;
        } else if !local_path.is_file() {
            warn!("can't upload special file {}", local_path.display());
            continue;
        }

        // Get our local path, and strip the prefix.
        let rel_path = local_path.strip_prefix("/pfs/out/")?;
        let rel_path_str = rel_path.to_str()
            .ok_or_else(|| format_err!("invalid characters in {:?}", rel_path))?;

        // Build the URI we want to upload to.
        let mut uri = job.output_uri.clone();
        if !uri.ends_with("/") {
            uri.push_str("/");
        }
        uri.push_str(&rel_path_str);

        // Upload the file.
        trace!("uploading {} to {}", local_path.display(), uri);
        let status = process::Command::new("gsutil")
            .arg("cp")
            .arg(&local_path)
            .arg(&uri)
            .status()?;
        if !status.success() {
            return Err(format_err!("could not upload {}", local_path.display()));
        }
    }
    Ok(())
}