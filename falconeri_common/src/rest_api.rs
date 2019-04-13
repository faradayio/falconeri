//! Types used in `falconerid`'s REST API.

use crate::prelude::*;

/// Request the reservation of a datum.
#[derive(Debug, Deserialize)]
pub struct DatumReservationRequest {
    /// The Kubernetes node name which will process this datum.
    pub node_name: String,
    /// The Kubernetes pod name which will process this datum.
    pub pod_name: String,
}

/// Information about a reserved datum.
#[derive(Debug, Serialize)]
pub struct DatumReservationResponse {
    /// The reserved datum to process.
    pub datum: Datum,
    /// The input files associated with this datum.
    pub input_files: Vec<InputFile>,
}

/// Information about a datum that we can update.
#[derive(Debug, Deserialize)]
pub struct DatumPatch {
    /// The new status for the datum. Must be either `Status::Done` or
    /// `Status::Error`.
    pub status: Status,
    /// The output of procesisng the datum.
    pub output: String,
    /// If and only if `status` is `Status::Error`, this should be the error
    /// message.
    pub error_message: Option<String>,
    /// If and only if `status` is `Status::Error`, this should be the error
    /// backtrace.
    pub backtrace: Option<String>,
}

/// Information about an output file that we can update.
#[derive(Debug, Deserialize)]
pub struct OutputFilePatch {
    /// The ID of the output file to update.
    pub id: Uuid,
    /// The status of the output file. Must be either `Status::Done` or
    /// `Status::Error`.
    pub status: Status,
}
