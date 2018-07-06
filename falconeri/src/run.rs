//! The `run` subcommand.

use failure::ResultExt;
use falconeri_common::Result;
use std::{io::BufRead, process::{Command, Stdio}};

use pipeline::*;

/// The `run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    match &pipeline_spec.input {
        InputInfo::Atom { repo, glob: _ } => {
            let output = Command::new("gsutil")
                .arg("ls")
                .arg(&repo)
                .stderr(Stdio::inherit())
                .output()
                .context("error running gsutil")?;
            for line in output.stdout.lines() {
                let line = line?;
                let path = line.trim_right();
                println!("{}", path);
            }
            Ok(())
        }
    }
}
