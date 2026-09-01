# Tasks: Harvest Data Importer

**Input**: Design documents from `/specs/004-harvest-importer/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The plan's Testing section and quickstart.md call for `horae-core` unit tests and `#[sqlx::test]` integration tests (the Harvest API adapter exercised against stubbed HTTP fixtures), so each story carries test tasks.

**Organization**: Tasks are grouped by user story (spec.md priorities) to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Exact file paths are included in each description

## Path Conventions

Two-crate workspace (plan.md **Project Structure**): pure domain in `crates/core/`, server engine + delivery in `crates/horae/`. Migrations in `crates/horae/migrations/`, integration tests + fixtures in `crates/horae/tests/`.

______________________________________________________________________

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Module scaffolding, dependencies, and configuration shared by every story.

- [X] T001 Create module roots and register them: `crates/core/src/harvest_import.rs` (+ `crates/core/src/harvest_import/` dir) declared in `crates/core/src/lib.rs`, and `crates/horae/src/harvest_import.rs` (+ `crates/horae/src/harvest_import/` dir) declared `#[cfg(feature = "server")]` in `crates/horae/src/main.rs`, following the `foo.rs` + `foo/` convention.
- [X] T002 [P] Confirm/add server-crate dependencies in `crates/horae/Cargo.toml`: an async HTTPS client (reuse `reqwest` if already transitive via the OIDC stack, else add), `serde_json`, and one audited AEAD crate for token encryption; leave `crates/core/Cargo.toml` free of I/O deps (Constitution II).
- [X] T003 [P] Add Harvest OAuth + encryption config to `crates/horae/src/config.rs`: client id, client secret, redirect URL (`/auth/harvest/callback`), and the token-encryption key, loaded from env in `AppConfig::from_env` alongside the existing OIDC/session secrets.

______________________________________________________________________

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The pure-domain conversions/keys, the shared source-agnostic engine (resolve → apply → report), and the provenance table — every user story depends on these.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Create the provenance migration `crates/horae/migrations/0007_harvest_import_map.sql`: a `harvest_entity_type` enum (`client|project|task|time_entry`) and the `harvest_import_map` table `(org_id, harvest_entity_type, harvest_id) PK → horae_id, harvest_updated_at, created_at)` per data-model.md — additive, altering no existing columns.
- [X] T005 [P] Define shared types in `crates/core/src/harvest_import/types.rs`: `SourceRow` (incl. optional Harvest ids), `RowOutcome` (`Created|Updated|Skipped|Errored{source_location, reason}`), `ImportSummary` (per-entity created/updated/skipped/errored), and the `ImportMode { DryRun, Commit }` / `SyncScope { Full, Incremental }` enums (no `Option<bool>`).
- [X] T006 [P] Implement decimal-safe conversions in `crates/core/src/harvest_import/convert.rs`: `hours_to_minutes` = `round(hours*60)` half-up and `money_to_cents` = `round(amount*100)` half-up, parsing decimals without `f64` boundary error (FR-005/FR-006).
- [X] T007 [P] Implement natural-key normalization (trim + case-fold) and the composite key builders (client/project/task/time-entry) in `crates/core/src/harvest_import/keys.rs` (FR-012).
- [X] T008 [P] Unit tests for conversions in `crates/core/src/harvest_import/convert.rs` (`#[cfg(test)]`): round-trip vs the exporter's `minutes/60` and `cents/100`, half-up boundaries, and the sufficient-precision caveat (SC-003/SC-007, research.md §3).
- [X] T009 [P] Unit tests for key normalization in `crates/core/src/harvest_import/keys.rs`: trim/case-fold equality and composite-key distinctness (research.md §4).
- [X] T010 Implement provenance access in `crates/horae/src/harvest_import/provenance.rs`: lookup `(org, entity_type, harvest_id) → horae_id` and persist a mapping row, both taking an executor so they enlist in the caller's transaction (data-model.md).
- [X] T011 Implement the resolver in `crates/horae/src/harvest_import/resolve.rs`: provenance-first, composite-natural-key fallback, org-scoped, with an in-run parent cache so each distinct client/project/task resolves once (FR-004, FR-012, research.md §9).
- [X] T012 Implement FK-safe per-record application in `crates/horae/src/harvest_import/apply.rs`: create/skip/update client → project → task (+ `project_tasks`) → time entry as an all-or-nothing savepoint unit, writing the provenance row in the same unit on commit (FR-004, FR-020, FR-026).
- [X] T013 Implement the report types and reconciliation in `crates/horae/src/harvest_import/report.rs`: `ImportReport { source, mode, summary, row_errors }` with the `processed = created+updated+skipped+errored` invariant per entity (FR-021).
- [X] T014 Implement the engine orchestrator in `crates/horae/src/harvest_import.rs`: drive a `SourceRow` stream through resolve → apply → report, source-agnostic, with the `DryRun` path running inside a rolled-back transaction so nothing (data, provenance, watermark) persists (FR-014, research.md §7).

