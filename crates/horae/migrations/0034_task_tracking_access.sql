-- Keep picker eligibility identical to manual entry and timer starts.
-- No active-context check is added to historical edits.
CREATE OR REPLACE VIEW time_entry_contexts AS
SELECT u.id AS user_id, u.org_id, p.id AS project_id, t.id AS task_id,
       (p.project_type <> 'non_billable' AND pt.billable) AS billable
FROM users u
JOIN projects p ON p.org_id = u.org_id
JOIN clients c ON c.id = p.client_id AND c.org_id = u.org_id
JOIN project_tasks pt ON pt.project_id = p.id
JOIN tasks t ON t.id = pt.task_id AND t.org_id = u.org_id
LEFT JOIN assignments a ON a.project_id = p.id AND a.user_id = u.id
LEFT JOIN project_task_settings s ON s.project_id = p.id AND s.task_id = t.id
WHERE u.active AND c.active AND p.active AND t.active
  AND (u.org_role = 'admin' OR a.id IS NOT NULL)
  AND (NOT COALESCE(s.restricted, false) OR EXISTS (
    SELECT 1 FROM project_task_members m
    WHERE m.project_id = p.id AND m.task_id = t.id AND m.user_id = u.id
  ));
