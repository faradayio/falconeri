// ! Code for starting a job on the server.

use falconeri_common::{
    cast, diesel::Connection, kubernetes, manifest::render_manifest, pipeline::*,
    prelude::*,
};
use serde_json::{self, json};
use std::cmp::min;

use crate::inputs::input_to_datums;

/// Run a new job on our cluster.
pub fn run_job(pipeline_spec: &PipelineSpec, conn: &PgConnection) -> Result<Job> {
    // Build our job.
    let job_id = Uuid::new_v4();
    let job_name = unique_kubernetes_job_name(&pipeline_spec.pipeline.name);
    let new_job = NewJob {
        id: job_id,
        pipeline_spec: json!({
            "pipeline": pipeline_spec.pipeline,
            "transform": pipeline_spec.transform,
            "parallelism_spec": pipeline_spec.parallelism_spec,
            "resource_requests": pipeline_spec.resource_requests,
            "job_timeout": pipeline_spec.job_timeout.map(|timeout| timeout.as_secs()),
            "node_selector": pipeline_spec.node_selector,
            "input": pipeline_spec.input,
            "egress": pipeline_spec.egress,
        }),
        job_name,
        command: pipeline_spec.transform.cmd.clone(),
        egress_uri: pipeline_spec.egress.uri.clone(),
    };

    // Get our datums and input files.
    let (new_datums, new_input_files) = input_to_datums(
        &pipeline_spec.transform.secrets,
        job_id,
        &pipeline_spec.input,
    )?;

    // Insert everthing into the database.
    let job = conn.transaction(|| -> Result<Job> {
        let job = new_job.insert(conn)?;
        NewDatum::insert_all(&new_datums, conn)?;
        NewInputFile::insert_all(&new_input_files, conn)?;
        Ok(job)
    })?;

    // Launch our batch job on the cluster.
    start_batch_job(pipeline_spec, &job)?;
    Ok(job)
}

/// The `job retry` subcommand.
pub fn retry_job(job: &Job, conn: &PgConnection) -> Result<Job> {
    let (pipeline_spec, new_job) = conn.transaction(|| -> Result<_> {
        // Load the original job, failed datums, and input files.
        if job.status != Status::Error {
            return Err(format_err!("can only retry jobs with status 'error'"));
        }
        let error_datums = job.datums_with_status(Status::Error, conn)?;
        let input_files = InputFile::for_datums(&error_datums, conn)?;

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
            id: Uuid::new_v4(),
            pipeline_spec: job.pipeline_spec.clone(),
            job_name,
            command: job.command.clone(),
            egress_uri: job.egress_uri.clone(),
        }
        .insert(conn)?;

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
                    datum_id,
                    uri: input_file.uri.clone(),
                    local_path: input_file.local_path.clone(),
                    job_id: new_job.id,
                });
            }
        }
        NewDatum::insert_all(&new_datums, conn)?;
        NewInputFile::insert_all(&new_input_files, conn)?;

        Ok((pipeline_spec, new_job))
    })?;

    // Start a new batch job.
    start_batch_job(&pipeline_spec, &new_job)?;
    Ok(new_job)
}

/// Generate a unique name for our job. To keep Kubernetes happy, this
/// must be a legal DNS name component (but we have a database constraint
/// to enforce that).
pub fn unique_kubernetes_job_name(pipeline_name: &str) -> String {
    let tag = kubernetes::resource_tag();
    format!("{}-{}", pipeline_name, tag)
        .replace("_", "-")
        .to_lowercase()
}

/// The manifest to use to run a job.
const RUN_MANIFEST_TEMPLATE: &str = include_str!("job_manifest.yml.hbs");

/// Parameters used to render `MANIFEST_TEMPLATE`.
#[derive(Serialize)]
struct JobParams<'a> {
    pipeline_spec: &'a PipelineSpec,
    job: &'a Job,
}

/// Start a new batch job running.
pub fn start_batch_job(pipeline_spec: &PipelineSpec, job: &Job) -> Result<()> {
    debug!("starting batch job on cluster");

    // Set up our template parameters, rendder our template, and deploy it.
    let params = JobParams { pipeline_spec, job };
    let manifest = render_manifest(RUN_MANIFEST_TEMPLATE, &params)
        .context("error rendering job template")?;
    kubernetes::deploy(&manifest)?;

    Ok(())
}

#[test]
fn render_template() {
    use serde_json;
    use serde_yaml;

    let json = include_str!("../../falconeri_common/src/example_pipeline_spec.json");
    let pipeline_spec: PipelineSpec = serde_json::from_str(json).expect("parse error");

    let job = Job::factory();
    let params = JobParams {
        pipeline_spec: &pipeline_spec,
        job: &job,
    };

    let manifest = render_manifest(RUN_MANIFEST_TEMPLATE, &params)
        .expect("error rendering job template");
    print!("{}", manifest);
    let _parsed: serde_json::Value =
        serde_yaml::from_str(&manifest).expect("rendered invalid YAML");
}
