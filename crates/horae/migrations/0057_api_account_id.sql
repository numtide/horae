-- The Harvest-compatible API's account id (specs/021-harvest-api-compat, FR-010).
--
-- On its own: adding an identity column rewrites organizations under ACCESS
-- EXCLUSIVE, so this commits before the api_ids backfill (0059) starts.
ALTER TABLE organizations ADD COLUMN api_account_id bigint GENERATED ALWAYS AS IDENTITY UNIQUE;