**Checkpoint**: Pure conversions/keys tested green; the shared engine compiles and can apply a hand-built `SourceRow` stream. Story adapters and surfaces can now be built in parallel.

______________________________________________________________________

## Phase 3: User Story 1 - Connect Harvest and pull a migration over the API (Priority: P1) 🎯 MVP

**Goal**: An admin connects their Harvest account via OAuth2 and pulls clients → projects → tasks → time entries through the shared engine, with exact minute/cent conversions and provenance recorded.

**Independent Test**: Connect a Harvest account (stubbed token exchange) on an empty org, run the API import, and confirm clients/projects/tasks/time entries appear with correct names, dates, exact minutes and cents, and matching created-counts.

### Tests for User Story 1

- [ ] T015 [P] [US1] Integration test in `crates/horae/tests/harvest_import_connect.rs` (`#[sqlx::test]`, `#[serial]`): the OAuth callback with a valid `state` stores encrypted credentials in `harvest_credentials`; a missing/mismatched `state` is rejected without exchanging the code (FR-022, contracts/importer-api.md).
- [ ] T016 [P] [US1] Integration test in `crates/horae/tests/harvest_import_api.rs` (`#[sqlx::test]`, `#[serial]`): against stubbed paged HTTP fixtures, a full API import creates all four entity levels in FK-safe order with `minutes = round(hours*60)` and integer cents + ISO currency, and writes a `harvest_import_map` row per created record (US1 acceptance scenarios, SC-001/SC-003).

### Implementation for User Story 1

- [X] T017 [US1] Create the credentials migration `crates/horae/migrations/0008_harvest_credentials.sql`: `harvest_credentials` (org-unique) with encrypted `access_token_enc`/`refresh_token_enc` (bytea), `harvest_account_id`, `token_expires_at`, `scope`, `synced_watermark` jsonb, timestamps (data-model.md) — additive.
- [X] T018 [US1] Implement `crates/horae/src/harvest_import/credentials.rs`: AEAD encrypt/decrypt with the config key, plus load/store/refresh-persist of `harvest_credentials`; tokens never returned to callers as plaintext beyond in-memory use, never logged (FR-022).
- [X] T019 [US1] Implement `crates/horae/src/harvest_import/oauth.rs`: build the authorization-code URL with a per-start random `state` nonce bound to the admin session (+ PKCE), exchange the callback `code` for tokens, resolve the Harvest account id, and **validate `state`** on callback, rejecting a mismatch before exchange (research.md §10, contracts/harvest-api.md §A).
- [X] T020 [US1] Register the plain Axum OAuth callback route `GET /auth/harvest/callback` beside `auth::router()` in the server wiring (`crates/horae/src/main.rs` / `auth/`), performing the token exchange + credential store then redirecting into the admin screen (Constitution IV note in plan.md).
- [X] T021 [US1] Implement the primary source adapter `crates/horae/src/harvest_import/api_source.rs`: fetch `clients`, `projects`, `tasks` + `task_assignments`, `users` (reference), `time_entries` with `Authorization`/`Harvest-Account-Id`/`User-Agent` headers, following pagination to completion, backing off on HTTP 429, and refreshing an expired token mid-run — yielding the shared `SourceRow` stream (FR-023/FR-024, research.md §11).
- [X] T022 [US1] Add admin-only `#[server]` functions in `crates/horae/src/server_fns.rs`: `harvest_connect_start`, `harvest_connection_status`, and `import_harvest_api(mode, sync)` — reject non-admins with `FORBIDDEN`, reject when no usable connection exists (FR-001/FR-003), calling the shared engine.
- [X] T023 [US1] Add the CLI subcommand `import harvest-api [--full|--incremental] [--dry-run]` in `crates/horae/src/cli.rs`, sharing the same engine and DB layer.
- [X] T024 [US1] Add the admin "Import from Harvest" screen in `crates/horae/src/pages/` (+ route): Connect button, connection status, run button, and the summary + per-record error report.

