-- NULL settings preserve the legacy/imported precedence. Callers only supply
-- inherited rates whose denomination matches the configured project currency.
-- Keep this pure function in parity with horae_core::invoice::resolve_project_rate.
CREATE FUNCTION resolve_project_rate(
  mode text,
  task_rate bigint,
  assignment_rate bigint,
  project_rate bigint,
  user_rate bigint,
  client_rate bigint
) RETURNS bigint
  LANGUAGE sql
  IMMUTABLE
  PARALLEL SAFE
  AS $$
    SELECT CASE COALESCE(mode, 'legacy')
      WHEN 'legacy' THEN COALESCE(task_rate, assignment_rate, project_rate, user_rate)
      WHEN 'person' THEN COALESCE(assignment_rate, user_rate, client_rate)
      WHEN 'task' THEN COALESCE(task_rate, client_rate)
      WHEN 'project' THEN project_rate
      ELSE NULL
    END
  $$;

-- Application queries run as the migration owner. Plugins do not need this
-- function or access to private project configuration; do not widen their grants.
REVOKE ALL ON FUNCTION resolve_project_rate(text, bigint, bigint, bigint, bigint, bigint) FROM PUBLIC;

ALTER TABLE invoices ADD CONSTRAINT invoices_id_org_unique UNIQUE (id, org_id);
ALTER TABLE project_fee_milestones ADD CONSTRAINT milestones_project_org_unique
  UNIQUE (id, project_id, org_id);

CREATE TABLE project_fee_occurrences (
  id            uuid PRIMARY KEY,
  org_id        uuid NOT NULL REFERENCES organizations(id),
  project_id    uuid NOT NULL,
  milestone_id  uuid,
  period_key    text NOT NULL CHECK (char_length(period_key) BETWEEN 1 AND 100),
  due_on        date NOT NULL,
  description   text NOT NULL,
  amount_cents  bigint NOT NULL CHECK (amount_cents >= 0),
  currency      char(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
  invoice_id    uuid,
  UNIQUE (project_id, period_key),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id),
  FOREIGN KEY (milestone_id, project_id, org_id)
    REFERENCES project_fee_milestones(id, project_id, org_id),
  FOREIGN KEY (invoice_id, org_id) REFERENCES invoices(id, org_id)
);

CREATE INDEX project_fee_occurrences_invoice ON project_fee_occurrences (invoice_id)
  WHERE invoice_id IS NOT NULL;
REVOKE ALL ON project_fee_occurrences FROM PUBLIC;

ALTER TABLE invoice_line_items
  ALTER COLUMN time_entry_id DROP NOT NULL,
  ALTER COLUMN minutes DROP NOT NULL,
  ALTER COLUMN rate_cents DROP NOT NULL,
  ADD COLUMN fee_occurrence_id uuid REFERENCES project_fee_occurrences(id),
  ADD CONSTRAINT invoice_line_items_fee_unique UNIQUE (invoice_id, fee_occurrence_id),
  ADD CONSTRAINT invoice_line_items_exclusive_source CHECK (
    (time_entry_id IS NOT NULL AND fee_occurrence_id IS NULL
      AND minutes IS NOT NULL AND rate_cents IS NOT NULL)
    OR
    (time_entry_id IS NULL AND fee_occurrence_id IS NOT NULL
      AND minutes IS NULL AND rate_cents IS NULL)
  );
