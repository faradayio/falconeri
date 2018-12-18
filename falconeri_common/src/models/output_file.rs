use crate::prefix::*;
use crate::schema::*;

/// An output file uploaded from a worker.
#[derive(Associations, Debug, Identifiable, Queryable, Serialize)]
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
    pub fn mark_as_done(datum: &Datum, conn: &PgConnection) -> Result<()> {
        diesel::update(OutputFile::belonging_to(datum))
            .set(output_files::status.eq(&Status::Done))
            .execute(conn)
            .context("can't mark output file as done")?;
        Ok(())
    }

    /// Mark this datum as having been unsuccessfully processed.
    pub fn mark_as_error(datum: &Datum, conn: &PgConnection) -> Result<()> {
        diesel::update(OutputFile::belonging_to(datum))
            .set(output_files::status.eq(&Status::Error))
            .execute(conn)
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
        trace!("Inserting output file: {:?}", self);
        Ok(diesel::insert_into(output_files::table)
            .values(self)
            .get_result(conn)
            .with_context(|_| format!("error inserting output file: {:?}", self))?)
    }
}
