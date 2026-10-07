-- Retain configuration and history independently from global task activity.
ALTER TABLE project_tasks ADD COLUMN active boolean NOT NULL DEFAULT true;

-- Canonical accounts already archived globally must not regain project access
-- merely by restoring the catalog task. Policy-zero semantics remain unchanged.
UPDATE project_tasks pt SET active = false
FROM tasks t, projects p, organizations o
WHERE pt.task_id = t.id AND pt.project_id = p.id AND t.org_id = p.org_id
  AND o.id = p.org_id AND o.permission_policy_version = 1 AND NOT t.active;

CREATE OR REPLACE VIEW time_entry_contexts AS
SELECT u.id AS user_id, u.org_id, p.id AS project_id, t.id AS task_id,
       (p.project_type <> 'non_billable' AND pt.billable) AS billable
FROM users u
JOIN organizations o ON o.id = u.org_id
JOIN projects p ON p.org_id = u.org_id
JOIN clients c ON c.id = p.client_id AND c.org_id = u.org_id
JOIN project_tasks pt ON pt.project_id = p.id
JOIN tasks t ON t.id = pt.task_id AND t.org_id = u.org_id
LEFT JOIN assignments a ON a.project_id = p.id AND a.user_id = u.id
LEFT JOIN project_task_settings s ON s.project_id = p.id AND s.task_id = t.id
WHERE u.active AND c.active AND p.active AND t.active
  AND (o.permission_policy_version = 0 OR (o.permission_policy_version = 1 AND pt.active))
  AND (u.org_role = 'admin' OR a.id IS NOT NULL)
  AND (NOT COALESCE(s.restricted, false) OR EXISTS (
    SELECT 1 FROM project_task_members m
    WHERE m.project_id = p.id AND m.task_id = t.id AND m.user_id = u.id
  ));
