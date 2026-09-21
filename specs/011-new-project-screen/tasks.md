# Tasks: New Project

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts](contracts/new-project.md).

**Tests**: Required by the constitution and specification. Write/run a failing check before its implementation; mark complete only after passing verification.

## Phase 1: Setup

- [x] T001 Create isolated branch/worktree and activate feature path in `.specify/feature.json`.
- [x] T002 Produce and validate specification, clarification coverage and technical contracts in `specs/011-new-project-screen/`.
- [x] T003 Verify repository ignore rules and isolated Nix/PostgreSQL/browser prerequisites using `.gitignore` and `crates/horae/tests/browser/run-design-checks.sh`; record validation commands in `specs/011-new-project-screen/quickstart.md`.

## Phase 2: Foundation

- [x] T004 Add failing pure tests for supported currency, exact percentages, billing mode choice, selected-type validation and calendar fee dates in `crates/core/src/project.rs` and `crates/core/src/invoice.rs`.
- [x] T005 Implement tested typed settings, exact validation and checked invoice adjustments in `crates/core/src/project.rs`, `crates/core/src/invoice.rs` and register in `crates/core/src/lib.rs`.
- [x] T006 Add raw form/draft/authorized option/result DTOs in `crates/horae/src/models/project_creation.rs` and module exports in `crates/horae/src/models.rs`.
- [x] T007 Add migration/constraint tests for private draft/configuration ownership, legacy defaults and entity limits in `crates/horae/src/server_fns/project_creation/tests.rs`.
- [x] T008 Add draft, configuration, private notes/costs, tags, task restrictions and scoped-budget schema in `crates/horae/migrations/0030_project_creation.sql`; preserve existing defaults and plugin grants.
- [x] T009 Register creation server module and reusable actor/entity validation in `crates/horae/src/server_fns/project_creation.rs` and `crates/horae/src/server_fns.rs`; prepare SQLx metadata in `.sqlx/`.

**Checkpoint**: Pure invariants and schema gates pass before user-story integration. No production database changes.

## Phase 3: US1 — Create a usable project (P1)

**Independent test**: Authorized create/reopen for each type, invalid input/foreign client rollback and real client dialog.

- [x] T010 [US1] Add failing finalization/client/tag/currency/rollback tests in `crates/horae/src/server_fns/project_creation/tests.rs` (FR-001–005, FR-008–009).
- [x] T011 [US1] Implement bounded creation options and atomic finalization of project/settings/tasks/team/tags in `crates/horae/src/server_fns/project_creation.rs`, revalidating active role and all org-scoped references under transaction locks.
- [x] T012 [US1] Implement explicit client-dialog creation/default rate persistence in `crates/horae/src/server_fns/clients.rs` and `crates/horae/src/models/project_creation.rs`, preserving existing client mutations.
- [x] T013 [US1] Add route/module and Projects creation links in `crates/horae/src/route.rs`, `crates/horae/src/pages.rs`, `crates/horae/src/pages/projects.rs`; keep existing edit/bulk behavior and active Projects navigation.
- [x] T014 [US1] Implement basic form/client modal/code suggestion/dates/currency/tags in `crates/horae/src/pages/new_project.rs` using actual options and shared controls.
  - Basic fields reuse shared form/menu/chip/calendar primitives with scoped dimensions and neutral chips. Real client search handles pending/error/empty results without changing the selection; explicit currency overrides survive client changes. Code suggestions remain opt-in, date values recover as ISO, and tag batches validate without dropping pending input. Thirty screen tests, nine navigation tests and the actual creation/recovery browser workflow pass, including keyboard and 390/768/1440 layouts. Downstream tag filtering (T015), complete-screen fidelity and cross-screen stability gates remain separate and open.
- [x] T015 [US1] Persist/reload details and add tag filtering to project/report consumers in `crates/horae/src/server_fns/projects.rs`, `crates/horae/src/server_fns/reports.rs`, `crates/horae/src/pages/projects.rs`, `crates/horae/src/pages/reports.rs`.
  - Saved details, authorized tag choices and UUID-based filters now reach Project Detail, Projects, grouped/detailed Reports and matching CSV/XLSX exports. Backend tests cover tenant boundaries, archived history, intersecting filters, multiple tags without duplicated totals and filtering before XLSX limits. The real creation → time entry → report/export browser workflow passes, including stale-row suppression during pending/failed reads and retry. Existing Projects design/bulk/recovery/action-error checks also pass. Complete-screen and transport gates remain separate.

