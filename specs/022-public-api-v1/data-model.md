# Data model: Public API `/api/v1`

Every new table has a UUID v7 primary key and `org_id` (Principle III). Durations stay integer
minutes and money integer minor units (Principle I); `/api/v1` exposes them as they are stored
(research R-8). New schema goes in `crates/server/migrations/0002_*.sql` onwards; the baseline is
never edited.

## New table: `api_tokens` (PR C)

021's design ([021 data-model](../021-harvest-api-compat/data-model.md#api_tokens-delivery-step-2)),
unchanged, shared by both APIs.

| Column | Type | Rule |
| --- | --- | --- |
| `id` | uuid PK | v7 |
| `org_id` | uuid FK `organizations` | the owner's organization |
| `user_id` | uuid FK `users` | the owner; the token acts as this person (FR-003) |
| `name` | text | 1–100 characters after trimming |
| `token_sha256` | bytea UNIQUE | SHA-256 of the whole token string; the token is never stored (FR-002) |
| `display_prefix` | text | the first 8 characters after `horae_pat_` |
| `created_at` | timestamptz | `now()` |
| `expires_at` | timestamptz NULL | optional, in the future when set (FR-001) |
| `last_used_at` | timestamptz NULL | stamped at most once a minute per token (FR-005) |
| `revoked_at` | timestamptz NULL | set once (FR-004) |

Indexes: unique `token_sha256`; `(org_id, user_id)` for the owner's list; `(org_id)` for the
administrator's list. Foreign keys `(org_id)` and `(user_id)` with `ON DELETE CASCADE` on the
user, as other per-person rows do.

**Authenticates only while** `revoked_at IS NULL`, `expires_at` is null or in the future, and the
owner is active in `org_id`. Permissions are read from the owner's stored state on every request;
nothing is copied onto the token (FR-003).

**Never logged** (FR-009): the secret exists only in the creation response and the request
header; `Debug` of the secret type prints `horae_pat_<prefix>…`.

### State

```text
active ──revoke──▶ revoked        (final)
   │
   └──expires_at passes──▶ expired (final)
```

Deactivating the owner makes every token refuse without changing its state; reactivating the
owner makes unexpired, unrevoked tokens work again (FR-004).

## Changed tables: catalogue update times (PR E)

| Table | Change |
| --- | --- |
| `projects`, `clients`, `users` | `updated_at timestamptz NOT NULL DEFAULT now()`, backfilled from `created_at`, trigger `set_updated_at()` |
| `tasks` | `created_at` and `updated_at timestamptz NOT NULL DEFAULT now()`, trigger `set_updated_at()` |

One migration, `000N_catalogue_updated_at.sql` with the next free number (research R-12). The migration snapshot tests
exclude the new columns. `time_entries.updated_at` already exists.

## Domain changes without schema (PR E)

- `TimesheetCommand::RestartTimer { entry_id }` in `horae-types`, applied by `ops::time::apply`
  (research R-11). Refused with a conflict when the entry is running, not open, or not dated
  today in the subject's time zone.
- `StartTimer` and `RestartTimer` stop the subject's running timer first, in the same
  transaction, with `StopTimer`'s clamp (021 D-6). The `one_running_timer_per_user` index stays
  as the last guard.

## New read operations (PRs G and H)

Shared operations in `horae-server`, each a keyset page ordered by `id` (research R-5). They take
the actor, a filter, an exclusive `after: Option<Uuid>` and a `limit`, and return rows plus
whether more follow. They reuse each domain's visibility helpers; the API adds no rule.

| Operation | Visibility (same as the web app) | Filters |
| --- | --- | --- |
| `ops::time_reads::list` | the actor's time-read scope (`time_reads(actor)`: own, managed or all) | `person_id`, `project_id`, `task_id`, `client_id`, `from`, `to` (spent date), `is_running`, `updated_since` |
| `ops::projects::reads::page` | `project_rows`' rules | `client_id`, `active`, `updated_since` |
| `ops::clients::page` | `clients::list`'s rules (details for catalog readers, names otherwise) | `active`, `updated_since` |
| `ops::projects::task_catalog::visible_page` | `TaskReadAll` sees the catalogue; anyone else the tasks of projects they may track (`time::tracking`'s rule) | `project_id`, `active`, `updated_since` |
| `ops::people::page` | `people::list`'s rules (active people for any active person; archived needs `PeopleReadAll`) | `active`, `updated_since` |

Each list has a `get` beside it (`time_reads::get`, `projects::reads::get`, `clients::get`,
`task_catalog::visible_get`, `people::get`) with the same visibility, answering `NotFound` for a
record outside it.

`ops::time::patch(tx, actor, entry_id, changes)` (PR H) locks the entry, merges the changed
fields into its current values and applies `TimesheetCommand::Update` with the same rules, so a
partial update never reads stale values outside the lock.

`ops::time::tracking` (project and task pairs a person may track) is reused as is for
`GET /api/v1/me/trackable`.

## Public entities (`horae-api::v1`, no storage)

Shapes are in [contracts/api-v1.md](contracts/api-v1.md); the draft schema is
[contracts/openapi-v1.draft.yaml](contracts/openapi-v1.draft.yaml).

- **Me**: person id, name, email, organization id and name, administrator flag, the permission
  profile label, the token's id and expiry.
- **TimeEntry**: id, person id, project id, task id, spent date, minutes, rounded minutes (null
  until locked), notes, billable, state, is running, timer started at, start minute, created at,
  updated at.
- **NewTimeEntry** / **TimeEntryPatch**: what a client may set; unknown fields refused.
- **Project**: id, client id, name, code, currency, type, budget kind, budget minutes or budget
  money (only when the actor may read budgets), starts on, ends on, active, created at, updated at.
- **Task**: id, name, billable default, active, created at, updated at.
- **Client**: id, name, currency, active, created at, updated at; address and tax id only for
  catalogue readers, as in the web app.
- **Person**: id, name, email, active, created at, updated at.
- **Money**: `{ amount_minor: i64, currency: string }`.
- **Page\<T>**: `{ data: [T], next_cursor: string | null }`.
- **Problem**: RFC 9457 body with `code` and optional `field` (research R-4).

## Configuration (no storage)

| Setting | Variable | NixOS option | Default |
| --- | --- | --- | --- |
| Harvest-compatible API on/off (PR D) | `HORAE_HARVEST_API` | `services.horae.harvestApi.enable` | off |
| Cursor signing key (PR F) | `HORAE_API_CURSOR_KEY_FILE` (secret file; `HORAE_API_CURSOR_KEY` for development) | `services.horae.api.cursorKeyFile` | random per process, development fallback with a start-up warning |
| Rate limit (PR F) | `HORAE_API_RATE_LIMIT` (`<requests>/<seconds>`) | `services.horae.api.rateLimit` | `100/15` |
