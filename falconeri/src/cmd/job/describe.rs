//! The `job describe` subcommand.

use falconeri_common::{db, prelude::*};

use crate::description::render_description;

/// Template for human-readable `describe` output.
const DESCRIBE_TEMPLATE: &str = include_str!("describe.txt.hbs");

// Convert it into a serializable object.
#[derive(Serialize)]
struct Params {
    job: Job,
    datum_status_counts: Vec<DatumStatusCount>,
    running_datums: Vec<Datum>,
    error_datums: Vec<Datum>,
}

/// The `job describe` subcommand.
pub fn run(job_name: &str) -> Result<()> {
    // Load the data we want to display.
    let mut conn = db::connect(ConnectVia::Proxy)?;
    let job = Job::find_by_job_name(job_name, &mut conn)?;
    let datum_status_counts = job.datum_status_counts(&mut conn)?;
    let running_datums = job.datums_with_status(Status::Running, &mut conn)?;
    let error_datums = job.datums_with_status(Status::Error, &mut conn)?;
    let params = Params {
        job,
        datum_status_counts,
        running_datums,
        error_datums,
    };

    // Print the description.
    print!("{}", render_description(DESCRIBE_TEMPLATE, &params)?);
    Ok(())
}

#[test]
fn render_template() {
    let job = Job::factory();
    let dsc = |status: Status, count: u64, rerunable_count: u64| DatumStatusCount {
        status,
        count,
        rerunable_count,
    };
    let datum_status_counts = vec![
        dsc(Status::Ready, 1, 0),
        dsc(Status::Running, 1, 0),
        dsc(Status::Error, 2, 1),
    ];
    let mut running_datum = Datum::factory(&job);
    running_datum.status = Status::Running;
    let running_datums = vec![running_datum];
    let mut error_datum = Datum::factory(&job);
    error_datum.status = Status::Error;
    error_datum.error_message = Some("Ooops.".to_owned());
    let error_datums = vec![error_datum];
    let params = Params {
        job,
        datum_status_counts,
        running_datums,
        error_datums,
    };

    render_description(DESCRIBE_TEMPLATE, &params).expect("could not render template");
}
