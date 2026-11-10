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

Tokens belong to the account (plan.md *Layering*): each request names its organization with
`Harvest-Account-Id`, which resolves to the account's membership there. While an account is a
`users` row with one membership, `org_id` is that membership's organization; step 2 revisits the
column against 020's accounts and memberships.

A token authenticates only while: not revoked, not expired, and its owner is active in `org_id`.
Its permissions are read from the owner's current role and grants on every request; nothing is
copied onto the token.

### `harvest_id` columns (delivery step 1)

`harvest_id bigint NOT NULL` on `clients`, `projects`, `tasks`, `time_entries` and `users`, unique
per `(org_id, harvest_id)`: **the id in the Harvest-compatible API; Horae's identity is `id`
(UUID v7).** It is the `id` a client of `/harvest/v2` sees for every record, native or
imported, and the number it sends back in paths and filters. Later steps add the column to the
kinds they expose (assignments, invoices, invoice lines, tags).

- **Native records**: the column has no default. A `BEFORE INSERT` trigger
  (`harvest_id_assign`) fills it when the insert gives none, from one global sequence per kind
  (`<table>_harvest_id_seq`), skipping a number another record of the kind in the organization
  has or that `harvest_import_map` holds as a Harvest id. `nextval` takes no lock and ignores the
  snapshot, so creators never wait on each other at any isolation level. Numbers are unique per
  organization and kind (FR-015); they are distinct across organizations too, and do not start
  at 1 in each.
- **Imported records**: the importer passes the record's Harvest id in its creating insert when
  no record of the kind in the organization has it. When one does, it passes nothing (the
  trigger numbers the record) and the import report gets an `IdNotice` (FR-016). A record the
  import only matches keeps its own number.
- **A number never names another record.** Sequence numbers are never handed out twice, and a
  Harvest id in the map stays reserved after its record is deleted, so only that record can come
  back with it.

Migrations: `0058` renames `harvest_import_map.harvest_id` to `harvest_import_id` (the id the
record had in Harvest), so the two never read alike. `0059` adds the columns, sequences, the
trigger and plain lookup indexes; the unique indexes come only after the backfill, because an
`UPDATE` of a column in a unique index takes `FOR UPDATE` row locks, which wait for the
`FOR KEY SHARE` locks of every insert's foreign-key checks: with them the backfill stopped new
time entries and deadlocked with them. `0059` also makes `set_updated_at` and the project edit
revision trigger skip a session that sets `horae.backfill`, so numbering is not an edit (the
importer compares `updated_at` to detect edits made in Horae). `0060` backfills without a
transaction, in batches of 10,000 rows in id order (UUID v7, creation order), each committed
with `horae.backfill` set locally; per batch, imported records take their Harvest id, the rest
the next sequence numbers, a record whose turn falls on a held number the next free one. A rerun
resumes. `0061` adds `CHECK (harvest_id IS NOT NULL) NOT VALID`; `0062` validates it (no write
lock), builds the unique indexes (writes to the table wait for the build), drops the lookup
indexes, sets `NOT NULL` from the valid check without a scan and drops the check.

Measured on 1M time entries (200 people, 5,000 projects, 100,000 imported): `0059` 0.26 s,
`0060` 114 s, `0061` 0.04 s, `0062` 0.73 s. During `0060` a read, an edit of an old entry and a
new entry each waited at most 0.3–0.7 s (one batch); during `0062` an edit or insert waited at
most 0.56 s.

## Changed tables

| Table | Change | Step |
| --- | --- | --- |
| `organizations` | `api_account_id bigint NOT NULL UNIQUE` from sequence `organizations_api_account_id_seq` (OQ-3) | 1 |
| `tasks` | `created_at`, `updated_at timestamptz NOT NULL DEFAULT now()`, trigger `set_updated_at` | 4 |
| `projects`, `clients`, `users` | `updated_at timestamptz NOT NULL DEFAULT now()`, backfilled from `created_at`, trigger `set_updated_at` | 4 |
| `harvest_import_map` | `harvest_id` renamed to `harvest_import_id`; read by the backfill and the trigger | 1 |
| `clients`, `projects`, `tasks`, `time_entries`, `users` | `harvest_id bigint NOT NULL`, unique per organization (above) | 1 |

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
