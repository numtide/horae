# Data Model: Harvest OAuth Client Settings

## Table `harvest_oauth_clients` (migration 0051)

| Column | Type | Rules |
|---|---|---|
| `id` | `uuid` | PK, UUIDv7 from the application |
| `org_id` | `uuid` | `NOT NULL UNIQUE REFERENCES organizations(id)`; one client per organization |
| `client_id` | `text` | `NOT NULL`, 1–255 characters |
| `client_secret_enc` | `bytea` | `NOT NULL`; `nonce(24) ‖ ciphertext` from `credentials::encrypt` |
| `redirect_url` | `text` | nullable; validated callback URL saved from the page |
| `updated_by` | `uuid` | `REFERENCES users(id)`; last administrator to save |
| `created_at`, `updated_at` | `timestamptz` | `DEFAULT now()`; `updated_at` maintained by `set_updated_at()` |

Removal deletes the row. History lives in `audit_log`.

## Environment: `HarvestDeployment` (read at startup)

- `client: Option<(client_id, client_secret)>`: set only when both `HORAE_HARVEST_CLIENT_ID` and `HORAE_HARVEST_CLIENT_SECRET` are non-empty.
- `redirect_url: Option<String>` from `HORAE_HARVEST_REDIRECT_URL`.
- `encryption_key_hex: Option<String>` from `HORAE_HARVEST_ENC_KEY`.

## Effective `HarvestConfig` (unchanged shape)

`resolve(org)` = `Some(HarvestConfig { client_id, client_secret, redirect_url, encryption_key_hex })` when the key is set, the client pair comes from the environment or a decryptable stored row, and the redirect URL comes from the environment or the stored row. Otherwise `None`.

## View `HarvestClientSettings` (core, serde)

| Field | Meaning |
|---|---|
| `source` | `Environment` / `Stored` / `None` |
| `client_id` | effective or stored client ID, if any |
| `secret_set` | a secret exists for `source` |
| `secret_readable` | false when the stored secret no longer decrypts |
| `stored_at` | `updated_at` of the stored row (RFC 3339), if any |
| `redirect_url` | the environment value, else the stored value |
| `redirect_from_environment` | redirect URL is locked by the environment |
| `encryption_key_set` | the key is configured |
| `stored_ignored` | a stored row exists while the environment client is in effect |

It never carries a secret. `editable = encryption_key_set && source != Environment`.

## Save input and rules (core)

`SaveClient { client_id, client_secret: Option<String>, redirect_url: String }`, trimmed.

- `client_id`: 1–255 characters, no whitespace.
- `client_secret`: when present, 1–1024 characters with no surrounding whitespace. Required when no row exists, when the client ID changes, or when the stored secret is unreadable.
- `redirect_url`: required unless the environment sets one (then ignored and stored as `NULL`). Absolute `http`/`https`, a host, no user info, query or fragment, path exactly `/auth/harvest/callback`.
- Disconnects when a row exists and `client_id` differs, and always on removal (when tokens exist).

## State transitions (per organization)

```text
None --save--> Stored --save(same id, new secret)--> Stored (connection kept)
Stored --save(new id)--> Stored (tokens deleted, binding kept)
Stored --remove--> None (tokens deleted, binding kept)
any --(env pair set)--> Environment (save/remove refused)
```

Every transition bumps `harvest_connection_generations.connection_revision` and writes one `audit_log` row. All transitions are refused while a Harvest import is queued or running, or while the import lock is held.
