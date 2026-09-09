\set ON_ERROR_STOP on
-- Reproduce a prepared provenance lookup planned against a tiny table, then
-- reused after 100,000 rows arrive in the same transaction. No real data is
-- read; the temporary table retains both original indexes. Compare the actual
-- index predicates and rows removed by filter, not just one timing sample.
BEGIN;
CREATE TEMP TABLE import_plan_map (LIKE public.harvest_import_map INCLUDING ALL) ON COMMIT DROP;
INSERT INTO import_plan_map(org_id,harvest_entity_type,harvest_id,horae_id)
SELECT '01900000-0000-7000-8000-000000000001', 'time_entry', n,
       ('01900000-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid
FROM generate_series(1,3) n;
PREPARE map_lookup(uuid,text,bigint) AS
SELECT horae_id FROM import_plan_map WHERE org_id=$1
AND harvest_entity_type=$2::harvest_entity_type AND harvest_id=$3;
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
INSERT INTO import_plan_map(org_id,harvest_entity_type,harvest_id,horae_id)
SELECT '01900000-0000-7000-8000-000000000001', 'time_entry', n,
       ('01900000-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid
FROM generate_series(4,100000) n;
SELECT name,generic_plans,custom_plans FROM pg_prepared_statements;
EXPLAIN (ANALYZE, BUFFERS) EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
SET LOCAL plan_cache_mode = force_custom_plan;
EXPLAIN (ANALYZE, BUFFERS) EXECUTE map_lookup('01900000-0000-7000-8000-000000000001','time_entry',100001);
DEALLOCATE map_lookup;
ROLLBACK;
