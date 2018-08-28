use cast;
use diesel::dsl;
use serde_json;
use std::env;

use prefix::*;
use schema::*;

/// A distributed data processing job.
#[derive(Debug, Identifiable, Queryable, Serialize)]
pub struct Job {
    /// The unique ID of this job.
    pub id: Uuid,
    /// When this job was created.
    ///
    /// TODO: Verify timezone handling is sensible.
    pub created_at: NaiveDateTime,
    /// When this job was last updated.
    pub updated_at: NaiveDateTime,
    /// The current status of this job.
    pub status: Status,
    /// A copy of our original pipeline spec (just for debugging).
    pub pipeline_spec: serde_json::Value,
    /// The Kubenetes `Job` name for this job.
    pub job_name: String,
    /// The command to run in the worker container.
    pub command: Vec<String>,
    /// The output bucket or bucket path.
    pub egress_uri: String,
}

impl Job {
    /// Find a job by ID.
    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Job> {
        Ok(jobs::table
            .find(id)
            .first(conn)
            .with_context(|_| format!("could not load job {}", id))?)
    }

    /// Find a job by job name.
    pub fn find_by_job_name(job_name: &str, conn: &PgConnection) -> Result<Job> {
        Ok(jobs::table
            .filter(jobs::job_name.eq(job_name))
            .first(conn)
            .with_context(|_| format!("could not load job {:?}", job_name))?)
    }

    /// Get all known jobs.
    pub fn list(conn: &PgConnection) -> Result<Vec<Job>> {
        Ok(jobs::table
            .order_by(jobs::created_at.desc())
            .load(conn)
            .context("could not list jobs")?)
    }

    /// Look up the next datum available to process, and set the status to
    /// `"processing"`. This is intended to be atomic from an SQL perspective.
    pub fn reserve_next_datum(
        &self,
        conn: &PgConnection,
    ) -> Result<Option<(Datum, Vec<InputFile>)>> {
        let node_name = env::var("FALCONERI_NODE_NAME")
            .context("couldn't get FALCONERI_NODE_NAME")?;
        let pod_name =
            env::var("FALCONERI_POD_NAME").context("couldn't get FALCONERI_POD_NAME")?;
        conn.transaction(|| {
            let datum_id: Option<Uuid> = datums::table
                .select(datums::id)
                .for_update()
                .skip_locked()
                .filter(
                    datums::job_id
                        .eq(&self.id)
                        .and(datums::status.eq(Status::Ready)),
                )
                .first(conn)
                .optional()
                .context("error trying to reserve next datum")?;
            if let Some(datum_id) = datum_id {
                let to_update = datums::table.filter(datums::id.eq(&datum_id));
                let datum: Datum = diesel::update(to_update)
                    .set((
                        datums::status.eq(&Status::Running),
                        datums::node_name.eq(&Some(node_name)),
                        datums::pod_name.eq(&Some(pod_name)),
                    ))
                    .get_result(conn)
                    .context("cannot mark datum as 'processing'")?;
                let files = InputFile::belonging_to(&datum)
                    .load(conn)
                    .context("cannot load file information")?;
                Ok(Some((datum, files)))
            } else {
                Ok(None)
            }
        })
    }

    /// Get the number of datums with each status.
    pub fn datum_status_counts(
        &self,
        conn: &PgConnection,
    ) -> Result<Vec<(Status, u64)>> {
        // Look up how many
        let raw_status_counts: Vec<(Status, i64)> = Datum::belonging_to(&*self)
            // Diesel doesn't fully support `GROUP BY`, but we can use the
            // undocumented `group_by` method and the `dsl::sql` helper to build
            // the query anyways. For details, see
            // https://github.com/diesel-rs/diesel/issues/210
            .group_by(datums::status)
            .select(dsl::sql::<(sql_types::Status, diesel::sql_types::BigInt)>("status, count(*)"))
            .order_by(datums::status)
            .load(conn)
            .context("cannot load status of datums")?;

        Ok(raw_status_counts
            .into_iter()
            .filter(|&(_status, count)| count > 0)
            .map(|(status, count)| Ok((status, cast::u64(count)?)))
            .collect::<Result<_>>()?)
    }

    /// Get all our our currently running datums (the ones being processed by
    /// a worker somewhere).
    pub fn datums_with_status(
        &self,
        status: Status,
        conn: &PgConnection,
    ) -> Result<Vec<Datum>> {
        Ok(Datum::belonging_to(self)
            .filter(datums::status.eq(&status))
            .order(datums::updated_at)
            .load(conn)
            .context("cannot load running datums for job")?)
    }

    /// Lock the underying database row using `SELECT FOR UPDATE`. Must be
    /// called from within a transaction.
    fn lock_for_update(&mut self, conn: &PgConnection) -> Result<()> {
        *self = jobs::table
            .find(self.id)
            .for_update()
            .first(conn)
            .with_context(|_| format!("could not load job {}", self.id))?;
        Ok(())
    }

    /// Update the overall job status if there's nothing left to do.
    pub fn update_status_if_done(&mut self, conn: &PgConnection) -> Result<()> {
        trace!("querying for status of datums for job {}", self.id);
        conn.transaction(|| {
            // Lock this job for update. This isn't necessary for this routine
            // by itself, but it should help avoid race conditions with job
            // retries.
            self.lock_for_update(conn)?;
            if self.status != Status::Running {
                // Nothing to do, so return immediately.
                return Ok(());
            }

            // Count the datums with various statuses and divide them into
            // groups.
            let status_counts = self.datum_status_counts(conn)?;
            let mut unfinished = 0;
            let mut successful = 0;
            let mut failed = 0;
            for (status, count) in status_counts {
                match status {
                    Status::Ready | Status::Running => {
                        unfinished += count;
                    }
                    Status::Done => {
                        successful += count;
                    }
                    // TODO: Be smarted about `Canceled` once we implement it.
                    Status::Error | Status::Canceled => {
                        failed += count;
                    }
                }
            }

            // Decide what to do, if anything.
            let job_status = if unfinished > 0 {
                trace!("{} datums remaining, not updating job status", unfinished);
                None
            } else if failed > 0 {
                debug!("{} datums had errors, marking job as error", failed);
                Some(Status::Error)
            } else {
                debug!(
                    "all {} datums finished successfully, marking job as done",
                    successful,
                );
                Some(Status::Done)
            };
            if let Some(job_status) = job_status {
                *self = diesel::update(jobs::table)
                    .filter(jobs::id.eq(&self.id))
                    .set(jobs::status.eq(&job_status))
                    .get_result(conn)
                    .context("could not update job status")?;
            }

            Ok(())
        })
    }

    /// Generate a sample value for testing.
    pub fn factory() -> Self {
        let now = Utc::now().naive_utc();
        Job {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            status: Status::Running,
            pipeline_spec: serde_json::Value::Object(Default::default()),
            job_name: "my-job-123az".to_owned(), // TODO: Make unique.
            command: vec!["echo".to_owned(), "hi".to_owned()],
            egress_uri: "gs://example-bucket/output/".to_owned(),
        }
    }
}

/// Data required to create a new `Job`.
#[derive(Debug, Insertable)]
#[table_name = "jobs"]
pub struct NewJob {
    /// A copy of our original pipeline spec (just for debugging).
    pub pipeline_spec: serde_json::Value,
    /// The Kubenetes `Job` name for this job.
    pub job_name: String,
    /// The command to run in the worker container.
    pub command: Vec<String>,
    /// The output bucket or bucket path.
    pub egress_uri: String,
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
