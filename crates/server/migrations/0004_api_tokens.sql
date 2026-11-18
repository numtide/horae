-- Personal access tokens (spec 022, data-model `api_tokens`). Only the
-- SHA-256 digest of a token is stored; the token itself is shown once.
CREATE TABLE api_tokens (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations (id),
    user_id uuid NOT NULL,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    token_sha256 bytea NOT NULL UNIQUE CHECK (octet_length(token_sha256) = 32),
    display_prefix text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz,
    last_used_at timestamptz,
    revoked_at timestamptz,
    -- The owner belongs to the token's organization.
    FOREIGN KEY (org_id, user_id) REFERENCES users (org_id, id) ON DELETE CASCADE
);

CREATE INDEX api_tokens_org_user_idx ON api_tokens (org_id, user_id);
CREATE INDEX api_tokens_org_idx ON api_tokens (org_id);
