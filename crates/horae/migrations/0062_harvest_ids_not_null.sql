-- VALIDATE scans under SHARE UPDATE EXCLUSIVE, so reads and writes continue.
-- The unique indexes replace 0059's lookup indexes; building one holds a
-- SHARE lock (writes to that table wait, reads continue) and fails if two
-- records of a kind in an organization had the same number. SET NOT NULL then
-- proves the column full from the valid check instead of scanning, so the
-- ACCESS EXCLUSIVE locks come last and are brief; the check is redundant
-- afterwards.
ALTER TABLE clients VALIDATE CONSTRAINT clients_harvest_id_present;
ALTER TABLE projects VALIDATE CONSTRAINT projects_harvest_id_present;
ALTER TABLE tasks VALIDATE CONSTRAINT tasks_harvest_id_present;
ALTER TABLE time_entries VALIDATE CONSTRAINT time_entries_harvest_id_present;
ALTER TABLE users VALIDATE CONSTRAINT users_harvest_id_present;

CREATE UNIQUE INDEX clients_harvest_id ON clients (org_id, harvest_id);
CREATE UNIQUE INDEX projects_harvest_id ON projects (org_id, harvest_id);
CREATE UNIQUE INDEX tasks_harvest_id ON tasks (org_id, harvest_id);
CREATE UNIQUE INDEX time_entries_harvest_id ON time_entries (org_id, harvest_id);
CREATE UNIQUE INDEX users_harvest_id ON users (org_id, harvest_id);
DROP INDEX clients_harvest_id_lookup;
DROP INDEX projects_harvest_id_lookup;
DROP INDEX tasks_harvest_id_lookup;
DROP INDEX time_entries_harvest_id_lookup;
DROP INDEX users_harvest_id_lookup;

ALTER TABLE clients ALTER COLUMN harvest_id SET NOT NULL;
ALTER TABLE projects ALTER COLUMN harvest_id SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN harvest_id SET NOT NULL;
ALTER TABLE time_entries ALTER COLUMN harvest_id SET NOT NULL;
ALTER TABLE users ALTER COLUMN harvest_id SET NOT NULL;

ALTER TABLE clients DROP CONSTRAINT clients_harvest_id_present;
ALTER TABLE projects DROP CONSTRAINT projects_harvest_id_present;
ALTER TABLE tasks DROP CONSTRAINT tasks_harvest_id_present;
ALTER TABLE time_entries DROP CONSTRAINT time_entries_harvest_id_present;
ALTER TABLE users DROP CONSTRAINT users_harvest_id_present;
