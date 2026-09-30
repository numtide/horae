# Tasks: Project Detail Dashboard

> Historical tasks, pending regeneration from the expanded 2026-09-29 spec and
> reconciled plan. Do not execute these tasks as the current implementation
> contract. None is newly marked complete by the specification revision.

## Current independent foundation (2026-09-29)

These tasks replace the historical fold work for this increment only. The full
dashboard/action task set below is not executable until reconciled; completing
F001–F003 does not complete Project Detail or the four-surface delivery.

- [x] F001 [US2] Add failing domain tests in `crates/core/src/project_activity.rs`
  for calendar ranges, clipped configurable weeks, empty buckets, cumulative
  reconciliation, invalid input and explicit bucket/overflow errors.
- [x] F002 [US2] Implement the tested reporting ranges and activity series in
  `crates/core/src/project_activity.rs`; register in `crates/core/src/lib.rs`.
- [x] F003 [US2] Run core tests, Clippy and formatting in the Nix dev shell;
  record results and remaining integration in `specs/004-project-dashboard/delivery.md`.
- [x] F004 [US2] Add authorized activity DTOs and server projection in
  `crates/horae/src/models/project.rs` and `crates/horae/src/server_fns/projects/activity.rs`.
- [x] F005 [US2] Verify real database interval/split/week behavior and the full
  progress-permission boundary in `crates/horae/src/server_fns/projects/activity/tests.rs`.
- [x] F006 [US2] Regenerate `.sqlx/` with server/all-targets against the disposable
  migrated database; check server/WASM builds and update `delivery.md` evidence.
- [x] F007 [US2] Connect real activity charts, reporting presets/custom dates and
  an accessible weekly table using shared controls and scoped SVG styling.
- [x] F008 [US2] Test period/geometry boundaries and request/route transitions;
  verify server/WASM, framework consistency and Chrome behavior.
- [x] F009 [US1] Extend the authorized identity projection and implement the
  design header with real client/type/status, permission-gated edit navigation
  and confirmed archive/reactivate using the existing server action.
- [ ] F010 [US1] Verify identity/role/archive/route behavior, SQLx metadata,
  server/WASM and responsive browser layout; record evidence in `delivery.md`.
- [x] F011 [US1] Test and implement checked scope-budget aggregation in core.
- [x] F012 [US1] Add and test the scoped lifetime-hours/current-budget/internal-cost
  summary projection, preserving current permissions and existing spending rules.
- [x] F013 [US1] Connect the three summary tiles and verify loading/error/route
  behavior, both build targets, cache metadata and browser layout.
- [x] F014 [US2] Test and implement exact reciprocal task/person totals in core.
- [x] F015 [US2] Add the authorized interval-scoped breakdown projection, current
  and historical identities, cost privacy, explicit bounds and PostgreSQL tests.
- [ ] F016 [US2] Connect Tasks/Team tabs, counts, stable sorting and reciprocal
  disclosures to the chart period; verify route/loading and real browser behavior.
- [x] F017 [US3] Test checked invoice lifecycle totals without mixing currencies.
- [x] F018 [US3] Read project-attributable time/fee invoice history with stored
  discounts, current billing permissions, explicit bounds and database tests.
- [ ] F019 [US3] Integrate the invoice tab, real counts/links, labelled history,
  empty/loading/error states and keyboard navigation without resetting chart scope.
- [x] F020 [US3] Reuse invoice preparation and recovery from Project Detail with
  fixed client/project context, safe cancellation and route/browser regression.
- [x] F021 [US3] Add the Invoiced tile using the history's shared authorized
  response; separate currencies/counts and test pending/error/route behavior.
- [x] F022 [US2] Add independent weekly chart-window navigation with configured
  weeks, cumulative carry-in, no future navigation and browser regression.
- [x] F023 [US2] Reuse the shared calendar for chart-week selection with a local
  no-future bound; verify focus, dismissal, responsiveness and scheduling regression.
- [x] F024 [US2] Move the reporting period into the Tasks/Team toolbar; preserve
  selected tabs and independent invoice access through pending/failed reads, and
  test custom validation, cancellation, retry and chart/report isolation.
- [x] F025 [US2] Add task filtering to grouped/detailed Reports and CSV/XLSX,
  including the bounded-export preflight. Verify filter intersections, organization
  isolation, unchanged permission gates, real downloads and SQLx metadata.
