CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE status AS ENUM ('ready', 'running', 'done', 'error', 'canceled');

CREATE TABLE jobs (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'running',
    pipeline_spec jsonb NOT NULL,
    command text[] NOT NULL CHECK (cardinality(command) > 0),
    egress_uri text NOT NULL
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

CREATE TABLE input_files (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    datum_id uuid NOT NULL REFERENCES datums(id) ON DELETE CASCADE,
    uri text NOT NULL,
    local_path text NOT NULL
);

CREATE INDEX input_file_datum_id ON input_files (datum_id);

CREATE TABLE output_files (
    id uuid NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    created_at timestamp NOT NULL DEFAULT now(),
    updated_at timestamp NOT NULL DEFAULT now(),
    status status NOT NULL DEFAULT 'running',
    job_id uuid NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    datum_id uuid NOT NULL REFERENCES datums(id) ON DELETE CASCADE,
    uri text NOT NULL,
    -- This constraint is critical, because it detects when one worker tries to
    -- clobber the output of another.
    UNIQUE (job_id, uri)
);

CREATE INDEX output_file_datum_id ON output_files (datum_id);

SELECT diesel_manage_updated_at('output_files');
