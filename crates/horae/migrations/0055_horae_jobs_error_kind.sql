-- What a failed attempt ran into, so the run page can say what to do without
-- reading last_error. NULL for failures with no action beyond retrying.
ALTER TABLE horae_jobs
    ADD COLUMN error_kind text
        CHECK (error_kind IN ('reconnect_required', 'forbidden', 'unavailable'));
