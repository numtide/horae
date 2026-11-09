-- Number the records that exist when api_ids is deployed (FR-017).
--
-- No DDL here, so writes are not blocked while it runs: records created
-- meanwhile were already numbered by 0058's triggers, and every insert below
-- skips a record or number that is taken. Imported records first, with their
-- Harvest ids; then every other record in creation order from its kind's
-- sequence, skipping numbers Harvest ids hold.
DO $$
DECLARE
  k record;
  r record;
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
      ON CONFLICT DO NOTHING
    $sql$, k.kind, k.tbl, k.harvest_type);

    -- One set-based pass; a record whose number a Harvest id holds is left
    -- for the loop below.
    EXECUTE format($sql$
      INSERT INTO api_ids (org_id, kind, number, horae_id)
      SELECT org_id, %1$L, nextval(%4$L::regclass), id
      FROM (
        SELECT r.org_id, r.id FROM %2$I r
        WHERE NOT EXISTS (
          SELECT 1 FROM api_ids a WHERE a.org_id = r.org_id AND a.kind = %1$L AND a.horae_id = r.id
        )
        ORDER BY %3$s
      ) pending
      ON CONFLICT DO NOTHING
    $sql$, k.kind, k.tbl, k.creation_order, format('api_ids_%s_seq', k.kind));

    FOR r IN EXECUTE format($sql$
      SELECT r.org_id, r.id FROM %2$I r
      WHERE NOT EXISTS (
        SELECT 1 FROM api_ids a WHERE a.org_id = r.org_id AND a.kind = %1$L AND a.horae_id = r.id
      )
      ORDER BY %3$s
    $sql$, k.kind, k.tbl, k.creation_order)
    LOOP
      PERFORM api_id_allocate(r.org_id, k.kind::api_id_kind, r.id);
    END LOOP;
  END LOOP;
END
$$;
