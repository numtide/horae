# Data model: OAuth 2.0 with PKCE

Every new table has a UUID v7 primary key and `org_id` (Principle III). One migration,
`crates/server/migrations/0005_oauth.sql` (the next free number when PR B lands); the baseline is
never edited. Credentials are stored as SHA-256 digests only (FR-050), like `api_tokens`.

## `oauth_clients` (an app)

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid FK `organizations` | |
| `client_id` | text UNIQUE | `horae_app_` + 16 random bytes base64url; `horae-android` for the built-in client (research R-5) |
| `name` | text | 1–100 characters after trimming; shown on the approval page |
| `kind` | `oauth_client_kind` enum | `public` or `confidential` |
| `secret_sha256` | bytea NULL | set exactly when `kind = 'confidential'` (CHECK) |
| `secret_prefix` | text NULL | first 8 characters after `horae_ocs_`, for display |
| `redirect_uris` | text[] | 1–10 entries, each valid for `kind` (FR-011, checked in `horae-core`) |
| `scopes` | text[] | non-empty subset of `time:read`, `time:write`, `catalog:read` |
| `built_in` | boolean | true only for the shipped Android client; its `client_id`, `kind` and `redirect_uris` cannot change |
| `enabled` | boolean | built-in client starts `false` (OQ-1 (a)); registered apps start `true` |
| `created_by` | uuid NULL FK `users` | the registering administrator; NULL for the built-in client |
| `created_at`, `updated_at` | timestamptz | `set_updated_at()` trigger |

Unique `(org_id, name)` so people can tell apps apart. The built-in client row is created by the
migration for each existing organization and by `init` for new ones (`ops::organization` creation
path), so every organization has exactly one, disabled.

**States**: `enabled ⇄ disabled`; `delete` removes the row and cascades to grants, codes and
tokens. The built-in client cannot be deleted, only disabled.

## `oauth_grants` (an authorization)

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid | |
| `client_id` | uuid FK `oauth_clients(id)` ON DELETE CASCADE | |
| `user_id` | uuid | `FOREIGN KEY (org_id, user_id) REFERENCES users (org_id, id) ON DELETE CASCADE` |
| `scopes` | text[] | what the person approved; subset of the client's scopes at approval |
| `approved_at` | timestamptz | `now()` |
| `last_used_at` | timestamptz NULL | stamped at most once a minute by authentication or refresh (FR-053) |
| `expires_at` | timestamptz | `approved_at` + absolute refresh limit (1 year, OQ-2 (a)) |
| `revoked_at` | timestamptz NULL | set once |
| `revoked_by` | uuid NULL | the person or administrator; NULL when the app or Horae revoked |
| `revoke_reason` | `oauth_revoke_reason` NULL | `person`, `administrator`, `client`, `refresh_reuse`, `code_reuse`, `client_deleted` |

Indexes: `(org_id, user_id)`, `(client_id)`.

**Active while** `revoked_at IS NULL`, `expires_at > now()`, the client is enabled and the person is
active in `org_id`.

```text
active ──revoke (person, administrator, client)──▶ revoked (final)
   ├──refresh or code reuse──────────────────────▶ revoked (final)
   ├──expires_at passes──────────────────────────▶ expired (final)
   └──client disabled / person deactivated──────▶ suspended (no state change; works again when undone, until expiry)
```

## `oauth_codes`

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid | |
| `grant_id` | uuid FK `oauth_grants` ON DELETE CASCADE | the grant created by the approval |
| `code_sha256` | bytea UNIQUE | |
| `redirect_uri` | text | exactly as sent in the request (loopback port included) |
| `code_challenge` | text | 43 characters base64url (S256 only) |
| `expires_at` | timestamptz | issue + 60 s |
| `used_at` | timestamptz NULL | set by the exchange; a second use revokes the grant |

Rows older than a day are deleted by the existing scheduler's cleanup pass.

## `oauth_access_tokens`

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7; the `token_id` of `Channel::OAuth` |
| `org_id` | uuid | |
| `grant_id` | uuid FK `oauth_grants` ON DELETE CASCADE | |
| `token_sha256` | bytea UNIQUE | |
| `scopes` | text[] | subset of the grant's (a refresh may narrow, FR-045) |
| `expires_at` | timestamptz | issue + 1 hour |

Expired rows are deleted by the scheduler after a day. No `revoked_at`: revocation is on the grant.

## `oauth_refresh_tokens`

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid | |
| `grant_id` | uuid FK `oauth_grants` ON DELETE CASCADE | |
| `token_sha256` | bytea UNIQUE | |
| `scopes` | text[] | carried into the next access token |
| `idle_expires_at` | timestamptz | issue + 30 days (OQ-2 (a)); never after the grant's `expires_at` |
| `used_at` | timestamptz NULL | set when exchanged; a token with `used_at` set that is presented again revokes the grant (R-8) |

## Session data (no table)

| Key | Value | Lifetime |
| --- | --- | --- |
| `oauth_requests` | map of request id (128-bit random, base64url) → `{client_id, redirect_uri, scopes, state, code_challenge, created_at}`; at most 5, oldest dropped | 10 minutes each (FR-027) |
| `return_to` | a path starting with `/oauth/` | until used or the session ends |

## Domain types (`horae-core::permissions::oauth`)

- `Scope { TimeRead, TimeWrite, CatalogRead }` with `as_str`/`parse` (`time:read`, …).
- `ScopeSet`: a `Copy` bit set; `parse_list("time:read catalog:read")`, `display()`, `contains`,
  `is_subset`, `normalize()` (`time:write` implies `time:read`), and `ceiling() -> PermissionSelection` (research R-6).
- `RedirectUri::parse(&str, ClientKind) -> Result<RedirectUri, RedirectUriError>` and
  `RedirectUri::matches(&self, requested: &str) -> bool` (exact, or loopback with any port).
- `pkce::verify(verifier: &str, challenge: &str) -> bool` (length and charset checks, S256,
  constant-time comparison).

## Changed types

- `ops::Channel::OAuth { grant_id: Uuid, token_id: Uuid, scopes: ScopeSet }`.
- `Actor`: for `Channel::OAuth`, grants are `stored ∩ scopes.ceiling()` and `administrator()` is
  `false`. `Actor::scopes() -> Option<ScopeSet>` (`None` for every other channel).
- `ops::api_tokens::authenticate` is joined by `ops::oauth::tokens::authenticate(pool, token) -> Option<Caller>`; `horae_api::token::ApiCaller` dispatches on the prefix (`horae_pat_` or
  `horae_oat_`).
- `horae_api::rate_limit::limit`: the key is `token_id` for personal tokens and `grant_id` for
  OAuth callers.

## Page DTOs (`horae-types::oauth`)

- `ConsentDetails { request_id, app_name, built_in, scopes: Vec<String>, return_to_display, person_name, organization_name }`.
- `AppRow { id, client_id, name, kind, scopes, redirect_uris, built_in, enabled, secret_prefix, active_grants, created_at }`, `NewApp`, `AppChanges`, `CreatedApp { app, secret: Option<String> }`.
- `GrantRow { id, app_name, client_id, person_id, person_name, scopes, approved_at, last_used_at, expires_at }`.

## Configuration (no storage)

See [contracts/configuration.md](contracts/configuration.md).
