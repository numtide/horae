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
  no record of the kind in the organization has it and it is more than 10,000 above the kind's
  sequence (`pg_sequence_last_value`). Otherwise it passes nothing (the trigger numbers the
  record) and the import report gets an `IdNotice` (FR-016). A number at or below the sequence
  may belong to a deleted record; the 10,000 margin means a native insert cannot reach the number
  while the import holds it uncommitted. The insert is `ON CONFLICT (org_id, harvest_id) DO NOTHING`; if another record took the number meanwhile, it is repeated without one. A record
  the import only matches keeps its own number. The report keeps the first 20 notices and counts
  the rest; a preview's notices carry no new id, because it is only known on import.
- **A number never names another record.** Sequence numbers are never handed out twice, and a
  Harvest id in the map stays reserved after its record is deleted, so only that record can come
  back with it. The importer never takes a number the sequence has handed out, so a deleted
  native record's number stays unused too.

Migrations: `0059` renames `harvest_import_map.harvest_id` to `harvest_import_id` (the id the
record had in Harvest), so the two never read alike. `0060` adds the columns, sequences, the
trigger and plain lookup indexes; the unique indexes come only after the backfill, because an
`UPDATE` of a column in a unique index takes `FOR UPDATE` row locks, which wait for the
`FOR KEY SHARE` locks of every insert's foreign-key checks: with them the backfill stopped new
time entries and deadlocked with them. `0060` also makes `set_updated_at` and the project edit
revision trigger skip a session that sets `horae.backfill`, so numbering is not an edit (the
importer compares `updated_at` to detect edits made in Horae). `0061` backfills without a
transaction, in batches of 10,000 rows in id order (UUID v7, creation order), each committed
with `horae.backfill` set locally; per batch, imported records take their Harvest id, the rest
the next sequence numbers, a record whose turn falls on a held number the next free one. A rerun
resumes. `0062` adds `CHECK (harvest_id IS NOT NULL) NOT VALID`; `0063` validates it (no write
lock), builds the unique indexes, drops the lookup indexes, sets `NOT NULL` from the valid check
without a scan, drops the check and restores `set_updated_at` and the edit revision trigger to
their original bodies, so `horae.backfill` no longer means anything. The file is one transaction:
writes to the five tables wait for all of it. It first names any duplicate number (table,
organization, number) with the statement that fixes it; sqlx then refuses older images, so the
fix is forward.

Measured on 1M time entries (200 people, 5,000 projects, 100,000 imported): `0060` 0.26 s,
`0061` 114 s, `0062` 0.04 s, `0063` 0.73 s. During `0061` a read, an edit of an old entry and a
new entry each waited at most 0.3–0.7 s (one batch); during `0063` an edit or insert waited at
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

## Step 5: numbers for assignments, tags and invoices

Step 5 exposes six more kinds, and FR-015 needs each record to have its own number. It reuses
step 1's design: a `harvest_id bigint` column, a global sequence per kind, the
`harvest_id_assign` trigger in the creating transaction, and unique `(org_id, harvest_id)`.

| Table | API kind | `harvest_entity_type` | Missing today |
| --- | --- | --- | --- |
| `assignments` | user assignment | `user_assignment` | `org_id`, `harvest_id`, `updated_at` |
| `project_tasks` | task assignment | `task_assignment` | `org_id`, `harvest_id`, `created_at`, `updated_at` (and a UUID: its key is `(project_id, task_id)`) |
| `project_tags` | tag | `tag` | `harvest_id`, `created_at`, `updated_at` |
| `invoices` | invoice | `invoice` (spec 025 adds it) | `harvest_id`, `updated_at` |
| `invoice_line_items` | invoice line item | `invoice_line` | `org_id`, `harvest_id` |
| `harvest_invoice_lines` (spec 025) | invoice line item | `invoice_line` | `harvest_id` |

- **Enum values** go in their own migration before the rest: PostgreSQL refuses to use an enum
  value in the transaction that adds it.
- **`org_id`** is added where missing, backfilled from the parent (project or invoice), `NOT NULL`
  with a foreign key to `organizations`. A `BEFORE INSERT` trigger fills it from the parent when
  the insert leaves it out, so the existing inserts (operations, importer, seed and many tests)
  need no change; the `harvest_id` trigger runs after it and reads `NEW.org_id`.
- **Invoice lines** are one kind in two tables. Both use one sequence, and their trigger checks
  both tables and `harvest_import_map` before taking a number, so a native and an imported line
  never share one. Line items have no timestamps on Harvest, so neither table gains any.
- **Imported records** follow step 1's rule. The invoice import (spec 025) passes the Harvest
  invoice id and each line's Harvest id as `harvest_id` when no record of the kind in the
  organization has it and it is more than 10,000 above the sequence; otherwise the trigger
  numbers the record and the report gets an `IdNotice`. The importer creates assignments and
  project tasks from time entries without reading Harvest's assignment ids, so those are always
  numbered by the trigger. Tags are not imported.
- **Update times** use `set_updated_at`, stamped at write time (step 4's
  `0006_updated_at_at_write.sql`). An invoice's `updated_at` also moves when one of its lines is
  inserted, changed or deleted, because the lines are part of the invoice object. As in step 4,
  a name shown from another record (a project's name on an assignment) does not move the time.
  Backfilled `created_at` and `updated_at` take the migration's time where the table has no
  earlier one (`project_tasks`, `project_tags`) and `created_at` elsewhere.
- **Backfill** numbers rows in creation order (`id`, UUID v7; `project_tasks` by
  `(project_id, task_id)`, having no creation time), imported invoices first with their Harvest
  ids from `harvest_import_map`, and imported lines with `harvest_line_id`. The unique indexes
  are built after the backfill, for the reason step 1 found (an `UPDATE` of a column in a unique
  index waits for the foreign-key locks of concurrent inserts). These tables are small next to
  `time_entries` except `invoice_line_items`, which has one row per invoiced entry; time the
  migration on a copy with 1,000,000 invoiced entries and record it here. If the line backfill
  holds writes for more than about a second, batch it as step 1's backfill did.
- **Keys for cursors.** `assignments`, `project_tags` and `invoices` are keyed by their UUID like
  step 4's lists. `project_tasks` has no UUID, so task assignment lists are keyed by
  `harvest_id`, which also never changes.

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
