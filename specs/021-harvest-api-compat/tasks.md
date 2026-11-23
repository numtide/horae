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

- [x] T031 [P] [US2] Unit tests in `crates/core/src/time_entry_timer.rs` for restart eligibility (own, stopped, open, dated today in the org's time zone) and the start-stops-running rule *(delivered with spec 022's time-entry operations)*
- [x] T032 [P] [US2] Integration tests in `crates/horae/tests/harvest_api/time_entry_writes.rs`: create by hours, create as timer, PATCH, DELETE, stop, restart; cookie-only write is 401; a member writing for someone else is 403; daily cap and locked entry are refused with the mapped status; form body and query-string parameters work (FR-025) *(router tests in `crates/harvest-api/src/api_tests/time_entry_writes.rs`)*
- [x] T033 [P] [US2] Integration test in `crates/horae/tests/timer_widget.rs` (existing) that starting a timer in the web app stops the running one instead of refusing (D-6) *(delivered with spec 022)*

### Implementation for User Story 2

- [x] T034 [US2] 🔒 Apply the approved amendment text to `.specify/memory/constitution.md` with its Sync Impact Report (done in step 1, 5.0.0)
- [x] T035 [US2] Implement `crates/core/src/time_entry_timer.rs` and export it from `crates/core/src/lib.rs`
- [x] T036 [US2] Add `TimesheetCommand::RestartTimer { entry_id }` in `crates/horae/src/models/scoped_time.rs`, and in `crates/horae/src/server_fns/time_entries/commands.rs` handle it and make `StartTimer`/`RestartTimer` stop the running timer in the same transaction; replace the "a timer is already running" refusal in `crates/horae/src/server_fns/time_entries.rs` *(T035–T037 delivered with spec 022: `ops::time` and `TimesheetCommand::RestartTimer`)*
- [x] T037 [US2] Make `commands::apply` `pub(crate)` and expose a small wrapper that takes `(org, actor)` from the token extractor and builds the `TimesheetWriteContext` the same way the server function does, in `crates/horae/src/server_fns/time_entries/commands.rs`
- [x] T038 [US2] Add `crates/horae/src/harvest/time_entry_writes.rs`: `POST /time_entries`, `PATCH /time_entries/{id}`, `DELETE /time_entries/{id}`, `PATCH /time_entries/{id}/stop`, `PATCH /time_entries/{id}/restart`, each a call to the wrapper with body parsing from JSON, form or query, refusing cookie-only callers; no validation of its own *(in `crates/harvest-api/src/time_entry_writes.rs`)*
- [x] T039 [US2] Show restart in the web app where a stopped entry today can be resumed, in `crates/horae/src/pages/timesheet/` (the entry row's actions), with the Dioxus lint
- [x] T040 [US2] Add the time workflows (log, start, stop, restart, edit, delete) to `nix/checks/harvest-compat.nix` and update the result table in `contracts/harvest-api-v2.md` *(through go-harvest's write calls)*

**Checkpoint**: with US4's T041–T042, delivery step 3 is complete; open its pull request.

______________________________________________________________________

## Phase 5: User Story 4 — The reads that end-user apps call around the timer (P2)

**Goal**: `company`, project assignments (ship with step 3) and the remaining catalogue and report
reads (step 5). Reads only; not gated.

**Independent Test**: the suite's picker and report workflows pass; quickstart §3 item 4.

**Step 5 starts from**: step 4 merged (PR #454: `crates/harvest-api/src/pagination.rs` with
`Window`, `Key`, `Listed`, `horizon` and `ListRequest::{pages, keyed}`, and `absent.rs`);
spec 025's migration merged (`invoice_status` value `closed`, `harvest_entity_type` value
`invoice`, tables `harvest_invoices` and `harvest_invoice_lines`, column `invoices.source`);
PR #453 merged (`invoices.subject`, `horae_core::invoice::is_overdue`,
`ops::invoices::uninvoiced`). If one of them has not landed, rebase onto it first rather than
work around it. The decisions these tasks follow are in [contracts/harvest-api-v2.md](contracts/harvest-api-v2.md)
*Step 5* and [data-model.md](data-model.md) *Step 5*.

