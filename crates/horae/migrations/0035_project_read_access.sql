-- Identity needed for an own timesheet is not project-wide reporting access.
CREATE VIEW project_read_access AS
SELECT u.id AS user_id, u.org_id, p.id AS project_id,
       u.org_role IN ('admin', 'manager') AS can_view_rates,
       (u.org_role IN ('admin', 'manager') OR a.id IS NOT NULL) AS can_view_team,
       (u.org_role IN ('admin', 'manager') OR (
         a.id IS NOT NULL AND (
           a.role IN ('lead', 'admin')
           OR COALESCE(ps.report_visibility, 'project_members') = 'project_members'
         )
       )) AS can_view_progress
FROM users u
JOIN projects p ON p.org_id = u.org_id
LEFT JOIN assignments a ON a.project_id = p.id AND a.user_id = u.id
LEFT JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
WHERE u.active AND (
  u.org_role IN ('admin', 'manager') OR a.id IS NOT NULL OR EXISTS (
    SELECT 1 FROM time_entries te
    WHERE te.org_id = u.org_id AND te.user_id = u.id AND te.project_id = p.id
  )
);

REVOKE ALL ON project_read_access FROM PUBLIC;
