# Quickstart: verifying R02

Run builds and browser checks only in a granted slot. The browser runner binds
`127.0.0.1:8093` and `/tmp/horae-browser.*`, and creates its own database.

```sh
cargo test -p horae-core money::                       # helper
DATABASE_URL=… cargo test -p horae --features server pages::reports   # cell text
cargo clippy -p horae --features server
nix fmt
HORAE_TEST_SERVER=… PLAYWRIGHT_MODULE=… \
  crates/horae/tests/browser/run-design-checks.sh \
  reports-currency-totals reports-permissions new-project new-project-permissions responsive-layout
```

`nix/checks/browser.nix` runs the runner's default list, which starts with
`reports-currency-totals`, so `nix flake check` covers it too. The suite needs
freshly seeded Acme (EUR) and TechStart (USD) data, and `invoice-preparation`
later rewrites those projects. A focused run therefore names it first, as
above, and every run starts from a fresh migrate and seed.

## Requirement → proof

| Requirement | Proof |
|-------------|-------|
| FR-001/002, SC-001 | Core tests: EUR and USD at `i64::MAX` each stay exact; byte-order output for any input order; mixed-case codes stay distinct and are not merged; empty input gives an empty list. |
| FR-005 | Core tests: a same-currency final sum out of range gives `None` while the other currency stays exact; a running sum that leaves the range and returns stays exact; the `i64::MIN` side is covered. |
| FR-003, SC-003 | Page test: one in-range currency gives exactly `["EUR 350.00"]`. The existing browser suites pass unmodified. |
| FR-004 | Core tests: `Err(Withheld)` when the withheld amount comes first, and when it follows two rows that already overflow together. Page test: a withheld cost with mixed currencies gives only `Restricted`. Browser: a real manager session with a member-cost override sees Cost total `Restricted` while the Billable lines stay exact. |
| US3 | Page test (existing, kept): USD and CHF billing with an EUR cost gives `["EUR 1.25"]`. |
| FR-006, SC-004 | `git diff --stat` shows only the listed files; the existing suites' CSV/XLSX assertions pass unmodified. |
| FR-007, SC-002 | Browser suite (see below). |

### Browser suite (`reports-currency-totals.cjs`)

- **Totals:** in the seeded current week, the Billable total has one line per
  row currency, each equal to the integer sum of that currency's row cells. The
  Cost total has one line.
- **Groupings and filters:** all four groupings and a client filter keep exact
  lines. Each is checked after its own response.
- **Loading and errors:** a delayed request shows `Loading report…` and no
  total row. A failed request shows the error, and keyboard retry restores the
  totals.
- **Themes, widths and text size:** light and dark, at 1440, 390 and 320px,
  each with base text and with a 32px root font size. The checks:
  - the table scroller fits the viewport;
  - each total line is right-aligned, unclipped, inside its cell, and stacked in
    order;
  - page overflow is recorded with its sources for comparison with the base
    build, not asserted away;
  - screenshots are kept.
- **Keyboard:** the total row adds no focusable elements.
- **Restricted:** fixture IDs and an existing override for the target are
  asserted absent first. Then the actor is switched to `manager`, and one
  seeded project and person get a `project_member_costs` override. The Cost
  total shows only `Restricted`. The role and fixture rows are restored in
  `finally`, even if closing the browser throws.
- **Shared-style audit:** it may report the new per-line `div`s on `/reports`.
  Each one is listed in the review, not suppressed.
