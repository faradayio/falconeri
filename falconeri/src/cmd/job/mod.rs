//! The `job` subcommand.

use failure::ResultExt;
use falconeri_common::Result;
use serde_json;
use std::{fs::File, path::PathBuf};

use pipeline::PipelineSpec;

mod ls;
mod run;

/// The `job` subcommand.
#[derive(Debug, StructOpt)]
pub enum Opt {
    /// List all jobs.
    #[structopt(name = "ls")]
    Ls,

    /// Run the specified pipeline as a one-off job.
    #[structopt(name = "run")]
    Run {
        /// Path to a JSON pipeline spec.
        #[structopt(parse(from_os_str))]
        pipeline_json: PathBuf,
    },
}

/// Run the `job` subcommand.
pub fn run(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Ls { } => ls::run(),
        Opt::Run { ref pipeline_json } => {
            let f = File::open(pipeline_json)
                .context("can't open pipeline JSON file")?;
            let pipeline_spec: PipelineSpec =
                serde_json::from_reader(f)
                .context("can't parse pipeline JSON file")?;
            run::run(&pipeline_spec)
        }
    }
}
