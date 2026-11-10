# Tasks: Harvest API v2 compatibility

**Input**: Design documents from `specs/021-harvest-api-compat/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md) §4,
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: included. The spec requires a compatibility suite (FR-070–FR-073) and exact
round trips (SC-005); integration tests follow the repo's `#[sqlx::test]` + `#[serial]` pattern.

**Gates**:

- 🔒 **AMENDMENT**: the task writes data through the Harvest-compatible API. It needed the
  maintainer's approval of [constitution-amendment.md](constitution-amendment.md); approved on
  2026-11-09 and applied as constitution 5.0.0 in step 1. Applies to all of US2, US5 and US6.
- ⛔ **CREDENTIALS**: needs `HARVEST_ACCESS_TOKEN` and `HARVEST_ACCOUNT_ID` for the test
  (evidence) Harvest account, from the maintainer. GET requests only (D-9, `harvest-parity.md`
  D-020). Never write the values or the account's name into the repository.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: the spec's user story (US1–US6)

______________________________________________________________________

## Phase 1: Setup

**Purpose**: module layout, dependencies, suite scaffolding. Lands in the step-1 pull request.

- [x] T001 Convert `crates/horae/src/harvest/mod.rs` to `crates/horae/src/harvest.rs` with the existing submodules under `crates/horae/src/harvest/`, with no behaviour change (research R-12)
- [ ] T002 [P] Add `sha2` as a direct dependency and the `raw_value` feature to `serde_json` in `crates/horae/Cargo.toml`, versions matching `Cargo.lock` so nothing new is downloaded. *Step 1: not needed. The `server` feature already enables serde_json's `arbitrary_precision`, so a `serde_json::Number` parsed from exact text is written verbatim; `sha2` moves to step 2 with the tokens.*
- [ ] T003 [P] Confirm which suite clients build in the Nix sandbox (becoded/go-harvest, minutes-harvest, hrvst-cli, harvest-tui, dedene/harvest-cli) as fixed-output derivations; record pinned revisions, hashes and any dropped client with its reason in `nix/checks/harvest-compat/clients/README.md` (research R-10). *Step 1: go-harvest pinned and running; the others are added with the step that first needs them.*
- [x] T004 Create the `harvest-compat` check skeleton in `nix/checks/harvest-compat.nix`: start PostgreSQL and the server as `nix/checks/tests.nix` does, seed one organization, and run a placeholder client step. *Blueprint registers every file in `nix/checks/`, so `flake.nix` is unchanged; the client step is go-harvest's read workflow on the dev-login session.*

______________________________________________________________________

## Phase 2: Foundational

**Purpose**: exact units, error bodies, path aliases and integer ids, which every story needs.

**⚠️ CRITICAL**: no story work starts before this phase is complete.

