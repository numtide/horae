-- no-transaction
-- Number the records that exist when `harvest_id` is deployed (FR-017).
--
-- In batches of 10,000 rows in id order, each committed on its own. Horae
-- creates its ids as UUID v7, so that is creation order; a row with another
-- kind of UUID (an old seed or fixture) is numbered by its id, not its age.
-- Each batch is committed so a row stays locked only for its batch and reads,
-- inserts and edits continue. No DDL. Records created meanwhile were already
-- numbered by 0059's trigger; a rerun after a failure resumes where it stopped.
-- In each batch: imported records take their Harvest id; then the rest take
-- the kind's next sequence numbers in order, skipping one another record has
-- or the import map holds; a record whose turn fell on such a number takes the
-- next free one.
--
-- One statement, so the DO block runs outside a transaction block and may
-- commit.
DO $$
DECLARE
  k record;
  batch uuid[];
  last_id uuid;
BEGIN
  FOR k IN SELECT * FROM (VALUES
    ('clients', 'client'),
    ('projects', 'project'),
    ('tasks', 'task'),
    ('time_entries', 'time_entry'),
    ('users', 'person')
  ) AS kinds(tbl, harvest_type)
  LOOP
    last_id := '00000000-0000-0000-0000-000000000000';
    LOOP
      EXECUTE format(
        'SELECT array_agg(id ORDER BY id) FROM (
           SELECT id FROM %I WHERE id > $1 AND harvest_id IS NULL ORDER BY id LIMIT 10000
         ) b', k.tbl)
        INTO batch USING last_id;
      EXIT WHEN batch IS NULL;
      last_id := batch[array_upper(batch, 1)];
      -- Local to this batch's transaction, so it never outlives the backfill.
      PERFORM set_config('horae.backfill', 'on', true);

      EXECUTE format($sql$
        UPDATE %1$I r SET harvest_id = m.harvest_import_id
        FROM (
          SELECT DISTINCT ON (org_id, horae_id) org_id, horae_id, harvest_import_id
          FROM harvest_import_map
          WHERE harvest_entity_type = %2$L AND harvest_import_id > 0 AND horae_id = ANY($1)
          ORDER BY org_id, horae_id, created_at, harvest_import_id
        ) m
        WHERE r.id = m.horae_id AND r.org_id = m.org_id AND r.harvest_id IS NULL
          AND NOT EXISTS (
            SELECT 1 FROM %1$I o WHERE o.org_id = r.org_id AND o.harvest_id = m.harvest_import_id
          )
      $sql$, k.tbl, k.harvest_type) USING batch;

      EXECUTE format($sql$
        UPDATE %1$I r SET harvest_id = n.number
        FROM (
          SELECT id, nextval(%3$L::regclass) AS number
          FROM (
            SELECT id FROM %1$I WHERE id = ANY($1) AND harvest_id IS NULL ORDER BY id
          ) pending
        ) n
        WHERE r.id = n.id
          AND NOT EXISTS (
            SELECT 1 FROM harvest_import_map m
            WHERE m.org_id = r.org_id AND m.harvest_entity_type = %2$L
              AND m.harvest_import_id = n.number
          )
          AND NOT EXISTS (SELECT 1 FROM %1$I o WHERE o.org_id = r.org_id AND o.harvest_id = n.number)
      $sql$, k.tbl, k.harvest_type, k.tbl || '_harvest_id_seq') USING batch;

      EXECUTE format($sql$
        UPDATE %1$I SET harvest_id = harvest_id_next(%1$L::regclass, %3$L::regclass, %2$L, org_id)
        WHERE id = ANY($1) AND harvest_id IS NULL
      $sql$, k.tbl, k.harvest_type, k.tbl || '_harvest_id_seq') USING batch;

      COMMIT;
    END LOOP;
  END LOOP;
END
$$;
