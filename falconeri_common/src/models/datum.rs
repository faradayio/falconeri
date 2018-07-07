use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use uuid::Uuid;

use Result;
use schema::*;
use super::Status;

/// A single chunk of work, consisting of one or more files.
#[derive(Debug, Queryable)]
pub struct Datum {
    /// The unique ID of this datum.
    pub id: Uuid,
    /// When this datum was created.
    pub created_at: NaiveDateTime,
    /// When this datum was created.
    /// When this job was last updated.
    pub updated_at: NaiveDateTime,
    /// The current status of this datum.
    pub status: Status,
    /// The job to which this datum belongs.
    pub job_id: Uuid,
    /// An error message associated with this datum, if any.
    pub error_message: Option<String>,
}

/// Data required to create a new `Datum`.
#[derive(Debug, Insertable)]
#[table_name = "datums"]
pub struct NewDatum {
    /// The job to which this datum belongs.
    pub job_id: Uuid,
}

impl NewDatum {
    /// Insert a new job into the database.
    pub fn insert(&self, conn: &PgConnection) -> Result<Datum> {
        Ok(diesel::insert_into(datums::table)
            .values(self)
            .get_result(conn)
            .context("error inserting datum")?)
    }
}