## Phase 4: US2 — Resume a truthful draft (P1)

**Independent test**: Save/reload, interrupted request, two-tab conflict, discard, lost final response and repeated submission.

- [x] T016 [US2] Add failing ownership, payload-limit, revision-race and completion-retry tests in `crates/horae/src/server_fns/project_creation/tests.rs` (FR-007–008).
- [x] T017 [US2] Implement idempotent initial draft save, load, compare-and-swap save, discard and completed lookup in `crates/horae/src/server_fns/project_creation.rs`.
- [x] T018 [US2] Add one transactional project-created event record and retry-safe dispatch using `crates/horae/src/server_fns/project_creation.rs` and scoped `crates/horae/src/jobs.rs` outbox handling.
- [ ] T019 [US2] Implement debounced serialized autosave, truthful status, pending-navigation warning, retry/conflict/discard and finalization sequencing in `crates/horae/src/pages/new_project.rs`.
- [x] T020 [US2] Add browser recovery/concurrency cases in `crates/horae/tests/browser/new-project.cjs`, including edits during autosave and no false saved state.

## Phase 5: US3 — Configure billing and budgets (P1)

**Independent test**: Known-duration fixtures for every rate/type/scope, unchanged legacy fixtures, fee availability and isolated mail retry.

- [ ] T021 [US3] Add failing Rust/SQL parity and legacy/import rate fixtures in `crates/horae/src/server_fns/projects/tests.rs`, `crates/horae/src/server_fns/invoices/tests.rs` and `crates/horae/src/harvest/mod.rs` (FR-009–013, FR-017).
- [x] T022 [US3] Implement SQL rate resolution and fee occurrence/invoice exclusive-source schema in `crates/horae/migrations/0031_project_billing.sql`; deliberately handle SQL function privileges/plugin allowlists.
- [ ] T023 [US3] Apply selected rate modes and separate billing/cost currencies to `crates/horae/src/server_fns/projects.rs`, `crates/horae/src/server_fns/reports.rs`, `crates/horae/src/server_fns/invoices.rs`, `crates/horae/src/harvest/mod.rs` and related report models/exports.
  - Hourly checkpoint: shared SQL resolver, all five operational queries, separate report cost currency, per-project cost overrides, mixed-invoice currency rejection and Harvest time-entry financial redaction are implemented. T021/T023 remain open for the imported-configuration/currency audit and complete downstream fee/export coverage; fee schema is verified under T022.
- [x] T024 [US3] Add scoped/monthly budget and threshold boundary tests in `crates/core/src/budget.rs` and `crates/horae/src/server_fns/budget_tests.rs`.
- [x] T025 [US3] Implement scoped/monthly/inclusion-aware consumption and unique logical alert records in `crates/horae/src/server_fns.rs`, `crates/horae/src/server_fns/projects.rs`, `crates/horae/src/scheduler.rs`; retain legacy plugin event behavior for legacy projects.
  - `server_fns/budgets.rs` evaluates configured scopes/periods for both authorized Projects progress and atomic alert/outbox records; migration 0032 and the recovery sweep are connected. Projects displays current-period remaining budgets, independent scope overruns and separate lifetime tracking, with no lifetime fallback during loading/errors. Backend, actual creation-to-overview and keyboard/responsive/error browser checks pass; absent allowances do not become zero or alert thresholds. Optional mail is verified under T027/T028; full-feature browser/transport gates remain separate.
