# Tasks: Approval Period Navigation

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[contracts/approval-week.md](contracts/approval-week.md), [quickstart.md](quickstart.md)

**Tests**: requested. Every test ID from the plan's requirement → test map is
written first and run red before its implementation task. A test added after
its code is recorded as such, never as red-green. Server and UI behaviour waits
until its red tests can actually run, which needs the heavy slot.

**Gate**:

- No code until root approves this package.
- Root granted `route.rs` and its callers.
- DB, heavy builds and native Chrome only in slots root grants.
- No push or PR before root collects the work.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Confirm the worktree is clean on `feat/approval-period-navigation` at base `8a796361`, and that `.specify/feature.json` still names `specs/016-expense-parity`.

## Phase 2: Foundational (blocks every story)

- [x] T002 [P] Add failing unit tests for `week_bounds` in crates/core/src/week.rs:
  - a Monday-start date maps to Mon..Sun;
  - a Sunday start maps to Sun..Sat;
  - a cross-year week keeps both ends;
  - the week containing `NaiveDate::MIN` returns `None` when its start underflows;
  - a week whose start + 6 exceeds `NaiveDate::MAX` returns `None`;
  - the last whole representable week is `Some`.

  Red, 2026-10-28: against a `todo!()` stub, 6 of 6 new tests panicked and the
  4 existing week tests passed.
- [x] T003 Implement `pub fn week_bounds(date, first_day) -> Option<(NaiveDate, NaiveDate)>` in crates/core/src/week.rs, reusing `week_start` and `checked_add_days(6)` (T002 green).

  Green: `cargo test -p horae-core` passed 237 tests. Core all-targets clippy
  with `-D warnings` was clean, and `rustfmt --check` was clean. Run in the
  offline dev shell with `CARGO_BUILD_JOBS=1` and a scratch `CARGO_TARGET_DIR`.
- [ ] T004 In crates/horae/src/route.rs, change the route to `#[route("/approvals?:week")] Approvals { week: String }`. In crates/horae/src/components/sidebar.rs, pass `Route::Approvals { week: String::new() }`. No other route changes.
- [ ] T005 Signature-only seam in crates/horae/src/server_fns/approvals.rs, with no behaviour change:
  - move the existing `list_approvals` body unchanged into `list_week(pool, manager, status, week_start: &str)`;
  - `list_approvals(status: Option<String>, week_start: String)` delegates to it;
  - `week_start` is not yet used. Its validation and the period filter come only in T011, after T009's tests have run red.
- [ ] T006 Keep crates/horae/tests/approval_labels_ui.rs compiling:
  - the `list_approvals(Option<String>, String)` stub records each requested week;
  - add a `get_week_start` stub driven by `Probe`;
  - add a minimal `route` module (`Routable` `Route` with only `Approvals { week: String }`);
  - give the three existing tests a fixed `week`.

  The existing assertions stay, and the click-index test is updated only for
  the added pager buttons.

**Checkpoint**: it compiles. The existing approval unit, isolation, submission and UI tests pass unchanged except T006's documented index update.

## Phase 3: User Story 1 - Know which week each approval covers (P1)

**Goal**: every row shows its dates.
**Independent test**: T-U1.

- [ ] T007 [US1] Add a failing T-U1 test in crates/horae/tests/approval_labels_ui.rs. Rows for 2026-07-13..19, 2026-07-20..26 and 2026-12-28..2027-01-03 render "13 Jul – 19 Jul 2026", "20 Jul – 26 Jul 2026" and "28 Dec 2026 – 03 Jan 2027".
- [ ] T008 [US1] In crates/horae/src/pages/approvals.rs, add `period_label(start, end)` per the contract and render it beside the teammate on each row (T007 green).

## Phase 4: User Story 2 - Move between weeks (P1)

**Goal**: one week at a time, in the address, defaulting to the current week.
**Independent test**: T-U2, T-U2b, T-U3, T-S1, T-S2.

- [ ] T009 [P] [US2] Add failing T-S1 and T-S2 tests in crates/horae/src/server_fns/approvals/period_tests.rs, registered with `#[cfg(all(test, feature = "server"))] mod period_tests;`:
  - **T-S1**: W−1, W and W+1 for one user plus a foreign-org W; `list_week(W)` returns exactly own-org W; `status = approved/submitted/None` combine.
  - **T-S2**: unparseable, non-aligned, and `NaiveDate::MAX`-week starts give `BAD_REQUEST` through the same validation `list_approvals` uses.
- [ ] T010 [P] [US2] Add failing T-U2, T-U2b and T-U3 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U2**:
    - `week = "2026-10-29"` requests `2026-10-26` with a Monday stub, and `2026-10-25` with a Sunday stub.
    - `week = ""` and `"garbage"` request the week computed at test time, as `week_bounds(Utc::now().date_naive(), first).0`, never a hard-coded date.
    - If the test is not run within one UTC date, the computed week and the page may straddle midnight. The test computes the expected week both before and after rendering and accepts only an equal pair.
  - **T-U2b**: a max-representable `week` shows "This week is outside the supported date range.", both directions disabled, and zero list calls.
  - **T-U3**: Prev, This week and Next target `/approvals?week=` −7d, today's start and +7d.
