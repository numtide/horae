-- OAuth 2.0 with PKCE (spec 024, data-model.md): apps, the authorizations
-- people give them, and the credentials issued under each. Every credential is
-- stored only as the SHA-256 digest of the whole string.

CREATE TYPE oauth_client_kind AS ENUM ('public', 'confidential');

CREATE TYPE oauth_revoke_reason AS ENUM (
    'person', 'administrator', 'client', 'refresh_reuse', 'code_reuse', 'client_deleted'
);

CREATE TABLE oauth_clients (
    id uuid PRIMARY KEY,
    -- Cascades because every organization gets a built-in client it never
    -- asked for, which must not keep the organization from being deleted.
    org_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    client_id text NOT NULL,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100 AND name = btrim(name)),
    kind oauth_client_kind NOT NULL,
    secret_sha256 bytea CHECK (octet_length(secret_sha256) = 32),
    secret_prefix text,
    redirect_uris text[] NOT NULL CHECK (cardinality(redirect_uris) BETWEEN 1 AND 10),
    scopes text[] NOT NULL CHECK (
        cardinality(scopes) >= 1 AND scopes <@ ARRAY['time:read', 'time:write', 'catalog:read']
    ),
    built_in boolean NOT NULL DEFAULT false,
    enabled boolean NOT NULL DEFAULT true,
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    -- A secret exactly for a confidential app, and its display prefix with it.
    CHECK ((kind = 'confidential') = (secret_sha256 IS NOT NULL)),
    CHECK ((secret_sha256 IS NULL) = (secret_prefix IS NULL)),
    -- The built-in client's id is the same fixed string in every organization.
    UNIQUE (org_id, client_id),
    UNIQUE (org_id, name),
    UNIQUE (org_id, id)
);

CREATE UNIQUE INDEX oauth_clients_one_built_in_idx ON oauth_clients (org_id) WHERE built_in;

CREATE TRIGGER oauth_clients_set_updated_at BEFORE UPDATE ON oauth_clients
    FOR EACH ROW WHEN (OLD.* IS DISTINCT FROM NEW.*) EXECUTE FUNCTION set_updated_at();

CREATE TABLE oauth_grants (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL,
    client_id uuid NOT NULL,
    user_id uuid NOT NULL,
    scopes text[] NOT NULL CHECK (
        cardinality(scopes) >= 1 AND scopes <@ ARRAY['time:read', 'time:write', 'catalog:read']
    ),
    approved_at timestamptz NOT NULL DEFAULT now(),
    last_used_at timestamptz,
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    revoked_by uuid REFERENCES users (id) ON DELETE SET NULL,
    revoke_reason oauth_revoke_reason,
    CHECK ((revoked_at IS NULL) = (revoke_reason IS NULL)),
    CHECK (revoked_by IS NULL OR revoked_at IS NOT NULL),
    FOREIGN KEY (org_id, client_id) REFERENCES oauth_clients (org_id, id) ON DELETE CASCADE,
    FOREIGN KEY (org_id, user_id) REFERENCES users (org_id, id) ON DELETE CASCADE,
    UNIQUE (org_id, id)
);

CREATE INDEX oauth_grants_org_user_idx ON oauth_grants (org_id, user_id);
CREATE INDEX oauth_grants_client_idx ON oauth_grants (client_id);

CREATE TABLE oauth_codes (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL,
    grant_id uuid NOT NULL,
    code_sha256 bytea NOT NULL UNIQUE CHECK (octet_length(code_sha256) = 32),
    redirect_uri text NOT NULL,
    -- S256 only: BASE64URL(SHA256(verifier)) is always 43 characters.
    code_challenge text NOT NULL CHECK (code_challenge ~ '^[A-Za-z0-9_-]{43}$'),
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    FOREIGN KEY (org_id, grant_id) REFERENCES oauth_grants (org_id, id) ON DELETE CASCADE
);

CREATE INDEX oauth_codes_grant_idx ON oauth_codes (grant_id);

CREATE TABLE oauth_access_tokens (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL,
    grant_id uuid NOT NULL,
    token_sha256 bytea NOT NULL UNIQUE CHECK (octet_length(token_sha256) = 32),
    scopes text[] NOT NULL CHECK (
        cardinality(scopes) >= 1 AND scopes <@ ARRAY['time:read', 'time:write', 'catalog:read']
    ),
    expires_at timestamptz NOT NULL,
    FOREIGN KEY (org_id, grant_id) REFERENCES oauth_grants (org_id, id) ON DELETE CASCADE
);

CREATE INDEX oauth_access_tokens_grant_idx ON oauth_access_tokens (grant_id);

CREATE TABLE oauth_refresh_tokens (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL,
    grant_id uuid NOT NULL,
    token_sha256 bytea NOT NULL UNIQUE CHECK (octet_length(token_sha256) = 32),
    scopes text[] NOT NULL CHECK (
        cardinality(scopes) >= 1 AND scopes <@ ARRAY['time:read', 'time:write', 'catalog:read']
    ),
    idle_expires_at timestamptz NOT NULL,
    used_at timestamptz,
    FOREIGN KEY (org_id, grant_id) REFERENCES oauth_grants (org_id, id) ON DELETE CASCADE
);

CREATE INDEX oauth_refresh_tokens_grant_idx ON oauth_refresh_tokens (grant_id);

-- Every organization has exactly one built-in client for Horae's Android app,
-- disabled until an administrator enables it (spec D-1). The trigger covers
-- every way an organization is created (`horae init`, the demo seed, the test
-- fixtures); the INSERT below covers the organizations that already exist. The
-- values are `ops::oauth::clients::BUILT_IN_*`; the redirect scheme is a
-- placeholder until the app's package name is fixed, and changing it is a
-- migration that updates these rows and this function.
CREATE FUNCTION oauth_insert_built_in_client(organization uuid) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO oauth_clients (id, org_id, client_id, name, kind, redirect_uris, scopes,
                               built_in, enabled)
    VALUES (
        -- A UUID v7: the millisecond clock over a random v4, version bits set to 7.
        encode(set_bit(set_bit(overlay(uuid_send(gen_random_uuid())
            PLACING substring(int8send(floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint)
                              FROM 3)
            FROM 1 FOR 6), 52, 1), 53, 1), 'hex')::uuid,
        organization,
        'horae-android',
        'Horae for Android',
        'public',
        ARRAY['dev.horae.android:/oauth/callback'],
        ARRAY['time:read', 'time:write', 'catalog:read'],
        true,
        false
    )
$$;

CREATE FUNCTION oauth_organization_created() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM oauth_insert_built_in_client(NEW.id);
    RETURN NULL;
END
$$;

-- Like every internal function, out of reach of the plugin database role.
REVOKE ALL ON FUNCTION oauth_insert_built_in_client(uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION oauth_organization_created() FROM PUBLIC;

CREATE TRIGGER organizations_oauth_built_in_client AFTER INSERT ON organizations
    FOR EACH ROW EXECUTE FUNCTION oauth_organization_created();

SELECT oauth_insert_built_in_client(id) FROM organizations;
