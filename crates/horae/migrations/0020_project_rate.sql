-- NULL preserves the existing task → assignment → user fallback until a manager
-- explicitly supplies a project rate. Existing invoices are not recalculated.
ALTER TABLE projects
  ADD COLUMN rate_cents bigint
    CHECK (rate_cents IS NULL OR rate_cents >= 0);