- [x] T026 [US3] Implement single/milestone/monthly fee preparation and duplicate-claim/void handling in `crates/horae/src/server_fns/invoices.rs` and `crates/horae/src/models/invoice.rs`, excluding configured fee work from hourly charges.
- [x] T027 [US3] Add failing mail availability, injection, timeout, retry/lease and acknowledged-delivery tests in `crates/horae/src/notifications.rs`.
- [x] T028 [US3] Implement optional bounded direct sendmail delivery, event-kind outbox claiming and sanitized terminal errors in `crates/horae/src/notifications.rs`, `crates/horae/src/config.rs`, `crates/horae/src/jobs.rs` and startup/shutdown wiring.
- [x] T029 [US3] Build all rate/type/fee/budget panels and per-task/person budget fields in `crates/horae/src/pages/new_project.rs` (or sibling section modules), with truthful unavailable-email state.
- [ ] T030 [US3] Re-run invoice, import, report, Harvest and budget fixtures and record unchanged legacy totals in `specs/011-new-project-screen/quickstart.md`.

## Phase 6: US4 — Tasks, team and privacy (P1)

**Independent test**: Direct endpoint role matrix, task restrictions including races, archived-history edits and safe timer stop after revocation.

- [x] T031 [US4] Add failing task/team/admin-note/cost privacy tests in `crates/horae/src/server_fns/project_creation/tests.rs` and `crates/horae/src/server_fns/projects/tests.rs` (FR-006, FR-014–015).
  - Creation/catalog and project/team read tests cover private notes/costs, role/tenant checks and rate-free member progress. The grouped report regression reproduced a manager reading the private override; report queries now revalidate current authority and omit restricted group costs without fabricating totals. Tests also cover a demoted creator's serialized draft and rejected save/finalize attempts containing private fields. Browser role verification remains T039.
- [ ] T032 [US4] Implement atomic catalog task creation/association, task restriction and deduplicated project memberships in `crates/horae/src/server_fns/project_creation.rs`, without promoting org roles or editing profiles.
- [x] T033 [US4] Add failing direct mutation restriction/race tests in `crates/horae/src/server_fns/time_entries/update_tests.rs`, preserving submission barriers and running-timer exit.
- [x] T034 [US4] Enforce task restriction in new-entry context SQL and update/reschedule/reorder/delete guards in `crates/horae/src/server_fns/time_entries.rs` and the feature migration; preserve privileged import/approval authority.
- [x] T035 [US4] Return rate-free allowed project progress and redact unauthorized project/task data in `crates/horae/src/server_fns/projects.rs`; retain manager gate on organization reports.
  - Six read-boundary integration checks cover current roles, leads/admins, member visibility, tenant mismatch, deactivation, revoked membership, archived identities, legacy settings and redacted-payload round trips. Tracking identities are separate from progress/catalog reads; configured monthly/scoped display is verified under T025, export/Harvest parity under T036 and the browser role matrix remains T039.
- [x] T036 [US4] Apply consistent authorization to project export count/stream and Harvest project/task/time-entry projections in `crates/horae/src/reports.rs`, `crates/horae/src/reports/limits.rs`, `crates/horae/src/reports/streaming.rs`, `crates/horae/src/harvest/mod.rs`.
  - CSV/XLSX and Harvest project reads share `project_read_access`; UI/Harvest task reads share private `task_read_access`. Real-session tests cover task counts/detail, archived personal history, redacted rates/budget metadata, current role changes and administrator-only cost overrides without fabricated manager fallback. Broader report cost privacy remains T031/T023.
- [x] T037 [US4] Verify private tables/functions stay outside plugin grants and payloads with tests in `crates/horae/src/plugin/database.rs` and `crates/horae/src/plugin/event.rs`.
  - A real restricted database role has no default access to all 14 new configuration tables/views or the rate resolver; intentionally granting each is rejected by the existing validator. Exact payload checks cover memberships and the actual transactional project-created outbox event from a form containing private notes/costs. No plugin allowlist expansion.
- [x] T038 [US4] Build task/team selectors, billable all/none, restrictions dialog, Add everyone, project leads, admin-only cost/notes and report visibility in `crates/horae/src/pages/new_project.rs` (or sibling section modules).
- [ ] T039 [US4] Exercise role matrix via browser/direct server responses and existing approval/import regressions in `crates/horae/tests/browser/new-project.cjs` and `specs/011-new-project-screen/quickstart.md`.

## Phase 7: US5 — Invoice defaults (P2)

**Independent test**: Defaults prefill, conflicting projects, explicit override, immutable prior invoice, exact displayed/exported components.