- [x] F026 [US2] Connect contextual task/person/total report links and the project
  export menu, preserving all-time/custom scope and current report authority;
  implement the PDF summary and verify complete journeys from Project Detail.
  The shared all-time report/export period and explicit Reports selector are
  implemented. Contextual route/link integration passes route, screen and native
  browser regressions. CSV/Excel/PDF downloads, inclusive/all-time scope, keyboard
  and responsive controls, invalid filters, foreign scope, size rejection and
  revoked report authority pass the isolated browser suite. Full dashboard
  acceptance remains separate.
- [x] F027 [US2] Group chart week controls to match PeriodNav, preserving the
  configured week, calendar, no-future bound and return-to-current-week action.
  Reuse the summary read for a labelled hours-budget reference only when its
  period matches cumulative reporting and allocations are complete. Never turn
  money into hours or show a monthly/lifetime allowance against a different
  interval; distinguish current allowance from configured rounded consumption.
  Verify scale, zero/large budgets, privacy/loading/retry, one summary request,
  weekly-mode isolation and responsive/keyboard/browser regressions.
- [x] F028 [US2] Add the handoff's current-week chart band and label, deriving
  its position from visible configured-week buckets rather than the last entry.
  Preserve both chart modes, exact totals and historical navigation; verify
  clipped/year-boundary weeks, empty/old intervals, keyboard and enlarged layouts.

**Input**: Design documents from `specs/004-project-dashboard/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/server-fns.md, quickstart.md

**Tests**: Included. This is correctness-critical (money/hours reconciliation), so the constitution requires `horae-core` unit tests and `#[sqlx::test]` integration tests; the spec's success criteria (SC-002/003/004/006) are reconciliation guarantees that must be asserted.

**Organization**: Tasks are grouped by user story. Stories are independently testable; US1 is the MVP.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- File paths are repo-relative from the worktree root.

______________________________________________________________________

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: No new project scaffolding — this feature extends existing crates. Only confirm the working baseline.

- [ ] T001 Confirm the dev baseline builds and the DB is migrated/seeded: `cargo build -p horae --features server`, then `migrate run` + `seed` (per quickstart.md Prerequisites). No new files.

______________________________________________________________________

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared pure-domain fold and the view-model DTOs that every user story depends on. **No user story work can begin until this phase is complete.**

**⚠️ CRITICAL**: T002–T006 are shared by US1–US4.

- [ ] T002 [P] Create the pure rollup module `crates/core/src/project_rollup.rs`: define `EntryRow`, `Rollup`, `Bucket` (per data-model.md) and `fold_entries(impl Iterator<Item = EntryRow>) -> Rollup`, reusing `crate::invoice::resolve_rate` + `line_amount_cents`; accumulate total/billable/non-billable minutes, `billable_cents`, `uninvoiced_billable_cents`, `unresolved_rate_minutes`, and `by_task`/`by_person` buckets. Add `progress_pct(spent, budget) -> Option<u8>` (clamp 0..=100, `None` for zero budget). Register the module in `crates/core/src/lib.rs`.
- [ ] T003 [P] Unit tests in `crates/core/src/project_rollup.rs` (`#[cfg(test)]`): `billable + nonbillable == total`; Σ`by_task` == Σ`by_person` == totals (minutes and cents); all-`None`-rate billable row adds 0 cents and increments `unresolved_rate_minutes`; `uninvoiced_billable_cents` excludes invoiced rows; `progress_pct` clamps at 100 and returns `None` for budget 0; money stays `i64` cents (no float). (Run: `cargo test -p horae-core`.)
- [ ] T004 [P] Create view-model DTOs in `crates/horae/src/models/dashboard.rs` (per data-model.md): `ProjectDashboard`, `BreakdownRow`, `ProjectBreakdowns`, `EnabledTaskRow`, `RecentEntryRow` — plain serde structs, money as `Option<i64>`; compile for both targets. Export from `crates/horae/src/models.rs`.
- [ ] T005 Refactor `list_project_spend` in `crates/horae/src/server_fns/projects.rs` to build `EntryRow`s from its existing query and call `horae_core::project_rollup::fold_entries`, reading `total_minutes`/`billable_cents` — same signature and result, now sharing the fold (guarantees SC-002). (Depends on T002.)
- [ ] T006 Regenerate the sqlx offline cache after any query change and confirm it builds: `cargo sqlx prepare --workspace -- --features server --all-targets`; `git add .sqlx/`. (Re-run at the end of each phase that touches SQL.)

