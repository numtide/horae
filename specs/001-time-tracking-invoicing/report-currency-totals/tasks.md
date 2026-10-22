# Tasks: Currency-separated totals in the legacy Time report (R02)

**Input**: [spec.md](spec.md), [plan.md](plan.md),
[contracts/total-cells.md](contracts/total-cells.md), and
[quickstart.md](quickstart.md) for the commands and the requirement → proof map.

**Tests**: requested (TDD). Within each phase, tests are written before code.

**Status (2026-10-22)**:

- **Done:** T001–T012 have passed locally.
- **Open:** T013 stays open until the remote exact-head checks pass and both
  reviews are resolved.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

No setup work. The change stays in the existing crates and the existing browser
runner (see the plan's Structure Decision).

## Phase 2: Foundational (blocks every story)

- [x] T001 Write failing tests in `crates/core/src/money.rs`:
  - totals per currency, in byte order of the stored code, whatever the input
    order;
  - mixed-case codes such as `EUR` and `eur` stay distinct;
  - EUR and USD at `i64::MAX` each stay exact;
  - empty input gives an empty list.
- [x] T002 Write failing tests in `crates/core/src/money.rs`:
  - a same-currency final sum out of range gives `None` while the other currency
    stays exact, in both directions;
  - a running sum that leaves the range and comes back is exact;
  - zero and signed sums are kept;
  - a withheld amount gives `Err(Withheld)` both when it comes first and when it
    follows two same-currency `i64::MAX` rows.
- [x] T003 Implement `Withheld` and `sum_by_currency` in
  `crates/core/src/money.rs`: `i128` accumulation, then `i64::try_from`.

**Checkpoint**: the core rule passes on its own, before any page work.

## Phase 3: User Story 1 — every currency's exact total (P1) 🎯 MVP

**Independent test**: load a period with EUR and USD rows and read the Billable
total cell.

- [x] T004 [US1] Tests in `crates/horae/src/pages/reports.rs`:
  - mixed currencies at `i64::MAX` give two exact lines;
  - a single in-range currency gives exactly `["EUR 350.00"]`;
  - empty input gives `["—"]`.
- [x] T005 [US1] In `crates/horae/src/pages/reports.rs`, replace `money_total`
  with `money_totals` over `sum_by_currency`, and render one `div` per line in
  the two existing total cells.
- [x] T006 [US1] Write `crates/horae/tests/browser/reports-currency-totals.cjs`,
  run on the seeded disposable database:
  - the period is the first to last seeded entry date, and expected totals are
    summed from the loaded row cells;
  - all four groupings and the Acme filter, each checked after its own response;
  - exact Export CSV/XLSX hrefs;
  - a delayed load shows no total row; a failure, then keyboard Retry, restores
    it;
  - light and dark at 1440, 390 and 320px, with base text and with a 32px root
    font size; geometry is asserted and page overflow is recorded with its
    sources;
  - the total row adds no focusable elements.

## Phase 4: User Story 2 — restricted and out-of-range totals (P1)

**Independent test**: a withheld cost, and a same-currency total out of range.

- [x] T007 [US2] Tests in `crates/horae/src/pages/reports.rs`:
  - a withheld cost alongside mixed currencies and an out-of-range currency
    gives only `Restricted`;
  - same-currency overflow beside another currency gives
    `EUR amount exceeds supported range` plus the exact other line;
  - `monetary_total_overflow_is_visible_instead_of_panicking` is rewritten for
    the new wording;
  - zero and negative totals are shown.
- [x] T008 [US2] The Restricted step in
  `crates/horae/tests/browser/reports-currency-totals.cjs`:
  - assert the fixture IDs and the target override are absent;
  - switch the actor to `manager` and add a `project_member_costs` override;
  - expect the Cost total to read only `Restricted`;
  - restore only after the fixture has committed, with nested cleanup.

## Phase 5: User Story 3 — cost in the organization currency (P2)

**Independent test**: USD and CHF billing rows in an EUR organization.

- [x] T009 [US3] Keep
  `cost_total_uses_organization_currency_across_billing_currencies` in
  `crates/horae/src/pages/reports.rs`, now on `Vec<String>`. In the browser
  suite, the Cost total has one line.

## Phase 6: Polish and verification

- [x] T010 Put `reports-currency-totals` first in the default `suites=(…)` in
  `crates/horae/tests/browser/run-design-checks.sh`:
  - the suite needs the fresh seed, which `invoice-preparation` later rewrites;
  - every other suite keeps its relative order;
  - this is the only change to the runner, and `nix/checks/browser.nix` is not
    edited.
- [x] T011 Tests and gates in the dev shell, run one at a time:
  - the full `cargo test -p horae-core`: 195 passed;
  - the page tests in `crates/horae/src/pages/reports.rs`
    (`cargo test -p horae --features server --bin horae pages::reports`): 8
    passed;
  - strict clippy as in `nix/checks/clippy.nix`, for core and for the server
    with all targets, plus `--features web --target wasm32-unknown-unknown`
    with `-D warnings`: all exit 0;
  - `nix fmt -- --ci`: exit 0, 0 files changed;
  - the candidate package `nix build .#default`: exit 0.
- [x] T012 Browser checks with `crates/horae/tests/browser/run-design-checks.sh`,
  on port 8093 against its own temporary database:
  - **Merged package (before):** only a before-state baseline was taken. The
    existing regression suites were not run here; they ran once, on the
    candidate.
    - the shared-style record was taken;
    - a before-state probe measured page overflow on the same 12 cases: only
      320px with 32px text overflowed (+54px, from the filter form), and the
      table fits;
    - the new suite failed as expected, because the Billable total had no
      per-currency lines.
  - **Candidate package:**
    - the shared-style compare passed;
    - `reports-currency-totals`, `reports-permissions`, `responsive-layout`,
      `new-project` and `new-project-permissions` all passed (exit 0);
    - after the final test-only fix to the request gate, the new suite was
      rerun alone and passed (exit 0);
    - page overflow is unchanged from before: +54px at 320px with 32px text,
      from the filter form, and none from the totals.
- [ ] T013 Delivery gates:
  - **Local gates:** T011, the candidate package, T012's focused browser
    suites with the shared-style baseline compare, and the coordinator's native
    acceptance. No full local `nix flake check` or full default browser run is
    claimed.
  - **Remote, on the exact head:** CI must pass the full default browser suite
    and every flake check. Neither may be removed or skipped.
  - **Approval:** both independent reviews resolved and the coordinator's
    approval given before anything is published.

## Dependencies and order

- Phase 2 (T001–T003) blocks every story.
- US1 (T004–T006) comes before US2 and US3, because they extend the same
  function and suite.
- Polish (T010–T013) comes last.

## Parallel opportunities

None that matter here. The stories share `pages/reports.rs` and one browser
suite, so tasks touching the same file run one after another. T011's heavy jobs
also run one at a time.

## Implementation strategy

- **MVP:** Phase 2 plus US1. That alone replaces the em dash with exact
  per-currency lines.
- **Next:** US2 keeps withheld and out-of-range totals truthful, then US3
  confirms the single cost currency.
- **Verification:** Phase 6 runs once a build slot is granted.
