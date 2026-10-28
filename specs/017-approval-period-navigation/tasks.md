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

  Green: `cargo test -p horae-core` passed 237 tests (seen in the terminal; no
  log of this run was kept, so it should be re-run). Core all-targets clippy
  with `-D warnings` was clean, and `rustfmt --check` was clean. Run in the
  offline dev shell with `CARGO_BUILD_JOBS=1` and a scratch `CARGO_TARGET_DIR`.

- [x] T004 In crates/horae/src/route.rs, change the route to `#[route("/approvals?:week")] Approvals { week: String }`. In crates/horae/src/components/sidebar.rs, pass `Route::Approvals { week: String::new() }`. No other route changes.

- [x] T005 Signature-only seam in crates/horae/src/server_fns/approvals.rs, with no behaviour change:

  - move the existing `list_approvals` body unchanged into `list_week(pool, manager, status, week_start: &str)`;
  - `list_approvals(status: Option<String>, week_start: String)` delegates to it;
  - `week_start` is not yet used. Its validation and the period filter come only in T011, after T009's tests have run red.

- [x] T006 Keep crates/horae/tests/approval_labels_ui.rs compiling:

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

- [x] T007 [US1] Add a failing T-U1 test in crates/horae/tests/approval_labels_ui.rs. Rows for 2026-07-13..19, 2026-07-20..26 and 2026-12-28..2027-01-03 render "13 Jul – 19 Jul 2026", "20 Jul – 26 Jul 2026" and "28 Dec 2026 – 03 Jan 2027".
- [x] T008 [US1] In crates/horae/src/pages/approvals.rs, add `period_label(start, end)` per the contract and render it beside the teammate on each row (T007 green).

## Phase 4: User Story 2 - Move between weeks (P1)

**Goal**: one week at a time, in the address, defaulting to the current week.
**Independent test**: T-U2, T-U2b, T-U3, T-S1, T-S2.

- [x] T009 [P] [US2] Add failing T-S1 and T-S2 tests in crates/horae/src/server_fns/approvals/period_tests.rs, registered with `#[cfg(all(test, feature = "server"))] mod period_tests;`:
  - **T-S1**: W−1, W and W+1 for one user plus a foreign-org W; `list_week(W)` returns exactly own-org W; `status = approved/submitted/None` combine.
  - **T-S2**: unparseable, non-aligned, and `NaiveDate::MAX`-week starts give `BAD_REQUEST` through the same validation `list_approvals` uses.
    Supporting changes:
  - `tests::insert_entry` in approvals.rs is now `pub(super)` and returns its
    id, so no new query text or `.sqlx` descriptor is needed;
  - T-S2 asserts each bad input's own message, so the overflow case cannot pass
    through the parse branch.
- [x] T010 [P] [US2] Add failing T-U2, T-U2b and T-U3 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U2**:
    - `week = "2026-10-29"` requests `2026-10-26` with a Monday stub, and `2026-10-25` with a Sunday stub.
    - `week = ""` and `"garbage"` request the week computed at test time, as `week_bounds(Utc::now().date_naive(), first).0`, never a hard-coded date.
    - If the test is not run within one UTC date, the computed week and the page may straddle midnight. The test computes the expected week both before and after rendering and accepts only an equal pair.
  - **T-U2b**: a max-representable `week` shows "This week is outside the supported date range.", both directions disabled, and zero list calls.
  - **T-U3**: Prev, This week and Next target `/approvals?week=` −7d, today's start and +7d.
- [x] T011 [US2] Only after T009 has been run and seen to fail on the T005 seam, implement the server behaviour in crates/horae/src/server_fns/approvals.rs (T009 green):
  - in `list_week`, parse `week_start`;
  - validate it in contract order: weekday against `organizations.week_start` (the same SQL text as `get_week_start` and the same numeric comparison as `submit_user_week`), then `checked_add_days(6)`, returning `BAD_REQUEST` errors;
  - add `AND a.period_start = $3` and order by `u.name, a.id`.
