#![feature(proc_macro_hygiene, decl_macro)]

// Needed for static linking to work right on Linux.
extern crate openssl;

// Include all of Rocket's macros.
#[macro_use]
extern crate rocket;

use env_logger;
use falconeri_common::{
    falconeri_common_version,
    prelude::*,
    rest_api::{
        DatumPatch, DatumReservationRequest, DatumReservationResponse, OutputFilePatch,
    },
};
use openssl_probe;
use rocket::http::Status as HttpStatus;
use rocket_contrib::json::Json;

mod util;

use util::{DbConn, User, UuidParam};

/// Return our `falconeri_common` version, which should match the client
/// exactly (for now).
#[get("/version")]
fn version() -> String {
    falconeri_common_version().to_string()
}

/// Look up a job and return it as JSON.
#[get("/jobs/<job_id>")]
fn job(_user: User, conn: DbConn, job_id: UuidParam) -> Result<Json<Job>> {
    let job = Job::find(job_id.into_inner(), &conn)?;
    Ok(Json(job))
}

/// Reserve the next available datum for a job, and return it along with a list
/// of input files.
#[post("/jobs/<job_id>/reserve_next_datum", data = "<request>")]
fn job_reserve_next_datum(
    _user: User,
    conn: DbConn,
    job_id: UuidParam,
    request: Json<DatumReservationRequest>,
) -> Result<Json<Option<DatumReservationResponse>>> {
    let job = Job::find(job_id.into_inner(), &conn)?;
    let reserved =
        job.reserve_next_datum(&request.node_name, &request.pod_name, &conn)?;
    if let Some((datum, input_files)) = reserved {
        Ok(Json(Some(DatumReservationResponse { datum, input_files })))
    } else {
        Ok(Json(None))
    }
}

/// Update a datum when it's done.
#[patch("/datums/<datum_id>", data = "<patch>")]
fn patch_datum(
    _user: User,
    conn: DbConn,
    datum_id: UuidParam,
    patch: Json<DatumPatch>,
) -> Result<Json<Datum>> {
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
    _user: User,
    conn: DbConn,
    new_output_files: Json<Vec<NewOutputFile>>,
) -> Result<Json<Vec<OutputFile>>> {
    let created = NewOutputFile::insert_all(&new_output_files, &conn)?;
    Ok(Json(created))
}

/// Update a batch of output files.
#[patch("/output_files", data = "<output_file_patches>")]
fn patch_output_files(
    _user: User,
    conn: DbConn,
    output_file_patches: Json<Vec<OutputFilePatch>>,
) -> Result<HttpStatus> {
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
    env_logger::init();
    openssl_probe::init_ssl_cert_env_vars();

    rocket::ignite()
        // Attach our custom connection pool.
        .attach(DbConn::fairing())
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
