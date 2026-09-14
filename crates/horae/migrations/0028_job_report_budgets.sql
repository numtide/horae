-- Check new writes immediately, including those from older worker instances.
-- Startup upgrades pre-existing reports before validating these constraints
-- and before accepting requests or starting the new worker.
ALTER TABLE horae_jobs
  ADD CONSTRAINT horae_jobs_report_budget
    CHECK (octet_length(report::text) <= 16384) NOT VALID,
  ADD CONSTRAINT horae_jobs_checkpoint_report_budget
    CHECK (octet_length((checkpoint->'report')::text) <= 16384) NOT VALID;
