# Tasks: project activity foundation

Input: [spec.md](spec.md) and [plan.md](plan.md). Historical #208 tasks are
reference only; these are the executable tasks for this increment. Pure domain
code; no database, account data, migrations or UI.

## Setup

- [x] T001 Confirm `week::week_start` and `project::monthly_fee_date` are unchanged
  from the #208 reference and record the scope in `specs/004-project-dashboard/activity-foundation/plan.md`.

## US1 — Exact reporting periods (P1)

Independent acceptance: every period kind has the documented inclusive bounds;
invalid input never becomes all time.

- [x] T002 [US1] Add `ActivityRange` tests (parse, reversed, leap day, month-end
  anchor, quarters, year, calendar extremes) in `crates/core/src/project_activity.rs`
  with `todo!()` bodies, register the module in `crates/core/src/lib.rs`, and
  observe the failure.
- [x] T003 [US1] Implement `ActivityError` and `ActivityRange` in
  `crates/core/src/project_activity.rs` until the US1 tests pass.

## US2 — Exact weekly activity (P1)

Independent acceptance: buckets for all seven week starts reconcile exactly;
limits, negative input and overflow fail with no partial series.

- [ ] T004 [US2] Add `weekly_activity` tests (clipping, empty weeks, inclusive
  filter, week start at year end, single day, duplicates/order, bucket limit,
  zero limit, negative minutes, three overflow paths, date extremes, full-year
  reconciliation, largest total) in `crates/core/src/project_activity.rs` and
  observe the failure.
- [ ] T005 [US2] Implement `DailyActivity`, `WeeklyActivity` and `weekly_activity`
  in `crates/core/src/project_activity.rs` until all tests pass.

## Polish

- [ ] T006 Run core tests, Clippy (`-D warnings`) and rustfmt in the Nix dev shell
  with a private target directory; record exact results in
  `specs/004-project-dashboard/activity-foundation/plan.md`.
- [x] T007 Run Spec Kit analysis across spec/plan/tasks and record it in
  `specs/004-project-dashboard/activity-foundation/plan.md`.
- [ ] T008 Coordinator: two independent reviews, then publication and required CI;
  record PR and integrated verification in `specs/004-project-dashboard/activity-foundation/plan.md`.

Dependencies: T007 (analysis, before implementation) → T001 → T002 (red) → T003
→ T004 (red) → T005 → T006 → T008.
All tasks touch the same source file, so none run in parallel. Single author.

Traceability: AFR-001/002 → T002–T003; AFR-003/004 and ASC-001 → T004–T005;
AFR-005 and ASC-002 → T002–T006; AFR-006 → T001/T006.
