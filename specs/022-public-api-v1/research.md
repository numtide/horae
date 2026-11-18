# Research: Public API `/api/v1`

Compiled 2026-11-18 for [spec.md](spec.md) on branch `022-public-api-v1-plan` (base `f059fb64`,
master `73325e6f`). Part 1 records what the code does today, checked against the spec's open
points. Part 2 records each planning decision with what else was weighed. The maintainer's
decisions are in [plan.md](plan.md) *Decisions*.

## 1. Horae today (checked in the code)

### 1.1 Timers

- **Restart does not exist.** `TimesheetCommand` (`crates/types/src/scoped_time.rs`) has
  `Create`, `Update`, `Delete`, `StartTimer`, `StopTimer`, `Reschedule` and `Reorder`. There is
  no restart in `ops::time` either.
- **Starting a timer while another runs is refused (021 D-6 is not implemented).**
  `ops::time::create` inserts with `ON CONFLICT (user_id) WHERE is_running DO NOTHING` and answers
  `OpError::conflict("This person already has a running timer")`. The partial unique index
  `one_running_timer_per_user` enforces one running timer per person.
- `ops::time::apply` is the one write path for the timesheet. It authorizes the actor against the
  subject (`TimesheetWriteContext { expected_requester, subject_id }`), locks the subject's
  writes, refuses entries under approval coverage or not `Open`, and clamps a stopped timer to the
  daily limit (`clamp_stopped_minutes`).

### 1.2 Update times

| Table | `created_at` | `updated_at` | Trigger |
| --- | --- | --- | --- |
| `time_entries` | yes | yes | `time_entries_set_updated_at` (`set_updated_at()`) |
| `projects` | yes | **no** | only `projects_edit_revision` |
| `clients` | yes | **no** | none |
| `users` | yes | **no** | none |
| `tasks` | **no** | **no** | none |
| `assignments` | yes | no | none |

021 planned these columns for its step 4 (data-model *Changed tables*), which has not been done.
Only time entries can honour "updated since" today (spec FR-046).

### 1.3 How the session builds the `Actor`

1. `server_fns::session_caller()` reads the user id from the `tower_sessions` session
   (`auth::http::session::get_session_user_id`).
1. `ops::active_caller(db, user_id)` looks up `users.org_id` for an active user and returns
   `Caller::session(org_id, user_id)`.
1. `ops::begin` / `ops::begin_read` open the transaction and call `Actor::load`, which locks the
   organization and the person (`FOR SHARE`) and loads the stored permission state.

`Caller` carries a `Channel`: `Session`, `Operator` or `System`. `Actor::load_with` loads a
person and permissions **only for `Channel::Session`** (`caller.channel != Channel::Session`
returns an actor with no authority), and its doc comment says "Tokens join later". A token
therefore needs its own channel that `Actor::load` treats like a session; `ops::people::add`
also matches on the channel and must decide what a token may do there (nothing new in v1).

The Harvest API builds its caller itself (`harvest/auth.rs::AuthUser`, its own `users` query)
and calls `Caller::session`.

### 1.4 Routes and their crates

- `crates/horae/src/main.rs` builds one Axum router: the Dioxus app, `/health` (an inline closure),
  `/favicon.ico`, nine export routes under `/api/reports`, `/api/projects` and `/api/invoices`,
  the import-errors download under `/api/import`, the Harvest OAuth callback
  (`http::harvest_oauth`), the `/auth/*` router and `harvest::router`, then the login-redirect
  guard and the session layer.
- `http/` (exports, import errors, Harvest OAuth callback) and `auth/http` (session, `/auth/*`,
  login page) use `crate::db`, `ops`, `models`, `state`, `render`, `importers` and `plugin`, and
  none use Dioxus. `auth/http` takes its asset URLs as `LoginAssets`, so it does not need
  `asset!`. About 5,600 lines with tests.
