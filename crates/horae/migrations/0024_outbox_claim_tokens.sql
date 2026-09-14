-- Each delivery attempt owns a distinct claim, including retries by the same
-- process. A stale consumer must not acknowledge a newer delivery attempt.
ALTER TABLE horae_outbox ADD COLUMN claim_token uuid;
