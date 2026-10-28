# Tasks: Timesheet Exports for Invoicing Transparency

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[quickstart.md](quickstart.md), [contracts/export-routes.md](contracts/export-routes.md)

**Scope of this file**: only the **Acceptance Increment: Legacy Report Export
Reconciliation (2026-10-25)**. It proves User Story 2 (FR-011 to FR-015, FR-017 to FR-019,
SC-005, SC-006, SC-L01 to SC-L04) on the legacy Reports page; T013 adds regression
coverage for the existing Teammate filter, which no FR names. It does not task User Story 1, User Story 3,
FR-001 to FR-010, FR-016 or FR-020. The invoice backup is not implemented, and it is
neither claimed nor waived here (spec, Evidence Rules). XLSX-D1 (`f64` XLSX durations) is an
approved decision under Constitution 2.0.0 whose conformance check is still pending; these
tasks change no writer and do not resolve it.

**Tests**: requested. The deliverable of this increment is tests; no product code changes.
If an assertion fails because the product disagrees with the fixture, stop and report
the product bug before widening scope.

**Fixture, states S1–S7 and proofs P1–P9**: plan.md "Acceptance Increment" is the only
source; tasks refer to it rather than repeat it.

**Status (2026-10-28)**: **no gate has evidence at the current head.** On 2026-10-28 the
branch was rebuilt on a newer base. The commits named below (b459f4f0, 962034c8, 86c60ce1)
are not ancestors of the current head, and the browser suite has changed since then (S7).
An `[x]` on T001–T010 records that the task was authored and, where it is a gate, that it
ran at the old commit named here; it is not a current-head result. All current-head
validation is pending under T013 and T011.

**History (2026-10-25)**, recorded from local logs that are not kept in the repository.
These results apply only to the commits they name:

- **T001–T010**:
  - The work was authored and committed up to 962034c8.
  - Their Rust runs (T003, the T004 GREEN/RED/restored runs, the T008 prepare and offline
    build) ran on the working tree before the formatter commit b459f4f0, so they are not
    evidence for a committed head.
  - Strict clippy, the server package build and the focused browser suites (three PASS
    lines) ran at committed 962034c8.
  - The browser suite's deliberate RED (expected 2.66, actual 2.67) was restored to the
    committed bytes without a rerun.
- **T012**, on committed 86c60ce1:
  - the focused Rust test: 1 passed;
  - the `SQLX_OFFLINE=true` all-targets `--no-run` build: exit 0;
  - strict clippy: exit 0;
  - formatting and `.sqlx` prepare were clean before the commit;
  - the independent re-reviews accepted the changed source;
  - the browser suite was not rerun at 86c60ce1; its source there was unchanged from
    962034c8, where it last ran.