- [x] T005 [P] Write property tests first in `crates/core/src/api_decimal.rs` (`#[cfg(test)]`) and `crates/core/tests/properties.rs`: every minute value 0..=1_440_000 round-trips through hours text; every minor-unit value round-trips for exponents 0, 2 and 3; input with excess money decimals is refused; hours input rounds half up (SC-005). *Done in `crates/core/tests/properties.rs` over the existing integer helpers (below), plus `money::format_major_units`, which writes each currency's ISO 4217 decimals (unit tests in `money.rs`).*
- [x] T006 Implement `crates/core/src/api_decimal.rs` (minutes→hours text half up to 2 decimals, hours text→minutes via `decimal::scale_decimal`, minor units⇄major-unit text by currency exponent) and export it from `crates/core/src/lib.rs` (research R-4). *No new module: `duration::format_hours2`, `importers::harvest::convert::hours_to_minutes`, `money::format_cents_plain` and `money::parse_cents` already are these conversions, integer-only.*
- [x] T007 [P] Add `crates/horae/src/harvest/decimal_json.rs`: `Hours` and `Amount` wrappers that serialize and deserialize through `serde_json::value::RawValue` using `api_decimal`, with no `f64`. *Serialize only through `serde_json::Number` (see T002); parsing lands with the writes in step 3.* *Now in `crates/harvest/src/v2/compat.rs`.*
- [x] T008 Replace every `f64` field in `crates/horae/src/harvest/types.rs` with the `decimal_json` wrappers and update the read handlers in `crates/horae/src/harvest.rs`, `project_reads.rs`, `client_reads.rs` and `task_reads.rs` *Response types now in `crates/harvest/src/v2/compat.rs`.*
- [x] T009 [P] Add `crates/horae/src/harvest/errors.rs`: one error type mapping to the statuses and bodies in `contracts/harvest-api-v2.md` *Error bodies*, plus the mapping from `ServerFnError` codes (research R-5); use it in every handler. *Query-string values that fail axum's parser (a non-integer `page`, an invalid date) still answer axum's plain 400.*
- [x] T010 Serve the path aliases `/v2`, `/api/v2` and the `.json` suffix in `crates/horae/src/harvest.rs` router (FR-026), with a test per alias in `crates/horae/src/harvest/api_tests.rs`
- [x] T011 Create the `harvest_api` integration test target: `crates/horae/tests/harvest_api.rs` with submodules under `crates/horae/tests/harvest_api/`, a shared seed helper and request helpers. *The app crate has no library target, so `crates/horae/tests/` cannot reach the router; the tests live in `crates/horae/src/harvest/api_tests.rs` beside `pagination_tests.rs` and share its helpers.*
- [x] T012 Write migration `crates/horae/migrations/0056_api_ids_and_account_id.sql`: enum `api_id_kind`, table `api_ids`, `organizations.api_account_id` from a sequence, and the backfill (import-map rows with `from_harvest = true` first, then the rest by `(created_at, id)`) per data-model.md. *Redone as `harvest_id` columns: `0058_api_account_id.sql`, `0059_harvest_import_id.sql` (renames `harvest_import_map.harvest_id`), `0060_harvest_ids.sql`, `0061_harvest_ids_backfill.sql` (batched UPDATE, no DDL, no transaction), `0062`/`0063` (NOT NULL via a NOT VALID check, unique indexes last); 0056 belongs to the time-zone branch.*
- [x] T013 Add `crates/horae/src/harvest/ids.rs`: `number_for(kind, uuid)`, `uuid_for(kind, number)`, batch lookup for list pages, and `allocate(tx, org, kind, uuid, harvest_id: Option<i64>)` under a transaction advisory lock; accept a UUID wherever a number is accepted (D-1). *`ids.rs` resolves request ids (number or UUID) against each table's `harvest_id`; responses read the column from rows their queries already join; numbering is the migration's trigger.*
- [x] T014 Call `ids::allocate` in the same transaction from every operation that creates a client, project, task, time entry, user, assignment or invoice (`crates/horae/src/server_fns/**`, `crates/horae/src/seed.rs`, the CLI `init`/`user create` paths); add an integration test that a record created through each path has a number. *A `BEFORE INSERT` trigger per table covers every path, present and future, in the creating transaction. Assignments and invoices are numbered with the step that exposes them.*
- [x] T015 Make the Harvest importer write the Harvest id, and on a collision allocate a fresh number and add a report line (OQ-2, FR-016). *The five creating inserts (`resolve.rs`, `apply.rs`) pass the Harvest id as `harvest_id` when free; otherwise the trigger numbers the record and the report gets an `IdNotice` (the first 20 kept, the rest counted), shown on the run page and by the CLI. Tests in `crates/horae/src/importers/harvest/sync_tests/api_ids.rs`.*
- [x] T016 Switch every response id and nested reference in `crates/horae/src/harvest/types.rs` and the read handlers to integers via `ids.rs`; update `crates/horae/src/harvest/pagination_tests.rs`
- [x] T017 Regenerate the `.sqlx` cache (`touch` + `cargo sqlx prepare --workspace -- --features server --all-targets`) and exclude the new columns from whole-row migration snapshot tests

