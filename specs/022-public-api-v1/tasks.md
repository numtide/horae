# Tasks: Public API `/api/v1`

**Input**: Design documents from `specs/022-public-api-v1/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: included. The constitution requires tests for correctness-critical changes, and the
spec's success criteria (SC-003, SC-004, SC-007, SC-008) are measured by tests.

**Organization**: one phase per pull request (plan.md *PR split*), as the maintainer asked, in an
order that respects the dependencies. Each task carries the user story it serves (`[US1]`–`[US6]`
from spec.md) when it serves one; infrastructure tasks carry none. Each PR ends with a checkpoint
that it passes `nix flake check` on its own.

## Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: can run in parallel with the other [P] tasks of its phase (different files, no
  dependency on an unfinished task)
- **[USn]**: the user story the task serves

## Dependencies between PRs

| PR | Depends on | Can start with |
| --- | --- | --- |
| A | — | B, E |
| B | — | A, E |
| C | B | D |
| D | B (merge after A) | C |
| E | — | A, B |
| F | C | D |
| G | E, F | H, D2 |
| H | A, E, F | G, D2 |
| D2 | C, D, F | G, H |
| I | G, H | — |

______________________________________________________________________

## Phase 1: PR A — Constitution amendment and the `/api/v1` namespace

**Goal**: Principle IV covers token-authenticated APIs (P-3); `/api/` belongs to Horae's own API.

**Independent test**: `/api/v2/users/me` answers 404 while `/harvest/v2/users/me` and
`/v2/users/me` still answer; the constitution reads 7.0.0.

- [ ] T001 [US3] Once the maintainer has approved the wording, apply the text of `specs/022-public-api-v1/constitution-amendment.md` to `.specify/memory/constitution.md`: retitle Principle IV "Mutations Through Shared Operations", replace the Harvest clause with the token-authenticated APIs clause, generalise the rationale's last two sentences, add the 7.0.0 Sync Impact Report and bump the version line (renumber if spec 020's amendment landed first)
- [ ] T002 [P] [US4] Remove the `/api/v2` nest from `harvest::router` and its comment in `crates/horae/src/harvest.rs`
- [ ] T003 [P] [US4] Replace the `/api/v2` alias assertions with a test that `/api/v2/users/me` and `/api/v2/users/me.json` answer 404 in `crates/horae/src/harvest/api_tests.rs`
- [ ] T004 [P] Update *Base paths* in `specs/021-harvest-api-compat/contracts/harvest-api-v2.md` and FR-026 in `specs/021-harvest-api-compat/spec.md` with a note pointing to 022 FR-052
- [ ] T005 Checkpoint: `nix flake check` passes; PR A opened

______________________________________________________________________

## Phase 2: PR B — Create `horae-api` (no behaviour change)

**Goal**: the HTTP surfaces live in `horae-api` (C-4), so later PRs add tokens and `v1` there.

**Independent test**: every existing route answers exactly as before (the moved tests pass
unchanged apart from paths), and `horae` builds against the new crate.

- [ ] T006 Create `crates/api/Cargo.toml` (`horae-api`: axum 0.8, tower-sessions, tower-sessions-sqlx-store, sqlx, serde, serde_json, tracing, horae-core, horae-types, horae-server; dev-dependency horae-server with `test-support`) and `crates/api/src/lib.rs`, and add `crates/api` to `members` in `Cargo.toml`
- [ ] T007 Move `crates/horae/src/http.rs` and `crates/horae/src/http/` (exports, import errors, Harvest OAuth callback, their tests) to `crates/api/src/` as `exports.rs` + `exports/`, `import_errors.rs` + `import_errors/`, `harvest_oauth.rs`, keeping `op_status` and its tests in `crates/api/src/lib.rs`
- [ ] T008 Move `crates/horae/src/auth/http.rs` and `crates/horae/src/auth/http/` (session, dev, oidc, page) to `crates/api/src/auth.rs` + `crates/api/src/auth/`; keep `LoginAssets` as the parameter through which `horae` passes asset URLs
- [ ] T009 Move `/health` from the closure in `crates/horae/src/main.rs` to `crates/api/src/health.rs`
- [ ] T010 Add `horae_api::router(options) -> axum::Router` in `crates/api/src/lib.rs` that merges health, exports, import errors, the Harvest OAuth callback and `/auth/*`, and exports `make_session_layer`, `login_redirect_guard` and `session::get_session_user_id`
- [ ] T011 Replace the moved routes in `crates/horae/src/main.rs` with `.merge(horae_api::router(…))`; repoint `crate::auth::http::…` and `crate::http::…` users (server functions, `harvest/auth.rs`) to `horae_api::…`; add the `horae-api` dependency in `crates/horae/Cargo.toml`
- [ ] T012 [P] Add `"../api/src"` to `watch_path` in `crates/horae/Dioxus.toml`
- [ ] T013 [P] Add `-p horae-api` to `nix/checks/tests.nix` (with the database thread limit) and `nix/checks/clippy.nix`
- [ ] T014 [P] Add `crates/api/src/lib.rs` to the `touch` line of the sqlx-prepare instructions in `AGENTS.md`, and describe `horae-api` in its *Architecture* section
- [ ] T015 Regenerate `.sqlx/` with `cargo sqlx prepare --workspace -- --features server --all-targets` and verify with an offline `cargo test --workspace --no-run`
- [ ] T016 Checkpoint: `nix flake check` passes; `git diff -M` shows moves; PR B opened

______________________________________________________________________

## Phase 3: PR C — Personal access tokens

**Goal**: people create, list and revoke tokens; administrators oversee them; `horae-api` can
authenticate a bearer token into a `Caller` (US1, US5).

**Independent test**: create a token in settings; a test route behind the extractor answers with
the owner; revoke it and the route answers 401; the database holds only a digest.

### Tests

- [ ] T017 [P] [US1] Unit tests for token generation, prefix, digest and `Debug` redaction in `crates/server/src/ops/api_tokens/tests.rs`
- [ ] T018 [P] [US1] `#[sqlx::test]` tests for `authenticate`: valid, unknown, revoked, expired, owner deactivated then reactivated, `last_used_at` stamped at most once a minute, in `crates/server/src/ops/api_tokens/tests.rs`
- [ ] T019 [P] [US5] `#[sqlx::test]` tests for `list_org` and `revoke` by administrator, owner and another member, in `crates/server/src/ops/api_tokens/tests.rs`
- [ ] T020 [P] [US1] Test that `Actor::load` with `Channel::Token` loads the same permissions as a session and none for an inactive person, in `crates/server/src/ops/actor/tests.rs`

### Implementation

- [ ] T021 [US1] Migration `crates/server/migrations/0003_api_tokens.sql` (number set at merge time) for `api_tokens` per `specs/022-public-api-v1/data-model.md`
- [ ] T022 [US1] Add `Channel::Token { token_id }` and `Caller::token(org_id, user_id, token_id)` in `crates/server/src/ops.rs`; make `Actor::load_with` in `crates/server/src/ops/actor.rs` load the person for it; make `ops::people::add` in `crates/server/src/ops/people.rs` refuse it
- [ ] T023 [US1] Implement `ops::api_tokens::{list_own, create, revoke, list_org, authenticate}` and the `Secret` type in `crates/server/src/ops/api_tokens.rs` (research R-6); add `sha2` and `subtle` as direct dependencies of `horae-server`
- [ ] T024 [P] [US1] Add the token DTOs (`ApiTokenRow`, `CreatedApiToken`) in `crates/types/src/api_tokens.rs`
- [ ] T025 [US1] Server functions `list_api_tokens`, `create_api_token`, `revoke_api_token`, `list_org_api_tokens` in `crates/horae/src/server_fns/api_tokens.rs` per `contracts/api-tokens.md`, with named status codes
- [ ] T026 [US1] Personal settings section "API tokens" (list, create with optional expiry, one-time reveal with copy, revoke with confirmation) in `crates/horae/src/pages/settings/api_tokens.rs`, linked from `crates/horae/src/pages/settings.rs`
- [ ] T027 [US5] Administration page of every token with owner and revoke in `crates/horae/src/pages/admin/api_tokens.rs`, linked from the admin navigation for administrators only
- [ ] T028 [US1] Bearer extractor `ApiCaller` in `crates/api/src/token.rs`: reads `Authorization: Bearer`, calls `ops::api_tokens::authenticate`, rejects with 401 and never logs the header
- [ ] T029 [P] [US1] Server-function tests for the four functions in `crates/horae/src/server_fns/api_tokens/tests.rs`
- [ ] T030 [P] [US1] UI test for the settings section (create, reveal once, revoke) in `crates/horae/tests/api_tokens_ui.rs`, and `cargo run -p horae-dioxus-lint` clean
- [ ] T031 Regenerate `.sqlx/`; checkpoint: `nix flake check` passes; PR C opened

______________________________________________________________________

## Phase 4: PR D — `horae-harvest-api` and its switch

**Goal**: the Harvest-compatible API is its own crate, mounted only when the operator turns it on
(US4).

**Independent test**: default start answers 404 with an empty body on `/harvest/v2/users/me` and
`/v2/users/me`; with `HORAE_HARVEST_API=1` the Harvest suite passes as before.

- [ ] T032 [US4] Create `crates/harvest-api/Cargo.toml` (`horae-harvest-api`: horae-harvest, horae-server, horae-api, axum, sqlx, serde) and add it to the workspace `members`
- [ ] T033 [US4] Move `crates/horae/src/harvest.rs` and `crates/horae/src/harvest/` to `crates/harvest-api/src/lib.rs` and its modules; `AuthUser` uses `horae_api::auth::session` for the session lookup
- [ ] T034 [US4] Add `harvest_api: bool` from `HORAE_HARVEST_API` (parsed like `DEV_LOGIN`) to `Config` in `crates/server/src/config.rs`, with a config test
- [ ] T035 [US4] Add `horae_harvest_api::mount(enabled, pool) -> axum::Router` in `crates/harvest-api/src/lib.rs`: the Harvest router when enabled, otherwise routes that answer 404 with an empty body for `/harvest/v2/*` and `/v2/*`; merge it in `crates/horae/src/main.rs` with `cfg.harvest_api`
- [ ] T036 [P] [US4] Router test of both settings of `mount` (404 empty body when off; served when on) in `crates/harvest-api/src/switch_tests.rs`
- [ ] T037 [P] [US4] NixOS option `services.horae.harvestApi.enable` (default `false`) setting `HORAE_HARVEST_API=1` in `nix/modules/nixos/horae.nix`
- [ ] T038 [P] [US4] Turn the switch on in `nix/checks/e2e.nix` and `nix/checks/harvest-compat/run.sh`, and add an e2e assertion that the default deployment answers 404
- [ ] T039 [P] Add `-p horae-harvest-api` to `nix/checks/tests.nix` and `nix/checks/clippy.nix`; add `"../harvest-api/src"` to `watch_path` in `crates/horae/Dioxus.toml`; add the crate to the sqlx `touch` line in `AGENTS.md`
- [ ] T040 [P] [US4] Upgrade note (off by default, how to turn it on, `/api/v2` gone) per `contracts/configuration.md`: update the *Harvest-style subset* section and the feature list in `README.md`, and repeat the note in the PR description
- [ ] T041 Regenerate `.sqlx/`; checkpoint: `nix flake check` passes; PR D opened

______________________________________________________________________

## Phase 5: PR E — Shared operations: restart, D-6, catalogue update times

**Goal**: the operations `v1` needs exist for the web app too (P-2, R-11, R-12).

**Independent test**: in the web app, a stopped entry of today can be restarted; starting a timer
while one runs stops the first; editing a project changes its `updated_at`.

### Tests

- [ ] T042 [P] [US3] Unit tests for restart eligibility (running, not open, not today, open today) in `crates/core/src/time_entry_timer.rs`
- [ ] T043 [P] [US3] `#[sqlx::test]` tests in `crates/server/src/ops/time/tests/timesheet/restart.rs`: restart continues the entry's minutes; restart and start stop a running timer with the stop clamp; refusals answer conflict with the web app's text
- [ ] T044 [P] [US6] Migration test that `updated_at` changes on update of a project, client, user and task, and that backfill equals `created_at`, in `crates/server/src/db/tests.rs` (or the migration tests module)

### Implementation

- [ ] T045 [US3] Create `crates/core/src/time_entry_timer.rs` (restart eligibility) and export it from `crates/core/src/lib.rs`
- [ ] T046 [US3] Add `TimesheetCommand::RestartTimer { entry_id }` in `crates/types/src/scoped_time.rs`
- [ ] T047 [US3] In `ops::time::apply` (`crates/server/src/ops/time.rs`): handle `RestartTimer`; make `StartTimer` and `RestartTimer` stop the subject's running timer first with `StopTimer`'s clamp, in the same transaction; keep `one_running_timer_per_user` as the guard
- [ ] T048 [US3] Web restart action on stopped entries of today in `crates/horae/src/pages/timesheet/data.rs` and the entry row component, through `apply_timesheet_command`; update `crates/horae/tests/timer_widget.rs` for "start stops the running timer"
- [ ] T049 [US6] Migration `crates/server/migrations/0002_catalogue_updated_at.sql` per `data-model.md`; exclude the new columns from the whole-row migration snapshot tests that cover `projects`, `clients`, `users` and `tasks`
- [ ] T050 Regenerate `.sqlx/`; checkpoint: `nix flake check` passes; PR E opened

______________________________________________________________________

## Phase 6: PR F — The `v1` base

**Goal**: `/api/v1` exists with its description, errors, cursors, limiter and identity (US1, US2).

**Independent test**: `GET /api/v1/me` with a token returns the owner; `GET /api/v1/openapi.json`
matches the snapshot; removing a field fails the snapshot test and the `api-compat` check.

### Tests

- [ ] T051 [P] [US6] Unit tests for cursors in `crates/api/src/cursor/tests.rs`: round trip; tampered byte, other list, other filters, older than 24 h and wrong key each refused
- [ ] T052 [P] [US1] Unit tests for the limiter in `crates/api/src/rate_limit/tests.rs`: window, reset, `Retry-After`, separate tokens
- [ ] T053 [P] [US3] Unit tests for `problem.rs`: each `OpError` kind to status and `code`, `field` and `reason` passed on, `Internal` detail withheld, in `crates/api/src/problem/tests.rs`
- [ ] T054 [US2] Test helper `assert_conforms(document, operation_id, status, body)` that validates a body against the generated schema with `jsonschema`, in `crates/api/src/v1/testing.rs`; every `v1` router test uses it
- [ ] T055 [US2] Snapshot test `openapi_snapshot_is_current` that compares the generated document with `crates/api/openapi/v1.json` and rewrites it under `UPDATE_OPENAPI=1`, in `crates/api/src/v1/tests.rs`
- [ ] T056 [P] [US1] Router tests for `/me`, `/me/trackable`, `/me/timer`: token required, cookie refused, query-string token refused, revoked token 401, 429 over the limit, in `crates/api/src/v1/me/tests.rs`

### Implementation

- [ ] T057 Add `utoipa` 5, `utoipa-axum` 0.2, `hmac`, `sha2`, `base64` and dev-dependency `jsonschema` to `crates/api/Cargo.toml`
- [ ] T058 [US3] `Problem` type and the single `OpError` mapping (research R-4), plus a JSON extractor that refuses unknown fields with 422 `unknown_field` and malformed bodies with 400, in `crates/api/src/problem.rs`
- [ ] T059 [US6] HMAC cursors (research R-5) in `crates/api/src/cursor.rs`; `cursor_key` from `HORAE_API_CURSOR_KEY` or a random key logged once, in `crates/server/src/config.rs`
- [ ] T060 [US1] Per-token fixed-window limiter as Axum middleware keyed by `Channel::Token`'s id in `crates/api/src/rate_limit.rs`; `rate_limit` from `HORAE_API_RATE_LIMIT` in `crates/server/src/config.rs`
- [ ] T061 [P] [US1] NixOS option `services.horae.api.rateLimit` in `nix/modules/nixos/horae.nix`; name `HORAE_API_CURSOR_KEY` in the `secretKeyFile` description
- [ ] T062 [US2] `v1` router with `OpenApiRouter`, security scheme, `GET /api/v1/openapi.json` (no auth) and the `Page<T>` and `Money` types in `crates/api/src/v1.rs` and `crates/api/src/v1/types.rs`; mount it from `horae_api::router`; add `/api/v1` handling to `login_redirect_guard` (already public under `/api/`)
- [ ] T063 [US1] `GET /me`, `/me/trackable` (`ops::time::tracking`) and `/me/timer` (`ops::time::current_timer`) in `crates/api/src/v1/me.rs`, using `ApiCaller`, `ops::begin_read` and `ReadTx::finish`
- [ ] T064 [US2] Generate and commit `crates/api/openapi/v1.json`; copy it to `crates/api/openapi/v1.baseline.json`; delete `specs/022-public-api-v1/contracts/openapi-v1.draft.yaml` or mark it superseded, and correct `contracts/api-v1.md` where the generated document differs
- [ ] T065 [P] [US2] Package `oasdiff` from a pinned release with `buildGoModule` in `nix/packages/oasdiff.nix`
- [ ] T066 [US2] Check `nix/checks/api-compat.nix`: `oasdiff breaking crates/api/openapi/v1.baseline.json crates/api/openapi/v1.json --fail-on ERR`
- [ ] T067 Checkpoint: `nix flake check` passes; PR F opened

______________________________________________________________________

## Phase 7: PR G — Catalogue reads

**Goal**: projects, tasks, clients and people are readable with the web app's visibility (US1,
US6).

**Independent test**: a member's token lists only the projects and tasks the member can see in
the web app, an administrator's lists all; a record outside scope answers 404; `updated_since`
returns only changed records.

### Tests

- [ ] T068 [P] [US1] `#[sqlx::test]` tests for each new `page`/`get` operation over the own, managed and all scopes, archived records and `updated_since`, in `crates/server/src/ops/projects/tests/api_reads.rs`, `crates/server/src/ops/clients/tests/api_reads.rs`, `crates/server/src/ops/projects/task_catalog/tests.rs` and `crates/server/src/ops/people/tests/api_reads.rs`
- [ ] T069 [P] [US6] Router tests for the eight endpoints with `assert_conforms`, cursor walks over more than one page, and field omission for non-catalogue readers, in `crates/api/src/v1/catalogue/tests.rs`

### Implementation

- [ ] T070 [P] [US1] `ops::projects::reads::{page, get}` reusing `project_rows`' rules, keyset by `id`, filters `client_id`, `active`, `updated_since`, in `crates/server/src/ops/projects/reads.rs`
- [ ] T071 [P] [US1] `ops::clients::{page, get}` reusing `clients::list`'s rules, in `crates/server/src/ops/clients.rs`
- [ ] T072 [P] [US1] `ops::projects::task_catalog::{visible_page, visible_get}`: `TaskReadAll` sees the catalogue, others the tasks of projects they may track, in `crates/server/src/ops/projects/task_catalog.rs`
- [ ] T073 [P] [US1] `ops::people::{page, get}` reusing `people::list`'s rules, in `crates/server/src/ops/people.rs`
- [ ] T074 [US1] `v1` types `Project`, `Task`, `Client`, `Person` and their conversions in `crates/api/src/v1/types.rs`
- [ ] T075 [US1] Handlers for `/projects`, `/tasks`, `/clients`, `/people` and their `/{id}` in `crates/api/src/v1/{projects,tasks,clients,people}.rs`
- [ ] T076 [US2] Refresh `crates/api/openapi/v1.json`; the `api-compat` check passes (additions only)
- [ ] T077 Regenerate `.sqlx/`; checkpoint: `nix flake check` passes; PR G opened

______________________________________________________________________

## Phase 8: PR H — Time entries

**Goal**: time entries are listed, read, created, changed, deleted and timed through the
timesheet's operations (US1, US3, US6).

**Independent test**: quickstart US1, US3 and US6 pass; every write refused in the web app is
refused through the API with the same reason.

### Tests

- [ ] T078 [P] [US6] `#[sqlx::test]` tests for `ops::time_reads::{list, get}` across scopes and every filter, and a 100,000-entry walk returning each row once, in `crates/server/src/ops/time_reads/tests/api_list.rs`
- [ ] T079 [P] [US3] `#[sqlx::test]` tests for `ops::time::patch`: partial changes, null clears, refusal on locked, approved and invoiced entries with the timesheet's reasons, in `crates/server/src/ops/time/tests/timesheet/patch.rs`
- [ ] T080 [P] [US3] Router tests for the eight time-entry operations with `assert_conforms`: success, 403, 404 outside scope, 409 conflicts, 422 validation and `unknown_field`, writes for another person allowed and refused by scope, plugin events dispatched after commit, in `crates/api/src/v1/time_entries/tests.rs`
- [ ] T081 [P] [US3] Parity test: for each refusal case of the timesheet's server-function tests, the API returns the same `detail` (SC-004), in `crates/api/src/v1/time_entries/parity_tests.rs`

### Implementation

- [ ] T082 [US6] `ops::time_reads::{list, get}` with the time-read scope, filters and keyset by `id`, in `crates/server/src/ops/time_reads.rs`
- [ ] T083 [US3] `ops::time::patch` (lock, merge, `Update` rules) in `crates/server/src/ops/time.rs`
- [ ] T084 [US3] `v1` types `TimeEntry`, `NewTimeEntry`, `TimeEntryPatch`, `NewTimer` and conversions in `crates/api/src/v1/types.rs`
- [ ] T085 [US1] `GET /time_entries` and `/time_entries/{id}` in `crates/api/src/v1/time_entries.rs`
- [ ] T086 [US3] `POST /time_entries`, `PATCH` and `DELETE /time_entries/{id}` through `ops::begin`, `ops::time::apply`/`patch` and `WriteTx::commit`, in `crates/api/src/v1/time_entries.rs`
- [ ] T087 [US3] `POST /timers`, `/time_entries/{id}/stop` and `/restart` in `crates/api/src/v1/timers.rs`
- [ ] T088 [US2] Refresh `crates/api/openapi/v1.json`; the `api-compat` check passes
- [ ] T089 Regenerate `.sqlx/`; checkpoint: `nix flake check` passes; PR H opened

______________________________________________________________________

## Phase 9: PR D2 — Harvest API on shared tokens and limiter

**Goal**: `/harvest/v2`, when on, authenticates with the shared tokens and shares the limiter
(FR-008, FR-062).

**Independent test**: with the switch on, one token reads `/harvest/v2/users/me` and
`/api/v1/me`; requests to both count against one limit; cookie reads still work.

- [ ] T090 [US4] Extend `AuthUser` in `crates/harvest-api/src/auth.rs` to accept `Authorization: Bearer` and `access_token`, then the session cookie for reads, through `ops::api_tokens::authenticate`; check `Harvest-Account-Id`/`account_id` against `organizations.api_account_id` (021 R-6)
- [ ] T091 [US4] Apply `horae_api::rate_limit` to the Harvest router; on 429 add `retry_after` to Harvest's body in `crates/harvest-api/src/errors.rs`
- [ ] T092 [P] [US4] Tests in `crates/harvest-api/src/api_tests.rs`: token reads, wrong account id 403, revoked 401, shared limit across both APIs, cookie reads unchanged
- [ ] T093 [P] [US4] Show the Harvest account id in the token settings section when the Harvest API is on, via a server function in `crates/horae/src/server_fns/api_tokens.rs` and `crates/horae/src/pages/settings/api_tokens.rs`
- [ ] T094 [P] [US4] Switch `nix/checks/harvest-compat/run.sh` and its Go client to a token
- [ ] T095 Checkpoint: `nix flake check` passes; PR D2 opened

______________________________________________________________________

## Phase 10: PR I — Documentation and end-to-end check

**Goal**: a person reaches a first call in under five minutes (SC-001), and a generated client
works unmodified (SC-002).

- [ ] T096 [US2] Confirm that `openapi-generator-cli` (nixpkgs 7.22.0) generates a Python client from `crates/api/openapi/v1.json` and that it runs offline in the sandbox; if not, record why and use `curl` plus `jq` scenarios
- [ ] T097 [US2] Check `nix/checks/api-v1.nix`: PostgreSQL and the server as in `nix/checks/tests.nix`, a seeded organization, a token from the test seed, and the generated client running quickstart US1, US3 and US6
- [ ] T098 [P] [US1] User guide `docs/api.md` (new directory): create a token, authenticate, list with cursors, incremental sync, errors, limits, compatibility promise; add a `/api/v1` subsection to the *API* section of `README.md` linking it, and link it from the token settings section
- [ ] T099 [P] Update the *Architecture* section of `AGENTS.md`: the API surfaces (`/api/v1`, `/harvest/v2` optional, exports, `/auth`), the crates and the Principle IV title
- [ ] T100 [P] Mark 021's step 2 (tokens) as delivered by 022 in `specs/021-harvest-api-compat/spec.md` *Delivery order*
- [ ] T101 Run every scenario in `specs/022-public-api-v1/quickstart.md` against `process-compose up` and record the result in the PR
- [ ] T102 Checkpoint: `nix flake check` passes; PR I opened

______________________________________________________________________

## Parallel examples

- **At once**: PR A (T001–T004), PR B (T006–T015) and PR E (T042–T049) touch different files.
- **After B**: PR C and PR D in two lanes.
- **After F**: PR G, PR H and PR D2 in three lanes; within G, T070–T073 are four files.
- **Inside C**: T017–T020 and T024 together, then T021–T023, then T025–T030.

## Implementation strategy

1. **MVP (US1 + US2)**: A, B, C, E, F. A token reads `/me` and the description is published and
   guarded.
1. **Useful v1**: G and H. Third parties read the catalogue and log time.
1. **Harvest users**: D early (its default changes deployments), D2 when C and F are in.
1. **Release**: I, then cut `v1.baseline.json` from the snapshot.
