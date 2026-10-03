-- Unknown historical provenance stays NULL; it is not service authority.
ALTER TABLE horae_jobs ADD COLUMN original_requester_id uuid;
ALTER TABLE horae_jobs ADD CONSTRAINT horae_jobs_original_requester_fkey
    FOREIGN KEY (org_id, original_requester_id) REFERENCES users(org_id, id);
