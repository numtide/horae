# Tasks: Timesheet Exports for Invoicing Transparency

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[quickstart.md](quickstart.md), [contracts/export-routes.md](contracts/export-routes.md)

**Scope of this file**: only the **Acceptance Increment: Legacy Report Export
Reconciliation (2026-10-25)**. It proves User Story 2 (FR-011 to FR-015, FR-017 to FR-019,
SC-005, SC-006, SC-L01 to SC-L04) on the legacy Reports page. It does not task User Story 1, User Story 3,
FR-001 to FR-010, FR-016 or FR-020. The invoice backup is not implemented, and it is
neither claimed nor waived here (spec, Evidence Rules). Open analyze finding D1 (`f64` XLSX
durations) is not waived or resolved by these tasks.

**Tests**: requested. The deliverable of this increment is tests; no product code changes.
If an assertion fails because the product disagrees with the fixture, stop and report
the product bug before widening scope.

**Fixture, states S1–S5 and proofs P1–P9**: plan.md "Acceptance Increment" is the only
source; tasks refer to it rather than repeat it.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 Start a disposable PostgreSQL in a fresh `mktemp -d /tmp/horae-r02.XXXXXX`
  directory (role with `CREATEDB` for `#[sqlx::test]`), apply migrations with
  `sqlx migrate run --source crates/horae/migrations`, and use it as `DATABASE_URL` for
  T003–T009. Keep its logs and data until the evidence has been collected; no automatic
  broad cleanup.

## Phase 2: Foundational (blocks the user story)

- [ ] T002 Move `xlsx_part`, `assert_cell` and `assert_text_cell`, verbatim, from the test
  module in `crates/horae/src/reports/limits.rs` into a new
  `crates/horae/src/reports/xlsx_cells.rs`, declared in `crates/horae/src/reports.rs` as
  `#[cfg(test)] pub(crate) mod xlsx_cells;`. The `limits.rs` tests import them. No helper is
  added and no assertion changes (research.md L4).
- [ ] T003 Run the existing tests that use the moved helpers, unchanged:
  `cargo test -p horae --features server --bin horae reports::limits`. Every test must pass
  as before.

## Phase 3: User Story 2 — filtered detailed export from the legacy page (P2)

**Goal**: prove proofs P1–P9 from plan.md against the fixture.

**Independent test**: the Rust check and the browser suite each pass against their own
disposable database.

- [ ] T004 [US2] Create
  `crates/horae/src/server_fns/importers/authorization_tests/time_reports/legacy_exports.rs`
  with a `check(pool, api)` that:
  - builds the plan fixture (all names created explicitly; ascending fixed entry IDs) in a
    new `test_seed::seed(pool, OrgRole::Manager)` organization at policy 0, with rounding set to 15, nearest;
  - downloads CSV and XLSX for S1–S5 with the Manager session;
  - **P7**: parses the CSV with `csv`; the header equals the eight `ENTRY_EXPORT_HEADERS`; records
    equal the literal rows. Each row is matched to exactly one expected entry; the raw,
    rounded and billable-rounded minute sums of the matched entries equal the plan totals.
    The S3 raw cells add up to 266 hundredths, against the displayed total `"2.67"`;
  - **P6**: the XLSX `<row ` count is entries + 1; `assert_text_cell` for A–D, G and H
    (XML-escaped expectations); `assert_cell` for E and F with the plan's exact literal text;
    the same entry match gives the integer totals; the excluded notes are absent from
    `sharedStrings.xml`;
  - **P8**: S5 returns 200 and only the header row, in both formats.
  - No float parsing or arithmetic.
- [ ] T005 [US2] In
  `crates/horae/src/server_fns/importers/authorization_tests/time_reports.rs`, add
  `mod legacy_exports;` and call `legacy_exports::check(pool, api).await` at the end of
  `check`. The existing org and role assertions (P9) stay untouched.
- [ ] T006 [P] [US2] Create `crates/horae/tests/browser/reports-legacy-exports.cjs`
  following the guards in `reports-currency-totals.cjs`:
  - **Preconditions**: the disposable `/tmp/horae-browser.*` database on port 8093; policy 0;
    organization rounding 15, nearest; no entries in 2041-03; fixture IDs unused.
  - **Fixture**: insert it in one transaction and set the committed flag only after it
    commits; nested cleanup in `finally` removes only the fixture rows, by ID.
  - **Flow**: drive S1 → S2 (Project select) → clear → S3 (Client select) → S4 (Project select)
    through the real selects. In each state, wait for that state's `report_time` and
    `report_detailed` responses, then assert P1–P5: exact link URLs; Time-tab rows and
    total; Detailed cells and absent entries; the clicked CSV download's bytes; and the
    clicked XLSX download's filename and `PK` signature.
  - `pageerror` is empty, and the suite prints one PASS line.
- [ ] T007 [US2] In `crates/horae/tests/browser/run-design-checks.sh`, insert
  `reports-legacy-exports` into the default `suites=(…)` right after `reports-currency-totals`
  (both need policy 0 and the fresh seed's rounding; `reports-permissions` later moves to
  policy 1). Every other suite keeps its relative order.

## Phase 4: Polish and verification

- [ ] T008 Regenerate the query cache against the T001 database: run
  `touch crates/horae/src/main.rs crates/horae/tests/*.rs`, then
  `cargo sqlx prepare --workspace -- --features server --all-targets`, and stage `.sqlx/`.
  Confirm with an offline build: `SQLX_OFFLINE=true cargo test -p horae --features server --no-run`.
- [ ] T009 Run the gates one at a time, with no edits while a check runs: the focused Rust
  test `cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization`;
  strict clippy as in `nix/checks/clippy.nix`; `nix fmt -- --ci`.
- [ ] T010 As the single heavy-build owner, build the server package once, then run
  `crates/horae/tests/browser/run-design-checks.sh reports-currency-totals reports-legacy-exports reports-permissions` (its own `/tmp` database). Record commands,
  exit codes and PASS lines.
- [ ] T011 Delivery gates, all required before publication:
  - small unsigned commits with plain messages;
  - **two independent reviews** resolved;
  - the relevant browser and native acceptance by the coordinator;
  - a final full local `nix flake check` on the candidate head, with one build owner;
  - mandatory CI passing on the exact published head;
  - analyze findings D1 (`f64` XLSX durations) and C1 (invoice backup not implemented)
    reported as open, not as resolved.
    No push, PR or merge before coordinator acceptance.

## Dependencies and order

- T001 → T002 → T003 → T004 → T005 → T008 → T009.
- T006 has no task dependency; T007 follows T006. T010 follows T007 and T009.
  T011 is last.

## Parallel opportunities

T006 (JavaScript) can be written while T004 is written, because they touch different
files. Heavy runs (T008, T009, T010) run one at a time, never side by side.
