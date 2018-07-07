CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE status AS ENUM ('creating', 'ready', 'running', 'done', 'error');

CREATE TABLE jobs (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'running',
    pipeline_spec jsonb NOT NULL,
    output_uri text NOT NULL
);

SELECT diesel_manage_updated_at('jobs');

CREATE TABLE datums (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'ready',
    job_id uuid NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    error_message text
);

CREATE INDEX datum_job_id_status ON datums (job_id, status);

SELECT diesel_manage_updated_at('datums');

CREATE TABLE files (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    datum_id uuid NOT NULL REFERENCES datums(id) ON DELETE CASCADE,
    uri text NOT NULL,
    local_path text NOT NULL
);

CREATE INDEX file_datum_id ON files (datum_id);
