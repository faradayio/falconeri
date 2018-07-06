use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
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
    /// The input bucket, bucket path or object to process.
    pub source_uri: String,
    /// The output bucket or bucket path.
    pub destination_uri: String,
}

/// Data required to create a new `Job`.
#[derive(Debug, Insertable)]
#[table_name = "jobs"]
pub struct NewJob {
    /// The input bucket, bucket path or object to process.
    pub source_uri: String,
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
