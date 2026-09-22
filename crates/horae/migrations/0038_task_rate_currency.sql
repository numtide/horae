-- Existing catalog amounts may come from CSV rows in any currency. Their
-- denomination cannot be inferred from the workspace or linked projects.
-- Leave them unknown; existing project-task snapshots remain unchanged.
ALTER TABLE tasks ADD COLUMN default_rate_currency text
  CHECK (default_rate_currency IS NULL OR default_rate_currency ~ '^[A-Z]{3}$');
