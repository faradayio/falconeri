//! The `run` subcommand.

use failure::ResultExt;
use falconeri_common::{db, diesel::prelude::*, Error, kubernetes, models::*, Result};
use handlebars::Handlebars;
use std::{io::BufRead, process::{Command, Stdio}};

use pipeline::*;

/// The `run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    match &pipeline_spec.input {
        InputInfo::Atom { uri, repo, glob } => {
            // Check to make sure we're using a supported glob mode.
            if glob != "/*" {
                return Err(format_err!("Glob {} not yet supported", glob));
            }

            // Shell out to gsutil to list the files we want to process.
            let output = Command::new("gsutil")
                .arg("ls")
                .arg(&uri)
                .stderr(Stdio::inherit())
                .output()
                .context("error running gsutil")?;
            let mut paths = vec![];
            for line in output.stdout.lines() {
                let line = line?;
                paths.push(line.trim_right().to_owned());
            }

            let job = add_job_to_database(pipeline_spec, &paths, repo)?;
            start_batch_job(pipeline_spec, &job)?;

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
        // Create our new job.
        let new_job = NewJob {
            pipeline_spec: json!(pipeline_spec),
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

        println!("{}", job.id);
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
const RUN_MANIFEST_TEMPLATE: &str = include_str!("run_manifest.yml");

/// Start a new batch job running.
fn start_batch_job(
    pipeline_spec: &PipelineSpec,
    job: &Job,
) -> Result<()> {
    debug!("starting batch job on cluster");

    // Set up handlebars.
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    // TODO: Fix escaping as per http://yaml.org/spec/1.2/spec.html#id2776092.
    //handlebars.register_escape_fn(...)

    // Set up our template parameters.
    #[derive(Serialize)]
    struct JobParams {
        name: String,
        parallelism: u32,
        image: String,
        job_id: String,
    }
    let params = JobParams {
        // Make sure our name is DNS-legal for Kubernetes.
        //
        // TODO: Add a unique identifier to job name. Ideally this should
        // be stored in the `jobs` table when create the `Job`.
        name: pipeline_spec.pipeline.name.replace("_", "-"),
        parallelism: pipeline_spec.parallelism_spec.constant,
        image: pipeline_spec.transform.image.clone(),
        job_id: job.id.to_string(),
    };

    // Render our template and deploy it.
    let manifest = handlebars.render_template(RUN_MANIFEST_TEMPLATE, &params)
        .context("error rendering job template")?;
    kubernetes::deploy(&manifest)?;

    Ok(())
}
