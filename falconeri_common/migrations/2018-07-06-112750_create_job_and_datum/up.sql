CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE status AS ENUM ('creating', 'ready', 'running', 'done', 'error');

CREATE TABLE jobs (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'creating',
    pipeline_spec jsonb NOT NULL,
    destination_uri text NOT NULL
);

SELECT diesel_manage_updated_at('jobs');

CREATE TABLE data (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'ready',
    job_id uuid NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    source_uri text NOT NULL,
    error_message text
);

CREATE INDEX data_job_id_status ON data (job_id, status);

SELECT diesel_manage_updated_at('data');
