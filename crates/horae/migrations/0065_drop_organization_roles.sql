-- Permission state is the only access model left. The organization role, the
-- policy switch and the role-based read views decide nothing any more.
--
-- The two views are the only objects that name either column (time_entry_contexts
-- stopped reading them in 0064), and nothing reads the views. task_read_access
-- selects from project_read_access, so it goes first.
DROP VIEW task_read_access;
DROP VIEW project_read_access;

ALTER TABLE users DROP COLUMN org_role;
DROP TYPE org_role;

ALTER TABLE organizations DROP COLUMN permission_policy_version;
