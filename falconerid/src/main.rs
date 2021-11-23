// Needed for static linking to work right on Linux.
extern crate openssl_sys;

use falconeri_common::{
    db, falconeri_common_version,
    pipeline::PipelineSpec,
    prelude::*,
    rest_api::{
        DatumPatch, DatumReservationRequest, DatumReservationResponse, OutputFilePatch,
    },
    tracing_support::initialize_tracing,
};
use rocket::{
    get, http::Status as HttpStatus, launch, patch, post, routes, serde::json::Json,
    Config,
};
use std::{env, process::exit};

mod babysitter;
pub(crate) mod inputs;
mod start_job;
mod util;

use crate::babysitter::start_babysitter;
use crate::start_job::{retry_job, run_job};
use crate::util::{DbConn, FalconeridResult, User};

/// initialize the server at startup.
fn initialize_server() -> Result<()> {
    // Print our some information about our environment.
    eprintln!("Running in {}", env::current_dir()?.display());
    let config = Config::figment().extract::<Config>()?;
    eprintln!("Will listen on {}:{}.", config.address, config.port);

    // Initialize the database.
    eprintln!("Connecting to database.");
    let conn = db::connect(ConnectVia::Cluster)?;
    eprintln!("Running any pending migrations.");
    db::run_pending_migrations(&conn)?;
    eprintln!("Finished migrations.");

    eprintln!("Starting babysitter thread to monitor jobs.");
    start_babysitter()?;
    eprintln!("Babysitter started.");

    Ok(())
}

/// Return our `falconeri_common` version, which should match the client
/// exactly (for now).
#[get("/version")]
fn version() -> String {
    falconeri_common_version().to_string()
}

/// Create a new job from a JSON pipeline spec.
#[post("/jobs", data = "<pipeline_spec>")]
fn post_job(
    _user: User,
    conn: DbConn,
    pipeline_spec: Json<PipelineSpec>,
) -> FalconeridResult<Json<Job>> {
    Ok(Json(run_job(&pipeline_spec, &conn)?))
}

/// Look up a job and return it as JSON.
#[get("/jobs?<job_name>")]
fn get_job_by_name(
    _user: User,
    conn: DbConn,
    job_name: String,
) -> FalconeridResult<Json<Job>> {
    let job = Job::find_by_job_name(&job_name, &conn)?;
    Ok(Json(job))
}

/// Look up a job and return it as JSON.
#[get("/jobs/<job_id>")]
fn get_job(_user: User, conn: DbConn, job_id: Uuid) -> FalconeridResult<Json<Job>> {
    let job = Job::find(job_id, &conn)?;
    Ok(Json(job))
}

/// Retry a job, and return the new job as JSON.
#[post("/jobs/<job_id>/retry")]
fn job_retry(_user: User, conn: DbConn, job_id: Uuid) -> FalconeridResult<Json<Job>> {
    let job = Job::find(job_id, &conn)?;
    Ok(Json(retry_job(&job, &conn)?))
}

/// Reserve the next available datum for a job, and return it along with a list
/// of input files.
#[post("/jobs/<job_id>/reserve_next_datum", data = "<request>")]
fn job_reserve_next_datum(
    _user: User,
    conn: DbConn,
    job_id: Uuid,
    request: Json<DatumReservationRequest>,
) -> FalconeridResult<Json<Option<DatumReservationResponse>>> {
    let job = Job::find(job_id, &conn)?;
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
    datum_id: Uuid,
    patch: Json<DatumPatch>,
) -> FalconeridResult<Json<Datum>> {
    let mut datum = Datum::find(datum_id, &conn)?;

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
        other => {
            return Err(format_err!("cannot update datum with {:?}", other).into())
        }
    }

    // If there are no more datums, mark the job as finished (either done or
    // error).
    datum.update_job_status_if_done(&conn)?;

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
) -> FalconeridResult<Json<Vec<OutputFile>>> {
    let created = NewOutputFile::insert_all(&new_output_files, &conn)?;
    Ok(Json(created))
}

/// Update a batch of output files.
#[patch("/output_files", data = "<output_file_patches>")]
fn patch_output_files(
    _user: User,
    conn: DbConn,
    output_file_patches: Json<Vec<OutputFilePatch>>,
) -> FalconeridResult<HttpStatus> {
    // Separate patches by status.
    let mut done_ids = vec![];
    let mut error_ids = vec![];
    for patch in output_file_patches.into_inner() {
        match patch.status {
            Status::Done => done_ids.push(patch.id),
            Status::Error => error_ids.push(patch.id),
            _ => {
                return Err(
                    format_err!("cannot patch output file with {:?}", patch).into()
                );
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

#[launch]
fn rocket() -> _ {
    initialize_tracing();
    openssl_probe::init_ssl_cert_env_vars();

    if let Err(err) = initialize_server() {
        eprintln!(
            "Failed to initialize server:\n{}",
            err.display_causes_and_backtrace()
        );
        exit(1);
    }

    rocket::build()
        // Attach our custom connection pool.
        .attach(DbConn::fairing())
        // Attach our basic authentication.
        .attach(User::fairing())
        .mount(
            "/",
            routes![
                version,
                post_job,
                get_job,
                get_job_by_name,
                job_reserve_next_datum,
                job_retry,
                patch_datum,
                create_output_files,
                patch_output_files,
            ],
        )
}
