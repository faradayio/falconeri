-- We want to enforce a conditional uniqueness constraint. There are a couple of
-- ways to do this, with various tradeoffs, as discussed at
-- https://stackoverflow.com/q/16236365
CREATE UNIQUE INDEX one_running_datum_per_pod_name
  ON datums (job_id, pod_name)
  WHERE (status = 'running');
