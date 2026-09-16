# Tasks: Consistent Harvest Import Experience

**Input**: `specs/008-harvest-import-ux/` specification, plan, research, model and UI contract.

**Tests**: Required by FR-015. Write matching behavior tests first and observe red before implementing each change; preserve current green regression cases. All checkboxes below are implementation work, not planning progress.

## Phase 1: Setup

- [x] T001 Read the applicable design/Rust/testing skills and full Importers handoff/support resources; record baseline states, component/token mapping and justified deviations in `specs/008-harvest-import-ux/acceptance.md` without touching the real account (FR-012, FR-014).

## Phase 2: Foundational presentation decisions

- [x] T002 Add failing table-driven cases for known/unknown labels, exact result summaries, empty-selection semantics and completeness independent of retry eligibility in `crates/horae/src/pages/importers/presentation.rs` (FR-005, FR-006, FR-007, FR-009, FR-011).
- [x] T003 Implement only the pure display helpers required by T002 and register the sibling module in `crates/horae/src/pages/importers.rs`; preserve existing request/state coordinator behavior (FR-004, FR-014).

**Checkpoint**: Helpers are independently tested; no new persistence or generic workflow abstraction.

## Phase 3: US1 — Connection management (P1, MVP)

**Independent test**: Seven connection categories and blockers can be exercised without importing.

- [x] T004 [US1] Add failing connection/loading/unavailable/expired/bound-state, blocker and disclosure cases in `crates/horae/tests/import_jobs_ui.rs` (FR-001, FR-002, FR-003, SC-001).
- [x] T005 [US1] Group connection management actions and preserve the stable native Change account modal, original reconnect identity and pending-action guards in `crates/horae/src/pages/importers.rs` (FR-001, FR-002, FR-003).
- [x] T006 [US1] Replace unsupported refresh/configuration promises and add action-specific connection errors with safe secondary detail/recovery in `crates/horae/src/pages/importers.rs` (FR-001, FR-011).
- [x] T007 [US1] Extend stale confirmation/cancel/release-recovery coverage in `crates/horae/tests/import_jobs_ui.rs` and run existing preservation/authorization cases in `crates/horae/src/importers/harvest/account_switch.rs` (FR-003, FR-014, SC-001, SC-005).

## Phase 4: US2 — Preview and truthful results (P1)

**Independent test**: API/CSV fixtures cover current/historical previews, mixed/zero/all-skipped outcomes and interrupted reports.

- [x] T008 [US2] Add failing result-copy, primary-action-count and preview-origin invalidation cases in `crates/horae/tests/import_jobs_ui.rs` before changing the corresponding page behavior (FR-004, FR-005, FR-006, FR-010, SC-002, SC-005).
- [x] T009 [US2] Apply evidence-based empty/preview/success/partial/no-report messages and exact outcome summaries in `crates/horae/src/pages/importers.rs` using presentation helpers (FR-005, FR-006, FR-011).
- [x] T010 [US2] Make confirmation the sole primary workflow action for a current successful preview; preserve source/reload/history/account invalidation and pending-action safety in `crates/horae/src/pages/importers.rs` (FR-004, FR-010, SC-005).
- [x] T011 [US2] Preserve inline-vs-archived error totals and complete NDJSON downloads, and remove unsupported CSV-format promises in `crates/horae/src/pages/importers.rs`; verify affected cases in `crates/horae/tests/import_jobs_ui.rs` (FR-006, FR-011, FR-014).

## Phase 5: US3 — Background progress and history (P2)

**Independent test**: Restore/paginate job fixtures, interrupt monitoring, and inspect blocked retries without changing report completeness.

- [x] T012 [US3] Add failing retry-projection cases in `crates/horae/src/jobs.rs`, missing/future-field compatibility cases in `crates/horae/src/cli/imports/tests.rs`, and organization/role projection checks in `crates/horae/src/server_fns/importers/authorization_tests.rs` (FR-003, FR-009, FR-010, FR-014).
- [x] T013 [US3] Add defaulted `retry_availability` in `crates/horae/src/models/jobs.rs` and bounded server projection in `crates/horae/src/jobs.rs`; update existing JobStatus test constructors while preserving `can_retry()` and write-side policy (FR-003, FR-009, FR-010, FR-014).
- [x] T014 [US3] Add failing history/selection/pagination, unknown metadata, progress, polling-failure/resume, stale-response and old-account-partial cases in `crates/horae/tests/import_jobs_ui.rs` (FR-007, FR-008, FR-009, FR-010, SC-003, SC-005).
- [x] T015 [US3] Render readable source/mode/time/status, selected history and honest history fetch/empty states in `crates/horae/src/pages/importers.rs` without per-row requests or changing pagination (FR-005, FR-009).
- [x] T016 [US3] Present queued/running/cancelling/monitoring-unavailable states and observed counts, retain same-ID resume and reject stale observations in `crates/horae/src/pages/importers.rs` (FR-007, FR-008, FR-011, SC-003).
- [x] T017 [US3] Drive retry affordances from availability snapshots, explain old-account/missing-upload/unknown cases and preserve partial reports independently in `crates/horae/src/pages/importers.rs`; verify stale server rejection recovery without resubmission (FR-006, FR-010, SC-005).

