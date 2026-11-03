-- Harvest OAuth client saved from the importer, for deployments that do not set
-- HORAE_HARVEST_CLIENT_ID/SECRET (those still take precedence).
--
-- The secret is sealed like the tokens in harvest_credentials (AEAD under
-- HORAE_HARVEST_ENC_KEY, nonce prefix). The key is never stored. A NULL
-- redirect_url means the deployment supplies HORAE_HARVEST_REDIRECT_URL.

CREATE TABLE harvest_oauth_clients (
  id                uuid        PRIMARY KEY,
  org_id            uuid        NOT NULL UNIQUE REFERENCES organizations(id),
  client_id         text        NOT NULL CHECK (length(client_id) BETWEEN 1 AND 255),
  client_secret_enc bytea       NOT NULL,
  redirect_url      text,
  updated_by        uuid        REFERENCES users(id),
  created_at        timestamptz NOT NULL DEFAULT now(),
  updated_at        timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER harvest_oauth_clients_set_updated_at
  BEFORE UPDATE ON harvest_oauth_clients
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
