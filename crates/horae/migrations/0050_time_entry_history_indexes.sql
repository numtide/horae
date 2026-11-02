-- Own-history checks (`project_read_access`, `task_read_access`, the canonical
-- tracking choices) ask whether a user has any entry on a project or task once
-- per candidate project or task. Without these, each probe intersects every
-- entry of the project with every entry of the user, so the cost grows with
-- history; with them, each probe is a single index-only lookup.
CREATE INDEX time_entries_org_id_user_id_project_id_idx
    ON time_entries (org_id, user_id, project_id);
CREATE INDEX time_entries_org_id_user_id_task_id_idx
    ON time_entries (org_id, user_id, task_id);
