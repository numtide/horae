# Tasks: Harvest Import and Job Management CLI

**Input**: [plan.md](./plan.md), [spec.md](./spec.md), research, data model and CLI contract.
**Tests**: Required by SC-001–SC-006; use TDD and real-session/executable acceptance.

## Phase 1: Setup

- [x] T001 Resolve specification assumptions and generate the design artifacts in `specs/006-harvest-jobs-cli/`, preserving Constitution IV.
- [x] T002 Record the Nix development baseline and isolated test setup in `specs/006-harvest-jobs-cli/acceptance.md`; verify repository ignore rules and no root-worktree changes.

## Phase 2: Foundation

- [x] T003 Write failing CLI parsing/configuration/JSON-error tests in `crates/horae/src/cli/imports/tests.rs` and `crates/horae/tests/cli_imports.rs` (FR-008–010, FR-013).
- [x] T004 Implement remote argument structures and pre-configuration dispatch in `crates/horae/src/cli.rs` and `crates/horae/src/main.rs`; server commands retain existing defaults.
- [x] T005 Implement private origin-bound session loading, safe transport, bounded JSON and result envelopes in `crates/horae/src/cli/imports.rs` and `crates/horae/src/cli/imports/transport.rs`; cover insecure files, redirects, HTTP policy, redaction and malformed replies.
- [x] T006 Name stable existing Dioxus routes in `crates/horae/src/server_fns/importers.rs` and extend registration tests in `crates/horae/src/server_fns/importers/authorization_tests.rs`; preserve the same authorization boundary.

## Phase 3: US1 — Submit imports (P1)

**Independent test**: Both sources/modes submit durable jobs; terminal exit does not cancel them, previews do not persist imported changes and resubmission is content-bound.

- [x] T007 [US1] Write failing identical/conflicting/resubmission-age tests in `crates/horae/src/jobs.rs` and `crates/horae/src/server_fns/importers/authorization_tests.rs` (FR-001–003, FR-011–012).
- [x] T008 [US1] Implement optional request identity validation and atomic payload/digest collision checks in `crates/horae/src/server_fns/importers.rs` and `crates/horae/src/jobs.rs`; preserve policy, upload and checkpoints on identical resubmission.
- [x] T009 [US1] Implement API/CSV submission, streaming size bounds, source validation and indeterminate acknowledgement in `crates/horae/src/cli/imports.rs`, `crates/horae/src/server_fns/importers.rs` and `crates/horae/src/importers/harvest/csv_source.rs`.
- [x] T010 [US1] Regenerate changed metadata in `.sqlx/`; verify real-session source/mode, missing-connection and detached-submission tests in `crates/horae/src/server_fns/importers/authorization_tests.rs` and `crates/horae/tests/cli_imports.rs` (SC-001, SC-003).

## Phase 4: US2 — Observe outcomes (P1)

**Independent test**: Jobs created from either surface have matching status/history/reports; complete archived errors download without overwrites or silent truncation.

- [x] T011 [US2] Write failing status/history/report/archive/error-file tests in `crates/horae/src/cli/imports/tests.rs` (FR-004–005, SC-002).
- [x] T012 [US2] Implement status, bounded paginated history and confirmed report output in `crates/horae/src/cli/imports.rs`, including foreign/expired/not-found handling.
- [x] T013 [US2] Implement complete-error streaming and private atomic no-clobber publication in `crates/horae/src/cli/imports/transport.rs`; interrupted or truncated downloads preserve an existing destination.

## Phase 5: US3 — Control and automation (P2)

**Independent test**: Cancellation acknowledgement, retry and interrupted/timed-out waits preserve actual durable state and return documented output/exits.

- [x] T014 [US3] Write failing cancellation/retry/wait/signal/exit-matrix tests in `crates/horae/src/cli/imports/tests.rs` and `crates/horae/tests/cli_imports.rs` (FR-006–008, SC-004, SC-006).
- [x] T015 [US3] Implement cancel/retry/wait with bounded pending requests, signal handling, partial-success presentation and stable exit codes in `crates/horae/src/cli/imports.rs`.

## Phase 6: Acceptance and delivery

- [x] T016 Exercise every operation with actual admin/member/manager/inactive/demoted/foreign/expired sessions in `crates/horae/src/server_fns/importers/authorization_tests.rs`; verify secret-safe errors and client parity (SC-005).
- [x] T017 Run actual CLI source/mode, interruption/restart/retry and complete-report scenarios from `specs/006-harvest-jobs-cli/quickstart.md`; record requirement-by-requirement evidence in `specs/006-harvest-jobs-cli/acceptance.md`.
- [x] T018 Update `specs/004-harvest-importer/tasks.md`, `specs/004-harvest-importer/contracts/importer-api.md`, `specs/005-durable-harvest-jobs/contracts/import-jobs.md` and `AGENTS.md` with verified commands, route upgrade notes and current completion state.
- [x] T019 Run core/server suites, all-target Clippy, WASM compilation, Nix formatting, fresh offline SQLx verification and full flake CI; audit the diff and publish verified work to PR #199. Record results in `specs/006-harvest-jobs-cli/acceptance.md`.

## Dependencies and Execution Order

T001 → T002 → T003–T006 foundation → US1 (T007–T010) → US2 (T011–T013) → US3 (T014–T015) → T016–T019.
Within each story write and run failing tests before implementation, then verify the story. US2 and US3 can be exercised with seeded existing jobs, independently of new CLI submission.

## Parallel Opportunities

- US1: isolated enqueue tests can run alongside pure parser tests after their implementations exist.
- US2: mock download tests and real-session report tests use separate fixtures.
- US3: subprocess signal tests and pure exit-code tests are independent.
- Avoid concurrent edits to shared CLI/importer modules and overlapping global-AppState tests. Only read-only research is delegated here.

## Implementation Strategy

Deliver each story as a testable increment in the same feature branch/PR. US1 is the first usable checkpoint, not the goal's completion condition. Completion requires all three stories, all acceptance criteria and all verification gates.