**Checkpoint**: reads return integer ids and exact decimals through the session cookie. This is
delivery step 1; open its pull request here.

______________________________________________________________________

## Phase 3: User Story 1 — A Harvest client reads Horae with a personal access token (P1) 🎯 MVP

**Goal**: people create tokens and any token-based Harvest client reads Horae (delivery step 2).

**Independent Test**: create a token, run the suite's read workflows (who am I, projects, time
entries) with every Phase 1 client; quickstart §2.

### Tests for User Story 1

- [ ] T018 [P] [US1] Integration tests in `crates/horae/tests/harvest_api/auth.rs`: bearer and `access_token` both work; missing, unknown, revoked and expired tokens return 401 with the contract body; wrong `Harvest-Account-Id` returns 403; an inactive owner's token returns 401; a member's token cannot see manager-only fields; no `User-Agent` is fine
- [ ] T019 [P] [US1] Integration tests in `crates/horae/tests/harvest_api/rate_limit.rs`: the 101st request within 15 s returns 429 with `Retry-After` and `retry_after`; limits are per token; report paths use the 15-minute window
- [ ] T020 [P] [US1] Server-function tests in `crates/horae/tests/harvest_api/token_management.rs`: create returns the token once; list never returns a secret; only the owner or an administrator revokes; the database holds only the digest

### Implementation for User Story 1

- [ ] T021 [US1] Write migration `crates/horae/migrations/<next>_api_tokens.sql` per data-model.md `api_tokens`
- [ ] T022 [US1] Add `crates/horae/src/harvest/token_auth.rs`: generate `horae_pat_` + base64url(32 random bytes), SHA-256 digest, lookup by digest with the owner active, `last_used_at` stamped at most once a minute (research R-1)
- [ ] T023 [US1] Rewrite the extractor in `crates/horae/src/harvest/auth.rs`: bearer, then `access_token` query, then session cookie; check `Harvest-Account-Id`/`account_id` against `organizations.api_account_id`; record whether the caller came by token, so writes can refuse cookie-only requests (research R-6)
- [ ] T024 [P] [US1] Add `crates/horae/src/harvest/rate_limit.rs`: fixed windows per token in `AppState` (`crates/horae/src/state.rs`), 100/15 s and 100/15 min for `/reports/*` (research R-7)
- [ ] T025 [P] [US1] Add server functions `list_api_tokens`, `create_api_token`, `revoke_api_token`, `list_org_api_tokens`, `api_account_id` in `crates/horae/src/server_fns/api_tokens.rs` per `contracts/api-tokens.md`, using named status constants
- [ ] T026 [US1] Add the API tokens section to personal settings in `crates/horae/src/pages/settings/api_tokens.rs` (account id, list, create with one-time reveal and copy, revoke with confirmation), following `DESIGN.md`; run the `dioxus-best-practices` skill and `cargo run -p horae-dioxus-lint`
- [ ] T027 [P] [US1] Add the administrators' token list with revoke to the admin area in `crates/horae/src/pages/admin/api_tokens.rs`, routed from `crates/horae/src/pages/admin.rs` and `crates/horae/src/route.rs`
- [ ] T028 [P] [US1] Browser check for create, copy, revoke in `crates/horae/tests/browser/api-tokens.cjs`
- [ ] T029 [US1] Seed a known token for the suite by inserting its digest with `psql` inside `nix/checks/harvest-compat.nix` (a test fixture, not a new CLI write path), and wire the read workflows of each Phase 1 client into `nix/checks/harvest-compat.nix`, with local base-URL patches under `nix/checks/harvest-compat/patches/` (never sent upstream, D-8)
- [ ] T030 [US1] Publish the first result table (client × workflow) in `contracts/harvest-api-v2.md` (FR-072, SC-001)

**Checkpoint**: delivery step 2 complete; open its pull request.

______________________________________________________________________

