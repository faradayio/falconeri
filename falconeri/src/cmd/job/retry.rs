//! The `job retry` subcommand.

use falconeri_common::{cast, db, diesel::Connection, prefix::*};
use serde_json;
use std::cmp::min;

use super::run::{start_batch_job, unique_kubernetes_job_name};
use pipeline::PipelineSpec;

/// The `job retry` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let (pipeline_spec, new_job) = conn.transaction(|| -> Result<_> {
        // Load the original job, failed datums, and input files.
        let job = Job::find_by_job_name(job_name, &conn)?;
        if job.status != Status::Error {
            return Err(format_err!("can only retry jobs with status 'error'"));
        }
        let error_datums = job.datums_with_status(Status::Error, &conn)?;
        let input_files = InputFile::for_datums(&error_datums, &conn)?;

        // Recover the original pipeline specification.
        let mut pipeline_spec: PipelineSpec =
            serde_json::from_value(job.pipeline_spec.clone())
                .context("could not parse original pipeline spec")?;
        pipeline_spec.parallelism_spec.constant = min(
            pipeline_spec.parallelism_spec.constant,
            cast::u32(error_datums.len())?,
        );

        // Create a new job record.
        let job_name = unique_kubernetes_job_name(&pipeline_spec.pipeline.name);
        let new_job = NewJob {
            pipeline_spec: job.pipeline_spec.clone(),
            job_name,
            command: job.command.clone(),
            egress_uri: job.egress_uri.clone(),
        }
        .insert(&conn)?;

        // Create new datums and input files.
        let mut new_datums = vec![];
        let mut new_input_files = vec![];
        for (_datum, input_files) in error_datums.into_iter().zip(input_files) {
            let datum_id = Uuid::new_v4();
            new_datums.push(NewDatum {
                id: datum_id,
                job_id: new_job.id,
            });
            for input_file in input_files {
                new_input_files.push(NewInputFile {
                    datum_id: datum_id,
                    uri: input_file.uri.clone(),
                    local_path: input_file.local_path.clone(),
                    job_id: new_job.id,
                });
            }
        }
        NewDatum::insert_all(&new_datums, &conn)?;
        NewInputFile::insert_all(&new_input_files, &conn)?;

        Ok((pipeline_spec, new_job))
    })?;

    // Start a new batch job.
    start_batch_job(&pipeline_spec, &new_job)?;
    println!("{}", new_job.job_name);
    Ok(())
}