## Phase 6: US4 — Design and browser usability (P2)

**Independent test**: Real browser keyboard/focus/layout checks against isolated fixtures at the specified viewport/zoom matrix.

- [x] T018 [US4] Add `crates/horae/tests/browser/importers.cjs` using existing browser-test conventions and explicit fixture URL; cover all four journeys, viewport bounds, disclosure/modal focus and pending actions, and run it red before the matching accessibility/layout fixes (FR-012, FR-013, FR-015, SC-004).
- [x] T019 [P] [US4] Align necessary importer structure in `crates/horae/assets/css/horae.css` with existing tokens, responsive layout and the handoff; avoid duplicating utilities or changing global shell styling (FR-012, FR-013).
- [x] T020 [US4] Add disclosure/selected-state semantics, meaningful polite announcements, keyboard-accessible errors and responsive utility composition in `crates/horae/src/pages/importers.rs`, preserving native Modal behavior (FR-012, FR-013).
- [x] T021 [US4] Execute the browser matrix including actual 200% zoom, using manual zoom evidence if necessary, and record outcomes/design deviations in `specs/008-harvest-import-ux/acceptance.md` (FR-012, FR-013, FR-015, SC-004).

## Phase 7: Verification and delivery

- [x] T022 [P] Document the additive snapshot contract and compatibility in `specs/005-durable-harvest-jobs/contracts/import-jobs.md` and `specs/006-harvest-jobs-cli/contracts/cli.md` (FR-014).
- [x] T023 Regenerate and review `.sqlx/` against the isolated migrated database with `cargo sqlx prepare --workspace -- --features server --all-targets` after all projection queries are final (FR-003, FR-014, FR-015).
- [x] T024 Run the core/server/UI/CLI tests, all-target Clippy, WASM build, formatting and full Nix gates from `specs/008-harvest-import-ux/quickstart.md`; record actual results and unchanged polling/pagination behavior in `specs/008-harvest-import-ux/acceptance.md` (FR-003, FR-014, FR-015).
- [x] T025 After explicit operator go-ahead, follow the real Preview-only acceptance in `specs/008-harvest-import-ux/quickstart.md` and record provider outcome plus unchanged business-data evidence in `specs/008-harvest-import-ux/acceptance.md`; otherwise leave this task blocked, not complete (FR-015, SC-006).
- [x] T026 Review final UX/state/security evidence against `specs/008-harvest-import-ux/contracts/importer-ui.md`, record results in `specs/008-harvest-import-ux/acceptance.md`, and open/update the scoped implementation PR without self-merging (FR-003, FR-012, FR-014, FR-015).

## Dependencies and execution order

- T001 precedes T002–T003. This foundation is the prerequisite for each independently testable story.
- US1 and US2 have no functional dependency on one another after the foundation, but both modify `importers.rs` and the UI harness: serialize those edits.
- US3 metadata work T012–T013 can proceed independently of connection/result rendering. T014 precedes T015–T017; do not use availability before T013.
- US4 browser scenarios may be prepared early but final execution depends on US1–US3. T018 precedes its matching fixes T019/T020; T021 follows both.
- T022 can run alongside independent validation work once T013's public contract is stable. T023 follows final query edits; T024 follows code completion and refreshed cache. T025 requires operator authorization and a verified matching instance. T026 follows all required evidence.

## Parallel examples

- **US1**: No parallel same-file UI edits; preservation tests may run while reviewing the pure-helper results.
- **US2**: Helper tests and the completed UI suite can run concurrently against separate test databases/targets; do not concurrently edit their shared files.
- **US3**: T012–T013 server/model work can run alongside US1/US2 page work only with ownership of the shared test-constructor edits coordinated.
- **US4**: After T018 establishes selectors/classes, T019 CSS and T020 page semantics touch different files and may proceed together; run T021 only after integration.
- T022 documentation and T023 cache preparation are independent once projection code is final.

`[P]` marks a genuine independent file lane; it is not authorization to spawn agents or run overlapping mutations.

## Implementation strategy

Deliver US1 as the first independently demonstrable increment, then results and history, then browser evidence. Each story remains testable with fixtures. Keep planning and implementation evidence distinct; do not mark tasks complete merely because the documentation exists. No real import commit, account change, deployment or PR merge is implied by this task list.
