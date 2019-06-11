//! The REST API for `falconerid`, including data types and a client.

use reqwest;
use serde::de::DeserializeOwned;
use std::usize;
use url::Url;

use crate::db;
use crate::kubernetes::{node_name, pod_name};
use crate::pipeline::PipelineSpec;
use crate::prelude::*;

/// Request the reservation of a datum.
#[derive(Debug, Deserialize, Serialize)]
pub struct DatumReservationRequest {
    /// The Kubernetes node name which will process this datum.
    pub node_name: String,
    /// The Kubernetes pod name which will process this datum.
    pub pod_name: String,
}

/// Information about a reserved datum.
#[derive(Debug, Deserialize, Serialize)]
pub struct DatumReservationResponse {
    /// The reserved datum to process.
    pub datum: Datum,
    /// The input files associated with this datum.
    pub input_files: Vec<InputFile>,
}

/// Information about a datum that we can update.
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
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
    username: String,
    password: String,
    client: reqwest::Client,
}

impl Client {
    /// Create a new client, connecting to `falconerid` as specified.
    pub fn new(via: ConnectVia) -> Result<Client> {
        // Choose an appropriate URL.
        let url = match via {
            ConnectVia::Cluster => "http://falconerid:8089/",
            ConnectVia::Proxy => "http://localhost:8089/",
        }
        .parse()
        .expect("could not parse URL in source code");

        // Get our credentials. For now, we use our database password for API
        // access, too.
        let username = "falconeri".to_owned();
        let password = db::postgres_password(via)?;

        // Decide how long to keep connections open.
        let max_idle = match via {
            // If we're running on the cluster, connection startup is cheap but
            // we may have hundreds of inbound connections, so drop connections
            // as fast as possible. This could be improved by putting an async
            // proxy server in front of `falconerid`, if we want that.
            ConnectVia::Cluster => 0,
            // Otherwise allow the maximum possible number of connections.
            ConnectVia::Proxy => usize::MAX,
        };

        // Create our HTTP client.
        let client = reqwest::Client::builder()
            .max_idle_per_host(max_idle)
            .build()
            .context("cannot build HTTP client")?;

        Ok(Client {
            via,
            url,
            username,
            password,
            client,
        })
    }

    /// Create a job. This does not automatically retry on network failure,
    /// because it's very expensive and not idempotent (and only called by
    /// `falconeri` and never `falconeri-worker`).
    ///
    /// `POST /jobs`
    pub fn new_job(&self, pipeline_spec: &PipelineSpec) -> Result<Job> {
        let url = self.url.join("jobs")?;
        let resp = self
            .client
            .post(url.clone())
            .basic_auth(&self.username, Some(&self.password))
            .json(pipeline_spec)
            .send()
            .with_context(|_| format!("error posting {}", url))?;
        self.handle_json_response(&url, resp)
    }

    /// Fetch a job by ID.
    ///
    /// `GET /jobs/<job_id>`
    pub fn job(&self, id: Uuid) -> Result<Job> {
        let url = self.url.join(&format!("jobs/{}", id))?;
        self.via.retry_if_appropriate(|| {
            let resp = self
                .client
                .get(url.clone())
                .basic_auth(&self.username, Some(&self.password))
                .send()
                .with_context(|_| format!("error getting {}", url))?;
            self.handle_json_response(&url, resp)
        })
    }

    /// Fetch a job by name.
    ///
    /// `GET /jobs?job_name=$NAME`
    pub fn find_job_by_name(&self, job_name: &str) -> Result<Job> {
        let mut url = self.url.join("jobs")?;
        url.query_pairs_mut()
            .append_pair("job_name", job_name)
            .finish();
        self.via.retry_if_appropriate(|| {
            let resp = self
                .client
                .get(url.clone())
                .basic_auth(&self.username, Some(&self.password))
                .send()
                .with_context(|_| format!("error getting {}", url))?;
            self.handle_json_response(&url, resp)
        })
    }

    /// Retry a job by ID.
    ///
    /// Not idempotent because it's expensive and only called by `falconeri`.
    ///
    /// `POST /jobs/<job_id>/retry`
    pub fn retry_job(&self, job: &Job) -> Result<Job> {
        let url = self.url.join(&format!("job_id/{}/retry", job.id))?;
        let resp = self
            .client
            .post(url.clone())
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .with_context(|_| format!("error posting {}", url))?;
        self.handle_json_response(&url, resp)
    }

