use crate::prelude::*;
use crate::schema::*;

/// A single chunk of work, consisting of one or more files.
#[derive(Associations, Debug, Deserialize, Identifiable, Queryable, Serialize)]
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
    /// The Kubernetes node on which this job is running / was run.
    pub node_name: Option<String>,
    /// The Kubernetes pod which is running / ran this job.
    pub pod_name: Option<String>,
    /// The backtrace associated with `error_message`, if any.
    pub backtrace: Option<String>,
    /// Combined stdout and stderr of the code which processed the datum.
    pub output: Option<String>,
}

impl Datum {
    /// Find a datum by ID.
    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Datum> {
        Ok(datums::table
            .find(id)
            .first(conn)
            .with_context(|_| format!("could not load datum {}", id))?)
    }

    /// Get the input files for this datum.
    pub fn input_files(&self, conn: &PgConnection) -> Result<Vec<InputFile>> {
        Ok(InputFile::belonging_to(self)
            .order_by(input_files::created_at)
            .load(conn)
            .context("could not load input file")?)
    }

    /// Mark this datum as having been successfully processed.
    pub fn mark_as_done(&mut self, output: &str, conn: &PgConnection) -> Result<()> {
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((datums::status.eq(&Status::Done), datums::output.eq(output)))
            .get_result(conn)
            .context("can't mark datum as done")?;
        Ok(())
    }

    /// Mark this datum as having been unsuccessfully processed.
    pub fn mark_as_error(
        &mut self,
        output: &str,
        error_message: &str,
        backtrace: &str,
        conn: &PgConnection,
    ) -> Result<()> {
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((
                datums::status.eq(&Status::Error),
                datums::output.eq(output),
                datums::error_message.eq(&error_message),
                datums::backtrace.eq(&backtrace),
            ))
            .get_result(conn)
            .context("can't mark datum as having failed")?;
        Ok(())
    }

    /// Generate a sample value for testing.
    pub fn factory(job: &Job) -> Self {
        let now = Utc::now().naive_utc();
        Datum {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            status: Status::Running,
            job_id: job.id,
            error_message: None,
            node_name: None,
            pod_name: None,
            backtrace: None,
            output: None,
        }
    }
}

/// Data required to create a new `Datum`.
#[derive(Debug, Insertable)]
#[table_name = "datums"]
pub struct NewDatum {
    /// The unique ID of this datum. This must be generated by the caller and
    /// supplied at creation time so that it can be immediately used for the
    /// associated `InputFiles` without first needing to insert this record and
    /// pay round-trip costs.
    pub id: Uuid,
    /// The job to which this datum belongs.
    pub job_id: Uuid,
}

impl NewDatum {
    /// Insert new datums into the database.
    pub fn insert_all(datums: &[Self], conn: &PgConnection) -> Result<()> {
        diesel::insert_into(datums::table)
            .values(datums)
            .execute(conn)
            .context("error inserting datums")?;
        Ok(())
    }
}