**Checkpoint**: An admin can connect and run a full API import that populates Horae exactly — MVP is demonstrable.

______________________________________________________________________

## Phase 4: User Story 2 - Re-sync without creating duplicates (Priority: P1)

**Goal**: Re-running the import matches existing records by Harvest provenance (exact, edit-robust) and creates only new rows; an incremental re-sync fetches only what changed since the last run.

**Independent Test**: Run the import twice unchanged → second run reports zero creations; edit an imported entry's notes in Horae and re-run → still matched by Harvest id, not duplicated; run incremental → only changed records fetched.

### Tests for User Story 2

- [ ] T025 [P] [US2] Integration test in `crates/horae/tests/harvest_import_idempotent.rs` (`#[sqlx::test]`, `#[serial]`): a second identical API run creates zero clients/projects/tasks/time entries; after editing an imported entry's notes, re-import still matches it by provenance (FR-011/FR-026, SC-002).
- [ ] T026 [P] [US2] Integration test in `crates/horae/tests/harvest_import_incremental.rs` (`#[sqlx::test]`, `#[serial]`): with a stored watermark, an incremental run sends `updated_since` and applies only the changed fixture records, leaving others intact (FR-025, SC-008).

### Implementation for User Story 2

- [X] T027 [US2] Wire skip/update counting into `crates/horae/src/harvest_import/apply.rs`: a provenance/natural-key match counts as `Skipped` (default) or `Updated` for a defined safe attribute subset, never a new creation (FR-017), and confirms/refreshes the provenance row.
- [X] T028 [US2] Implement the incremental watermark in `crates/horae/src/harvest_import/credentials.rs` + `api_source.rs`: read the per-entity `synced_watermark` and send `updated_since`, and advance it only after a successful committing run (FR-025).
- [X] T029 [US2] Thread `SyncScope::{Full,Incremental}` through `import_harvest_api` (`crates/horae/src/server_fns.rs`) and the `--full|--incremental` CLI flag (`crates/horae/src/cli.rs`), defaulting to incremental when a watermark exists.

**Checkpoint**: Re-runs and incremental syncs are safe and edit-robust; US1 + US2 both demonstrable.

______________________________________________________________________

## Phase 5: User Story 3 - Preview an import before committing (dry-run) (Priority: P2)

**Goal**: A dry-run reports would-create/update/skip/error per entity without writing anything (no data, no provenance, no watermark), and matches the subsequent real run.

**Independent Test**: Dry-run a source with new + existing + problem records; verify the counts and that row counts, `harvest_import_map`, and `synced_watermark` are unchanged; then commit and confirm the outcome matches the preview.

### Tests for User Story 3

- [ ] T030 [P] [US3] Integration test in `crates/horae/tests/harvest_import_dryrun.rs` (`#[sqlx::test]`, `#[serial]`): a `DryRun` API run writes nothing (clients/time_entries/`harvest_import_map`/`synced_watermark` all unchanged) and its counts equal a following `Commit` on the same fixtures (FR-014/FR-015, SC-004).

### Implementation for User Story 3

