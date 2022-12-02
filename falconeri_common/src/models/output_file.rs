use crate::prelude::*;
use crate::schema::*;

/// An output file uploaded from a worker.
#[derive(Associations, Debug, Deserialize, Identifiable, Queryable, Serialize)]
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
    /// Find an output file by ID.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn find(id: Uuid, conn: &PgConnection) -> Result<OutputFile> {
        output_files::table
            .find(id)
            .first(conn)
            .with_context(|| format!("could not load output file {}", id))
    }

    /// Fetch all the input files corresponding to `datums`, returning grouped
    /// in the same order.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn delete_for_datum(datum: &Datum, conn: &PgConnection) -> Result<()> {
        diesel::delete(OutputFile::belonging_to(datum))
            .execute(conn)
            .context("could not delete output files belonging to failed datums")?;
        Ok(())
    }

    /// Mark the specified output files as having been successfully processed.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_ids_as_done(ids: &[Uuid], conn: &PgConnection) -> Result<()> {
        diesel::update(output_files::table.filter(output_files::id.eq_any(ids)))
            .set((
                output_files::updated_at.eq(Utc::now().naive_utc()),
                output_files::status.eq(&Status::Done),
            ))
            .execute(conn)
            .context("can't mark output file as done")?;
        Ok(())
    }

    /// Mark the specified output files as having been successfully processed.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_ids_as_error(ids: &[Uuid], conn: &PgConnection) -> Result<()> {
        diesel::update(output_files::table.filter(output_files::id.eq_any(ids)))
            .set((
                output_files::updated_at.eq(Utc::now().naive_utc()),
                output_files::status.eq(&Status::Error),
            ))
            .execute(conn)
            .context("can't mark output file as done")?;
        Ok(())
    }

    /// Mark the output files of this datum as having been successfully
    /// processed.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_as_done_by_datum(datum: &Datum, conn: &PgConnection) -> Result<()> {
        diesel::update(OutputFile::belonging_to(datum))
            .set((
                output_files::updated_at.eq(Utc::now().naive_utc()),
                output_files::status.eq(&Status::Done),
            ))
            .execute(conn)
            .context("can't mark output file as done")?;
        Ok(())
    }

    /// Mark the output files of this datum as having been unsuccessfully
    /// processed.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_as_error_by_datum(datum: &Datum, conn: &PgConnection) -> Result<()> {
        diesel::update(OutputFile::belonging_to(datum))
            .set((
                output_files::updated_at.eq(Utc::now().naive_utc()),
                output_files::status.eq(&Status::Error),
            ))
            .execute(conn)
            .context("can't mark output file as error")?;
        Ok(())
    }
}

/// Data required to create a new `OutputFile`.
#[derive(Debug, Deserialize, Insertable, Serialize)]
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
    /// Insert new output files into the database.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn insert_all(
        output_files: &[Self],
        conn: &PgConnection,
    ) -> Result<Vec<OutputFile>> {
        let output_files = diesel::insert_into(output_files::table)
            .values(output_files)
            .get_results::<OutputFile>(conn)
            .context("error inserting datums")?;
        Ok(output_files)
    }
}
