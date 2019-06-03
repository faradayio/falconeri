//! The `job wait` subcommand.

use falconeri_common::{prelude::*, rest_api::Client};
use std::{thread::sleep, time::Duration};

/// The `job wait` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    let client = Client::new(ConnectVia::Proxy)?;
    let mut job = client.find_job_by_name(job_name)?;
    while !job.status.has_finished() {
        sleep(Duration::from_secs(30));
        job = client.job(job.id)?;
    }
    println!("{}", job.status);
    Ok(())
}
