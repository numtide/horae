ALTER TABLE horae_jobs
  ADD COLUMN claim_token uuid,
  ADD COLUMN cancellation_requested boolean NOT NULL DEFAULT false;
