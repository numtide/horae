-- Integer ids for the Harvest-compatible API (specs/021-harvest-api-compat/data-model.md).
--
-- Harvest clients expect integer ids per kind and an integer account id. Horae's
-- primary keys stay UUID v7; `api_ids` maps each record to its number. A number
-- is never reused: rows stay after their record is deleted.

ALTER TABLE organizations ADD COLUMN api_account_id bigint GENERATED ALWAYS AS IDENTITY UNIQUE;

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

-- A record's number, for API responses.
CREATE FUNCTION api_number(p_org uuid, p_kind api_id_kind, p_id uuid) RETURNS bigint
LANGUAGE sql STABLE AS $$
  SELECT number FROM api_ids WHERE org_id = p_org AND kind = p_kind AND horae_id = p_id
$$;

-- Serializes numbering per organization and kind until the transaction ends.
CREATE FUNCTION api_id_lock(p_org uuid, p_kind api_id_kind) RETURNS void
LANGUAGE sql AS $$
  SELECT pg_advisory_xact_lock(hashtextextended(p_org::text || '/' || p_kind::text, 0))
$$;

-- Every insert path (server functions, importer, CLI, seed) numbers its record
-- in its own transaction through this trigger.
CREATE FUNCTION api_ids_number_new_record() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  v_kind api_id_kind := TG_ARGV[0]::api_id_kind;
BEGIN
  PERFORM api_id_lock(NEW.org_id, v_kind);
  INSERT INTO api_ids (org_id, kind, number, horae_id)
  SELECT NEW.org_id, v_kind, COALESCE(max(number), 0) + 1, NEW.id
  FROM api_ids WHERE org_id = NEW.org_id AND kind = v_kind;
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
-- created in this same transaction takes its Harvest id as its number, unless
-- another record of its kind already holds it (then it keeps the fresh one).
-- A record that existed before keeps its number: clients may already know it.
CREATE FUNCTION api_ids_claim_harvest_id() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  v_kind api_id_kind := CASE NEW.harvest_entity_type
    WHEN 'person' THEN 'user'
    ELSE NEW.harvest_entity_type::text
  END::api_id_kind;
BEGIN
  PERFORM api_id_lock(NEW.org_id, v_kind);
  UPDATE api_ids SET number = NEW.harvest_id, from_harvest = true
  WHERE org_id = NEW.org_id AND kind = v_kind AND horae_id = NEW.horae_id
    AND NOT from_harvest AND created_at = now()
    AND NOT EXISTS (
      SELECT 1 FROM api_ids
      WHERE org_id = NEW.org_id AND kind = v_kind AND number = NEW.harvest_id
    );
  RETURN NULL;
END
$$;

CREATE TRIGGER harvest_import_map_api_id AFTER INSERT OR UPDATE OF horae_id ON harvest_import_map
  FOR EACH ROW EXECUTE FUNCTION api_ids_claim_harvest_id();

-- Only the application role calls these; the plugin reader must not (plugin/database.rs).
REVOKE ALL ON FUNCTION api_number(uuid, api_id_kind, uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION api_id_lock(uuid, api_id_kind) FROM PUBLIC;
REVOKE ALL ON FUNCTION api_ids_number_new_record() FROM PUBLIC;
REVOKE ALL ON FUNCTION api_ids_claim_harvest_id() FROM PUBLIC;

-- Backfill: imported records first with their Harvest ids, then every other
-- record in creation order from the next free number of its organization.
DO $$
DECLARE
  k record;
BEGIN
  FOR k IN SELECT * FROM (VALUES
    ('client', 'clients', 'client', 'r.created_at, r.id'),
    ('project', 'projects', 'project', 'r.created_at, r.id'),
    ('task', 'tasks', 'task', 'r.id'),  -- tasks have no created_at; ids are v7
    ('time_entry', 'time_entries', 'time_entry', 'r.created_at, r.id'),
    ('user', 'users', 'person', 'r.created_at, r.id')
  ) AS kinds(kind, tbl, harvest_type, creation_order)
  LOOP
    EXECUTE format($sql$
      INSERT INTO api_ids (org_id, kind, number, horae_id, from_harvest)
      SELECT DISTINCT ON (m.org_id, m.horae_id) m.org_id, %L, m.harvest_id, m.horae_id, true
      FROM harvest_import_map m JOIN %I r ON r.id = m.horae_id AND r.org_id = m.org_id
      WHERE m.harvest_entity_type = %L AND m.harvest_id > 0
      ORDER BY m.org_id, m.horae_id, m.created_at, m.harvest_id
    $sql$, k.kind, k.tbl, k.harvest_type);

    EXECUTE format($sql$
      INSERT INTO api_ids (org_id, kind, number, horae_id)
      SELECT r.org_id, %1$L,
             COALESCE(top.number, 0) + row_number() OVER (PARTITION BY r.org_id ORDER BY %3$s),
             r.id
      FROM %2$I r
      LEFT JOIN (SELECT org_id, max(number) AS number FROM api_ids WHERE kind = %1$L GROUP BY org_id) top
        ON top.org_id = r.org_id
      WHERE NOT EXISTS (
        SELECT 1 FROM api_ids a WHERE a.org_id = r.org_id AND a.kind = %1$L AND a.horae_id = r.id
      )
    $sql$, k.kind, k.tbl, k.creation_order);
  END LOOP;
END
$$;