## Phase 4: User Story 2 — Log time and run timers from a Harvest client (P1) 🔒 AMENDMENT

**Goal**: Harvest clients create, edit, delete, start, stop and restart time (delivery step 3).

**Independent Test**: the suite's time workflows pass, and each write shows the same entry, totals
and timer in the web app (SC-003); quickstart §3.

### Tests for User Story 2

- [ ] T031 [P] [US2] Unit tests in `crates/core/src/time_entry_timer.rs` for restart eligibility (own, stopped, open, dated today in the org's time zone) and the start-stops-running rule
- [ ] T032 [P] [US2] Integration tests in `crates/horae/tests/harvest_api/time_entry_writes.rs`: create by hours, create as timer, PATCH, DELETE, stop, restart; cookie-only write is 401; a member writing for someone else is 403; daily cap and locked entry are refused with the mapped status; form body and query-string parameters work (FR-025)
- [ ] T033 [P] [US2] Integration test in `crates/horae/tests/timer_widget.rs` (existing) that starting a timer in the web app stops the running one instead of refusing (D-6)

### Implementation for User Story 2

- [x] T034 [US2] 🔒 Apply the approved amendment text to `.specify/memory/constitution.md` with its Sync Impact Report (done in step 1, 5.0.0)
- [ ] T035 [US2] Implement `crates/core/src/time_entry_timer.rs` and export it from `crates/core/src/lib.rs`
- [ ] T036 [US2] Add `TimesheetCommand::RestartTimer { entry_id }` in `crates/horae/src/models/scoped_time.rs`, and in `crates/horae/src/server_fns/time_entries/commands.rs` handle it and make `StartTimer`/`RestartTimer` stop the running timer in the same transaction; replace the "a timer is already running" refusal in `crates/horae/src/server_fns/time_entries.rs`
- [ ] T037 [US2] Make `commands::apply` `pub(crate)` and expose a small wrapper that takes `(org, actor)` from the token extractor and builds the `TimesheetWriteContext` the same way the server function does, in `crates/horae/src/server_fns/time_entries/commands.rs`
- [ ] T038 [US2] Add `crates/horae/src/harvest/time_entry_writes.rs`: `POST /time_entries`, `PATCH /time_entries/{id}`, `DELETE /time_entries/{id}`, `PATCH /time_entries/{id}/stop`, `PATCH /time_entries/{id}/restart`, each a call to the wrapper with body parsing from JSON, form or query, refusing cookie-only callers; no validation of its own
- [ ] T039 [US2] Show restart in the web app where a stopped entry today can be resumed, in `crates/horae/src/pages/timesheet/` (the entry row's actions), with the Dioxus lint
- [ ] T040 [US2] Add the time workflows (log, start, stop, restart, edit, delete) to `nix/checks/harvest-compat.nix` and update the result table in `contracts/harvest-api-v2.md`

**Checkpoint**: with US4's T041–T042, delivery step 3 is complete; open its pull request.

______________________________________________________________________

## Phase 5: User Story 4 — The reads that end-user apps call around the timer (P2)

**Goal**: `company`, project assignments (ship with step 3) and the remaining catalogue and report
reads (step 5). Reads only; not gated.

**Independent Test**: the suite's picker and report workflows pass; quickstart §3 item 4.

- [ ] T041 [P] [US4] Add `GET /company` in `crates/horae/src/harvest/company.rs`: Horae's own values for name, week start, currency, time format; `wants_timestamp_timers` false; feature flags for absent domains off (D-2); test in `crates/horae/tests/harvest_api/company.rs`
- [ ] T042 [P] [US4] Add `GET /users/me/project_assignments` and `/users/{id}/project_assignments` with nested task assignments in `crates/horae/src/harvest/assignments.rs`, respecting the caller's grants; test in `crates/horae/tests/harvest_api/assignments.rs`
- [ ] T043 [P] [US4] Add `GET /users/{id}` and `/users/{id}/teammates` in `crates/horae/src/harvest/users.rs`
- [ ] T044 [P] [US4] Add user and task assignment lists (per project and account-wide) in `crates/horae/src/harvest/assignments.rs`
- [ ] T045 [P] [US4] Add `GET /invoices` and `/invoices/{id}` with line items in `crates/horae/src/harvest/invoices.rs`, amounts through `horae_harvest::v2::compat::Amount`
- [ ] T046 [US4] Add `GET /reports/time/{clients,projects,tasks,team}`, `/reports/uninvoiced` and `/reports/project_budget` in `crates/horae/src/harvest/reports.rs`, totals computed in integers before conversion (FR-045), reusing the existing report queries in `crates/horae/src/server_fns/reports.rs`
- [ ] T047 [US4] Add the picker and report workflows to the suite and update the result table

**Checkpoint**: delivery step 5 complete; open its pull request.

______________________________________________________________________

## Phase 6: User Story 3 — Sync everything into a warehouse or archive (P2)

**Goal**: full and incremental sync through page and cursor pagination (delivery step 4).

**Independent Test**: the suite's sync workflow with one link-following and one page-counting
client over the 100,000-entry fixture, then an incremental run that sees only changes (SC-004).

- [ ] T048 [P] [US3] Integration tests in `crates/horae/tests/harvest_api/pagination.rs`: `per_page` 1, 2000 and 2001 (422); default 2000; absolute links on the request host; cursor walk equals page walk; `updated_since` returns only changed records per kind
- [ ] T049 [US3] Write migration `crates/horae/migrations/<next>_catalogue_updated_at.sql` per data-model.md (tasks `created_at`/`updated_at`, `updated_at` on projects, clients, users, `set_updated_at` triggers, backfill); regenerate `.sqlx`
- [ ] T050 [US3] Add `crates/horae/src/harvest/pagination.rs`: page envelope, opaque cursor over `(updated_at, id)`, absolute links from `Host` and `X-Forwarded-Proto` (research R-8); move every list handler onto it
- [ ] T051 [US3] Implement every documented time-entry filter and field Horae can fill (FR-022, FR-042) in `crates/horae/src/harvest.rs` time-entry handlers, with an index check against `crates/horae/migrations/0014_time_entry_query_indexes.sql`
- [ ] T052 [P] [US3] Return empty enveloped lists for absent kinds (expenses, expense categories, estimates, estimate item categories, contacts, invoice payments, invoice item categories, roles) in `crates/horae/src/harvest/absent.rs` (D-2, FR-043)
- [ ] T053 [US3] Measure the 2000-row page on the 100,000-entry fixture and record the timing in `specs/021-harvest-api-compat/quickstart.md`; add the sync workflow to the suite

**Checkpoint**: delivery step 4 complete; open its pull request.

______________________________________________________________________

## Phase 7: User Story 5 — Administrators automate the catalogue and invoices (P3) 🔒 AMENDMENT

**Goal**: catalogue writes and invoice status changes (delivery step 6).

**Independent Test**: the suite's catalogue workflow with a library client on an administrator
token; a member token is refused.

- [ ] T054 [P] [US5] Integration tests in `crates/horae/tests/harvest_api/catalogue_writes.rs` for each write below, including permission refusals
- [ ] T055 [US5] 🔒 Extract the inner operations of the client, project, task, assignment and user server functions (`crates/horae/src/server_fns/{clients,projects,project_creation,users}.rs`) into `pub(crate)` functions taking `(org, actor)`, the server functions becoming thin wrappers
- [ ] T056 [US5] 🔒 Add `crates/horae/src/harvest/catalogue_writes.rs`: POST/PATCH/DELETE for clients, projects, tasks, user and task assignments, POST/PATCH users, each calling one extracted operation
- [ ] T057 [US5] 🔒 Add invoice status changes Horae supports (draft, sent, closed) in `crates/horae/src/harvest/invoices.rs`, through the existing invoice operations
- [ ] T058 [US5] Add the catalogue workflow to the suite and update the result table

______________________________________________________________________

## Phase 8: User Story 6 — Sign in to a Harvest OAuth2 client with Horae (P3) 🔒 AMENDMENT

**Goal**: Horae as an OAuth2 provider (delivery step 7, OQ-1 (a)).

**Independent Test**: register a client, run authorization code and refresh, call the API with
the access token, revoke the grant.

- [ ] T059 [US6] 🔒 Write the step-7 design addendum in `specs/021-harvest-api-compat/data-model.md` (`oauth_clients`, `oauth_grants`) and the endpoint contract in `specs/021-harvest-api-compat/contracts/oauth2.md` before code
- [ ] T060 [US6] 🔒 Migration, server functions for client registration and grant revocation, the approval screen, and authorize/token/accounts endpoints in `crates/horae/src/harvest/oauth.rs`; access tokens resolve through `crates/horae/src/harvest/auth.rs`
- [ ] T061 [US6] Add an OAuth client (an Airbyte or n8n fork patched locally) to the suite

______________________________________________________________________

## Phase 9: Reference responses and polish

- [ ] T062 [P] Write the hand-made reference shapes for each implemented endpoint from the public docs in `crates/horae/tests/fixtures/harvest_reference/` and a field-by-field shape test in `crates/horae/tests/harvest_api/reference_shapes.rs` (FR-073)
- [ ] T063 ⛔ CREDENTIALS Write `scripts/harvest-record.sh`: GET only (refuse any other method in code), token and account id from the environment only, scrub names, emails, notes, ids and amounts to same-type placeholders keeping decimal counts, write to `crates/horae/tests/fixtures/harvest_reference/` (research R-11)
- [ ] T064 ⛔ CREDENTIALS Run the recorder against the test account; check the scrubbed files with `rg` for any account name, account id, email or person name before committing; replace the hand-made shapes; settle the open facts (error bodies, hours precision, locked-entry status, restart of a past day, D-6 behaviour) in `contracts/harvest-api-v2.md`; add an iteration-log entry to `specs/harvest-parity.md`
- [ ] T065 [P] Update AGENTS.md *Architecture* (the API is no longer read-only; tokens) and mark `specs/001-time-tracking-invoicing/contracts/harvest-api.md` as superseded by 021's contract
- [ ] T066 Run `nix fmt`, `cargo clippy -p horae --features server`, the Dioxus lint and `nix flake check` before each step's pull request

______________________________________________________________________

## Dependencies

- Phase 1 → Phase 2 → everything else.
- **US1** (Phase 3) needs Phase 2. It is the MVP.
- **US2** (Phase 4) needs US1 (tokens) and **the amendment**. Its pull request also carries US4's
  T041–T042, which need only US1.
- **US4** (Phase 5) T043–T047 need US1; independent of US2.
- **US3** (Phase 6) needs US1; T050 changes every list handler, so merge it before or rebase
  US4's later reads on it.
- **US5** needs US1, US3's pagination and **the amendment**.
- **US6** needs US1 and **the amendment**; it is last.
- T063–T064 need **credentials**; nothing else waits for them. T062 covers FR-073 until then.

Delivery order (spec): Phase 2 (step 1) → US1 (step 2) → US2 + T041–T042 (step 3) → US3
(step 4) → US4 rest (step 5) → US5 (step 6) → US6 (step 7).

## Parallel examples

- Phase 2: T005, T007 and T009 together; T012 alongside them.
- US1: T018, T019 and T020 together; then T024, T025 and T027 in parallel once T021–T023 land.
- US4: T041 to T045 are separate files and can run together.
- US3: T048 and T052 together, before T050.

## Implementation strategy

1. **MVP**: Phases 1–3. Every token-based client can read Horae with integer ids and exact
   numbers. Stop and validate with quickstart §1–2 and the suite.
1. The amendment was approved before Phase 4 (2026-11-09), so US2, US5 and US6 proceed.
1. Each later phase is one pull request that leaves the API working and the suite green.