    /// Reserve the next available datum to process, and return it along with
    /// the corresponding input files. This can only be called from inside a
    /// pod.
    ///
    /// `POST /jobs/<job_id>/reserve_next_datum`
    pub fn reserve_next_datum(
        &self,
        job: &Job,
    ) -> Result<Option<(Datum, Vec<InputFile>)>> {
        let url = self
            .url
            .join(&format!("jobs/{}/reserve_next_datum", job.id))?;
        let resv_resp: Option<DatumReservationResponse> =
            self.via.retry_if_appropriate(|| {
                let resp = self
                    .client
                    .post(url.clone())
                    .basic_auth(&self.username, Some(&self.password))
                    .json(&DatumReservationRequest {
                        node_name: node_name()?,
                        pod_name: pod_name()?,
                    })
                    .send()
                    .with_context(|_| format!("error posting {}", url))?;
                self.handle_json_response(&url, resp)
            })?;
        Ok(resv_resp.map(|r| (r.datum, r.input_files)))
    }

    /// Mark `datum` as done, and record the output of the commands we ran.
    pub fn mark_datum_as_done(&self, datum: &mut Datum, output: String) -> Result<()> {
        let patch = DatumPatch {
            status: Status::Done,
            output,
            error_message: None,
            backtrace: None,
        };
        self.patch_datum(datum, &patch)
    }

    /// Mark `datum` as having failed, and record the output and error
    /// information.
    pub fn mark_datum_as_error(
        &self,
        datum: &mut Datum,
        output: String,
        error_message: String,
        backtrace: String,
    ) -> Result<()> {
        let patch = DatumPatch {
            status: Status::Error,
            output,
            error_message: Some(error_message),
            backtrace: Some(backtrace),
        };
        self.patch_datum(datum, &patch)
    }

    /// Apply `patch` to `datum`.
    ///
    /// `PATCH /datums/<datum_id>`
    fn patch_datum(&self, datum: &mut Datum, patch: &DatumPatch) -> Result<()> {
        let url = self.url.join(&format!("datums/{}", datum.id))?;
        let updated_datum = self.via.retry_if_appropriate(|| {
            let resp = self
                .client
                .patch(url.clone())
                .basic_auth(&self.username, Some(&self.password))
                .json(patch)
                .send()
                .with_context(|_| format!("error patching {}", url))?;
            self.handle_json_response(&url, resp)
        })?;
        *datum = updated_datum;
        Ok(())
    }

    /// Create new output files.
    ///
    /// `POST /output_files`
    pub fn create_output_files(
        &self,
        files: &[NewOutputFile],
    ) -> Result<Vec<OutputFile>> {
        let url = self.url.join("output_files")?;
        // TODO: We might want finer-grained retry here? This isn't remotely
        // idempotent. Though I suppose if we encounter a "double create", all
        // the retries should just fail until we give up, then we'll eventually
        // fail the datum, allowing it to be retried.
        self.via.retry_if_appropriate(|| {
            let resp = self
                .client
                .post(url.clone())
                .basic_auth(&self.username, Some(&self.password))
                .json(files)
                .send()
                .with_context(|_| format!("error posting {}", url))?;
            self.handle_json_response(&url, resp)
        })
    }

    /// Update the status of existing output files.
    ///
    /// PATCH /output_files
    pub fn patch_output_files(&self, patches: &[OutputFilePatch]) -> Result<()> {
        let url = self.url.join("output_files")?;
        self.via.retry_if_appropriate(|| -> Result<()> {
            let resp = self
                .client
                .patch(url.clone())
                .basic_auth(&self.username, Some(&self.password))
                .json(patches)
                .send()
                .with_context(|_| format!("error patching {}", url))?;
            self.handle_empty_response(&url, resp)
        })
    }

    /// Check the HTTP status code and parse a JSON response.
    fn handle_json_response<T>(
        &self,
        url: &Url,
        mut resp: reqwest::Response,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        if resp.status().is_success() {
            let value = resp
                .json()
                .with_context(|_| format!("error parsing {}", url))?;
            Ok(value)
        } else {
            Err(self.handle_error_response(url, resp))
        }
    }

    /// Check the HTTP status code and parse a JSON response.
    fn handle_empty_response(&self, url: &Url, resp: reqwest::Response) -> Result<()> {
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(self.handle_error_response(url, resp))
        }
    }

    /// Extract an error from an HTTP respone payload.
    fn handle_error_response(&self, url: &Url, mut resp: reqwest::Response) -> Error {
        match resp.text() {
            Ok(body) => format_err!(
                "unexpected HTTP status {} for {}:\n{}",
                resp.status(),
                url,
                body,
            ),
            Err(err) => err.into(),
        }
    }
}