- [x] T040 [US5] Add failing invoice default/conflict/tax/discount/due-date/void tests in `crates/horae/src/server_fns/invoices/tests.rs` (FR-016–017).
- [x] T041 [US5] Add legacy-safe invoice snapshot fields and map optional time/fee sources in `crates/horae/migrations/0037_invoice_defaults.sql` and `crates/horae/src/models/invoice.rs`.
  - Exact adjustment and conflict tests reproduced ignored project terms and silently accepted mixed defaults. Eight new regression checks cover terms/PO/two taxes/discount, explicit conflict resolution, tenant/project selection, frozen values and draft-only edits, send/edit locking, fee selection, overflow rollback and an actual upgrade of legacy invoice data. Additive 0037 preserves existing dates/totals; source exclusivity remains unchanged. All 692 server-bin tests pass (11 pre-existing ignored).
- [x] T042 [US5] Implement project invoice preparation, mixed-default resolution, checked adjustments and draft-only override mutation in `crates/horae/src/server_fns/invoices.rs`.
  - Generation inherits or explicitly resolves defaults, limits selected projects, snapshots checked components and allows row-locked draft edits. The manager-authorized preparation API returns read-only time/fee estimates and per-project defaults; it shares source queries and fee calendars with generation. Real browser workflows now exercise inherited and conflicting defaults, stale-review invalidation, project selection, exact adjustments, read-only fee estimates, generation and persisted draft-only edits. T043 owns editable controls; visible/exported component breakdowns are complete under T044.
- [ ] T043 [US5] Implement payment terms/custom days/PO/tax/second-tax/discount form controls in `crates/horae/src/pages/new_project.rs` and editable prepared defaults/fee rows in `crates/horae/src/pages/invoices.rs`.
  - Shared invoice value controls and draft edits are implemented; fee rows remain source-owned/read-only. A clarification is pending because this task's “editable … fee rows” wording conflicts with the contract's preservation of source line amounts. Do not silently count individual fee-line editing as complete or broaden financial mutation behavior without resolving that ambiguity.
- [x] T044 [US5] Render/export invoice-owned components and fee rows in `crates/horae/src/reports.rs`, `crates/horae/src/reports/streaming.rs`, `crates/horae/src/reports/limits.rs`, `crates/horae/templates/invoice.typ`, with CSV/XLSX/PDF regressions.

## Phase 8: US6 — Design and accessibility (P2)

**Independent test**: Every conditional panel at 390/768/1440 widths and keyboard-only, plus existing shared-screen regressions.

- [x] T045 [P] [US6] Map remaining target/imported component reference states to tokens in `specs/011-new-project-screen/contracts/new-project.md`; do not render prototypes (FR-018–020).
  - Full-source mapping now records every panel, conditional state, reference dimension and shared-component relationship, with explicit outstanding layout/accessibility evidence. This completes the mapping, not the visual alignment: compact rate/budget/fee/invoice controls, task/team rows, main/footer geometry and full keyboard/failure coverage remain under T046–T050.
- [ ] T046 [US6] Add failing accessibility/layout/recovery cases in `crates/horae/tests/browser/new-project.cjs` before shared-control changes.
- [x] T047 [US6] Add only necessary opt-in labels/disabled options/focus/keyboard capabilities to `crates/horae/src/components/combobox.rs`, `crates/horae/src/components/form.rs` and other reused controls; preserve default callers.
  - Shared-input default/opt-in render tests pass. New Project selectors use SelectField and data-attribute-scoped menu/calendar behavior; existing Combobox, Checkbox, Menu and DatePicker defaults do not need replacement. Diff review, eight-screen computed-style comparison and the complete menu/navigation/browser suite verify existing callbacks, disabled choices, focus, viewport placement and keyboard behavior. Whole-screen field-error and conditional acceptance remain separate under T046/T048/T051.
