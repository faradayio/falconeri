//! The `job retry` subcommand.

use falconeri_common::{prelude::*, rest_api::Client};

/// The `job retry` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    let client = Client::new(ConnectVia::Proxy)?;
    let job = client.find_job_by_name(job_name)?;
    let client2 = Client::new(ConnectVia::Proxy)?;
    let new_job = client2.retry_job(&job)?;
    println!("{}", new_job.job_name);
    Ok(())
}