Code paths: the API is the `horae-harvest-api` crate (`crates/harvest-api/src/`), its response
shapes are in `horae-harvest` (`crates/harvest/src/v2/compat.rs`), reads and their authorization
are operations in `horae-server` (`crates/server/src/ops/`), and migrations go in
`crates/server/migrations/`, numbered after the last one on master when written. Router tests go
in `crates/harvest-api/src/api_tests/` (one file per area, declared from `api_tests.rs`) and run
with `cargo test -p horae-harvest-api`.

### Tests for step 5

- [ ] T067 [P] [US4] Migration tests in a new `crates/server/src/ops/tests/harvest_ids.rs`: every existing `assignments`, `project_tasks`, `invoices`, `invoice_line_items` and `harvest_invoice_lines` row gets a number and an `org_id`, and every `project_tasks` row a v7 `id`; an insert that names neither (as every existing insert does) gets both from the triggers, and its number skips one held in `harvest_import_map` (which fails if the `harvest_id` trigger fires before `org_id` is filled); an imported line's Harvest id stays reserved after a re-import replaces the lines; numbers are unique per organization and kind; a deleted record's number is not handed out again; an imported invoice keeps its Harvest id when free
- [ ] T068 [P] [US4] Router tests in `crates/harvest-api/src/api_tests/assignments.rs`: two people on one project get different user-assignment ids and the same task-assignment id; an id survives a rename of the project and a rate change; removing a person from a project and adding them again gives a new id; `/users/me/project_assignments` and `/users/{id}/project_assignments` list one entry per assignment with only the tasks the person may track (restricted tasks left out); `/users/{id}/project_assignments` is 404 for an unknown person or one the caller may not see, and 403 for one they see without authority over them; `updated_since` on project assignments follows the user assignment (a task assignment change does not bring the entry back, and does bring the task assignment back on `/task_assignments`); per-project and account-wide user and task assignment lists honour `user_id`, `is_active` and `updated_since`; a cursor walk equals a page walk; rates appear with `BillableRateReadAll`, with `BillableRateReadManaged` only on projects the caller manages, and otherwise are null; a time entry carries its `user_assignment` and `task_assignment`
- [ ] T069 [P] [US4] Router tests in `crates/harvest-api/src/api_tests/users.rs`: `GET /users/{id}` for self, for someone visible and for someone not (404, as an id the caller may not see); `/users/{id}/teammates` lists the people that user manages, is 403 for a visible user when the caller lacks `PeopleReadAll` and is not that user, and 404 for a user the caller may not see
- [ ] T070 [P] [US4] Router tests in `crates/harvest-api/src/api_tests/invoices.rs`: `state` for draft, sent, paid and closed, and a void invoice absent from the list and 404 by id; a sent invoice past its due date is `open` with that `due_date`; exact amounts in EUR, JPY and BHD; `subject`; `due_amount` and `paid_date` for native and imported invoices; line items from time lines, fee lines and imported lines, each with its project and kind, Horae's in `id` order and imported ones by `position`; the list runs newest `issue_date` first, and an import of old invoices does not jump ahead of newer native ones; filters `client_id`, `project_id`, `state` (another value, `void` included, is 422), `from`, `to`, `updated_since`; cursor walk equals page walk; a member is 403; `GET /invoices/{id}/payments` is an empty list and 404 for an unknown invoice; a time entry carries its `invoice` (`id`, `number`), also for a member who cannot read invoices, and `is_billed` follows the entry's state, so an entry billed in Harvest without an imported invoice reads billed with a null `invoice`
- [ ] T071 [P] [US4] Router tests in `crates/harvest-api/src/api_tests/reports.rs` over a fixture with rounding on, two currencies, a fixed-fee project and a person whose rates the caller may not read: each `/reports/time/*` row equals the matching `ops::reports::time` row to the minute and the minor unit; a withheld amount is `null`; on the time and uninvoiced reports `from` or `to` missing, or a range over 365 days, is 422, and `20261101` and `2026-11-01` both parse; `/reports/uninvoiced` per project, over a range that covers every fixture entry through today with every client active, sums to `ops::invoices::uninvoiced::list` per client; a fixed-fee project's fees due by the earlier of `to` and today count with the default `include_fixed_fee`, a fee due after `to` does not, the amount is time only with `include_fixed_fee=false`, and the fee is 0 when `to` is before the project's start date; an inactive client's projects are left out; `/reports/project_budget` needs no dates, honours `is_active` and equals `ops::projects::budget_progress`; `per_page` defaults to 2000 and 2001 is 422 on every report; the 101st report request in 15 minutes is 429 while other paths still answer
- [ ] T072 [P] [US4] If spec 024 has landed: the route table names a scope for every new route, and an OAuth token is refused with `insufficient_scope` on invoices and reports and accepted with `catalog:read` on assignments and users; an OAuth token with `catalog:read` whose owner has rate grants reads `hourly_rate: null` on user and task assignments (024 FR-063); an OAuth token with only `time:read` gets a time entry's `invoice` (`id`, `number`) and is refused on `GET /invoices/{id}`