- [x] T012 [US2] In crates/horae/src/pages/approvals.rs (T010 green):
  - `Approvals(week: String)`, which loads `get_week_start` and resolves the week with `week_bounds`;
  - the pager (`ts-pager` classes, "This week"/"Week" label, the `period_label` range, Prev/This week/Next as router `Link`s to `Route::Approvals { … }`, with a disabled button at the range's end);
  - the list resource keyed by `(status, week start, refresh)`.

## Phase 5: User Story 3 - Totals and actions apply only to the shown week (P1)

**Goal**: totals and bulk approval stay inside the week; the result is truthful.
**Independent test**: T-U4, T-U5, T-U9, T-S3, T-S4.

- [x] T013 [P] [US3] Add failing T-S3 and T-S4 tests in crates/horae/src/server_fns/approvals/period_tests.rs:
  - **T-S3**: `approve_periods` with W's ids leaves W−1's approval state and entry states unchanged.
  - **T-S4**: `reopen_period(W)` leaves W+1 approved, with its entries approved.
- [x] T014 [P] [US3] Add failing T-U4, T-U5 and T-U9 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U4**: rows of 450 and 135 minutes give the total "9:45".
  - **T-U5**: the bulk click submits exactly the shown submitted ids.
  - **T-U9**: the stub returns 2 for 3 ids, and the page shows "Approved 2 of 3 timesheets", hidden after the week changes.
- [x] T015 [US3] In crates/horae/src/pages/approvals.rs (T014 green), add a week-keyed bulk-result notice using `alert-success`. The totals and ids come only from the rendered rows. T013 should pass without production change, which confirms existing behaviour. If it fails, stop and report: approve and reopen are out of scope to change.

## Phase 6: User Story 4 - Safe loading, errors and denials (P2)

**Goal**: no stale rows, no double submit, no read without authority.
**Independent test**: T-U6, T-U7, T-U8.

- [x] T016 [US4] Add failing T-U6, T-U7 and T-U8 tests in crates/horae/tests/approval_labels_ui.rs:
  - **T-U6**: a response tagged with another week start renders "Loading…" and none of its rows.
  - **T-U7**: with an approve stub pending, every Approve, Reopen and bulk button has `disabled`.
  - **T-U8**: a non-manager makes zero `list_approvals` calls and sees the existing card; a `get_week_start` error shows its message and makes zero list calls.
- [x] T017 [US4] In crates/horae/src/pages/approvals.rs (T016 green):
  - the resource returns `(week_start, result)`, and rows render only on a match;
  - a page `busy` signal is set and cleared around `spawn_action`, with `disabled` on every action button;
  - the resource returns early, with no request, until `get_me` is a manager and `get_week_start` has loaded.

## Phase 7: Polish & gates

- [x] T018 Static sweep of the diff:
  - `rg -n 'style="'` and hex literals in the changed `.rs` files return nothing;
  - no new CSS;
  - `git diff --check`;
  - `.specify/feature.json` unchanged.
- [x] T019 Heavy slot (root grants): run the quickstart's "Disposable database" block. That includes `cargo sqlx prepare` with the `touch` step, checking that `.sqlx` changes only the `list_approvals` descriptor. Then the offline all-targets build, clippy, the wasm check, `nix fmt`, and `responsive-layout` unmodified.
- [ ] T020 Root native smoke per quickstart.md.
- [ ] T021 Two independent reviews of the exact head, then an exact-head `nix flake check`. Small unsigned commits with no AI attribution. No push or PR before root collects.
  - Done locally for evidence head `fd83e00d`:
    - Both independent reviews approved `fd83e00d`. Root and the second reviewer checked its runtime results directly.
    - `nix flake check -L --max-jobs 1` exited 0 (see Evidence).
  - Open: remote CI. The head is not published.

### Review fixes (second review)

- [x] T022 H1: crates/horae/src/server_fns/importers/authorization_tests/approval_labels.rs posted `list_approvals` without `week_start` at all five call sites. Each now sends the fixture's Monday, `2026-09-21`, and the authorization assertions are unchanged. That test has an approval row in the manager's own organization whose submitter belongs to another organization. It starts on `2026-09-28`, outside that week, so one added request checks that this row is hidden in its own week too. Pre-fix: the already-built test executable, run against the private cluster with no compilation, failed `job_endpoints_enforce_session_role_and_organization`: `list_approvals` returned 500, "missing field `week_start`". Post-fix, at `fd83e00d`: the full server suite in `nix flake check` passed this test (see Evidence).
- [x] T023 M2: in crates/horae/tests/approval_labels_ui.rs, T-U3 compared hrefs only. During the week of 2026-10-26, Next and This week lead to the same week, so a missing Next still passed. Each link is now found by its name (`aria-label` for Previous and Next, the text for This week), and the test asserts there is exactly one of each and checks its href. At `fd83e00d`, `approval_labels_ui` passed 14/14 in `nix flake check`, including this test.
- [ ] T024 M1 deployment gate (owner decision recorded in spec.md Clarifications): immediately before deploying, run the read-only query in quickstart.md, "Deployment gate", against the target database. Any row: do not deploy, and keep the previous version. No overlap, normalization, migration or mutation change is made. Not run against any real database.

## Evidence (2026-10-28)

The cargo runs used:

- a disposable private PostgreSQL 17 cluster, on a Unix socket only
  (`listen_addresses` empty), with role `horae` (CREATEDB);
- the offline dev shell;
- a scratch `CARGO_TARGET_DIR`.

`nix flake check` uses its own PostgreSQL inside the build sandbox.

**Status, 2026-10-28**:

- T001–T019, T022 and T023 are done.
- T021 is done locally (reviews and exact-head `nix flake check`); its remote
  CI is open.
- T020 (native smoke) and T024 (the deployment gate) are open.
- The earlier server runs listed below were filtered to
  `server_fns::approvals`, so they missed the HTTP authorization check that H1
  broke. The full suite under "Exact-head gates" covers it.

### Exact-head gates (evidence head `fd83e00d`)

These ran on the clean head `fd83e00dadb1cd6173a712a7b4e8723f96aec7e6`. The
commit that records them changes only this file, and nothing was re-run on it.

- `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo test -p horae-core`: 237
  passed.
- `nix flake check -L --max-jobs 1`: exit 0. It covered:
  - the package build;
  - the full test suite;
  - clippy;
  - the `.sqlx` prepare check;
  - treefmt;
  - the browser checks;
  - both NixOS e2e VM tests (plain and OIDC).
- In that run:
  - the server unit suite passed 1558, failed 0, with 11 ignored and none
    filtered out. That includes
    `job_endpoints_enforce_session_role_and_organization` (T022);
  - `approval_labels_ui` passed 14/14, including
    `the_pager_links_to_the_adjacent_and_current_weeks` (T023).
- H1 before the fix: the prebuilt test executable failed that test with a 500,
  "missing field `week_start`".
- Not run here:
  - remote CI;
  - native smoke (T020);
  - the deployment-gate query against any real database (T024).

### Red → green

| Tests | Red (before code) | Green |
|-------|-------------------|-------|
| core `week_bounds` (6) | 6/6 panicked on a `todo!()` stub | 237/237 core tests |
| T-S1 membership, T-S2 validation | Both failed on the T005 seam: three weeks returned for W, and `garbage` returned `Ok` | 21/21 `server_fns::approvals` after T011 |
| T-U1, T-U2 (×2), T-U2b, T-U3, T-U6, T-U7, T-U8, T-U9, and the click-index test (6 → 9) | 10 failed against the seam page, each on its own assertion | 14/14 `approval_labels_ui` after T008, T012, T015 and T017 |
| T-U6, T-U9 guards | Mutation check: removing the week guard on rows and on the notice fails exactly these two | Guards restored; 14/14 |

Before the green run, T-U2b was revised to count the hrefs actually rendered,
because the router renders `href` before `aria-label`. It was not re-run red in
that form; the mutation check above covers the guards only.

### Characterization, not red-green

These passed on the seam, before any production change. They confirm existing
behaviour:

- T-S3 (approving W leaves W−1 pending);
- T-S4 (reopening W leaves W+1 approved);
- the status-filter test with a single week;
- T-U4 (totals) and T-U5 (bulk ids).

### Regressions and gates

- `cargo test --test integration approv`: 5/5.
- Existing approval unit, submission and isolation tests in
  `server_fns::approvals`: unchanged and green. The full server suite was not
  run (see T022).
- `cargo sqlx prepare` (with the `touch` step) replaced exactly one descriptor:
  `be742e50…` removed, `ab8a7e02…` added, both the `list_approvals` query. The
  org read reuses `get_week_start`'s query text.
- Offline strict clippy, all targets (horae with server, plus core): clean.
- WASM `cargo check --features web`: clean.
- `nix fmt -- --ci`: the first run reformatted 6 files, and the re-run changed 0.
  The tests were re-run green after formatting.
- Static sweep: no inline style, hex or CSS changes; `git diff --check` clean;
  `.specify/feature.json` unchanged.
- `nix build .#default` from source `12ae010b`, with the same code as the next
  docs-only commit, exited 0:
  `/nix/store/kx781vm0h1q7brw1vv67nagmhnkaj3f3-horae-0.1.0`.
- On that package, `run-design-checks.sh responsive-layout` exited 0 with
  60/60 PASS, including `/approvals` headers at 320, 390, 768, 1280 and 1440px.
  It used the runner's own temporary database and port 8093.
- Each run's log ends with an `exit=` line. The logs are kept locally for
  review and are not part of the repository.

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