- `harvest/` (`/harvest/v2`, `/v2`, `/api/v2`) holds its own SQL for reads (021's recorded debt)
  and maps `OpError` through `ApiError`.
- `login_redirect_guard` treats `/api/`, `/harvest/` and `/v2/` as public prefixes.
- The **exports already live under `/api/`** (`/api/reports/export/csv`, …) with cookie
  authentication. They are Horae's own routes, so spec FR-052 ("`/api/` belongs to Horae's own
  API") holds; `/api/v1/*` is a sibling namespace.
- `/api/v2` is referenced only by `harvest.rs` and `harvest/api_tests.rs` (plus Harvest's own
  identity paths in `crates/harvest/src/oauth.rs`, which are Harvest's hosts, not Horae's). The
  e2e check (`nix/checks/e2e.nix`) reads `/harvest/v2/time_entries` with a cookie.

### 1.5 Reads in `ops` that v1 can call

The existing reads are shaped for screens:

| Need | Existing operation | Fit for v1 |
| --- | --- | --- |
| Time entries | `time_reads::sheet` (one subject, a date window, 100 rows, descending cursor) | No: v1 lists across people, filters and `updated_since` |
| Projects | `projects::reads::list(client_id, include_inactive, purpose)` | Rules yes; no pagination, no update time |
| Clients | `clients::list(include_inactive)` (catalog readers see details, others names only) | Rules yes; no pagination |
| Tasks | `projects::task_catalog::page` (needs `TaskReadAll`); `time::tracking` (pairs a person may track) | Members have no `TaskReadAll`, so v1 needs both |
| People | `people::list` (pickers: active people for anyone active; archived needs `PeopleReadAll`), `people::directory::page` (needs `PeopleReadAll`/`Managed`) | Rules yes; no update time |
| Identity | `profile::permissions`, `Actor` | Yes |

So v1 needs **new list operations in `ops`**, keyset-paginated and filterable, that reuse each
domain's visibility rules (the same helpers the existing reads use). They are shared operations,
not API code: the API calls them and holds no SQL (spec FR-020, C-3).

### 1.6 Configuration and deployment

- `crates/server/src/config.rs` reads `HORAE_*` variables; secrets reach the service through the
  NixOS module's `secretKeyFile` (systemd `EnvironmentFile`).
- The NixOS module (`nix/modules/nixos/horae.nix`) has `enable`, `package`, `host`, `port`,
  database, backup, `secretKeyFile`, `logLevel`, `openFirewall`; nothing for the Harvest API.

### 1.7 Dependencies already present

`Cargo.lock` has `sha2 0.10`, `hmac 0.12`, `subtle 2.6`, `base64 0.22` and `rand`. It has no
`utoipa`, `utoipa-axum` or `jsonschema`. `oasdiff` is **not packaged in nixpkgs** (the flake's
nixpkgs has no attribute of that name).

## 2. Planning decisions

### R-1 OpenAPI generated from the code (`utoipa`)

- **Decision**: the public types in `horae-api::v1` derive `utoipa::ToSchema`; handlers carry
  `#[utoipa::path]` and are registered through `utoipa-axum`'s `OpenApiRouter`, so a route cannot
  exist without its description. The document is OpenAPI 3.1, built once at startup and served
  at `GET /api/v1/openapi.json` without authentication.
- **Rationale**: FR-047 asks for a description produced from the same definitions the server uses.
  With `OpenApiRouter` the route table and the document come from one list. utoipa 5 supports
  axum 0.8 through `utoipa-axum` 0.2.
- **Alternatives**: `aide` (similar, smaller community, its own router wrapper); a hand-written
  YAML checked against the code (drifts, the failure FR-047 forbids); `poem-openapi` (another web
  framework).

### R-2 Contract validated in the tests

- **Decision**: a test helper in `horae-api` validates every `/api/v1` response body in the
  router tests against the schema of its operation and status, taken from the generated document,
  with the `jsonschema` crate (OpenAPI 3.1 schemas are JSON Schema 2020-12). Every router test
  goes through the helper, so SC-003 is a property of the suite, not a separate test.
- **Alternatives**: schemathesis-style fuzzing (Python, network-free but heavy; possible later);
  typed round trips only (prove serde agrees with itself, not with the document).

### R-3 Breaking-change check in Nix

- **Decision**: the generated document is written to `crates/api/openapi/v1.json` by a test that
  fails when the file differs from what the code generates (refresh with
  `UPDATE_OPENAPI=1 cargo test -p horae-api openapi`). A new check,
  `nix/checks/api-compat.nix`, runs `oasdiff breaking` between that snapshot and a frozen
  baseline, `crates/api/openapi/v1.baseline.json`, and fails on any error-level change. The
  baseline is replaced only when a `v1` release is cut (by copying the snapshot), and its diff in
  a pull request is the signal reviewers look for. `oasdiff` is packaged in
  `nix/packages/oasdiff.nix` with `buildGoModule` from a pinned release.
- **Rationale**: the snapshot keeps the contract visible in review; the baseline stops a pull
  request that edits both the code and the snapshot from hiding a break. Additions pass. The
  check runs in `nix flake check` with no network.
- **Residual risk**: a pull request that also edits the baseline can still break `v1`. That edit is
  a visible diff of a file that otherwise never changes, and the plan names it in review
  (*Layering* rule 5).
- **Alternatives**: `openapi-changes` (pb33f, also unpackaged, larger); a Rust comparison written
  for Horae (a second, weaker oasdiff); comparing against `origin/master` (needs git history in the
  sandbox).

### R-4 Error shape

- **Decision**: errors are `application/problem+json` (RFC 9457) with `type` (always
  `about:blank`, so no URL has to stay reachable), `title`, `status`, `detail`, and the extension
  members `code` (stable, snake case; clients branch on it) and `field` (optional, from
  `ErrorDetail`). One function maps `OpError` to status and code:

  | `OpError` | Status | `code` |
  | --- | --- | --- |
  | `NotFound` | 404 | `not_found` |
  | `Forbidden` | 403 | `forbidden` |
  | `Conflict` | 409 | `conflict` (or the error's `reason`) |
  | `Invalid` | 422 | `invalid` (or the error's `reason`) |
  | `Internal` | 500 | `internal` (detail withheld) |

  The API layer adds `unauthenticated` (401), `bad_request` (400, malformed input or cursor),
  `rate_limited` (429) and `not_acceptable`. `detail` is the `OpError` message, the same text the
  web app shows (FR-044).

- **Rationale**: `OpError::Detailed` already carries `field` and `reason` "in the spirit of
  problem+json" (`ops/error.rs`); a standard shape lets generated clients decode it.

- **Alternatives**: a bespoke `{error: {code, message}}` (no standard tooling); reusing Harvest's
  shapes (they differ by status and are undocumented).

### R-5 Cursors and keyset pagination

- **Decision**: every list orders by `id` ascending. Ids are UUID v7, so this is creation order and
  is stable. A page is `limit` rows (default 100, maximum 1,000; outside the range is 422) after
  the cursor's position. The cursor is base64url of `version ‖ list ‖ filter_digest ‖ last_id ‖ issued_at ‖ HMAC-SHA256`, where `filter_digest` is a SHA-256 of the list's normalized filters,
  so a cursor cannot be replayed on another list or with other filters. A cursor older than 24
  hours, from another list, with other filters or a bad MAC answers 400 `invalid_cursor`.
  Responses are `{ "data": [...], "next_cursor": string | null }`; no totals.
- **Key**: `HORAE_API_CURSOR_KEY` (64 hex characters) from configuration or the secret file. When
  unset, the server draws a random key at start and logs once that cursors will not survive a
  restart. Cursors carry no secret data, so the key's only job is tamper evidence.
- **Incremental sync**: `updated_since` is a filter, not an order. A client stores the time it
  started its previous run and passes it next time; a record changed during a walk has
  `updated_at` after that time and comes back on the next run (quickstart).
- **Rationale**: FR-045 asks for opaque, tamper-evident cursors; keyset on the primary key needs no
  new index and does not skip or repeat rows when earlier rows are inserted or deleted (SC-007).
- **Alternatives**: offset pages (repeat or skip under concurrent writes, slow at depth); ordering
  by `(updated_at, id)` (a record edited mid-walk jumps ahead and is seen twice); encrypted
  cursors (nothing to hide); an unsigned cursor (FR-045 forbids it).

### R-6 Tokens

- **Decision**: 021's design (research R-1, data-model `api_tokens`), unchanged: 32 random bytes,
  base64url, prefix `horae_pat_`, SHA-256 digest stored and looked up, the first eight characters
  after the prefix kept for display. Operations in `ops::api_tokens` (`list_own`, `create`,
  `revoke`, `list_org`, `authenticate`). Issue and revocation are server functions in
  `crates/horae/src/server_fns/api_tokens.rs` (constitution IV), per 021's
  [contracts/api-tokens.md](../021-harvest-api-compat/contracts/api-tokens.md), without
  `api_account_id()` (that belongs to the Harvest API).
- **Authentication**: `ops::api_tokens::authenticate(pool, secret) -> Option<Caller>` hashes the
  secret, finds an unrevoked, unexpired token whose owner is active, stamps `last_used_at` at
  most once a minute, and returns `Caller::token(org_id, user_id, token_id)`. `Channel::Token`
  is added; `Actor::load` loads person and permissions for it as for a session, so every
  permission check is the session's (FR-003, SC-004).
- **Organization**: tokens carry `org_id` today (one membership per account). FR-011's choice
  between memberships is 020's work; v1 needs no header until then.
- **Alternatives**: as in 021 R-1.

### R-7 Rate limiting

- **Decision**: one limiter in `horae-api` (`rate_limit.rs`), a fixed window per token id held in
  memory: 100 requests per 15 seconds by default, configurable with
  `HORAE_API_RATE_LIMIT=<requests>/<seconds>`. Both `/api/v1` and, through D2, `/harvest/v2` take
  the same limiter, so a token's requests to either count together. A refusal is 429 with
  `Retry-After` and the problem body (code `rate_limited`). The report bucket of 021 FR-050 is
  added when a report endpoint exists.
- **Rationale**: 021 R-7 (Horae runs as one process; a 30-line counter needs no dependency).
- **Alternatives**: `tower_governor` (a dependency for the same counter); a PostgreSQL counter (a
  write per request).

### R-8 Public types and formats

- **Decision**: `horae-api::v1` defines its own structs (`Me`, `TimeEntry`, `NewTimeEntry`,
  `TimeEntryPatch`, `Project`, `Task`, `Client`, `Person`, `Page<T>`, `Problem`) with
  `#[serde(deny_unknown_fields)]` on inputs. Conversions from `horae-types` and `ops` results are
  `From` impls in `horae-api` (both are foreign to it, so they are free functions where the orphan
  rule forbids an impl). Ids are UUID strings; durations `*_minutes: i32`; money
  `{ "amount_minor": i64, "currency": "EUR" }`; instants RFC 3339 UTC; dates `YYYY-MM-DD`.
- **Rationale**: C-1, C-6, FR-040 to FR-043. No decimal conversion, so Principle I needs no
  clarification for v1.
- **Alternatives**: deriving `ToSchema` on `horae-types` (couples the public contract to the UI's
  shapes, which FR-040 forbids).

### R-9 Crate layout

- **Decision**: `crates/api` (`horae-api`) depends on `horae-core`, `horae-types` (for the moved
  internal routes only), `horae-server`, axum, tower-sessions, `utoipa`, `utoipa-axum` and, for
  the moved export routes, `sqlx` (decision P6 of the crate study: `horae-api` starts with sqlx).
  `v1` never uses `sqlx`: its handlers call `ops`. `crates/harvest-api` (`horae-harvest-api`)
  depends on `horae-harvest`, `horae-server` and `horae-api`. `horae` depends on both and mounts
  them. The graph is `server → api → harvest-api → horae` (decision P8).
- **Rationale**: the agreed shape (memory of 2026-11-17/18, crate study PR 8). `horae-harvest` stays
  the pure Harvest language; the shim cannot live there, because `horae-server` depends on it.
- **Alternatives**: one crate for both APIs (the toggle would still exist, but the Harvest dialect
  would share a namespace with the stable contract); a thin binary crate over `ui` and `api`
  (study P8 option, more impact on `dx` and the lint baseline; not chosen).

### R-10 Harvest-compatible API switch

- **Decision**: `HORAE_HARVEST_API=1` (parsed like `DEV_LOGIN`; default off) and the NixOS option
  `services.horae.harvestApi.enable` (default `false`), which sets the variable. When off,
  `horae-harvest-api` is not mounted and the binary mounts a fallback that answers `404` with an
  empty body for `/harvest/v2/*` and `/v2/*`, so those paths do not fall through to the web app's
  HTML shell. When on, the router is mounted as today. The switch is read at start (spec FR-060).
- **Rationale**: FR-061 asks for "404 as an unknown path does"; for an API path, an empty 404 is
  that answer, and it reveals nothing.
- **Alternatives**: a Cargo feature (out of scope, C-5); a check per request (no benefit, and the
  router would still be built).

### R-11 Timer restart and D-6 (PR E)

- **Decision**: `TimesheetCommand::RestartTimer { entry_id }`, handled in `ops::time::apply`:
  allowed when the entry belongs to the subject, is open, not running and dated today in the
  subject's time zone; it becomes running again and keeps its minutes (the stop adds the new
  elapsed time). `StartTimer` and `RestartTimer` first stop the subject's running timer in the
  same transaction, with `StopTimer`'s rule and clamp (021 D-6, confirmed again 2026-11-18). The
  web app gets a restart action on stopped entries of today.
- **Rationale**: 021 data-model *Domain changes*; the maintainer chose to do it in E.
- **Open fact**: Harvest's rule for restarting another day's entry. v1 refuses with a conflict,
  which the contract documents; widening it later is additive.

### R-12 Catalogue update times (PR E)

- **Decision**: one migration `0002_catalogue_updated_at.sql` adds `updated_at timestamptz NOT NULL DEFAULT now()` to `projects`, `clients` and `users`, and `created_at` plus `updated_at` to
  `tasks`, with the existing `set_updated_at()` trigger. Backfill `updated_at` from `created_at`
  (tasks from the migration time). It is 021's step-4 change, done once for both APIs. The project
  edit-revision trigger is left as is.
- **Caveat**: migration snapshot tests compare whole rows; exclude the new columns there.

### R-13 Constitution amendment (PR A)

- **Decision**: generalise Principle IV's token clause from "the Harvest-compatible API" to
  "Horae's token-authenticated HTTP APIs" (today `/harvest/v2` and `/api/v1`) with the same
  bounds, and rename the principle "Mutations Through Shared Operations". Text in
  [constitution-amendment.md](constitution-amendment.md). MAJOR: 6.0.0 → 7.0.0.

### R-14 End-to-end check (PR I)

- **Decision**: `nix/checks/api-v1.nix` starts PostgreSQL and the server as
  `nix/checks/tests.nix` does, seeds an organization, creates a token through the CLI test seed,
  and runs a client **generated from the snapshot** (`openapi-generator-cli` 7.22.0 is in the
  flake's nixpkgs; the Python target) through the quickstart scenarios. This measures SC-002 without
  network.
- **Unconfirmed**: that the generator's output builds offline in the sandbox; the first task of PR
  I checks it, and falls back to `curl` plus `jq` scenarios if it does not.
