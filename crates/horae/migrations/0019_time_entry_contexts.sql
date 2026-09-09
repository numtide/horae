-- New-time selectors and INSERT ... SELECT share the same eligibility rules.
-- Existing entries remain readable/editable after their context is archived.
CREATE VIEW time_entry_contexts AS
SELECT u.id AS user_id, u.org_id, p.id AS project_id, t.id AS task_id,
       (p.project_type <> 'non_billable' AND pt.billable) AS billable
FROM users u
JOIN projects p ON p.org_id = u.org_id
JOIN clients c ON c.id = p.client_id AND c.org_id = u.org_id
JOIN project_tasks pt ON pt.project_id = p.id
JOIN tasks t ON t.id = pt.task_id AND t.org_id = u.org_id
LEFT JOIN assignments a ON a.project_id = p.id AND a.user_id = u.id
WHERE u.active AND c.active AND p.active AND t.active
  AND (u.org_role = 'admin' OR a.id IS NOT NULL);