- [X] T031 [US3] Harden the `DryRun` path in `crates/horae/src/harvest_import.rs`: guarantee the rolled-back transaction also suppresses provenance writes and the watermark advance, and that `import_harvest_api`/`import_harvest_csv` + the `--dry-run` flag surface the would-\* summary distinctly (research.md §7).

**Checkpoint**: Dry-run preview is trustworthy across both sources.

______________________________________________________________________

## Phase 6: User Story 4 - Survive bad records with a per-record error report (Priority: P2)

**Goal**: A bad record is skipped and reported (with its source location + reason) without aborting the run; totals reconcile; no partial fragments remain.

**Independent Test**: Import a source with a known-invalid subset (unknown user, unparseable value); valid records import, invalid ones are skipped and reported, and `processed = created + updated + skipped + errored`.

### Tests for User Story 4

- [ ] T032 [P] [US4] Integration test in `crates/horae/tests/harvest_import_resilience.rs` (`#[sqlx::test]`, `#[serial]`): a mixed valid/invalid source imports all valid records, reports each invalid one with its source location + reason, leaves no partial fragment, and reconciles totals (FR-018/FR-020/FR-021, SC-005).

### Implementation for User Story 4

- [X] T033 [US4] Capture per-record failures in `crates/horae/src/harvest_import/apply.rs`: on a savepoint rollback, record a `RowOutcome::Errored{source_location, reason}` (Harvest id for API, CSV line for CSV) and continue the run (FR-018/FR-019).
- [X] T034 [US4] Add user resolution in `crates/horae/src/harvest_import/resolve.rs`: match each entry's person to a Horae user by email (from pulled Harvest users, or the CSV email/name), erroring the record when unmatched and never writing `users` (FR-010).
- [X] T035 [US4] Surface the per-record error report in the CLI (`crates/horae/src/cli.rs`, non-zero exit only on up-front rejection) and the admin screen (`crates/horae/src/pages/`), keeping partial success a success (FR-018).

**Checkpoint**: Migration-scale resilience is demonstrable on both sources.

______________________________________________________________________

## Phase 7: User Story 5 - Import from a Harvest CSV export (Priority: P3)

**Goal**: The secondary CSV adapter feeds the same engine; matching falls back to the composite natural key (no Harvest ids), and re-import is duplicate-free.

**Independent Test**: Upload a Harvest detailed-time-report CSV to an empty org; the four levels appear correctly; re-importing the same file creates zero duplicates (natural-key match).

### Tests for User Story 5

- [ ] T036 [P] [US5] Integration test in `crates/horae/tests/harvest_import_csv.rs` (`#[sqlx::test]`, `#[serial]`) with a fixture CSV under `crates/horae/tests/`: import populates the four levels; a re-import creates zero duplicates via the composite natural key; a malformed file is rejected up front with no writes (US5, FR-003).

### Implementation for User Story 5

- [X] T037 [US5] Implement the secondary adapter `crates/horae/src/harvest_import/csv_source.rs`: stream-parse the detailed-time-report CSV (per contracts/csv-format.md) into the shared `SourceRow` stream with Harvest ids `None`, matching headers case-insensitively (research.md §1/§9).
- [X] T038 [US5] Add CSV recognition/rejection: validate required columns and reject an unrecognized/empty file up front with a clear message and no writes (FR-003, contracts/csv-format.md).
- [X] T039 [US5] Add the `import_harvest_csv(file, mode)` `#[server]` function (`crates/horae/src/server_fns.rs`), the `import harvest-csv <FILE> [--dry-run]` CLI subcommand (`crates/horae/src/cli.rs`), and the upload control on the admin screen (`crates/horae/src/pages/`).

**Checkpoint**: All five stories independently functional; API primary + CSV secondary share one engine.

