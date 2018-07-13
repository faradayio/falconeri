//! The `job ls` subcommand.

use falconeri_common::{db, models::*, Result};
use prettytable::{format::consts::FORMAT_CLEAN, Table};

/// The `job ls` subcommand.
pub fn run() -> Result<()> {
    // Look up the information to display.
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let jobs = Job::list(&conn)?;

    // Create a new table. This library makes some rather unusual API choices,
    // but it does the job well enough.
    let mut table = Table::new();
    table.set_format(*FORMAT_CLEAN);
    table.add_row(row!["JOB_ID", "STATUS"]);

    // Print information about each job.
    for job in jobs {
        table.add_row(row![&job.job_name, job.status]);
    }

    table.printstd();
    Ok(())
}
