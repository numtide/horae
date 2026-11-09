# Data model: Harvest API v2 compatibility

Every new table has a UUID v7 primary key and `org_id` (Principle III). Durations stay integer
minutes and money integer minor units (Principle I); the API converts at the edge (research R-4).

## New tables

### `api_tokens` (delivery step 2)

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid FK organizations | the token's organization |
| `user_id` | uuid FK users | the owner; the token acts as this person (FR-003) |
| `name` | text | 1–100 characters, chosen by the owner |
| `token_sha256` | bytea UNIQUE | SHA-256 of the full token string; the token itself is never stored (FR-002) |
| `display_prefix` | text | first 8 characters after `horae_pat_`, to tell tokens apart |
| `created_at` | timestamptz | |
| `expires_at` | timestamptz NULL | optional; past it the token is refused (FR-004) |
| `last_used_at` | timestamptz NULL | updated at most once a minute per token (FR-005) |
| `revoked_at` | timestamptz NULL | set once; a revoked token is refused |

A token authenticates only while: not revoked, not expired, and its owner is active in `org_id`.
Its permissions are read from the owner's current role and grants on every request; nothing is
copied onto the token.

### `api_ids` (delivery step 1)

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid FK organizations | |
| `kind` | `api_id_kind` enum | `client`, `project`, `task`, `time_entry`, `user`, `project_assignment`, `task_assignment`, `invoice`, `invoice_line_item`, `tag` |
| `number` | bigint | > 0 |
| `horae_id` | uuid | the record |
| `from_harvest` | boolean | true when `number` is the record's Harvest id (FR-016) |
| `created_at` | timestamptz | |

Unique `(org_id, kind, number)` and unique `(org_id, kind, horae_id)`. A number is never reused,
also after the record is deleted: the row stays and a deleted record's number is not handed out
again. New numbers are `max(number) + 1` per organization and kind, allocated under a
transaction-level advisory lock on `(org_id, kind)` in the transaction that creates the record.

Backfill (migration): for each kind, insert the `harvest_import_map` rows with
`from_harvest = true`, then every other record by `(created_at, id)` from the next free number.
`user_assignments` and `task_assignments` have no Harvest ids in the map and are numbered fresh.

## Changed tables

| Table | Change | Step |
| --- | --- | --- |
| `organizations` | `api_account_id bigint NOT NULL UNIQUE` from sequence `organizations_api_account_id_seq` (OQ-3) | 1 |
| `tasks` | `created_at`, `updated_at timestamptz NOT NULL DEFAULT now()`, trigger `set_updated_at` | 4 |
| `projects`, `clients`, `users` | `updated_at timestamptz NOT NULL DEFAULT now()`, backfilled from `created_at`, trigger `set_updated_at` | 4 |
| `harvest_import_map` | unchanged; read by the backfill and by the importer's id reuse | 1 |

The migration snapshot tests compare whole rows; the new columns are excluded there.

## Domain changes (no schema)

- `TimesheetCommand::RestartTimer { entry_id }` (FR-032). Allowed when the entry belongs to the
  subject, is not running, is open (not submitted, approved or invoiced) and is dated today in the
  organization's time zone; otherwise refused with a conflict. Harvest's exact rule for
  restarting a past day's entry is confirmed by recording (research R-11) before release.
- `StartTimer` and `RestartTimer` stop the subject's running timer in the same transaction,
  instead of refusing (D-6). Stopping uses the same rule as `StopTimer`.

## State: token

```text
active ──revoke──▶ revoked
   │
   └──expires_at passes──▶ expired
```

Both end states are final. Deactivating the owner makes every token refuse without changing its
state; reactivating the owner makes the unexpired, unrevoked tokens work again.

## OAuth2 (delivery step 7, OQ-1)

Designed when step 7 starts: `oauth_clients` (name, redirect URLs, secret hash, registering
administrator) and `oauth_grants` (person, client, organization, hashed single-use code, hashed
access and refresh tokens with expiry and rotation). Access tokens resolve through the same
extractor as personal tokens.
