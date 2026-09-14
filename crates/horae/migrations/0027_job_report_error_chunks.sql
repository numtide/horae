-- Large error lists are append-only JSON-lines streams owned by their job.
CREATE TABLE horae_job_report_error_chunks (
  id uuid PRIMARY KEY,
  job_id uuid NOT NULL,
  org_id uuid NOT NULL REFERENCES organizations(id),
  sequence bigint NOT NULL CHECK (sequence >= 0),
  schema_version smallint NOT NULL DEFAULT 1 CHECK (schema_version = 1),
  body bytea NOT NULL CHECK (octet_length(body) BETWEEN 1 AND 65536),
  UNIQUE (job_id, sequence),
  FOREIGN KEY (job_id, org_id) REFERENCES horae_jobs(id, org_id) ON DELETE CASCADE
);
