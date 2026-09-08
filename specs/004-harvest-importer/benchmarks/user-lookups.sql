-- Run with psql -X -f against a migrated development database.
-- Only temporary copies are populated; ROLLBACK removes every probe object.
\set ON_ERROR_STOP on
BEGIN;
CREATE TEMP TABLE import_lookup_results (
    users integer, indexed boolean, field text, sample integer,
    plan text, execution_ms numeric, index_bytes bigint
) ON COMMIT DROP;

DO $$
DECLARE
    volume integer;
    indexed boolean;
    field text;
    sample integer;
    plan json;
    index_bytes bigint;
    probe_org uuid := '01960000-0000-7000-8000-000000000001';
BEGIN
    FOREACH volume IN ARRAY ARRAY[100, 1000, 50000] LOOP
        -- Baseline keeps ordinary uniqueness but excludes expression indexes,
        -- even when the development database has already applied migration 22.
        CREATE TEMP TABLE import_lookup_users (LIKE public.users INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP;
        ALTER TABLE pg_temp.import_lookup_users ADD PRIMARY KEY (id), ADD UNIQUE (email), ADD UNIQUE (oidc_subject);
        CREATE INDEX ON pg_temp.import_lookup_users (org_id);
        INSERT INTO pg_temp.import_lookup_users (id, org_id, email, name)
        SELECT ('01960000-0000-7000-8000-' || lpad(n::text, 12, '0'))::uuid,
               probe_org, 'person' || n || '@example.invalid', 'Person ' || n
        FROM generate_series(1, volume) n;
        FOREACH indexed IN ARRAY ARRAY[false, true] LOOP
            index_bytes := 0;
            IF indexed THEN
                CREATE INDEX import_lookup_email_idx ON pg_temp.import_lookup_users (org_id, public.harvest_norm(email));
                CREATE INDEX import_lookup_name_idx ON pg_temp.import_lookup_users (org_id, public.harvest_norm(name));
                index_bytes := pg_relation_size('pg_temp.import_lookup_email_idx') + pg_relation_size('pg_temp.import_lookup_name_idx');
            END IF;
            ANALYZE pg_temp.import_lookup_users;
            FOREACH field IN ARRAY ARRAY['email', 'name'] LOOP
                -- One warm-up and five measured samples for each plan.
                FOR sample IN 0..5 LOOP
                    EXECUTE format('EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
                        SELECT id FROM pg_temp.import_lookup_users
                        WHERE org_id = $1 AND public.harvest_norm(%I) = $2 LIMIT 2', field)
                    INTO plan USING probe_org, CASE field
                        WHEN 'email' THEN 'person' || volume || '@example.invalid'
                        ELSE 'person ' || volume END;
                    IF sample > 0 THEN
                        INSERT INTO pg_temp.import_lookup_results VALUES (
                            volume, indexed, field, sample,
                            plan->0->'Plan'->'Plans'->0->>'Node Type',
                            (plan->0->>'Execution Time')::numeric, index_bytes);
                    END IF;
                END LOOP;
            END LOOP;
        END LOOP;
        DROP TABLE pg_temp.import_lookup_users;
    END LOOP;
END $$;

SELECT users, indexed, field, plan,
       min(execution_ms) AS min_ms,
       percentile_cont(0.5) WITHIN GROUP (ORDER BY execution_ms) AS median_ms,
       max(execution_ms) AS max_ms,
       max(index_bytes) AS combined_index_bytes
FROM pg_temp.import_lookup_results
GROUP BY users, indexed, field, plan
ORDER BY users, indexed, field;
ROLLBACK;
