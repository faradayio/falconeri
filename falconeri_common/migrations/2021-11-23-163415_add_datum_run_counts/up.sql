-- Add retry-related columns to `datums`.
ALTER TABLE datums
    ADD attempted_run_count integer NOT NULL DEFAULT 0,
    ADD maximum_allowed_run_count integer NOT NULL DEFAULT 1;

-- If an existing datum is in any state but 'ready', that means we attempted to
-- run it. So set a reasonable value here.
UPDATE datums
    SET attempted_run_count = 1
    WHERE "status" != 'ready';

-- Index jobs on status so we can look up running jobs and join them to datums.
CREATE INDEX jobs_status_id ON jobs (status, id);