-- Integer ids for the Harvest-compatible API (specs/021-harvest-api-compat/data-model.md).
--
-- Harvest clients expect integer ids per kind. Horae's primary keys stay UUID
-- v7; `api_ids` maps each record to its number. A number is never reused:
-- rows stay after their record is deleted. Existing records are numbered by
-- 0059, after this migration's triggers already number new ones.

CREATE TYPE api_id_kind AS ENUM (
  'client', 'project', 'task', 'time_entry', 'user',
  'project_assignment', 'task_assignment', 'invoice', 'invoice_line_item', 'tag'
);

-- A mapping table keyed by what it maps, like harvest_import_map.
CREATE TABLE api_ids (
  org_id       uuid        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  kind         api_id_kind NOT NULL,
  number       bigint      NOT NULL CHECK (number > 0),
  horae_id     uuid        NOT NULL,
  -- True when `number` is the record's Harvest id (FR-016).
  from_harvest boolean     NOT NULL DEFAULT false,
  created_at   timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, kind, number),
  UNIQUE (org_id, kind, horae_id)
);

-- One sequence per kind. nextval needs no lock and ignores the snapshot, so
-- numbering works at every isolation level and never makes creators wait on
-- each other. Numbers are unique per organization and kind as required; they
-- are also distinct across organizations, which costs nothing.
CREATE SEQUENCE api_ids_client_seq;
CREATE SEQUENCE api_ids_project_seq;
CREATE SEQUENCE api_ids_task_seq;
CREATE SEQUENCE api_ids_time_entry_seq;
CREATE SEQUENCE api_ids_user_seq;

-- A record's number, for API responses.
CREATE FUNCTION api_number(p_org uuid, p_kind api_id_kind, p_id uuid) RETURNS bigint
LANGUAGE sql STABLE AS $$
  SELECT number FROM api_ids WHERE org_id = p_org AND kind = p_kind AND horae_id = p_id
$$;

-- The next free number of the kind. A number an imported record took as its
-- Harvest id is skipped.
CREATE FUNCTION api_id_allocate(p_org uuid, p_kind api_id_kind, p_id uuid) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  LOOP
    INSERT INTO api_ids (org_id, kind, number, horae_id)
    VALUES (p_org, p_kind, nextval(format('api_ids_%s_seq', p_kind)::regclass), p_id)
    ON CONFLICT (org_id, kind, number) DO NOTHING;
    EXIT WHEN FOUND;
  END LOOP;
END
$$;

-- Every insert path (server functions, importer, CLI, seed) numbers its record
-- in its own transaction through this trigger.
CREATE FUNCTION api_ids_number_new_record() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  PERFORM api_id_allocate(NEW.org_id, TG_ARGV[0]::api_id_kind, NEW.id);
  RETURN NULL;
END
$$;

CREATE TRIGGER clients_api_id AFTER INSERT ON clients
  FOR EACH ROW EXECUTE FUNCTION api_ids_number_new_record('client');
CREATE TRIGGER projects_api_id AFTER INSERT ON projects
  FOR EACH ROW EXECUTE FUNCTION api_ids_number_new_record('project');
CREATE TRIGGER tasks_api_id AFTER INSERT ON tasks
  FOR EACH ROW EXECUTE FUNCTION api_ids_number_new_record('task');
CREATE TRIGGER time_entries_api_id AFTER INSERT ON time_entries
  FOR EACH ROW EXECUTE FUNCTION api_ids_number_new_record('time_entry');
CREATE TRIGGER users_api_id AFTER INSERT ON users
  FOR EACH ROW EXECUTE FUNCTION api_ids_number_new_record('user');

-- The importer maps a Harvest id to the record it created or matched. A record
-- created in this same transaction (created_at is the transaction's start, so
-- this holds across the importer's savepoints) takes its Harvest id as its
-- number, unless another record of its kind already holds it; then it keeps
-- the fresh one. A record that existed before keeps its number: clients may
-- already know it.
CREATE FUNCTION api_ids_claim_harvest_id() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  v_kind api_id_kind := CASE NEW.harvest_entity_type::text
    WHEN 'client' THEN 'client'
    WHEN 'project' THEN 'project'
    WHEN 'task' THEN 'task'
    WHEN 'time_entry' THEN 'time_entry'
    WHEN 'person' THEN 'user'
  END::api_id_kind;
BEGIN
  -- An entity type with no API kind is mapped without a number.
  IF v_kind IS NULL OR NEW.harvest_import_id <= 0 THEN
    RETURN NULL;
  END IF;
  UPDATE api_ids SET number = NEW.harvest_import_id, from_harvest = true
  WHERE org_id = NEW.org_id AND kind = v_kind AND horae_id = NEW.horae_id
    AND NOT from_harvest AND created_at = now()
    AND NOT EXISTS (
      SELECT 1 FROM api_ids
      WHERE org_id = NEW.org_id AND kind = v_kind AND number = NEW.harvest_import_id
    );
  RETURN NULL;
END
$$;

CREATE TRIGGER harvest_import_map_api_id AFTER INSERT OR UPDATE OF horae_id ON harvest_import_map
  FOR EACH ROW EXECUTE FUNCTION api_ids_claim_harvest_id();

-- Only the application role calls these; the plugin reader must not (plugin/database.rs).
REVOKE ALL ON FUNCTION api_number(uuid, api_id_kind, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION api_id_allocate(uuid, api_id_kind, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION api_ids_number_new_record() FROM PUBLIC;
REVOKE ALL ON FUNCTION api_ids_claim_harvest_id() FROM PUBLIC;
