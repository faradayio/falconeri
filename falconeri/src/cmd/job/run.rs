//! The `job run` subcommand.

use failure::ResultExt;
use falconeri_common::{db, diesel::prelude::*, Error, kubernetes, models::*, Result, storage::CloudStorage};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use std::iter;

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

            // Figure out what files to process, add our job to the database,
            // and launch our batch job on the cluster.
            let storage = CloudStorage::for_uri(&uri)?;
            let paths = storage.list(uri)?;
            let job = add_job_to_database(pipeline_spec, &paths, repo)?;
            start_batch_job(pipeline_spec, &job)?;
            println!("{}", job.job_name);

            Ok(())
        }
    }
}

/// Register a new job in the database.
fn add_job_to_database(
    pipeline_spec: &PipelineSpec,
    inputs: &[String],
    repo: &str,
) -> Result<Job> {
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let job = conn.transaction::<_, Error, _>(|| -> Result<Job> {
        // Generate a unique name for our job. To keep Kubernetes happy, this
        // must be a legal DNS name component (but we have a database constraint
        // to enforce that).
        let mut rng = thread_rng();
        let tag = iter::repeat(())
            // Note that this random distribution is biased, because we generate
            // both upper and lowercase letters and then convert to lowercase
            // later. This isn't a big deal for now.
            .map(|()| rng.sample(Alphanumeric))
            .take(5)
            .collect::<String>();
        let job_name = format!("{}-{}", pipeline_spec.pipeline.name, tag)
            .replace("_", "-")
            .to_lowercase();

        // Create our new job.
        let new_job = NewJob {
            pipeline_spec: json!(pipeline_spec),
            job_name: job_name,
            command: pipeline_spec.transform.cmd.clone(),
            egress_uri: pipeline_spec.egress.uri.clone(),
        };
        let job = new_job.insert(&conn)?;

        // Create a datum for each input file. For now, we only handle the
        // trivial case of one file per datum.
        for input in inputs {
            let new_datum = NewDatum {
                job_id: job.id,
            };
            let datum = new_datum.insert(&conn)?;

            let new_file = NewInputFile {
                datum_id: datum.id,
                uri: input.to_owned(),
                local_path: uri_to_local_path(input, repo)?,
            };
            let _file = new_file.insert(&conn)?;
        }
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
    let pos = uri.rfind('/').ok_or_else(|| format_err!("No '/' in {:?}", uri))?;
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
const RUN_MANIFEST_TEMPLATE: &str = include_str!("job_manifest.yml");

/// Start a new batch job running.
fn start_batch_job(
    pipeline_spec: &PipelineSpec,
    job: &Job,
) -> Result<()> {
    debug!("starting batch job on cluster");

    // Set up our template parameters.
    #[derive(Serialize)]
    struct JobParams<'a> {
        pipeline_spec: &'a PipelineSpec,
        job: &'a Job,
    }
    let params = JobParams { pipeline_spec, job };

    // Render our template and deploy it.
    let manifest = render_manifest(RUN_MANIFEST_TEMPLATE, &params)
        .context("error rendering job template")?;
    kubernetes::deploy(&manifest)?;

    Ok(())
}
