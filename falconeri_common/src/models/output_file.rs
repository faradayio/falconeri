use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use uuid::Uuid;

use Result;
use schema::*;
use super::{Datum, Job, Status};

/// An output file uploaded from a worker.
#[derive(Associations, Debug, Identifiable, Queryable)]
#[belongs_to(Datum, foreign_key = "datum_id")]
#[belongs_to(Job, foreign_key = "job_id")]
pub struct OutputFile {
    /// The unique ID of this file.
    pub id: Uuid,
    /// When we created this record.
    pub created_at: NaiveDateTime,
    /// When we last updated this record.
    pub updated_at: NaiveDateTime,
    /// The status of this record. This will be `running` while the file is
    /// being uploaded.
    pub status: Status,
    /// The job which created this file.
    pub job_id: Uuid,
    /// The datum which created this file.
    pub datum_id: Uuid,
    /// The URI to which we uploaded this file.
    pub uri: String,
}

impl OutputFile {
    /// Mark this datum as having been successfully processed.
    pub fn mark_as_done(&mut self, conn: &PgConnection) -> Result<()> {
        *self = diesel::update(output_files::table.filter(output_files::id.eq(&self.id)))
            .set(output_files::status.eq(&Status::Done))
            .get_result(conn)
            .context("can't mark output file as done")?;
        Ok(())
    }

    /// Mark this datum as having been unsuccessfully processed.
    pub fn mark_as_error(&mut self, conn: &PgConnection) -> Result<()> {
        *self = diesel::update(output_files::table.filter(output_files::id.eq(&self.id)))
            .set(output_files::status.eq(&Status::Error))
            .get_result(conn)
            .context("can't mark output file as error")?;
        Ok(())
    }
}

/// Data required to create a new `OutputFile`.
#[derive(Debug, Insertable)]
#[table_name = "output_files"]
pub struct NewOutputFile {
    /// The job which created this file.
    pub job_id: Uuid,
    /// The datum which created this file.
    pub datum_id: Uuid,
    /// The URI to which we uploaded this file.
    pub uri: String,
}

impl NewOutputFile {
    /// Insert a new job into the database.
    pub fn insert(&self, conn: &PgConnection) -> Result<OutputFile> {
        Ok(diesel::insert_into(output_files::table)
            .values(self)
            .get_result(conn)
            .context("error inserting output file")?)
    }
}
