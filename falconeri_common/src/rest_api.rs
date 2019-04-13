//! The REST API for `falconerid`, including data types and a client.

use reqwest;
use url::Url;

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

/// A client for talking to `falconerid`.
pub struct Client {
    via: ConnectVia,
    url: Url,
    client: reqwest::Client,
}

impl Client {
    /// Create a new client, connecting to `falconerid` as specified.
    pub fn new(via: ConnectVia) -> Result<Client> {
        // Choose an appropriate URL.
        let url = match via {
            ConnectVia::Cluster => "http://falconerid/",
            ConnectVia::Proxy => "http://localhost:8089/",
        }
        .parse()
        .expect("could not parse URL in source code");

        // Create our HTTP client.
        let client = reqwest::Client::builder()
            .build()
            .context("cannot build HTTP client")?;

        Ok(Client { via, url, client })
    }

    /// Fetch a job by ID.
    pub fn job(&self, id: Uuid) -> Result<Job> {
        let url = self.url.join(&format!("jobs/{}", id))?;
        self.via.retry_if_appropriate(|| {
            let mut resp = self
                .client
                .get(url.clone())
                .send()
                .with_context(|_| format!("error getting {}", url))?;
            if resp.status().is_success() {
                let job = resp
                    .json()
                    .with_context(|_| format!("error parsing {}", url))?;
                Ok(job)
            } else {
                Err(format_err!("unexpected HTTP status {}", resp.status()))
            }
        })
    }

    // POST /jobs/<job_id>/reserve_next_datum
    // PATCH /datums/<datum_id>
    // POST /output_files
    // PATCH /output_files
}
