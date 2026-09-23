-- Logical alert identity survives consumption falling and crossing again.
-- NULLS NOT DISTINCT keeps project/task/person scopes equally retry-safe.
CREATE TABLE project_budget_notifications (
  id             uuid PRIMARY KEY,
  org_id         uuid NOT NULL REFERENCES organizations(id),
  project_id     uuid NOT NULL,
  task_id        uuid,
  user_id        uuid,
  recipient_id   uuid NOT NULL,
  period_key     text NOT NULL CHECK (period_key = 'lifetime' OR period_key ~ '^[0-9]{4}-[0-9]{2}$'),
  threshold      smallint NOT NULL CHECK (threshold BETWEEN 0 AND 100),
  created_at     timestamptz NOT NULL DEFAULT now(),
  CHECK (task_id IS NULL OR user_id IS NULL),
  UNIQUE NULLS NOT DISTINCT (project_id, task_id, user_id, period_key, threshold, recipient_id),
  FOREIGN KEY (project_id, org_id) REFERENCES projects(id, org_id),
  FOREIGN KEY (task_id, org_id) REFERENCES tasks(id, org_id),
  FOREIGN KEY (user_id, org_id) REFERENCES users(id, org_id),
  FOREIGN KEY (recipient_id, org_id) REFERENCES users(id, org_id)
);

REVOKE ALL ON project_budget_notifications FROM PUBLIC;
