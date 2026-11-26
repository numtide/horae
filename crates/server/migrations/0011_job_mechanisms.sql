-- Queue mechanisms (spec 027): concurrency keys, priority, heartbeats and a
-- record of every attempt.

ALTER TABLE horae_jobs
    ADD COLUMN concurrency_key text,
    ADD COLUMN priority smallint NOT NULL DEFAULT 0,
    ADD COLUMN heartbeat_at timestamptz,
    ADD COLUMN stale_after_secs integer NOT NULL DEFAULT 60
        CONSTRAINT horae_jobs_stale_after_secs_check CHECK (stale_after_secs BETWEEN 15 AND 3600);

-- At most one queued or running job per organization, kind and key.
CREATE UNIQUE INDEX horae_jobs_concurrency_key_idx ON horae_jobs (org_id, kind, concurrency_key)
    WHERE concurrency_key IS NOT NULL AND status IN ('queued', 'running');

DROP INDEX horae_jobs_claim_idx;
CREATE INDEX horae_jobs_claim_idx ON horae_jobs (priority DESC, available_at, created_at)
    WHERE status IN ('queued', 'running');

CREATE TABLE horae_job_attempts (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL,
    job_id uuid NOT NULL,
    attempt integer NOT NULL CHECK (attempt >= 1),
    worker_id text NOT NULL,
    started_at timestamptz NOT NULL,
    -- Null while running, and for an attempt whose worker died.
    finished_at timestamptz,
    outcome text CHECK (outcome IN ('succeeded', 'failed', 'retrying', 'cancelled', 'interrupted')),
    error text CHECK (octet_length(error) <= 4096),
    UNIQUE (job_id, attempt),
    FOREIGN KEY (job_id, org_id) REFERENCES horae_jobs (id, org_id) ON DELETE CASCADE
);

-- Only one attempt of a job is open at a time; finishing it looks it up here.
CREATE UNIQUE INDEX horae_job_attempts_open_idx ON horae_job_attempts (job_id) WHERE outcome IS NULL;
