//! The `job describe` subcommand.

use falconeri_common::{db, models::*, Result};

use description::print_description;

/// Template for human-readable `describe` output.
const DESCRIBE_TEMPLATE: &str = include_str!("describe.txt.hbs");

/// The `job describe` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    // Load the data we want to display.
    let conn = db::connect(db::ConnectVia::Proxy)?;
    let job = Job::find_by_job_name(job_name, &conn)?;
    let datum_status_counts = job.datum_status_counts(&conn)?;
    let running_datums = job.datums_with_status(Status::Running, &conn)?;
    let error_datums = job.datums_with_status(Status::Error, &conn)?;

    // Convert it into a serializable object.
    #[derive(Serialize)]
    struct Params {
        job: Job,
        datum_status_counts: Vec<(Status, u64)>,
        running_datums: Vec<Datum>,
        error_datums: Vec<Datum>,
    }
    let params = Params { job, datum_status_counts, running_datums, error_datums };

    // Print the description.
    print_description(DESCRIBE_TEMPLATE, &params)
}