**Checkpoint**: Shared fold + DTOs exist and are unit-tested; the list rollup uses them. User stories can begin.

______________________________________________________________________

## Phase 3: User Story 1 - Budget & progress at a glance (Priority: P1) 🎯 MVP

**Goal**: The project detail page shows header identity + budget/progress + hours totals, computed from real data and matching the Projects-list "Spent".

**Independent Test**: Open an hours-budgeted, a fee-budgeted, and a no-budget project; confirm header, spent/budget/remaining, the progress bar (or its absence), and that "Spent" equals the list row (quickstart steps 1–5).

- [ ] T007 [US1] Add `get_project_dashboard(project_id)` to `crates/horae/src/server_fns/projects.rs` (per contracts/server-fns.md): session-authenticated, org-scoped (`NOT_FOUND` for foreign org); fetch project + client, the per-entry rows (the `list_project_spend` join plus `billable`/`invoice_id`/`task_id`/`user_id`), `SUM(invoice_line_items.amount_cents)` for the project; fold via `fold_entries`; build `ProjectDashboard` (budget fields selected by `budget_kind`, `progress_pct`, `money_incomplete`). Money fields `None` when the viewer may not see money. (Depends on T002, T004, T005.)
- [ ] T008 [P] [US1] Integration test in `crates/horae/tests/integration.rs` (`#[sqlx::test]`, `#[serial]`): seed a project with billable + non-billable entries; assert `get_project_dashboard` totals/split, `progress_pct` for hours/amount/none budgets, and that `spent_minutes`/`spent_billable_cents` equal `list_project_spend` for the same project (SC-002); assert `NOT_FOUND` for a foreign-org id; assert an empty-state project returns zeros.
- [ ] T009 [US1] Replace the `ProjectDetail` stub body in `crates/horae/src/pages/projects.rs`: load `get_project_dashboard` via `use_resource`; render the header (name/code, client, type badge, currency, active/archived, start/end when set) and a budget/hours summary card reusing `proj-bar`/`proj-bar-fill` for the progress bar; hours via `horae_core::duration::format_decimal`, money via `format_cents` (or "—" when `None`); no-budget → totals only. Keep the existing assignments block for now.
- [ ] T010 [P] [US1] Add dashboard card/stat styles to `crates/horae/assets/css/horae.css` (reuse existing tokens; extend `proj-bar` etc. as needed).

**Checkpoint**: MVP — the detail page is a real budget/progress dashboard, reconciling with the list.

______________________________________________________________________

## Phase 4: User Story 2 - Breakdown by task and by person (Priority: P2)

**Goal**: Per-task and per-person tables (hours, billable/non-billable split, amount) that reconcile to the headline totals.

**Independent Test**: For a project with two tasks and two people (some non-billable), confirm each table sums to the headline totals and each row's split adds up (quickstart step 6).

- [ ] T011 [US2] Add `list_project_breakdowns(project_id)` to `crates/horae/src/server_fns/projects.rs`: reuse the same per-entry query + `fold_entries`, join `by_task`/`by_person` buckets to `tasks.name`/`users.name`, return `ProjectBreakdowns` sorted by hours desc; money `None` when hidden. (Depends on T002, T004.)
- [ ] T012 [P] [US2] Integration test in `crates/horae/tests/integration.rs`: seed two tasks × two people, mixed billable; assert Σ`by_task` == Σ`by_person` == dashboard totals for both minutes and cents (SC-003), and a non-billable entry contributes 0 to `billable_cents`.
- [ ] T013 [US2] Render the by-task and by-person breakdown tables in `ProjectDetail` (`crates/horae/src/pages/projects.rs`), loaded via `use_resource`; columns: label, total hours, billable/non-billable hours, amount ("—" when `None`).

**Checkpoint**: US1 + US2 both work; breakdowns reconcile to the headline.

______________________________________________________________________

## Phase 5: User Story 3 - Invoiced vs uninvoiced (Priority: P2)

**Goal**: Show invoiced (from stored line-item amounts) and uninvoiced (resolved billable value of not-yet-invoiced work).

**Independent Test**: For a project with some invoiced billable entries, confirm invoiced = Σ line-item amounts and invoiced + uninvoiced = total billable (quickstart step 7).

