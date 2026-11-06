# Tasks: Harvest people import and the Harvest import section

**Input**: [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/)

**Tests**: Requested by the brief. Write each test before its implementation and see it fail.

## Phase 1: Setup

- [ ] T001 Migration `crates/horae/migrations/0053_harvest_person_entity.sql` adds `'person'` to `harvest_entity_type`

## Phase 2: Foundational (core report types, blocks both stories)

- [ ] T002 [P] Core serde tests in `crates/core/src/importers/harvest/types.rs`: an old report without `people`/`summary.people`/`RowError.kind/person` loads; a new one round-trips; `reconciles()` counts person errors
- [ ] T003 Add `EntityType::Person`, `ImportSummary.people`, `ImportReport.people` + `ReportPerson`/`PersonOutcome`, `RowErrorKind`, `ErrorPerson`, and the optional fields on `RowOutcome::Errored` and `RowError` in `crates/core/src/importers/harvest/types.rs`; update every `RowOutcome::Errored` constructor
- [ ] T004 `job_report` counts people in `crates/horae/src/importers/harvest.rs`; label "People" in `crates/horae/src/pages/importers.rs` and `pages/importers/presentation.rs`

## Phase 3: User Story 1, people import (P1, PR A)

**Goal**: Harvest people are matched or created as deactivated members, and their entries import in the same run.

**Independent Test**: `cargo test -p horae --features server people`

### Tests (write first)

- [ ] T005 [P] [US1] Engine tests in `crates/horae/src/importers/harvest/engine_tests/people.rs`:
  - matched, both active and deactivated accounts, with every field unchanged and provenance written;
  - created as a deactivated member with no rates and no OIDC;
  - missing email, duplicate Harvest email and ambiguous Horae match, each with its kind;
  - entries of a created person import, and entries of an errored person get `person_not_imported`;
  - an idempotent second commit;
  - a provenance match after the email changes in Horae;
  - a dry run leaves no users, map rows or entries;
  - `report.people` rows with `harvest_active`.
- [ ] T006 [P] [US1] Durable tests in `crates/horae/src/importers/harvest/sync_tests/people.rs`, using the HTTP test server:
  - a multi-page durable dry run restores simulated people across entry pages, and its counts match a commit;
  - a resume inside the people batch.
- [ ] T007 [P] [US1] CSV tests in `crates/horae/src/importers/harvest/engine_tests/csv_streaming.rs` (or a new `engine_tests/csv_people.rs`):
  - an email row creates a person, counted once across many rows;
  - a name-only row still errors with `unmatched_name`;
  - a rolled-back row leaves no account;
  - a durable CSV preview restores simulated people.
- [ ] T008 [P] [US1] Event helper test: one `UserCreated` per `created` person on Commit, none on DryRun, `method: None`

### Implementation

- [ ] T009 [US1] `ApiUser.email: Option<String>`, `is_active` in `api_source.rs`; `RowLookup::row` keeps the email
- [ ] T010 [US1] `resolve_person` + `create_person` in `resolve.rs`; errors carry kind/person; `resolve_user` errors gain kinds
- [ ] T011 [US1] `Parent::Person` and the duplicate-email pre-pass in `parents.rs`; failed people marked; `report.people` rows appended
- [ ] T012 [US1] `apply_row` checks a failed person (`person_not_imported`); CSV person outcomes in `apply.rs`; CSV creation branch in `resolve_user`
- [ ] T013 [US1] `ParentSnapshot.users` capture and restore in `resolve/preview.rs`; `parent_count` includes users in `streaming/durable.rs`
- [ ] T014 [US1] Dispatch `UserCreated` after a committing job completes in `crates/horae/src/jobs.rs`
- [ ] T015 [US1] Scale fixture includes users in `sync_tests/scale.rs`; existing fixtures still pass
- [ ] T016 [US1] Docs: spec 005 Non-Goals, spec 004 FR-010/assumption/scope notes, `specs/harvest-parity.md` row and decision

**Checkpoint (PR A ready)**: `nix fmt`, clippy, `cargo test -p horae-core`, the full server tests on the scratch DB, sqlx prepare plus the offline check, the dioxus lint. Then report READY TO SIGN.

## Phase 4: User Story 2, the Harvest section (P2, PR B on `feat/harvest-import-section`)

**Goal**: Addressable Harvest pages with grouped errors, missing people and commit-from-preview.

**Independent Test**: `harvest-section.cjs` and the `importers` server tests

### Tests (write first)

- [ ] T017 [P] [US2] Pure `summarize` fold tests: lines split across chunks, groups by kind or reason, people counts, caps
- [ ] T018 [P] [US2] Server fn tests:
  - `harvest_import_error_summary`: admin only, scoped to the org;
  - `commit_harvest_import_preview`:
    - an API preview with the same account generation commits, and a changed generation is refused;
    - a CSV preview with its upload retained commits, and an expired upload is refused;
    - a newer job of the same source refuses the commit;
    - a repeated call returns the same job;
    - a non-admin is refused;
  - `add_harvest_people`: deactivated by default, active on request, a duplicate refused, events emitted, non-admin refused;
  - `JobStatus.mode/source` read from the payload.
- [ ] T019 [P] [US2] Route parse tests for each URL and `matches_navigation`; SSR tests in `detail_navigation*` and `admin_shell.rs`
- [ ] T020 [P] [US2] Browser `crates/horae/tests/browser/harvest-section.cjs`; `importers.cjs` updated; no planned-source assertions

### Implementation

- [ ] T021 [US2] `jobs/report.rs` `summarize` + `harvest_import_error_summary`
- [ ] T022 [US2] `commit_harvest_import_preview` and `JobStatus` fields
- [ ] T023 [US2] `insert_user` gains `active`; `add_harvest_people(people, Activation)`
- [ ] T024 [US2] Routes in `route.rs`; OAuth callback redirects to `/admin/importers/harvest`
- [ ] T025 [US2] Pages under `pages/importers/harvest/`, one component per file; remove the monolithic `HarvestImport`
- [ ] T026 [US2] Index lists Harvest API and Harvest CSV only; remove `PlannedSource` (Toggl Track, Clockify) and the component if unused
- [ ] T027 [US2] Dioxus lint baseline, screenshots, gates

## Dependencies

- T001 → T003 → everything else. Within US1, the tests (T005–T008) come before T009–T015.
- US2 needs US1's `RowError.kind/person` and `report.people`, so PR B stacks on PR A.
- [P] tasks touch different files.