- [ ] T048 [US6] Align label grid/cards/footer/spacing and responsive states in `crates/horae/src/pages/new_project.rs`, `crates/horae/assets/css/horae.css`, `crates/horae/build.rs`; no inline styles, copied literals or utility duplication.
  - Type/visibility/rate cards and compact rate, total-budget, single/monthly-fee and invoice-default fields now match their mapped dimensions. The scrolling form is separate from persistent actions, with scoped shell behavior and mapped gutters. Task/team rows use compact labelled overrides, scoped avatars, project roles and removal controls. Task entry has inline suggestions; teammate search uses the shared selector. Both suppress stale choices while loading and preserve input on failure with retry. Budget, monthly fee day and payment terms also reuse SelectField without changing choices or custom terms. Milestones use four-column desktop rows and stack below 900px, with exact values/totals, labelled native dates and removal focus return. Browser checks cover search, keyboard selection, duplicate protection, numeric typography, wrapping and forward Tab traversal, including 769/900/1180 sidebar/layout transitions. Heading/label typography, quiet dividers, section margins, the Back control and the theme-aware task information strip now pass computed-style assertions. This task remains open for the complete conditional keyboard/error-state pass.
- [x] T049 [US6] Integrate New Project tests into `crates/horae/tests/browser/run-design-checks.sh` and run all existing navigation/menu/project selection/recovery checks plus Clients/Timesheet/Invoices smoke checks.
  - The unfiltered nine-suite runner passes on the milestone/selector build, including Clients/Timesheet/Invoices responsive smoke checks, shared menus, mobile navigation, bulk selection/recovery, action errors, New Project and invoice preparation. Earlier intermittent failures and the deterministic interrupted-response-body regression are not resolved by this green run; T019/T051 remain open.
- [ ] T050 [US6] Verify every displayed control against persisted/downstream behavior and record design deviations and basic-creation timing in `specs/011-new-project-screen/quickstart.md`.

## Phase 9: Final verification and PR

- [ ] T051 Perform adversarial review of the complete diff against `specs/011-new-project-screen/spec.md`, Rust skills and shared CSS; fix all material findings and preserve regression tests.
- [ ] T052 Regenerate `.sqlx/` against isolated migrated PostgreSQL; run core/server tests, server Clippy, WASM check, formatting, generated utilities and flake gates from `specs/011-new-project-screen/quickstart.md`.
- [ ] T053 Document deployment mail configuration/limits and feature route in `DESIGN.md` and existing appropriate configuration docs; ensure `design/project/` remains unchanged.
- [ ] T054 Commit scoped changes on `feat/new-project-screen`, open a human-readable PR with test evidence and explicit limitations from `specs/011-new-project-screen/quickstart.md`; do not merge.

## Dependencies and execution

Setup → Foundation → US1 → US2 → US3 → US4 → US5 → US6 → Final. Each story has independent fixtures but end-to-end UI completion depends on prior persistence. US1 may call the provisional draft finalization path established in Foundation; US2 adds recovery/concurrency behavior. No intermediate slice is the final feature.

T041 uses additive migration 0037 rather than rewriting already-applied 0031; optional time/fee sources remain in T022's migration. Do not rewrite a migration after it has shipped. T023 depends on T022; T026 depends on T022 and precedes T042. T034 uses additive feature migration 0034 instead of rewriting historical 0019 or the already-applied 0030. T045 is independent documentation and can run beside implementation. Tasks touching the same file run sequentially.

Parallel examples: US1 browser scenario design can accompany server tests; US2 browser recovery scenarios accompany revision tests; US3 core budget tests can run alongside mail stub tests; US4 plugin grant checks accompany frontend role scenarios; US5 PDF fixture design accompanies core adjustment checks; US6 viewport runs can be parallelized by the existing runner. These are task opportunities, not authorization to launch additional implementation agents.

## Strategy and coverage

First prove exact validation and atomic basic creation. Add one verified story at a time, preserving legacy fixtures. All six stories and final gates remain required.

| Requirements | Tasks |
|---|---|
| FR-001–005 | T004–015 |
| FR-006 | T031, T035–039 |
| FR-007–008 | T010–011, T016–020 |
| FR-009–013 | T021–030 |
| FR-014–015 | T031–039 |
| FR-016–017 | T004–005, T040–044 |
| FR-018–020 | T014, T019–020, T045–050 |
| SC-001–002 | T010–015, T029, T038, T043, T050 |
| SC-003–005 | T016–018, T021, T024, T030–039, T040–044 |
| SC-006–007 | T046–052 |

**Counts**: 54 tasks; setup 3, foundation 6, US1 6, US2 5, US3 10, US4 9, US5 5, US6 6, final 4.
