//! The `job run` subcommand.

use falconeri_common::{pipeline::*, prelude::*, rest_api::Client};

/// The `job run` subcommand.
pub fn run(pipeline_spec: &PipelineSpec) -> Result<()> {
    let client = Client::new(ConnectVia::Proxy)?;
    let job = client.new_job(pipeline_spec)?;
    println!("{}", job.job_name);
    Ok(())
}
