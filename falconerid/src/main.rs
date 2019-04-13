#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use falconeri_common::{db, falconeri_common_version, prelude::*};
use rocket::http::Status as HttpStatus;
use rocket_contrib::json::Json;

mod util;

use util::UuidParam;

/// Return our `falconeri_common` version, which should match the client
/// exactly (for now).
#[get("/version")]
fn version() -> String {
    falconeri_common_version().to_string()
}

/// Look up a job and return it as JSON.
#[get("/jobs/<job_id>")]
fn job(job_id: UuidParam) -> Result<Json<Job>> {
    let conn = db::connect(db::ConnectVia::Cluster)?;
    let job = Job::find(job_id.into_inner(), &conn)?;
    Ok(Json(job))
}

/// Request the reservation of a datum.
#[derive(Debug, Deserialize)]
struct ReservationRequest {
    node_name: String,
    pod_name: String,
}

/// Information about a reserved datum.
#[derive(Debug, Serialize)]
struct ReservationResponse {
    datum: Datum,
    input_files: Vec<InputFile>,
}

/// Reserve the next available datum for a job, and return it along with a list
/// of input files.
#[post("/jobs/<job_id>/reserve_next_datum", data = "<request>")]
fn job_reserve_next_datum(
    job_id: UuidParam,
    request: Json<ReservationRequest>,
) -> Result<Json<Option<ReservationResponse>>> {
    let conn = db::connect(db::ConnectVia::Cluster)?;
    let job = Job::find(job_id.into_inner(), &conn)?;
    let reserved =
        job.reserve_next_datum(&request.node_name, &request.pod_name, &conn)?;
    if let Some((datum, input_files)) = reserved {
        Ok(Json(Some(ReservationResponse { datum, input_files })))
    } else {
        Ok(Json(None))
    }
}

/// Information about a datum that we can update.
#[derive(Debug, Deserialize)]
struct DatumPatch {
    status: Status,
    output: String,
    error_message: Option<String>,
    backtrace: Option<String>,
}

/// Update a datum when it's done.
#[patch("/datums/<datum_id>", data = "<patch>")]
fn patch_datum(datum_id: UuidParam, patch: Json<DatumPatch>) -> Result<Json<Datum>> {
    let conn = db::connect(db::ConnectVia::Cluster)?;
    let mut datum = Datum::find(datum_id.into_inner(), &conn)?;

    // We only support a few very specific types of patches.
    match &patch.into_inner() {
        // Set status to `Status::Done`.
        DatumPatch {
            status: Status::Done,
            output,
            error_message: None,
            backtrace: None,
        } => {
            datum.mark_as_done(output, &conn)?;
        }

        // Set status to `Status::Error`.
        DatumPatch {
            status: Status::Error,
            output,
            error_message: Some(error_message),
            backtrace: Some(backtrace),
        } => {
            datum.mark_as_error(output, error_message, backtrace, &conn)?;
        }

        // All other combinations are forbidden.
        other => return Err(format_err!("cannot update datum with {:?}", other)),
    }

    // If there are no more datums, mark the job as finished (either done or
    // error).
    let mut job = Job::find(datum.job_id, &conn)?;
    job.update_status_if_done(&conn)?;

    Ok(Json(datum))
}

/// Create a batch of output files.
///
/// TODO: These include `job_id` and `datum_id` values that might be nicer to
/// move to our URL at some point.
#[post("/output_files", data = "<new_output_files>")]
fn create_output_files(
    new_output_files: Json<Vec<NewOutputFile>>,
) -> Result<Json<Vec<OutputFile>>> {
    let conn = db::connect(db::ConnectVia::Cluster)?;
    let created = NewOutputFile::insert_all(&new_output_files, &conn)?;
    Ok(Json(created))
}

/// Information about an output file that we can update.
#[derive(Debug, Deserialize)]
struct OutputFilePatch {
    id: Uuid,
    status: Status,
}

/// Update a batch of output files.
#[patch("/output_files", data = "<output_file_patches>")]
fn patch_output_files(
    output_file_patches: Json<Vec<OutputFilePatch>>,
) -> Result<HttpStatus> {
    let conn = db::connect(db::ConnectVia::Cluster)?;

    // Separate patches by status.
    let mut done_ids = vec![];
    let mut error_ids = vec![];
    for patch in output_file_patches.into_inner() {
        match patch.status {
            Status::Done => done_ids.push(patch.id),
            Status::Error => error_ids.push(patch.id),
            _ => {
                return Err(format_err!("cannot patch output file with {:?}", patch));
            }
        }
    }

    // Apply our updates.
    conn.transaction(|| -> Result<()> {
        OutputFile::mark_ids_as_done(&done_ids, &conn)?;
        OutputFile::mark_ids_as_error(&error_ids, &conn)?;
        Ok(())
    })?;

    Ok(HttpStatus::NoContent)
}

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![
                version,
                job,
                job_reserve_next_datum,
                patch_datum,
                create_output_files,
                patch_output_files,
            ],
        )
        .launch();
}