______________________________________________________________________

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T040 [P] Regenerate and commit the sqlx query cache for the new tables/queries: `cargo sqlx prepare --workspace -- --features server --all-targets` then `git add .sqlx/`.
- [ ] T041 [P] Confirm streaming keeps memory bounded on a ≥100k-record source (paged API stream / streamed CSV + in-run parent cache) — a large-fixture smoke assertion backing SC-006.
- [ ] T042 [P] Add a reconciliation integration check that full-import time and money totals equal the Harvest source totals with zero drift (SC-003/SC-007).
- [ ] T043 Run `crates/horae/.../quickstart.md` scenarios end-to-end (connect, dry-run, import, re-sync, CSV fallback) and reconcile the counts.
- [ ] T044 [P] `nix fmt` + `cargo clippy -p horae --features server` clean; `nix flake check` green before merge.

______________________________________________________________________

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately.
- **Foundational (Phase 2)**: depends on Setup — **blocks all user stories**.
- **User Stories (Phase 3–7)**: all depend on Foundational.
  - US1 (P1) is the MVP and should land first (it also creates the OAuth/credentials + API-source machinery US2/US3/US4 exercise).
  - US2 (P1) depends on US1's API source + credentials (watermark, provenance-on-commit).
  - US3 (P2) and US4 (P2) build on the engine and are largely independent of each other; both are demonstrable on the US1 API path (and later the US5 CSV path).
  - US5 (P3) is an added source adapter — independent of US1's OAuth, but reuses the same engine, resolve/apply, dry-run, and error reporting.
- **Polish (Phase 8)**: after the desired stories are complete.

### Within Each User Story

- Tests are written first and expected to fail before implementation.
- Core (horae-core) before engine; engine before adapters/surfaces; adapters before UI/CLI wiring.

### Parallel Opportunities

- Setup: T002, T003 in parallel (T001 first to create the module roots).
- Foundational: the horae-core tasks T005/T006/T007 (+ tests T008/T009) run in parallel; T010–T014 are server-engine and mostly sequential (shared files), gated on the migration T004.
- Within a story: the `[P]` test tasks run together; implementation tasks touching distinct files can overlap.
- Across stories: once Foundational is done, US5 (CSV adapter) can be built in parallel with US1 by a second developer, since it does not touch OAuth/credentials.

______________________________________________________________________

## Parallel Example: User Story 1

```bash
# Tests for US1 together (distinct files):
Task: "Integration test for OAuth callback + credential storage in crates/horae/tests/harvest_import_connect.rs"
Task: "Integration test for API import in crates/horae/tests/harvest_import_api.rs"

# Foundational horae-core tasks earlier, together:
Task: "Conversions in crates/core/src/harvest_import/convert.rs"
Task: "Key normalization in crates/core/src/harvest_import/keys.rs"
Task: "Shared types in crates/core/src/harvest_import/types.rs"
```

______________________________________________________________________

## Implementation Strategy

### MVP First (User Story 1 only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational — conversions, keys, engine, provenance table).
1. Complete Phase 3 (US1): OAuth connect + API pull + one delivery surface.
1. **STOP and VALIDATE**: connect a (stubbed/real) Harvest account and confirm an exact, provenance-recorded import.
1. Demo — this is the MVP.

### Incremental Delivery

1. Setup + Foundational → engine ready.
1. US1 → connect + API import (MVP).
1. US2 → safe re-sync + incremental.
1. US3 → dry-run preview; US4 → resilient per-record errors.
1. US5 → CSV secondary source.
1. Polish → reconciliation, scale, quickstart, formatting/cache gates.

______________________________________________________________________

## Notes

- `[P]` = different files, no dependency on an incomplete task.
- `[Story]` labels map tasks to spec.md user stories for traceability; Setup/Foundational/Polish carry no story label.
- The Harvest API adapter is tested against **stubbed HTTP fixtures** (paged JSON, a 429 backoff, a token refresh) so no live Harvest account is needed.
- Constitution guardrails: exact integer minutes/cents in `horae-core`; UUID v7 + `org_id` on every created row; all domain writes through the server-side path; the OAuth callback is the only new plain Axum route (credential exchange only).
- Deferred (not in these tasks): propagating Harvest deletions (mirror-delete mode), scheduled/automatic re-sync, multi-account, and Harvest entities beyond clients/projects/tasks/time entries.
