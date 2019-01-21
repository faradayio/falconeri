use env_logger;
use falconeri_common::{
    common_failures::display::DisplayCausesAndBacktraceExt, db, prelude::*,
    storage::CloudStorage,
};
use glob;
use openssl_probe;
use std::{env, fs, process, thread::sleep, time::Duration};

/// Instructions on how to use this program.
const USAGE: &str = "Usage: falconeri-worker <job id>";

/// Our main entry point.
fn main() -> Result<()> {
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();

    // Parse our arguments (manually, so we don't need to drag in a ton of
    // libraries).
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("{}", USAGE);
        process::exit(1);
    }
    if args[1] == "--version" {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        process::exit(0);
    } else if args[1] == "--help" {
        println!("{}", USAGE);
        process::exit(0);
    }
    let job_id = args[1].parse::<Uuid>().context("can't parse job ID")?;
    debug!("job ID: {}", job_id);

    // Connect to the database.
    let mut conn = db::connect(db::ConnectVia::Cluster)?;

    // Loop until there are no more datums.
    loop {
        // Fetch our job, and make sure that it's still running.
        let mut job = Job::find(job_id, &conn)?;
        trace!("job: {:?}", job);
        if job.status != Status::Running {
            break;
        }

        // Get the next datum and process it.
        if let Some((mut datum, files)) = job.reserve_next_datum(&conn)? {
            let result = process_datum(&job, &datum, &files, &job.command);

            // Reconnect to the database after processing the datum, in case our DB
            // connection has timed out or something horrible like that.
            conn = db::connect(db::ConnectVia::Cluster)?;

            // Handle the processing results.
            match result {
                Ok(()) => datum.mark_as_done(&conn)?,
                Err(err) => {
                    error!(
                        "failed to process datum {}: {}",
                        datum.id,
                        err.display_causes_and_backtrace(),
                    );
                    datum.mark_as_error(&err, &conn)?
                }
            }
        } else {
            debug!("no more datums to process");
            job.update_status_if_done(&conn)?;

            // Don't exit until all the other workers are ready to exit, because
            // we might be getting run as a Kubernetes `Job`, and if so, a 0
            // exit status would mean that it's safe to start descheduling all
            // other workers. Yes this is weird.
            while job.status == Status::Running {
                trace!("waiting for job to finish");
                sleep(Duration::from_secs(30));
                job = Job::find(job_id, &conn)?;
            }
            debug!("all workers have finished");
            break;
        }
    }

    Ok(())
}

/// Process a single datum.
fn process_datum(
    job: &Job,
    datum: &Datum,
    files: &[InputFile],
    cmd: &[String],
) -> Result<()> {
    debug!("processing datum {}", datum.id);

    // Download each file.
    reset_work_dirs()?;
    for file in files {
        // We don't pass in any `secrets` here, because those are supposed to
        // be specified in our Kubernetes job when it's created.
        let storage = CloudStorage::for_uri(&file.uri, &[])?;
        storage.sync_down(&file.uri, Path::new(&file.local_path))?;
    }

    // Run our command.
    if cmd.is_empty() {
        return Err(format_err!("job {} command is empty", job.id));
    }
    let status = process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .with_context(|_| format!("could not run {:?}", &cmd[0]))?;
    if !status.success() {
        return Err(format_err!("could not run {:?}", cmd));
    }

    // Finish up.
    upload_outputs(job, datum).context("could not upload outputs")?;
    reset_work_dirs()?;
    Ok(())
}

/// Reset our working directories to a default, clean state.
fn reset_work_dirs() -> Result<()> {
    reset_work_dir("/pfs/*")?;
    fs::create_dir("/pfs/out").context("cannot create /pfs/out")?;
    reset_work_dir("/scratch/*")?;
    Ok(())
}

/// Restore a directory to a default, clean state.
fn reset_work_dir(work_dir_glob: &str) -> Result<()> {
    let paths = glob::glob(work_dir_glob)
        .with_context(|_| format!("error listing directory {}", work_dir_glob))?;
    for path in paths {
        let path = path
            .with_context(|_| format!("error listing directory {}", work_dir_glob))?;
        trace!("deleting: {}", path.display());
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        }
    }
    Ok(())
}

/// Upload `/pfs/out` to our output bucket.
fn upload_outputs(job: &Job, datum: &Datum) -> Result<()> {
    debug!("uploading outputs");

    // Make a new database connection, because any one we created before running
    // our command might have expired.
    let conn = db::connect(db::ConnectVia::Cluster)?;

    // Create records describing the files we're going to upload.
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
        let rel_path_str = rel_path
            .to_str()
            .ok_or_else(|| format_err!("invalid characters in {:?}", rel_path))?;

        // Build the URI we want to upload to.
        let mut uri = job.egress_uri.clone();
        if !uri.ends_with('/') {
            uri.push_str("/");
        }
        uri.push_str(&rel_path_str);

        // Create a database record for the file we're about to upload.
        NewOutputFile {
            datum_id: datum.id,
            job_id: job.id,
            uri: uri.clone(),
        }
        .insert(&conn)?;
    }

    // Upload all our files in a batch, for maximum performance, and record
    // what happened.
    let storage = CloudStorage::for_uri(&job.egress_uri, &[])?;
    let result = storage.sync_up(Path::new("/pfs/out/"), &job.egress_uri);
    match result {
        Ok(()) => OutputFile::mark_as_done(datum, &conn)?,
        Err(_) => OutputFile::mark_as_error(datum, &conn)?,
    }
    result
}
