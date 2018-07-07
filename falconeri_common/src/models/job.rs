use chrono::NaiveDateTime;
use diesel::{self, PgConnection, prelude::*};
use failure::ResultExt;
use serde_json;
use uuid::Uuid;

use {Error, Result};
use schema::*;
use super::{Datum, File, Status};

/// A distributed data processing job.
#[derive(Debug, Identifiable, Queryable)]
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
    pub output_uri: String,
}

impl Job {
    /// Find a job by ID.
    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Job> {
        Ok(jobs::table
            .find(id)
            .first(conn)
            .with_context(|_| format_err!("could not load job {}", id))?)
    }

    /// Look up the next datum available to process, and set the status to
    /// `"processing"`. This is intended to be atomic from an SQL perspective.
    pub fn reserve_next_datum(
        &self,
        conn: &PgConnection,
    ) -> Result<Option<(Datum, Vec<File>)>> {
        conn.transaction::<_, Error, _>(|| {
            let datum_id: Option<Uuid> = datums::table
                .select(datums::id)
                .filter(
                    datums::job_id.eq(&self.id).and(datums::status.eq(Status::Ready))
                )
                .first(conn)
                .optional()
                .context("error trying to reserve next datum")?;
            if let Some(datum_id) = datum_id {
                let to_update = datums::table.filter(datums::id.eq(&datum_id));
                let datum: Datum = diesel::update(to_update)
                    .set(datums::status.eq(&Status::Running))
                    .get_result(conn)
                    .context("cannot mark datum as 'processing'")?;
                let files = File::belonging_to(&datum)
                    .load(conn)
                    .context("cannot load file information")?;
                Ok(Some((datum, files)))
            } else {
                Ok(None)
            }
        })
    }
}

/// Data required to create a new `Job`.
#[derive(Debug, Insertable)]
#[table_name = "jobs"]
pub struct NewJob {
    /// A copy of our original pipeline spec (just for debugging).
    pub pipeline_spec: serde_json::Value,
    /// The output bucket or bucket path.
    pub output_uri: String,
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
