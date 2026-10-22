# Implementation Plan: Currency-separated totals in the legacy Time report (R02)

**Branch**: `fix/report-currency-totals` | **Date**: 2026-10-22 | **Base**: `0243a386` | **Spec**: [spec.md](spec.md)

## Summary

Replace the em dash in the legacy Time report's Billable amount and Cost total
cells with one exact, code-labelled total per currency. A small pure helper in
`horae-core::money` does the summing and owns the all-or-nothing withheld rule.
The page maps the helper's result to cell lines inside the existing cells. No
query, schema, model, export, CSS or permission changes.

- Evidence and decisions: [research.md](research.md)
- Data: [data-model.md](data-model.md)
- Helper signature and cell text: [contracts/total-cells.md](contracts/total-cells.md)
- Verification and requirement → proof: [quickstart.md](quickstart.md)

## Technical Context

- **Language/Version**: Rust 2024 (toolchain pinned by the flake).
- **Primary Dependencies**: Dioxus fullstack (SSR + WASM). The core helper uses
  only `std`.
- **Storage**: PostgreSQL, unchanged. No query, migration or `.sqlx` change.
- **Testing**: `cargo test` for `horae-core` and for the page module; Playwright
  through `run-design-checks.sh` against a disposable database.
- **Target Platform**: Linux server plus browser WASM.
- **Project Type**: web application (one app crate and one domain crate).
- **Performance Goals**: none new. One linear pass over already loaded report
  rows.
- **Scale/Scope**: as many rows as one grouped report returns, plus a handful
  of currencies.
- **Touched code**:
  - `crates/core/src/money.rs`: `Withheld` and `sum_by_currency`, with tests.
  - `crates/horae/src/pages/reports.rs`: `money_total` becomes `money_totals`;
    the two total cells render one `div` per line; tests.
  - `crates/horae/tests/browser/reports-currency-totals.cjs`: new suite.
  - `crates/horae/tests/browser/run-design-checks.sh`: add
    `reports-currency-totals` as the first default suite, ahead of suites that
    rewrite the seed. That is one scoped addition, and nothing else in the
    runner changes. `nix/checks/browser.nix` already runs the default list and
    is not edited.
- **Not touched**: `core/duration.rs` (owned by another author), project
  budget/read access, migrations, `.sqlx`, exports, `horae.css`, scoped reports.
- **Constraints**: integer minor units, no floats, no partial disclosure, exact
  stored currency identity.

## Constitution Check

| Principle | Status |
|-----------|--------|
| I. Exactness | Pass. Integer per-currency sums, never converted or combined across currencies. |
| II. Domain purity | Pass. Summing and withheld precedence live in `horae-core`, unit-tested there. The page only formats. |
| III. Single datastore | N/A. No schema or query change. |
| IV. Mutations via server fns | N/A. Read-only display. |
| V. Formatting gate | `nix fmt` and `nix flake check` on the exact head before merge. |

There are no deviations, so there is no Complexity Tracking.

## Project Structure

```text
specs/001-time-tracking-invoicing/report-currency-totals/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
├── contracts/total-cells.md
└── checklists/requirements.md
```

**Structure Decision**: The change stays in the existing crates.
`crates/core/src/money.rs` takes the domain rule, and
`crates/horae/src/pages/reports.rs` the rendering. The browser coverage lives in
the existing `crates/horae/tests/browser/` runner.

## Gates

- Application edits start after the coordinator approves the plan.
- Builds and browser runs start only in a granted slot.
- Merge requires two independent reviews arranged by the coordinator and CI
  passing on the exact head.
- No commit, push or merge happens without coordinator approval.
