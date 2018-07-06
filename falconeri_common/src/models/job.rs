use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use serde_json;
use uuid::Uuid;

use Result;
use schema::*;
use super::Status;

/// A distributed data processing job.
#[derive(Debug, Queryable)]
pub struct Job {
    /// The unique ID of this job.
    pub id: Uuid,
    /// When this job was created.
    pub created_at: NaiveDateTime,
    /// When this job was last updated.
    pub updated_at: NaiveDateTime,
    /// The current status of this job.
    pub status: Status,
    /// A copy of our original pipeline spec (just for debugging).
    pub pipeline_spec: serde_json::Value,
    /// The output bucket or bucket path.
    pub destination_uri: String,
}

/// Data required to create a new `Job`.
#[derive(Debug, Insertable)]
#[table_name = "jobs"]
pub struct NewJob {
    /// A copy of our original pipeline spec (just for debugging).
    pub pipeline_spec: serde_json::Value,
    /// The output bucket or bucket path.
    pub destination_uri: String,
}

impl NewJob {
    /// Insert a new job into the database.
    pub fn insert(&self, conn: &PgConnection) -> Result<Job> {
        Ok(diesel::insert_into(jobs::table)
            .values(self)
            .get_result(conn)
            .context("error inserting job")?)
    }
}
