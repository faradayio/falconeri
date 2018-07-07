extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate falconeri_common;
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
    _job: &Job,
    datum: &Datum,
    files: &[File],
    cmd: &[String],
) -> Result<()> {
    debug!("Processing datum {}", datum.id);

    // Download each file.
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

    // Delete input files.
    //
    // TODO: Do this even if the command fails, or one of the downloads fails.
    // Or just clean up `/pfs` completely.
    for file in files {
        fs::remove_file(&file.local_path)
            .with_context(|_| format!("could not delete {:?}", &file.local_path))?;
    }

    Ok(())
}