- [ ] T014 [US3] Surface `invoiced_cents`/`uninvoiced_cents`/`money_incomplete` in the dashboard money summary within `ProjectDetail` (`crates/horae/src/pages/projects.rs`); these already come from `get_project_dashboard` (T007) — this task adds the money-summary section and the "money may be incomplete" flag / "—" handling. (Depends on T007, T009.)
- [ ] T015 [P] [US3] Integration test in `crates/horae/tests/integration.rs`: seed billable entries, invoice a subset (create invoice + line items, set `time_entries.invoice_id`); assert `invoiced_cents` == Σ `invoice_line_items.amount_cents` for the project and `invoiced_cents + uninvoiced_cents == billable total` (SC-004); a project with no invoices → invoiced 0, uninvoiced == total billable.

**Checkpoint**: US1–US3 deliver the full money picture.

______________________________________________________________________

## Phase 6: User Story 4 - Team, enabled tasks, recent activity (Priority: P3)

**Goal**: The team (assignments), the project's enabled tasks with billable flag/rate, and a newest-first recent-entries feed.

**Independent Test**: Confirm assignments, enabled-tasks list (billable + rate/"—"), and a recent-entries list ordered newest-first (quickstart step 8).

- [ ] T016 [P] [US4] Add `list_project_enabled_tasks(project_id)` to `crates/horae/src/server_fns/projects.rs`: `project_tasks JOIN tasks` → `Vec<EnabledTaskRow>` (billable, rate_cents, ordered by name; rate `None` when unset or hidden). (Depends on T004.)
- [ ] T017 [P] [US4] Add `list_recent_project_entries(project_id, limit)` to `crates/horae/src/server_fns/projects.rs`: `time_entries JOIN users JOIN tasks`, `ORDER BY spent_date DESC, created_at DESC LIMIT` (clamped, default 10) → `Vec<RecentEntryRow>` with a truncated note snippet. (Depends on T004.)
- [ ] T018 [US4] Render the enabled-tasks section and the recent-entries feed in `ProjectDetail`; keep/relocate the existing assignments block as the "Team" section. (Depends on T016, T017.)

**Checkpoint**: All four stories are independently functional.

______________________________________________________________________

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T019 Regenerate and commit the sqlx cache once all queries are final: `cargo sqlx prepare --workspace -- --features server --all-targets`; `git add .sqlx/`.
- [ ] T020 [P] Run `cargo clippy -p horae --features server` and `cargo test -p horae-core` clean; fix warnings.
- [ ] T021 Run `nix fmt` (treefmt) and ensure `nix flake check` is green.
- [ ] T022 Walk quickstart.md manual steps 1–10 (parity, reconciliation, invoiced/uninvoiced, empty/over-budget/unresolved-rate/hidden-money edge cases); confirm no migration was added (`git status crates/horae/migrations/` shows no new file — FR-016/SC-007).

______________________________________________________________________

## Dependencies & Execution Order

- **Phase 1 (Setup)** → **Phase 2 (Foundational)** blocks everything.
- **US1 (P1)** depends only on Foundational — MVP.
- **US2 (P2)**, **US3 (P2)**, **US4 (P3)** depend on Foundational; US3's rendering (T014) also depends on US1's `get_project_dashboard`/page (T007/T009); US2 and US4 are independent of US1's page beyond sharing the same `ProjectDetail` component.
- **Phase 7 (Polish)** after the desired stories.

### Within each story

- Server function → integration test (may be written first, TDD) → page rendering.
- The pure fold (T002) and its tests (T003) precede all server functions.

### Parallel opportunities

- T002 / T004 (different crates/files) in parallel; T003 alongside T002.
- Integration tests (T008, T012, T015) are [P] with each other (independent seeds).
- Server functions T016/T017 are [P] (different functions, same file — coordinate edits).
- CSS (T010) is [P] with server-side work.

______________________________________________________________________

## Implementation Strategy

- **MVP**: Phases 1–3 (Setup + Foundational + US1) — a real budget/progress dashboard that reconciles with the Projects list. Stop and validate.
- **Increment**: add US2 (breakdowns), then US3 (invoiced/uninvoiced), then US4 (team/tasks/recent), each independently testable.
- **Guardrails**: no migration (read-only); all money integer cents; the shared `fold_entries` keeps the list and dashboard from drifting.

## Notes

- [P] = different files, no incomplete-task dependency.
- Several server functions live in the same `server_fns/projects.rs`; when done in parallel, coordinate to avoid edit conflicts.
- Commit after each task or logical group; regenerate `.sqlx/` whenever a query changes.
