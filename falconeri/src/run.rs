//! The `run` subcommand.

use failure::ResultExt;
use falconeri_common::{db, diesel::prelude::*, Error, models::*, Result};
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

            add_job_to_database(pipeline_spec, &paths, repo)?;

            Ok(())
        }
    }
}

/// Register a new job in the database.
fn add_job_to_database(
    pipeline_spec: &PipelineSpec,
    inputs: &[String],
    repo: &str,
) -> Result<()> {
    let conn = db::connect()?;
    conn.transaction::<_, Error, _>(|| -> Result<()> {
        // Create our new job.
        let new_job = NewJob {
            pipeline_spec: json!(pipeline_spec),
            command: pipeline_spec.transform.cmd.clone(),
            output_uri: pipeline_spec.output.uri.clone(),
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
        Ok(())
    })?;

    Ok(())
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
