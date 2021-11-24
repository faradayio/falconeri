DROP INDEX jobs_status_id;

ALTER TABLE datums
    DROP attempted_run_count,
    DROP maximum_allowed_run_count;
