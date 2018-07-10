use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use std::fmt::Display;
use uuid::Uuid;

use Result;
use schema::*;
use super::{Job, Status};

/// A single chunk of work, consisting of one or more files.
#[derive(Associations, Debug, Identifiable, Queryable)]
#[belongs_to(Job, foreign_key = "job_id")]
pub struct Datum {
    /// The unique ID of this datum.
    pub id: Uuid,
    /// When this datum was created.
    pub created_at: NaiveDateTime,
    /// When this job was last updated.
    pub updated_at: NaiveDateTime,
    /// The current status of this datum.
    pub status: Status,
    /// The job to which this datum belongs.
    pub job_id: Uuid,
    /// An error message associated with this datum, if any.
    pub error_message: Option<String>,
}

impl Datum {
    /// How many datums are left to process in the specified job?
    ///
    /// TODO: Carefully cast result type to an unsigned type.
    pub fn left_to_process(job: &Job, conn: &PgConnection) -> Result<i64> {
        use diesel::dsl::{any, count_star};
        Ok(Datum::belonging_to(job)
            .filter(datums::status.eq(any(vec![Status::Ready, Status::Running])))
            .select(count_star())
            .get_result(conn)?)
    }

    /// Mark this datum as having been successfully processed.
    pub fn mark_as_done(&mut self, conn: &PgConnection) -> Result<()> {
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set(datums::status.eq(&Status::Done))
            .get_result(conn)
            .context("can't mark datum as done")?;
        Ok(())
    }

    /// Mark this datum as having been unsuccessfully processed.
    pub fn mark_as_error(
        &mut self,
        error_message: &dyn Display,
        conn: &PgConnection,
    ) -> Result<()> {
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((
                datums::status.eq(&Status::Error),
                datums::error_message.eq(&format!("{}", error_message)),
            ))
            .get_result(conn)
            .context("can't mark datum as having failed")?;
        Ok(())
    }
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
