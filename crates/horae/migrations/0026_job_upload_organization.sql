-- An upload belongs to the job's tenant, not merely to any existing tenant.
-- Existing mismatches must fail migration rather than silently reassign data.
ALTER TABLE horae_jobs
  ADD CONSTRAINT horae_jobs_id_org_unique UNIQUE (id, org_id);

ALTER TABLE horae_job_uploads
  ADD CONSTRAINT horae_job_uploads_job_org_fkey
    FOREIGN KEY (job_id, org_id) REFERENCES horae_jobs (id, org_id) ON DELETE CASCADE,
  DROP CONSTRAINT horae_job_uploads_job_id_fkey;
