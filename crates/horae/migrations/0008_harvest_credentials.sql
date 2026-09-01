-- Harvest OAuth2 connection credentials (data-model.md, FR-022/FR-024/FR-025).
-- Additive: introduces a new table only.
--
-- One row per organization (v1 supports a single connected Harvest account).
-- Access and refresh tokens are stored encrypted at rest (AEAD, deployment key);
-- they are never returned to the browser or logged. `synced_watermark` holds the
-- per-entity `updated_since` high-water marks for incremental re-sync and is
-- advanced only after a successful committing run.

CREATE TABLE harvest_credentials (
  id                 uuid        PRIMARY KEY,
  org_id             uuid        NOT NULL UNIQUE REFERENCES organizations(id),
  harvest_account_id text        NOT NULL,
  access_token_enc   bytea       NOT NULL,
  refresh_token_enc  bytea       NOT NULL,
  token_expires_at   timestamptz,
  scope              text,
  synced_watermark   jsonb       NOT NULL DEFAULT '{}'::jsonb,
  created_at         timestamptz NOT NULL DEFAULT now(),
  updated_at         timestamptz NOT NULL DEFAULT now()
);