### Implementation for step 5

- [ ] T073 [US4] Migration `<n>_harvest_id_kinds.sql`: add the `harvest_entity_type` values `user_assignment`, `task_assignment` and `invoice_line` (and `invoice` if spec 025 has not added it). It is its own file because PostgreSQL refuses a new enum value in the transaction that adds it
- [ ] T074 [US4] Migration `<n+1>_harvest_ids_step5.sql` per data-model.md *Step 5*: `org_id` on `assignments`, `project_tasks` and `invoice_line_items`, filled from the parent row by a `BEFORE INSERT` trigger named `<table>_fill` (it must sort before `<table>_harvest_id`, since PostgreSQL fires same-event triggers in name order) when the insert leaves it out; a unique v7 `id` on `project_tasks`, filled by the same trigger through a small SQL v7 generator and backfilled in `(project_id, task_id)` order; `harvest_id` with a sequence and the `harvest_id_assign` trigger on `assignments`, `project_tasks` and `invoices`, and one shared sequence and a trigger that checks both line tables on `invoice_line_items` and `harvest_invoice_lines`; `created_at` and `updated_at` where missing, stamped at write time like `0006_updated_at_at_write.sql`; the backfill (imported invoices and lines take their Harvest ids, the rest numbers in creation order); unique indexes `(org_id, harvest_id)` built after the backfill, as 0060–0063 did. Time the migration on a copy with 1,000,000 time entries invoiced and record it in data-model.md; if the line backfill holds writes longer than a second, batch it like the old `0061`. Regenerate `.sqlx` (AGENTS.md *sqlx query cache*)
- [ ] T075 [US4] Extend `importers::harvest::ids::Kind` (`crates/server/src/importers/harvest/ids.rs`) with `UserAssignment`, `TaskAssignment`, `Invoice` and `InvoiceLine`, so `crates/harvest-api/src/ids.rs` resolves their path and filter ids; make the invoice import (spec 025, `crates/server/src/importers/harvest/`) pass the Harvest invoice and line ids as `harvest_id` under the step-1 rule (free in the organization and more than 10,000 above the sequence, else the trigger numbers it and the report gets an `IdNotice`), and record each imported line in `harvest_import_map` (kind `invoice_line`), repointing the entry when a re-import replaces the lines. Project tasks are still created from time entries without Harvest's task assignment ids, so task assignments are always numbered by the trigger
- [ ] T080 [US4] Make the Harvest API import (`crates/server/src/importers/harvest/api_source.rs` and the apply path) bring Harvest's user assignments (`GET /v2/user_assignments`) as `assignments` rows (decided 2026-11-23): one row per active Harvest assignment whose person and project the import brings or matches, its Harvest id passed as `harvest_id` under the step-1 rule and recorded in `harvest_import_map` (kind `user_assignment`); an existing row for the same person and project is matched and keeps its number. Inactive Harvest assignments are skipped, since a Horae row lets the person track. The rows carry no rate and no manager flag: importing Harvest's assignment rates and project-manager flags was not part of the decision. An organization imported earlier gets its rows on its next API import; the CSV import is unchanged and creates none. Tests in `crates/server/src/importers/harvest/sync_tests.rs`: an imported administrator's `/users/me/project_assignments` lists the projects of their Harvest assignments with the Harvest ids; a re-import adds no duplicate; an inactive assignment is not imported; a CSV import creates no assignment
- [ ] T042 [US4] Rework `GET /users/me/project_assignments` in `crates/harvest-api/src/project_assignments.rs` and add `GET /users/{id}/project_assignments`: one entry per `assignments` row of the person whose project is trackable, `id` its `harvest_id`, nested task assignments with their `project_tasks.harvest_id`, limited to the tasks the person may track. The pairs come from the Timesheet's rule (`ops::time::tracking` for oneself); for another person add a read operation in `crates/server/src/ops/time.rs` that runs the same `choices` query under read authority over that person (`PeopleReadAll`, or managing them), not the write authority `tracking` checks. Keyed by `assignments.id`, so the list takes `cursor` and `updated_since` like the other catalogue lists; drop the step-4 422 on those two parameters. Rates, budgets and timestamps are filled as T044 describes. *(Step 3 served `/users/me/project_assignments` with project and task ids as assignment ids: unique within one person's list only, the FR-015 debt this task closes. An administrator's projects without an assignment row are left out, as on Harvest: decided 2026-11-23, see contracts/harvest-api-v2.md Step 5. Imported people get their rows from the Harvest API import, T080.)*
- [ ] T044 [P] [US4] Add `crates/harvest-api/src/assignments.rs`: `GET /user_assignments`, `/projects/{id}/user_assignments`, `/projects/{id}/user_assignments/{id}`, `/task_assignments`, `/projects/{id}/task_assignments` and `/projects/{id}/task_assignments/{id}` with Harvest's filters (`user_id` on user assignments, `is_active`, `updated_since`), keyed lists through `ListRequest::keyed` with the `horizon`. Reads are new operations in `crates/server/src/ops/projects/` (assignments of the projects the actor may read; rates with `BillableRateReadAll`, or `BillableRateReadManaged` on managed projects, as `crates/harvest-api/src/project_reads.rs` shows them). Field mapping: contracts/harvest-api-v2.md *Step 5*. Both kinds are keyed by their UUID (`project_tasks.id` from T074), so `pagination::Key` and its cursor format do not change
- [ ] T076 [US4] Fill the time-entry fields step 4 left null in `crates/harvest-api/src/lib.rs`: `user_assignment` (the `assignments` row of the entry's person and project, null without one), `task_assignment` (the entry's `project_tasks` row, null if removed) and `invoice` (`id` and `number`, null without a linked invoice); `is_billed`, both the field (`row.invoice_id.is_some()` on #454) and the `is_billed` filter (`te.invoice_id IS NOT NULL`), reads the entry's state (`invoiced`), as spec 025 decides for every surface. *(Already done if spec 025's implementation has landed, since 025 R-10 moves both; then only the three fields remain.)*
- [ ] T043 [P] [US4] Add `GET /users/{id}` (the users list's read with an id filter, as step 4 serves `/tasks/{id}`) and `GET /users/{id}/teammates` (the people that user manages through `person_management_assignments`, with Harvest's teammate fields) in `crates/harvest-api/src/users.rs`. Move the users list's own scoping (`scoped_user_filter`, `user_access.rs`) into an operation beside `ops::people::list` while touching it, which closes the step-1 debt recorded in plan.md *Layering*
- [ ] T045 [US4] Add `crates/harvest-api/src/invoices.rs`: `GET /invoices`, `/invoices/{id}` with `line_items`, and `/invoices/{id}/payments` (always empty, FR-043). Read through a new operation `ops::invoices::page(tx, actor, filters, window)` that calls `authorize_read(actor, READ)` itself (Principle IV), applies the Harvest filters, leaves void invoices out and keys by `(issue_date, id)`. Leave `ops::invoices::list` alone: it takes no actor, leaves authorization to its callers and orders by `created_at`. Amounts go through `horae_harvest::v2::compat::Amount` in the invoice's currency, percentages as exact decimals from basis points, state and due mapping per contracts/harvest-api-v2.md *Step 5*. Add the response types to `crates/harvest/src/v2/compat.rs`
- [ ] T046 [US4] Add `crates/harvest-api/src/reports.rs`: `GET /reports/time/{clients,projects,tasks,team}` over `ops::reports::time` (group `client`, `project`, `task`, `person`), mapping each row's UUID to its `harvest_id` in one batch query and adding the client to project rows; `GET /reports/uninvoiced` over a new `ops::invoices::uninvoiced::by_project(tx, actor, (from, to), include_fixed_fee)` that applies `list`'s eligibility per project (inactive clients left out), time in the range and, with `include_fixed_fee` (default true), a fixed-fee project's fees due on or before the earlier of `to` and today when `to` is on or after the project's start date; `GET /reports/project_budget` over `ops::projects::budget_progress`, taking only `page`, `per_page` and `is_active`. Totals stay integer minutes and minor units until the response (FR-045). Reports are paged by number (`ListRequest::pages`, `per_page` 1–2000, default 2000) and take no `cursor` and no `updated_since`; the time and uninvoiced reports require `from` and `to`, which accept `YYYYMMDD` and `YYYY-MM-DD`
- [ ] T078 [US4] Give `/reports/*` the second rate window FR-050 asks for (100 requests per 15 minutes per token, operator-configurable beside the existing limit) in `crates/api/src/rate_limit.rs`, applied by the Harvest router to its report routes only
- [ ] T079 [US4] If spec 024 has landed, name each new route's scope in the Harvest route table: `catalog:read` for assignments, `/users/{id}` and teammates; none (OAuth refused) for invoices and reports (024 FR-063). If it has not, add these rows to `specs/024-oauth-pkce/contracts/api-changes.md` *Harvest API* in the step-5 pull request so 024 picks them up
- [ ] T047 [US4] First confirm that the pinned go-harvest client exposes report, invoice and user-assignment calls; if it does not, name the client that does and put the workflow there. Add a report workflow to that client (time report by project and by client for the seed's range, checked against totals the seed fixes; the project budget report; one invoice with its lines). The suite has one token, the demo administrator's, so add a second assigned person to the seed (`crates/server/src/seed.rs`) and check that assignment ids differ across the two through `/projects/{id}/user_assignments` with that token. Update the result table in `contracts/harvest-api-v2.md` and the endpoint list in `docs/harvest.md`; the release notes say that project assignment ids change once and that Horae's paid invoices have a `null` `paid_date` and no payments, since Horae records no payments yet

**Checkpoint**: delivery step 5 complete; open its pull request.

______________________________________________________________________

## Phase 6: User Story 3 — Sync everything into a warehouse or archive (P2)

**Goal**: full and incremental sync through page and cursor pagination (delivery step 4).

**Independent Test**: the suite's sync workflow with one link-following and one page-counting
client over the 100,000-entry fixture, then an incremental run that sees only changes (SC-004).

- [x] T048 [P] [US3] Integration tests in `crates/horae/tests/harvest_api/pagination.rs`: `per_page` 1, 2000 and 2001 (422); default 2000; absolute links on the request host; cursor walk equals page walk; `updated_since` returns only changed records per kind *(in `crates/harvest-api/src/pagination_tests/sync.rs`)*
- [x] T049 [US3] Write migration `crates/horae/migrations/<next>_catalogue_updated_at.sql` per data-model.md (tasks `created_at`/`updated_at`, `updated_at` on projects, clients, users, `set_updated_at` triggers, backfill); regenerate `.sqlx` *(already done by `crates/server/migrations/0002_catalogue_updated_at.sql` and `0003_updated_at_on_change.sql`)*
- [x] T050 [US3] Add `crates/horae/src/harvest/pagination.rs`: page envelope, opaque cursor over `(updated_at, id)`, absolute links from `Host` and `X-Forwarded-Proto` (research R-8); move every list handler onto it *(in `crates/harvest-api/src/pagination.rs`; catalogue lists are now ordered by `(updated_at, id)`, time entries and my project assignments page by number only)*
- [x] T051 [US3] Implement every documented time-entry filter and field Horae can fill (FR-022, FR-042) in `crates/horae/src/harvest.rs` time-entry handlers, with an index check against `crates/horae/migrations/0014_time_entry_query_indexes.sql` *(the baseline's `time_entries` indexes cover `org_id, updated_at`, `task_id`, `org_id, user_id, project_id` and `org_id, spent_date`; `client_id` filters through the joined project)*
- [x] T052 [P] [US3] Return empty enveloped lists for absent kinds (expenses, expense categories, estimates, estimate item categories, contacts, invoice payments, invoice item categories, roles) in `crates/horae/src/harvest/absent.rs` (D-2, FR-043) *(in `crates/harvest-api/src/absent.rs`; invoice payments are listed under an invoice, so they come with invoices in step 5)*
- [x] T053 [US3] Measure the 2000-row page on the 100,000-entry fixture and record the timing in `specs/021-harvest-api-compat/quickstart.md`; add the sync workflow to the suite *(the fixture is loaded by SQL into a dev database, as quickstart §5 describes; the workflow is `nix/checks/harvest-compat/clients/go-harvest/sync.go`)*

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

- [ ] T062 [P] Write the hand-made reference shapes for each implemented endpoint from the public docs in `crates/horae/tests/fixtures/harvest_reference/` and a field-by-field shape test in `crates/horae/tests/harvest_api/reference_shapes.rs` (FR-073) *(Step 5 facts checked against the public reference on 2026-11-22 and recorded in contracts/harvest-api-v2.md Step 5: the invoice `state` filter takes one value; invoices have `period_start` and `period_end`; the project budget report lists only projects with a budget, pages 1–2000 with default 2000, rounds hours per the account's setting, and takes only `page`, `per_page` and `is_active` (no `from`/`to`); the time reports and the uninvoiced report take `per_page` 1–2000 with default 2000 and require `from` and `to`; the uninvoiced report takes `include_fixed_fee` (default true: a fixed-fee project's uninvoiced fee shows when the range is on or after the project's start date, else 0). The public API v2 reference has no tags resource, so tags (FR-044 item 4, formerly T077) are dropped from step 5. Checked on 2026-11-23: the invoice list runs newest `issue_date` first (no tie-break stated; id descending is Horae's); on the time reports `include_fixed_fee` is an optional string whose `true` adds billable amounts for fixed-fee projects, with no default stated, so Horae treats absent as `false`. Nothing from step 5 is left to check here.)*
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
- **US4** (Phase 5) T043–T047 and T067–T080 need US1 and step 4's pagination (#454); the
  invoice tasks (T045, T070, invoice parts of T074–T076) also need spec 025's migration and
  #453. T073 → T074 → T075 → T080 come first; T042, T044 and T076 need the assignment numbers, T045
  the invoice and line numbers. T043, T046 and T078 need no migration. (T077, tags, was dropped:
  see T062.)
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
- US4 (step 5): the tests T067–T072 together; after the migrations, T042 with T044, and T043,
  T045, T046 and T078 in parallel (separate files).
- US3: T048 and T052 together, before T050.

## Implementation strategy

1. **MVP**: Phases 1–3. Every token-based client can read Horae with integer ids and exact
   numbers. Stop and validate with quickstart §1–2 and the suite.
1. The amendment was approved before Phase 4 (2026-11-09), so US2, US5 and US6 proceed.
1. Each later phase is one pull request that leaves the API working and the suite green.
