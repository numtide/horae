-- VALIDATE scans under SHARE UPDATE EXCLUSIVE. The unique indexes replace
-- 0060's lookup indexes; each build takes a SHARE lock, and since this file is
-- one transaction those locks are held until it commits: reads continue, writes
-- to the five tables wait for the whole file (0.73 s at 1M time entries). SET
-- NOT NULL then proves the column full from the valid check instead of
-- scanning, so the ACCESS EXCLUSIVE locks come last; the check is redundant
-- afterwards.

-- A duplicate number would make an index build fail with a bare constraint
-- error; name the first one instead. sqlx refuses to start an older image once
-- 0060 is applied, so the fix is forward: renumber the duplicate and restart.
DO $$
DECLARE
  k record;
  dup record;
BEGIN
  FOR k IN SELECT * FROM (VALUES
    ('clients', 'client'), ('projects', 'project'), ('tasks', 'task'),
    ('time_entries', 'time_entry'), ('users', 'person')
  ) AS kinds(tbl, harvest_type)
  LOOP
    EXECUTE format(
      'SELECT org_id, harvest_id, count(*) AS records FROM %I
       WHERE harvest_id IS NOT NULL
       GROUP BY org_id, harvest_id HAVING count(*) > 1 LIMIT 1', k.tbl)
      INTO dup;
    IF dup.records IS NOT NULL THEN
      RAISE EXCEPTION '% records in % share harvest_id % in organization %',
        dup.records, k.tbl, dup.harvest_id, dup.org_id
        USING HINT = format(
          'Renumber all but one with UPDATE %1$I SET harvest_id = harvest_id_next(%1$L, %2$L, %3$L, org_id) WHERE id = ..., then restart.',
          k.tbl, k.tbl || '_harvest_id_seq', k.harvest_type);
    END IF;
  END LOOP;
END
$$;

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

-- The backfill is done: restore 0011's and 0039's bodies, so no session can
-- skip these stamps by setting `horae.backfill`.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION advance_project_edit_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;
