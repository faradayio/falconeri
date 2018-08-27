//! The `job run` subcommand.

use falconeri_common::{db, kubernetes, prefix::*, storage::CloudStorage};

use manifest::render_manifest;
use pipeline::*;

/// The `job run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    match &pipeline_spec.input {
        Input::Atom { uri, repo, glob } => {
            // Check to make sure we're using a supported glob mode.
            if glob != "/*" {
                return Err(format_err!("Glob {} not yet supported", glob));
            }

            // Figure out what files to process
            let storage = CloudStorage::for_uri(&uri, &pipeline_spec.transform.secrets)?;
            let paths = storage.list(uri)?;

            // Make sure we have no nested directories, which we don't handle
            // correctly for "/*" yet.
            let mut base = uri.to_owned();
            if !base.ends_with("/") {
                base.push_str("/");
            }
            for path in &paths {
                // These assertions should always be true because of how
                // `storage.list` is supposed to work.
                assert!(path.len() > base.len());
                assert_eq!(path[..base.len()], base);

                // Strip base and look for a remaining '/'.
                if path[base.len()..].find('/').is_some() {
                    return Err(format_err!(
                        "we cannot handle directory inputs yet: {:?}",
                        path,
                    ));
                }
            }

            // Add our job to the database, and launch our batch job on the
            // cluster.
            let job = add_job_to_database(pipeline_spec, &paths, repo)?;
            start_batch_job(pipeline_spec, &job)?;
            println!("{}", job.job_name);

            Ok(())
        }
    }
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

/// Register a new job in the database.
fn add_job_to_database(
    pipeline_spec: &PipelineSpec,
    inputs: &[String],
    repo: &str,
) -> Result<Job> {
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let job = conn.transaction(|| -> Result<Job> {
        // Create our new job.
        let job_name = unique_kubernetes_job_name(&pipeline_spec.pipeline.name);
        let new_job = NewJob {
            pipeline_spec: json!(pipeline_spec),
            job_name: job_name,
            command: pipeline_spec.transform.cmd.clone(),
            egress_uri: pipeline_spec.egress.uri.clone(),
        };
        let job = new_job.insert(&conn)?;

        // Create a datum for each input file. For now, we only handle the
        // trivial case of one file per datum.
        let mut datums = vec![];
        let mut input_files = vec![];
        for input in inputs {
            let datum_id = Uuid::new_v4();
            datums.push(NewDatum { id: datum_id, job_id: job.id });

            input_files.push(NewInputFile {
                datum_id,
                uri: input.to_owned(),
                local_path: uri_to_local_path(input, repo)?,
            });
        }
        NewDatum::insert_all(&datums, &conn)?;
        NewInputFile::insert_all(&input_files, &conn)?;
        Ok(job)
    })?;
    Ok(job)
}

/// Given a URI and a repo name, construct a local path starting with "/pfs"
/// pointing to where we should download the file.
///
/// TODO: This will need to get fancier if we actually implement globs
/// correctly.
fn uri_to_local_path(uri: &str, repo: &str) -> Result<String> {
    let pos = uri.rfind('/')
        .ok_or_else(|| format_err!("No '/' in {:?}", uri))?;
    let basename = &uri[pos..];
    if basename.is_empty() {
        Err(format_err!("{:?} ends with '/'", uri))
    } else {
        Ok(format!("/pfs/{}{}", repo, basename))
    }
}

#[test]
fn uri_to_local_path_works() {
    let path = uri_to_local_path("gs://bucket/path/data1.csv", "myrepo").unwrap();
    assert_eq!(path, "/pfs/myrepo/data1.csv");
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
    let pipeline_spec: PipelineSpec = serde_json::from_str(json)
        .expect("parse error");

    let job = Job::factory();
    let params = JobParams {
        pipeline_spec: &pipeline_spec,
        job: &job,
    };

    let manifest = render_manifest(RUN_MANIFEST_TEMPLATE, &params)
        .expect("error rendering job template");
    print!("{}", manifest);
    let _parsed: serde_json::Value = serde_yaml::from_str(&manifest)
        .expect("rendered invalid YAML");
}
