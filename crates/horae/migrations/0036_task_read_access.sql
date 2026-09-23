-- Share catalog identity authorization between the UI and compatibility API.
-- A task identity is not permission to track it; time_entry_contexts owns that.
CREATE VIEW task_read_access AS
SELECT u.id AS user_id, u.org_id, t.id AS task_id,
       u.org_role IN ('admin', 'manager') AS can_view_rates,
       EXISTS (
         SELECT 1 FROM time_entries te
         WHERE te.org_id = u.org_id AND te.user_id = u.id AND te.task_id = t.id
       ) AS has_own_history
FROM users u
JOIN tasks t ON t.org_id = u.org_id
WHERE u.active AND (
  u.org_role IN ('admin', 'manager') OR EXISTS (
    SELECT 1 FROM project_tasks pt
    JOIN project_read_access access ON access.project_id = pt.project_id
    WHERE pt.task_id = t.id AND access.org_id = u.org_id
      AND access.user_id = u.id AND access.can_view_team
  ) OR EXISTS (
    SELECT 1 FROM time_entries te
    WHERE te.org_id = u.org_id AND te.user_id = u.id AND te.task_id = t.id
  )
);

REVOKE ALL ON task_read_access FROM PUBLIC;
