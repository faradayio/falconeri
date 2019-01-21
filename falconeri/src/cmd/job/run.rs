//! The `job run` subcommand.

use falconeri_common::{db, kubernetes, prefix::*};
use serde_json::json;

use crate::inputs::input_to_datums;
use crate::manifest::render_manifest;
use crate::pipeline::*;

/// The `job run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    // Build our job.
    let job_id = Uuid::new_v4();
    let job_name = unique_kubernetes_job_name(&pipeline_spec.pipeline.name);
    let new_job = NewJob {
        id: job_id,
        pipeline_spec: json!(pipeline_spec),
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
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let job = conn.transaction(|| -> Result<Job> {
        let job = new_job.insert(&conn)?;
        NewDatum::insert_all(&new_datums, &conn)?;
        NewInputFile::insert_all(&new_input_files, &conn)?;
        Ok(job)
    })?;

    // Launch our batch job on the cluster.
    start_batch_job(pipeline_spec, &job)?;
    println!("{}", job.job_name);
    Ok(())
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

    let json = include_str!("../../example_pipeline_spec.json");
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