- [ ] T011 [US2] Only after T009 has been run and seen to fail on the T005 seam, implement the server behaviour in crates/horae/src/server_fns/approvals.rs (T009 green):
  - in `list_week`, parse `week_start`;
  - validate it in contract order against `organizations.week_start` (the same SQL text as `get_week_start`) and `week_bounds`, returning `BAD_REQUEST` errors;
  - add `AND a.period_start = $3` and order by `u.name, a.id`.
- [ ] T012 [US2] In crates/horae/src/pages/approvals.rs (T010 green):
  - `Approvals(week: String)`, which loads `get_week_start` and resolves the week with `week_bounds`;
  - the pager (`ts-pager` classes, "This week"/"Week" label, the `period_label` range, Prev/This week/Next via `navigator().push(Route::Approvals { … })`, boundary-disabled);
  - the list resource keyed by `(status, week start, refresh)`.

## Phase 5: User Story 3 - Totals and actions apply only to the shown week (P1)

**Goal**: totals and bulk approval stay inside the week; the result is truthful.
**Independent test**: T-U4, T-U5, T-U9, T-S3, T-S4.

- [ ] T013 [P] [US3] Add failing T-S3 and T-S4 tests in crates/horae/src/server_fns/approvals/period_tests.rs:
  - **T-S3**: `approve_periods` with W's ids leaves W−1's approval state and entry states unchanged.
  - **T-S4**: `reopen_period(W)` leaves W+1 approved, with its entries approved.
- [ ] T014 [P] [US3] Add failing T-U4, T-U5 and T-U9 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U4**: rows of 450 and 135 minutes give the total "9:45".
  - **T-U5**: the bulk click submits exactly the shown submitted ids.
  - **T-U9**: the stub returns 2 for 3 ids, and the page shows "Approved 2 of 3 timesheets", hidden after the week changes.
- [ ] T015 [US3] In crates/horae/src/pages/approvals.rs (T014 green), add a week-keyed bulk-result notice using `alert-success`. The totals and ids come only from the rendered rows. T013 should pass without production change, which confirms existing behaviour. If it fails, stop and report: approve and reopen are out of scope to change.

## Phase 6: User Story 4 - Safe loading, errors and denials (P2)

**Goal**: no stale rows, no double submit, no read without authority.
**Independent test**: T-U6, T-U7, T-U8.

- [ ] T016 [US4] Add failing T-U6, T-U7 and T-U8 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U6**: a response tagged with another week start renders "Loading…" and none of its rows.
  - **T-U7**: with an approve stub pending, every Approve, Reopen and bulk button has `disabled`.
  - **T-U8**: a non-manager makes zero `list_approvals` calls and sees the existing card; a `get_week_start` error shows its message and makes zero list calls.
- [ ] T017 [US4] In crates/horae/src/pages/approvals.rs (T016 green):
  - the resource returns `(week_start, result)`, and rows render only on a match;
  - a page `busy` signal is set and cleared around `spawn_action`, with `disabled` on every action button;
  - the resource returns early, with no request, until `get_me` is a manager and `get_week_start` has loaded.

## Phase 7: Polish & gates

- [ ] T018 Static sweep of the diff:
  - `rg -n 'style="'` and hex literals in the changed `.rs` files return nothing;
  - no new CSS;
  - `git diff --check`;
  - `.specify/feature.json` unchanged.
- [ ] T019 Heavy slot (root grants): run the quickstart's "Disposable database" block. That includes `cargo sqlx prepare` with the `touch` step, checking that `.sqlx` changes only the `list_approvals` descriptor. Then the offline all-targets build, clippy, the wasm check, `nix fmt`, and `responsive-layout` unmodified.
- [ ] T020 Root native smoke per quickstart.md.
- [ ] T021 Two independent reviews of the exact head, then an exact-head `nix flake check`. Small unsigned commits with no AI attribution. No push or PR before root collects.

## Dependencies

- Phase 2 blocks every story. T003 needs T002. T005 and T006 need T003 and T004.
- US1 (T007–T008) needs only Phase 2.
- US2 (T009–T012) needs Phase 2.
- US3 (T013–T015) needs T012, since the bulk ids come from the week view.
- US4 (T016–T017) needs T012.
- Phase 7 comes after every story. T021 is last.

## Parallel opportunities

- T002 can run alongside the reading for T004.
- T009 (server tests) and T010 (UI tests) touch different files.
- T013 and T014 touch different files.

## Implementation strategy

1. Phase 2, then US1. This is the smallest reviewable slice: rows identify
   their week.
1. US2 adds the week scope and navigation, with the server change.
1. US3 and US4 add correctness under actions and races.
1. Commits follow these phases, one per phase where possible, so each review
   checkpoint is small.
