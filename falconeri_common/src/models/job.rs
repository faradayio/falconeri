use cast;
use diesel::dsl;
use serde_json;

use crate::prelude::*;
use crate::schema::*;

/// A distributed data processing job.
#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
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
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn find(id: Uuid, conn: &mut PgConnection) -> Result<Job> {
        jobs::table
            .find(id)
            .first(conn)
            .with_context(|| format!("could not load job {}", id))
    }

    /// Find a job by job name.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn find_by_job_name(job_name: &str, conn: &mut PgConnection) -> Result<Job> {
        jobs::table
            .filter(jobs::job_name.eq(job_name))
            .first(conn)
            .with_context(|| format!("could not load job {:?}", job_name))
    }

    /// Find all jobs with specified status.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn find_by_status(
        status: Status,
        conn: &mut PgConnection,
    ) -> Result<Vec<Job>> {
        jobs::table
            .filter(jobs::status.eq(status))
            .load(conn)
            .with_context(|| format!("could not load jobs with status {}", status))
    }

    /// Get all known jobs.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn list(conn: &mut PgConnection) -> Result<Vec<Job>> {
        jobs::table
            .order_by(jobs::created_at.desc())
            .load(conn)
            .context("could not list jobs")
    }

    /// Look up the next datum available to process, and set the status to
    /// `"processing"`. This is intended to be atomic from an SQL perspective.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn reserve_next_datum(
        &self,
        node_name: &str,
        pod_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Option<(Datum, Vec<InputFile>)>> {
        // Check for existing reservation (which shouldn't happen unless
        // a reservation got lost somewhere between `falconeri-postgres` and
        // `falconeri-worker`), and if none exists, make a new one.
        let mut datum = self.find_already_reserved_datum(pod_name, conn)?;
        if let Some(ref datum) = datum {
            warn!(
                "pod {} tried to reserve datum {} more than once",
                pod_name, datum.id,
            );
        } else {
            datum = self.actually_reserve_next_datum(node_name, pod_name, conn)?;
        }

        // If we've got a datum, get the `input_files` to go with it.
        if let Some(datum) = datum {
            let files = InputFile::belonging_to(&datum)
                .load(conn)
                .context("cannot load file information")?;
            Ok(Some((datum, files)))
        } else {
            Ok(None)
        }
    }

    /// Find any datum which has already been assignd to `pod_name`. This can
    /// happen if an HTTP client calls `reserve_next_datum`, the reservation
    /// succeeds at the database layer, but the HTTP response never reaches the
    /// client.
    ///
    /// But if the reservation has been made at the database layer, we can make
    /// the reservation idempotent by looking for an existing reservation.
    #[tracing::instrument(skip(conn), level = "trace")]
    fn find_already_reserved_datum(
        &self,
        pod_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Option<Datum>> {
        Ok(datums::table
            .filter(
                datums::job_id
                    .eq(&self.id)
                    .and(datums::pod_name.eq(pod_name))
                    .and(datums::status.eq(Status::Running)),
            )
            .get_result(conn)
            .optional()?)
    }

    /// Internal helper for `reserve_next_datum` which performs the actual
    /// atomic reservation part itself, if we actually need to do so.
    #[tracing::instrument(skip(conn), level = "trace")]
    fn actually_reserve_next_datum(
        &self,
        node_name: &str,
        pod_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Option<Datum>> {
        conn.transaction(|conn| {
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
                let now = Utc::now().naive_utc();
                let datum: Datum = diesel::update(to_update)
                    .set((
                        datums::updated_at.eq(now),
                        datums::status.eq(&Status::Running),
                        datums::node_name.eq(&Some(node_name)),
                        datums::pod_name.eq(&Some(pod_name)),
                        datums::attempted_run_count
                            .eq(datums::attempted_run_count + 1),
                    ))
                    .get_result(conn)
                    .context("cannot mark datum as 'processing'")?;
                Ok(Some(datum))
            } else {
                Ok(None)
            }
        })
    }

    /// Get the number of datums with each status.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn datum_status_counts(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Vec<DatumStatusCount>> {
        // Look up how many
        let raw_status_counts: Vec<(Status, i64, i64)> = Datum::belonging_to(self)
            // Diesel doesn't fully support `GROUP BY`, but we can use the
            // undocumented `group_by` method and the `dsl::sql` helper to build
            // the query anyways. For details, see
            // https://github.com/diesel-rs/diesel/issues/210
            .group_by(datums::status)
            .select(dsl::sql::<(
                sql_types::Status,
                diesel::sql_types::BigInt,
                diesel::sql_types::BigInt,
            )>(
                "status, count(*), count(*) filter (where status = 'error' and attempted_run_count < maximum_allowed_run_count)",
            ))
            .order_by(datums::status)
            .load(conn)
            .context("cannot load status of datums")?;

        raw_status_counts
            .into_iter()
            .filter(|&(_status, count, _rerunable_count)| count > 0)
            .map(|(status, count, rerunable_count)| {
                Ok(DatumStatusCount {
                    status,
                    count: cast::u64(count)?,
                    rerunable_count: cast::u64(rerunable_count)?,
                })
            })
            .collect::<Result<_>>()
    }

    /// Get all our our currently running datums (the ones being processed by
    /// a worker somewhere).
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn datums_with_status(
        &self,
        status: Status,
        conn: &mut PgConnection,
    ) -> Result<Vec<Datum>> {
        Datum::belonging_to(self)
            .filter(datums::status.eq(&status))
            .order(datums::updated_at)
            .load(conn)
            .context("cannot load running datums for job")
    }

    /// Lock the underying database row using `SELECT FOR UPDATE`. Must be
    /// called from within a transaction.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn lock_for_update(&mut self, conn: &mut PgConnection) -> Result<()> {
        *self = jobs::table
            .find(self.id)
            .for_update()
            .first(conn)
            .with_context(|| format!("could not load job {}", self.id))?;
        Ok(())
    }

    /// Update the overall job status if there's nothing left to do.
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn update_status_if_done(&mut self, conn: &mut PgConnection) -> Result<()> {
        trace!("querying for status of datums for job {}", self.id);
        conn.transaction(|conn| {
            // Lock this job for update. This isn't necessary for this routine
            // by itself, but it should help avoid race conditions with job
            // retries and the babysitter.
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
            let mut rerunable = 0;
            for status_count in status_counts {
                match status_count.status {
                    Status::Ready | Status::Running => {
                        assert_eq!(status_count.rerunable_count, 0);
                        unfinished += status_count.count;
                    }
                    Status::Done => {
                        assert_eq!(status_count.rerunable_count, 0);
                        successful += status_count.count;
                    }
                    Status::Error => {
                        assert!(status_count.rerunable_count <= status_count.count);
                        failed += status_count.count - status_count.rerunable_count;
                        rerunable += status_count.rerunable_count;
                    }

                    // TODO: Be smarted about `Canceled` once we implement it.
                    Status::Canceled => {
                        assert_eq!(status_count.rerunable_count, 0);
                        failed += status_count.count;
                    }
                }
            }

            // Decide what to do, if anything.
            let job_status = if unfinished > 0 || rerunable > 0 {
                trace!(
                    "{} datums remaining, {} rerunable, not updating job status",
                    unfinished,
                    rerunable
                );
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
                    .set((
                        jobs::updated_at.eq(Utc::now().naive_utc()),
                        jobs::status.eq(&job_status),
                    ))
                    .get_result(conn)
                    .context("could not update job status")?;
            }

            Ok(())
        })
    }

    /// Mark this job as having errored.
    ///
    /// This is not the typical way jobs are marked as having errored, which is
    /// the responsibility of [`Job::update_status_if_done`].
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn mark_as_error(&mut self, conn: &mut PgConnection) -> Result<()> {
        debug!("marking job {} as having errored", self.job_name);
        *self = diesel::update(jobs::table)
            .filter(jobs::id.eq(&self.id))
            .set((
                jobs::updated_at.eq(Utc::now().naive_utc()),
                jobs::status.eq(Status::Error),
            ))
            .get_result(conn)
            .context("could not update job status")?;
        Ok(())
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

/// The number of datums with a specified status, plus how many are retryable.
#[derive(Debug, Queryable, Serialize)]
pub struct DatumStatusCount {
    /// The status we're counting.
    pub status: Status,
    /// The number of datums with this status.
    pub count: u64,
    /// The number of datums which could be re-run. This will be zero if
    /// `status` is not `Status::Error`.
    pub rerunable_count: u64,
}

/// Data required to create a new `Job`.
#[derive(Debug, Insertable)]
#[diesel(table_name = jobs)]
pub struct NewJob {
    /// The unique ID for this job.
    pub id: Uuid,
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
    #[tracing::instrument(skip(conn), level = "trace")]
    pub fn insert(&self, conn: &mut PgConnection) -> Result<Job> {
        diesel::insert_into(jobs::table)
            .values(self)
            .get_result(conn)
            .context("error inserting job")
    }
}
