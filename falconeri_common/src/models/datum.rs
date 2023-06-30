use crate::kubernetes;
use crate::prelude::*;
use crate::schema::*;

/// A single chunk of work, consisting of one or more files.
#[derive(Associations, Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(belongs_to(Job, foreign_key = job_id))]
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
    /// How many times have we tried to process this datum (counting attempts in
    /// progress)?
    pub attempted_run_count: i32,
    /// How many times are we allowed to attempt to process this datum before
    /// failing for good?
    ///
    /// We store this on the `datum`, not the `job`, because (1) it simplifies
    /// several queries, and (2) it gives us the option of allowing extra
    /// retries on a particular datum someday.
    pub maximum_allowed_run_count: i32,
}

impl Datum {
    /// Find a datum by ID.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn find(id: Uuid, conn: &mut PgConnection) -> Result<Datum> {
        datums::table
            .find(id)
            .first(conn)
            .with_context(|| format!("could not load datum {}", id))
    }

    /// Find all datums with the specified status that belong to a running job.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn active_with_status(
        status: Status,
        conn: &mut PgConnection,
    ) -> Result<Vec<Datum>> {
        let datums = datums::table
            .inner_join(jobs::table)
            .filter(jobs::status.eq(Status::Running))
            .filter(datums::status.eq(status))
            .select(datums::all_columns)
            .load::<Datum>(conn)
            .with_context(|| {
                format!("could not load datums with status {}", status)
            })?;
        Ok(datums)
    }

    /// Find datums which claim to be running, but whose `pod_name` points to a
    /// non-existant pod.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn zombies(conn: &mut PgConnection) -> Result<Vec<Datum>> {
        let running = Self::active_with_status(Status::Running, conn)?;
        trace!("running datums: {:?}", running);
        let running_pod_names = kubernetes::get_running_pod_names()?;
        Ok(running
            .into_iter()
            .filter(|datum| match &datum.pod_name {
                Some(pod_name) => !running_pod_names.contains(pod_name),
                None => {
                    warn!("datum {} has status=\"running\" but no pod_name", datum.id);
                    true
                }
            })
            .collect::<Vec<_>>())
    }

    /// Find all datums which have errored, but that we can re-run.
    ///
    /// This will only return datums associated with running jobs.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn rerunable(conn: &mut PgConnection) -> Result<Vec<Datum>> {
        let datums = datums::table
            .inner_join(jobs::table)
            .filter(jobs::status.eq(Status::Running))
            .filter(datums::status.eq(Status::Error))
            .filter(datums::attempted_run_count.lt(datums::maximum_allowed_run_count))
            .select(datums::all_columns)
            .load::<Datum>(conn)
            .context("could not load rerunable datums")?;
        debug!("found {} re-runable jobs", datums.len());
        Ok(datums)
    }

    /// Is this datum re-runable, assuming it belongs to a running job?
    ///
    /// The logic here should mirror [`Datum::rerunnable`] above, except we
    /// don't check the job status. We use this to double-check the results of
    /// `Self::rerunnable` _after_ loading them and locking an individual
    /// `Datum`. We do this to prevent holding locks on more than one `Datum`.
    pub fn is_rerunable(&self) -> bool {
        self.status == Status::Error
            && self.attempted_run_count < self.maximum_allowed_run_count
    }

    /// Get the input files for this datum.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn input_files(&self, conn: &mut PgConnection) -> Result<Vec<InputFile>> {
        InputFile::belonging_to(self)
            .order_by(input_files::created_at)
            .load(conn)
            .context("could not load input file")
    }

    /// Lock the underying database row using `SELECT FOR UPDATE`. Must be
    /// called from within a transaction.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn lock_for_update(&mut self, conn: &mut PgConnection) -> Result<()> {
        *self = datums::table
            .find(self.id)
            .for_update()
            .first(conn)
            .with_context(|| format!("could not load datum {}", self.id))?;
        Ok(())
    }

    /// Mark this datum as having been successfully processed.
    #[tracing::instrument(skip(conn, output), level = "trace")]
    pub fn mark_as_done(
        &mut self,
        output: &str,
        conn: &mut PgConnection,
    ) -> Result<()> {
        let now = Utc::now().naive_utc();
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((
                datums::updated_at.eq(now),
                datums::status.eq(&Status::Done),
                datums::output.eq(output),
            ))
            .get_result(conn)
            .context("can't mark datum as done")?;
        Ok(())
    }

    /// Mark this datum as having been unsuccessfully processed.
    #[tracing::instrument(skip(conn, output, backtrace), level = "trace")]
    pub fn mark_as_error(
        &mut self,
        output: &str,
        error_message: &str,
        backtrace: &str,
        conn: &mut PgConnection,
    ) -> Result<()> {
        let now = Utc::now().naive_utc();
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((
                datums::updated_at.eq(now),
                datums::status.eq(&Status::Error),
                datums::output.eq(output),
                datums::error_message.eq(&error_message),
                datums::backtrace.eq(&backtrace),
            ))
            .get_result(conn)
            .context("can't mark datum as having failed")?;
        Ok(())
    }

    /// Mark this datum as eligible to be re-run another time.
    ///
    /// We assume that the datum's row is locked by `lock_for_update` when we
    /// are called.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_as_eligible_for_rerun(
        &mut self,
        conn: &mut PgConnection,
    ) -> Result<()> {
        let now = Utc::now().naive_utc();
        *self = diesel::update(datums::table.filter(datums::id.eq(&self.id)))
            .set((
                datums::updated_at.eq(now),
                datums::status.eq(&Status::Ready),
                // Don't do this here! This is done when we start running in
                // `actually_reserve_next_datum`.
                //
                // datums::attempted_run_count.eq(self.attempted_run_count + 1),
            ))
            .get_result(conn)
            .context("can't mark datum as eligible")?;
        Ok(())
    }

    /// Update the status of our associate job, if it has finished.
    ///
    /// This calls [`Job::update_status_if_done`].
    pub fn update_job_status_if_done(&self, conn: &mut PgConnection) -> Result<()> {
        let mut job = Job::find(self.job_id, conn)?;
        job.update_status_if_done(conn)
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
            attempted_run_count: 0,
            maximum_allowed_run_count: 1,
        }
    }
}

/// Data required to create a new `Datum`.
#[derive(Debug, Insertable)]
#[diesel(table_name = datums)]
pub struct NewDatum {
    /// The unique ID of this datum. This must be generated by the caller and
    /// supplied at creation time so that it can be immediately used for the
    /// associated `InputFiles` without first needing to insert this record and
    /// pay round-trip costs.
    pub id: Uuid,
    /// The job to which this datum belongs.
    pub job_id: Uuid,
    /// How many times are we allowed to attempt to process this datum before
    /// failing for good?
    pub maximum_allowed_run_count: i32,
}

impl NewDatum {
    /// Insert new datums into the database.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn insert_all(datums: &[Self], conn: &mut PgConnection) -> Result<()> {
        diesel::insert_into(datums::table)
            .values(datums)
            .execute(conn)
            .context("error inserting datums")?;
        Ok(())
    }
}
