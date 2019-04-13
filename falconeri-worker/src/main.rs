// Needed for static linking to work right on Linux.
extern crate openssl;

use crossbeam::{self, thread::Scope};
use env_logger;
use falconeri_common::{
    common_failures::display::DisplayCausesAndBacktraceExt,
    db,
    kubernetes::{node_name, pod_name},
    prelude::*,
    storage::CloudStorage,
};
use glob;
use openssl_probe;
use std::{
    env, fs,
    io::{self, prelude::*},
    process,
    sync::{Arc, RwLock},
    thread::sleep,
    time::Duration,
};

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
    let mut conn = db::connect(ConnectVia::Cluster)?;

    // Look up the Kubernetes node and pod we're running under.
    let node_name = node_name()?;
    let pod_name = pod_name()?;

    // Loop until there are no more datums.
    loop {
        // Fetch our job, and make sure that it's still running.
        let mut job = Job::find(job_id, &conn)?;
        trace!("job: {:?}", job);
        if job.status != Status::Running {
            break;
        }

        // Get the next datum and process it.
        if let Some((mut datum, files)) =
            job.reserve_next_datum(&node_name, &pod_name, &conn)?
        {
            // Process our datum, capturing its output.
            let output = Arc::new(RwLock::new(vec![]));
            let result =
                process_datum(&job, &datum, &files, &job.command, output.clone());
            let output_str = String::from_utf8_lossy(
                &output.read().expect("background thread panic"),
            )
            .into_owned();

            // Reconnect to the database after processing the datum, in case our DB
            // connection has timed out or something horrible like that.
            conn = db::connect(ConnectVia::Cluster)?;

            // Handle the processing results.
            match result {
                Ok(()) => datum.mark_as_done(output_str.as_ref(), &conn)?,
                Err(err) => {
                    error!(
                        "failed to process datum {}: {}",
                        datum.id,
                        err.display_causes_and_backtrace(),
                    );
                    let error_message =
                        format!("{}", err.display_causes_without_backtrace());
                    let backtrace = format!("{}", err.backtrace());
                    datum.mark_as_error(
                        output_str.as_ref(),
                        &error_message,
                        &backtrace,
                        &conn,
                    )?
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
    to_record: Arc<RwLock<dyn Write + Send + Sync>>,
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

    // Set up a worker thread scope so that we can handle background I/O.
    crossbeam::scope(|scope| -> Result<()> {
        // Run our command.
        if cmd.is_empty() {
            return Err(format_err!("job {} command is empty", job.id));
        }
        let mut child = process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .with_context(|_| format!("could not run {:?}", &cmd[0]))?;

        // Listen on stdout.
        tee_child(scope, &mut child, to_record)?;

        let status = child
            .wait()
            .with_context(|_| format!("error running {:?}", &cmd[0]))?;
        if !status.success() {
            return Err(format_err!(
                "command {:?} failed with status {}",
                cmd,
                status
            ));
        }

        // Finish up.
        upload_outputs(job, datum).context("could not upload outputs")?;
        reset_work_dirs()?;
        Ok(())
    })
    .expect("background panic")
}

/// Copy the stdout and stderr of `child` to either stdout or stderr,
/// respectively, and write a copy to `to_record`.
///
/// This function will panic if `child` does not have a `stdout` or `stderr`.
fn tee_child<'a>(
    scope: &'a Scope,
    child: &mut process::Child,
    to_record: Arc<RwLock<dyn Write + Send + Sync>>,
) -> Result<()> {
    // Tee `stdout`.
    let mut stdout = child
        .stdout
        .take()
        .expect("child should always have a stdout");
    let to_record_for_stdout = to_record.clone();
    let stdout_handle = scope.spawn(move |_| {
        tee_output(&mut stdout, &mut io::stdout(), to_record_for_stdout)
    });

    // Tee `stderr`.
    let mut stderr = child
        .stderr
        .take()
        .expect("child should always have a stderr");
    let to_record_for_stderr = to_record.clone();
    let stderr_handle = scope.spawn(move |_| {
        tee_output(&mut stderr, &mut io::stderr(), to_record_for_stderr)
    });

    // Wait for our child process to close `stdout` and `stderr`, or at least
    // for Rust to return 0-byte reads and writes.
    stdout_handle.join().expect("background panic")?;
    stderr_handle.join().expect("background panic")?;

    Ok(())
}

/// Copy output from `from_child` to `to_console` and `to_record`.
fn tee_output(
    from_child: &mut dyn Read,
    to_console: &mut dyn Write,
    to_record: Arc<RwLock<dyn Write>>,
) -> Result<()> {
    // Use a small buffer, because I/O performance doesn't matter for reading
    // output to the user.
    let mut buf = vec![0; 4 * 1024];
    loop {
        match from_child.read(&mut buf) {
            // No more output, so give up.
            Ok(0) => return Ok(()),
            // We have output, so print it.
            Ok(count) => {
                let data = &buf[..count];
                to_console.write(data).context("error writing to console")?;
                to_record
                    .write()
                    .expect("background panic")
                    .write(data)
                    .context("error writing to record")?;
            }
            // Retry if reading was interrupted by kernal shenigans.
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            // An actual error occurred.
            Err(e) => {
                return Err(e)
                    .context("error reading from child process")
                    .map_err(|e| e.into());
            }
        }
    }
}

/// Reset our working directories to a default, clean state.
fn reset_work_dirs() -> Result<()> {
    reset_work_dir(Path::new("/pfs/"))?;
    fs::create_dir("/pfs/out").context("cannot create /pfs/out")?;
    reset_work_dir(Path::new("/scratch/"))?;
    Ok(())
}

/// Restore a directory to a default, clean state.
fn reset_work_dir(work_dir: &Path) -> Result<()> {
    debug!("resetting work dir {}", work_dir.display());

    // Make sure our work dir still exists.
    if !work_dir.is_dir() {
        return Err(format_err!(
            "the directory {} does not exist, but `falconeri_worker` expects it",
            work_dir.display()
        ));
    }

    // Delete everything in our work dir.
    let entries = work_dir
        .read_dir()
        .with_context(|_| format!("error listing directory {}", work_dir.display()))?;
    for entry in entries {
        let path = entry
            .with_context(|_| {
                format!("error listing directory {}", work_dir.display())
            })?
            .path();
        trace!("deleting {}", path.display());
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|_| format!("cannot delete {}", path.display()))?;
        }
    }

    // Make sure we haven't deleted our work dir accidentally.
    assert!(work_dir.is_dir());
    Ok(())
}

/// Upload `/pfs/out` to our output bucket.
fn upload_outputs(job: &Job, datum: &Datum) -> Result<()> {
    debug!("uploading outputs");

    // Make a new database connection, because any one we created before running
    // our command might have expired.
    let conn = db::connect(ConnectVia::Cluster)?;

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
        NewOutputFile::insert_all(
            &[NewOutputFile {
                datum_id: datum.id,
                job_id: job.id,
                uri: uri.clone(),
            }],
            &conn,
        )?;
    }

    // Upload all our files in a batch, for maximum performance, and record
    // what happened.
    let storage = CloudStorage::for_uri(&job.egress_uri, &[])?;
    let result = storage.sync_up(Path::new("/pfs/out/"), &job.egress_uri);
    match result {
        Ok(()) => OutputFile::mark_as_done_by_datum(datum, &conn)?,
        Err(_) => OutputFile::mark_as_error_by_datum(datum, &conn)?,
    }
    result
}
