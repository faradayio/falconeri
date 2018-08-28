ALTER TABLE input_files ADD COLUMN job_id uuid NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
UPDATE input_files
  SET job_id = datums.job_id
  FROM datums
  WHERE input_files.datum_id = datums.id;
ALTER TABLE input_files ALTER COLUMN job_id DROP DEFAULT;
