-- `harvest_id`: the id in the Harvest-compatible API; Horae's identity is `id`
-- (UUID v7). specs/021-harvest-api-compat/data-model.md.
--
-- Every record of a numbered table has one, native or imported, unique per
-- organization. The column has no default, so an insert can give it (the
-- importer passes the record's Harvest id when it is free) or leave it to the
-- trigger, which takes the next number of the kind's global sequence that no
-- record of the kind in the organization has and that no imported record held
-- as its Harvest id. So a number never names another record, also after the
-- record it named is deleted.
--
-- Existing rows are numbered by 0061; 0062 and 0063 make the column NOT NULL
-- and unique.

CREATE FUNCTION harvest_id_next(
  p_table regclass, p_sequence regclass, p_type harvest_entity_type, p_org uuid
) RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE
  candidate bigint;
  taken boolean;
BEGIN
  LOOP
    candidate := nextval(p_sequence);
    EXECUTE format('SELECT EXISTS (SELECT 1 FROM %s WHERE org_id = $1 AND harvest_id = $2)', p_table)
      INTO taken USING p_org, candidate;
    EXIT WHEN NOT taken AND NOT EXISTS (
      SELECT 1 FROM harvest_import_map
      WHERE org_id = p_org AND harvest_entity_type = p_type AND harvest_import_id = candidate
    );
  END LOOP;
  RETURN candidate;
END
$$;

-- Arguments: the kind's sequence and its harvest_import_map entity type.
CREATE FUNCTION harvest_id_assign() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.harvest_id IS NULL THEN
    NEW.harvest_id := harvest_id_next(
      TG_RELID::regclass, TG_ARGV[0]::regclass, TG_ARGV[1]::harvest_entity_type, NEW.org_id
    );
  END IF;
  RETURN NEW;
END
$$;

ALTER TABLE clients ADD COLUMN harvest_id bigint;
ALTER TABLE projects ADD COLUMN harvest_id bigint;
ALTER TABLE tasks ADD COLUMN harvest_id bigint;
ALTER TABLE time_entries ADD COLUMN harvest_id bigint;
ALTER TABLE users ADD COLUMN harvest_id bigint;

COMMENT ON COLUMN clients.harvest_id IS
  'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';
COMMENT ON COLUMN projects.harvest_id IS
  'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';
COMMENT ON COLUMN tasks.harvest_id IS
  'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';
COMMENT ON COLUMN time_entries.harvest_id IS
  'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';
COMMENT ON COLUMN users.harvest_id IS
  'The id in the Harvest-compatible API; Horae''s identity is id (UUID v7).';

-- One sequence per kind, shared by every organization: nextval takes no lock
-- and ignores the snapshot, so creators never wait on each other at any
-- isolation level.
CREATE SEQUENCE clients_harvest_id_seq;
CREATE SEQUENCE projects_harvest_id_seq;
CREATE SEQUENCE tasks_harvest_id_seq;
CREATE SEQUENCE time_entries_harvest_id_seq;
CREATE SEQUENCE users_harvest_id_seq;

-- For the trigger's lookups until 0063 replaces them with unique indexes. Not
-- unique yet on purpose: an UPDATE of a column in a unique index takes a
-- FOR UPDATE row lock, which waits for every FOR KEY SHARE an insert's
-- foreign-key check holds, so the backfill would stop new time entries and
-- could deadlock with them. Sequence numbers and Harvest ids are distinct by
-- construction; 0063 proves it when it builds the unique index.
CREATE INDEX clients_harvest_id_lookup ON clients (org_id, harvest_id);
CREATE INDEX projects_harvest_id_lookup ON projects (org_id, harvest_id);
CREATE INDEX tasks_harvest_id_lookup ON tasks (org_id, harvest_id);
CREATE INDEX time_entries_harvest_id_lookup ON time_entries (org_id, harvest_id);
CREATE INDEX users_harvest_id_lookup ON users (org_id, harvest_id);

-- The backfill (0061) sets `horae.backfill` for each batch: adding the API id
-- is not an edit, so it must not move `updated_at` (the importer compares it
-- to detect an edit made in Horae) or a project's edit revision. 0063 restores
-- both bodies.
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF current_setting('horae.backfill', true) = 'on' THEN
    RETURN NEW;
  END IF;
  NEW.updated_at = now();
  RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION advance_project_edit_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF current_setting('horae.backfill', true) = 'on' THEN
    RETURN NEW;
  END IF;
  IF OLD IS DISTINCT FROM NEW THEN
    NEW.edit_revision := OLD.edit_revision + 1;
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER clients_harvest_id BEFORE INSERT ON clients
  FOR EACH ROW EXECUTE FUNCTION harvest_id_assign('clients_harvest_id_seq', 'client');
CREATE TRIGGER projects_harvest_id BEFORE INSERT ON projects
  FOR EACH ROW EXECUTE FUNCTION harvest_id_assign('projects_harvest_id_seq', 'project');
CREATE TRIGGER tasks_harvest_id BEFORE INSERT ON tasks
  FOR EACH ROW EXECUTE FUNCTION harvest_id_assign('tasks_harvest_id_seq', 'task');
CREATE TRIGGER time_entries_harvest_id BEFORE INSERT ON time_entries
  FOR EACH ROW EXECUTE FUNCTION harvest_id_assign('time_entries_harvest_id_seq', 'time_entry');
CREATE TRIGGER users_harvest_id BEFORE INSERT ON users
  FOR EACH ROW EXECUTE FUNCTION harvest_id_assign('users_harvest_id_seq', 'person');

-- Only the application role calls these; the plugin reader must not (plugin/database.rs).
REVOKE ALL ON FUNCTION harvest_id_next(regclass, regclass, harvest_entity_type, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION harvest_id_assign() FROM PUBLIC;