- **Native acceptance**, by the coordinator on the 962034c8 package (unchanged production
  code at 86c60ce1), recorded in the coordinator's local notes:
  - all 4 stages pass, covering 16 filter states;
  - CSV by click and XLSX by Enter each returned a 200 response carrying the visible query;
  - no download event fired (microsoft/playwright#39253);
  - the downloaded files were inspected afterwards on disk, the XLSX with `unzip`. The
    browser decoded no XLSX.

**Current head**: T013 (S7) is written and has not run: no build, test, browser or
formatter run. T011 is open; no gate in it is claimed.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [x] T001 Start a disposable PostgreSQL in a fresh `mktemp -d /tmp/horae-r02.XXXXXX`
  directory (role with `CREATEDB` for `#[sqlx::test]`), apply migrations with
  `sqlx migrate run --source crates/horae/migrations`, and use it as `DATABASE_URL` for
  T003–T009. Keep its logs and data until the evidence has been collected; no automatic
  broad cleanup.

## Phase 2: Foundational (blocks the user story)

- [x] T002 Move `xlsx_part`, `assert_cell` and `assert_text_cell`, verbatim, from the test
  module in `crates/horae/src/reports/limits.rs` into a new
  `crates/horae/src/reports/xlsx_cells.rs`, declared in `crates/horae/src/reports.rs` as
  `#[cfg(test)] pub(crate) mod xlsx_cells;`. The `limits.rs` tests import them. No helper is
  added and no assertion changes (research.md L4).
- [x] T003 Run the existing tests that use the moved helpers, unchanged:
  `cargo test -p horae --features server --bin horae reports::limits`. Every test must pass
  as before.

## Phase 3: User Story 2 — filtered detailed export from the legacy page (P2)

**Goal**: prove proofs P1–P9 from plan.md against the fixture.

**Independent test**: the Rust check and the browser suite each pass against their own
disposable database.

- [x] T004 [US2] Create
  `crates/horae/src/server_fns/importers/authorization_tests/time_reports/legacy_exports.rs`
  with a `check(pool, api)` that:
  - builds the plan fixture (all names created explicitly; ascending fixed entry IDs) in a
    new `test_seed::seed(pool, OrgRole::Manager)` organization at policy 0, with rounding set to 15, nearest;
  - downloads CSV and XLSX for S1–S5 with the Manager session (T012 adds S6 and T013 adds S7, so the loop now runs S1–S7);
  - **P7**: parses the CSV with `csv`; the header equals the eight `ENTRY_EXPORT_HEADERS`; records
    equal the literal rows. Each row is matched to exactly one expected entry; the raw,
    rounded and billable-rounded minute sums of the matched entries equal the plan totals.
    The S3 raw cells add up to 266 hundredths, against the displayed total `"2.67"`;
  - **P6**: the XLSX `<row ` count is entries + 1; `assert_text_cell` for A–D, G and H
    (XML-escaped expectations); `assert_cell` for E and F with the plan's exact literal text;
    the same entry match gives the integer totals; the excluded notes are absent from
    `sharedStrings.xml`;
  - **P8**: S5 returns 200 and only the header row, in both formats.
  - **P9** (added by T012): asserts policy 0 before downloading; S6, another
    organization's client with an entry in the period, returns header-only files and
    that entry never appears.
  - No float parsing or arithmetic.
- [x] T005 [US2] In
  `crates/horae/src/server_fns/importers/authorization_tests/time_reports.rs`, add
  `mod legacy_exports;` and call `legacy_exports::check(pool, api).await` at the end of
  `check`. The existing org and role assertions (P9) stay untouched.
- [x] T006 [P] [US2] Create `crates/horae/tests/browser/reports-legacy-exports.cjs`
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
- [x] T007 [US2] In `crates/horae/tests/browser/run-design-checks.sh`, insert
  `reports-legacy-exports` into the default `suites=(…)` right after `reports-currency-totals`
  (both need policy 0 and the fresh seed's rounding; `reports-permissions` later moves to
  policy 1). Every other suite keeps its relative order.

## Phase 4: Polish and verification

- [x] T008 Regenerate the query cache against the T001 database: run
  `touch crates/horae/src/main.rs crates/horae/tests/*.rs`, then
  `cargo sqlx prepare --workspace -- --features server --all-targets`, and stage `.sqlx/`.
  Confirm with an offline build: `SQLX_OFFLINE=true cargo test -p horae --features server --all-targets --no-run`.
- [x] T009 Run the gates one at a time, with no edits while a check runs: the focused Rust
  test `cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization`;
  strict clippy as in `nix/checks/clippy.nix`; `nix fmt -- --ci`.
- [x] T010 As the single heavy-build owner, build the server package once, then run
  `crates/horae/tests/browser/run-design-checks.sh reports-currency-totals reports-legacy-exports reports-permissions` (its own `/tmp` database). Record commands,
  exit codes and PASS lines.
- [ ] T012 Review remediation (technical and UX reviews of 962034c8): remove the unproven
  "XLSX values convert back to whole minutes" claim (M1); describe the layers as the same
  route and query template over parallel fixtures (F1, L4); assert policy 0 and add S6
  (L2, L3); correct the P9 citations (L3). Then, one at a time on the committed head:
  `nix fmt -- --ci`, `.sqlx` prepare, the focused Rust test, and the offline all-targets
  build. Changed lines need independent re-review. L1 and L5 are left as optional.
  Unchecked again: its final validation and reviews are still pending.
- [ ] T013 [US2] Add S7 (teammate "Export colleague" only, plan.md) as regression coverage
  for the existing Teammate filter. Existing helpers and queries only; no product, query,
  `.sqlx` or dependency change.
  - In `legacy_exports.rs`, add `&user_id=` with expected entry E2 and totals 50 / 45 / 45
    (P6, P7).
  - In `reports-legacy-exports.cjs`, after S4 clear the client, select the colleague in the
    Teammate select, and wait for responses whose `user_id` is the colleague. Then assert
    P1–P5 with the S7 Time-row override for alpha (0.83 / 0.75 / 0.75). Every existing
    state now also requires `user_id` to be null.
  - Gates, one at a time on the exact current head after a resource grant. They also stand
    in for the T009, T010 and T012 runs, which have no current-head evidence: the focused
    Rust test, the `.sqlx` prepare check (no diff expected), the offline all-targets build,
    strict clippy, `nix fmt -- --ci`, and the browser suites as in T010.
    Optionally, as a sensitivity check that is never committed, drop `user_id` from the
    request with the expectations unchanged and confirm the state fails.
- [ ] T011 Delivery gates, all required before publication:
  - small unsigned commits with plain messages;
  - **two independent reviews** resolved;
  - the relevant browser and native acceptance by the coordinator;
  - a final full local `nix flake check` on the candidate head, with one build owner;
  - mandatory CI passing on the exact published head;
  - XLSX-D1 (`f64` XLSX durations) reported as decided with its conformance check pending,
    and C1 (invoice backup not implemented) reported as open; neither is reported as resolved.
    No push, PR or merge before coordinator acceptance.

## Dependencies and order

- T001 → T002 → T003 → T004 → T005 → T008 → T009.
- T006 has no task dependency; T007 follows T006. T010 follows T007 and T009.
  T013 follows T012. T011 is last.

## Parallel opportunities

T006 (JavaScript) can be written while T004 is written, because they touch different
files. Heavy runs (T008, T009, T010) run one at a time, never side by side.
