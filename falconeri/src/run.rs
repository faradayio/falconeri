//! The `run` subcommand.

use failure::ResultExt;
use falconeri_common::{db, diesel::{self, prelude::*}, Error, models::*, schema::jobs, Result};
use std::{io::BufRead, process::{Command, Stdio}};

use pipeline::*;

/// The `run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    match &pipeline_spec.input {
        InputInfo::Atom { repo, glob } => {
            // Check to make sure we're using a supported glob mode.
            if glob != "/*" {
                return Err(format_err!("Glob {} not yet supported", glob));
            }

            // Shell out to gsutil to list the files we want to process.
            let output = Command::new("gsutil")
                .arg("ls")
                .arg(&repo)
                .stderr(Stdio::inherit())
                .output()
                .context("error running gsutil")?;
            let mut paths = vec![];
            for line in output.stdout.lines() {
                let line = line?;
                paths.push(line.trim_right().to_owned());
            }

            add_job_to_database(pipeline_spec, &paths)?;

            Ok(())
        }
    }
}

/// Register a new job in the database.
pub fn add_job_to_database(
    pipeline_spec: &PipelineSpec,
    inputs: &[String],
) -> Result<()> {
    let conn = db::connect()?;
    conn.transaction::<_, Error, _>(|| -> Result<()> {
        // Create our new job.
        let new_job = NewJob {
            pipeline_spec: json!(pipeline_spec),
            destination_uri: pipeline_spec.output.repo.clone(),
        };
        let mut job = new_job.insert(&conn)?;

        // Create a datum for each input file.
        for input in inputs {
            let new_datum = NewDatum {
                job_id: job.id,
                source_uri: input.to_owned(),
            };
            new_datum.insert(&conn)?;
        }

        // Update our job status.
        job = diesel::update(jobs::table.filter(jobs::id.eq(&job.id)))
            .set(jobs::status.eq(&Status::Running))
            .get_result(&conn)
            .context("couldn't set job status")?;

        println!("{}", job.id);
        Ok(())
    })?;

    Ok(())
}
