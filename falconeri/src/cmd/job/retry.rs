//! The `job retry` subcommand.

use falconeri_common::{prelude::*, rest_api::Client};

/// The `job retry` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    let mut client = Client::new(ConnectVia::Proxy)?;
    let job = client.find_job_by_name(job_name)?;
    // TODO: We need to create a new client here because we don't have HTTP
    // keepalive set up, and we don't have a good way to run `retry_job`
    // idempotently yet.
    client = Client::new(ConnectVia::Proxy)?;
    let new_job = client.retry_job(&job)?;
    println!("{}", new_job.job_name);
    Ok(())
}
