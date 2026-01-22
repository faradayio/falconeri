//! The `job` subcommand.

use clap::Parser;
use falconeri_common::{pipeline::PipelineSpec, prelude::*};
use serde_json;

mod describe;
mod list;
mod retry;
mod run;
// Disabled because it's broken by recurive `"input"` types.
//
// mod schema;
mod wait;

/// The `job` subcommand.
#[derive(Debug, Parser)]
pub enum Opt {
    /// Describe a specific job.
    #[command(name = "describe")]
    Describe {
        /// The Kubernetes name of the job to describe.
        job_name: String,
    },

    /// List all jobs.
    #[command(name = "list")]
    List,

    /// Retry failed datums.
    #[command(name = "retry")]
    Retry {
        /// The name of the job for which to retry failed datums.
        job_name: String,
    },

    /// Run the specified pipeline as a one-off job.
    #[command(name = "run")]
    Run {
        /// Path to a JSON pipeline spec.
        pipeline_json: PathBuf,
    },
    // Disabled because `BsonSchema` doesn't handle recursive types.
    //
    // /// Output a JSON schema for a falconeri job.
    // #[command(name = "schema")]
    // Schema,
    /// Wait for the specified job to finish, either successfully or with an
    /// error.
    #[command(name = "wait")]
    Wait {
        /// The name of the job to wait for.
        job_name: String,
    },
}

/// Run the `job` subcommand.
pub fn run(opt: &Opt) -> Result<()> {
    match opt {
        Opt::Describe { job_name } => describe::run(job_name),
        Opt::List => list::run(),
        Opt::Retry { job_name } => retry::run(job_name),
        Opt::Run { pipeline_json } => {
            let f =
                File::open(pipeline_json).context("can't open pipeline JSON file")?;
            let pipeline_spec: PipelineSpec = serde_json::from_reader(f)
                .context("can't parse pipeline JSON file")?;
            run::run(&pipeline_spec)
        }
        // Disabled because it's broken by recurive `"input"` types.
        //
        // Opt::Schema => schema::run(),
        Opt::Wait { job_name } => wait::run(job_name),
    }
}
