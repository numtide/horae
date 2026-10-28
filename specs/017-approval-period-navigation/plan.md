# Implementation Plan: Approval Period Navigation

**Branch**: `feat/approval-period-navigation` | **Date**: 2026-10-28 | **Base**: `8a796361` | **Spec**: [spec.md](spec.md)

## Summary

The existing weekly Approvals page shows one week at a time, chosen in the
address and aligned to the organization's first weekday. It defaults to the
current week.

- Every row and the pager show the week's dates.
- Totals, the pending count and "Approve visible timesheets" cover only the
  shown week. The bulk result reports the server's count.
- `list_approvals` takes a required week start, validates it and matches
  `period_start` exactly.

Approve, reopen, submission, authority, locks and plugin events are unchanged.

- Decisions and evidence: [research.md](research.md)
- Behaviour contract: [contracts/approval-week.md](contracts/approval-week.md)
- Verification: [quickstart.md](quickstart.md)
- No data model: no schema, entity or migration change. The approval row
  already has `period_start` and `period_end`.

## Technical Context

- **Language/Version**: Rust 2024, pinned by the flake. Dioxus 0.7 fullstack
  (SSR + WASM).
- **Primary Dependencies**: existing only.
- **Storage**: PostgreSQL. One changed `query!`, `list_approvals`, which needs a
  regenerated `.sqlx` descriptor. The org week-start read reuses the existing
  `get_week_start` query text and descriptor.
- **Testing**:
  - `horae-core` unit tests for `week_bounds`;
  - `#[sqlx::test]` cases in a new `server_fns/approvals/period_tests.rs`;
  - SSR/VirtualDom page tests in `tests/approval_labels_ui.rs`;
  - existing approval, integration and browser suites unchanged;
  - native Chrome smoke, by root.
- **Target Platform**: browsers (Chromium acceptance) and the Linux server.
- **Project Type**: web application.
- **Performance Goals**: none new. One list query per week, as today; the added
  org read is a primary-key lookup.
- **Constraints**:
  - integer minutes only;
  - no overlap or widening;
  - no new CSS, tokens or shared component changes;
  - 015 stays inactive (`permission_policy_version` paths untouched).
- **Scale/Scope**: one page, one server function, one core helper, one route
  variant and one caller.

### Touched files

| File | Change |
|------|--------|
| `crates/core/src/week.rs` | `week_bounds(date, first_day) -> Option<(NaiveDate, NaiveDate)>` plus unit tests. |
| `crates/horae/src/server_fns/approvals.rs` | `list_approvals(status, week_start)`. Its body moves to `list_week(pool, manager, status, week_start)`, which validates per the contract and matches `a.period_start = $ws`, ordered by `u.name, a.id`. No change to approve, reopen or submit. |
| `crates/horae/src/server_fns/approvals/period_tests.rs` | New `#[sqlx::test]` cases (T-S1..T-S6 below), registered beside the existing `submission_tests` and `isolation_tests`. |
| `crates/horae/src/pages/approvals.rs` | `Approvals(week: String)`: pager, period labels, week-tagged resource, manager and week-start gating, busy flag, bulk result notice. |
| `crates/horae/src/route.rs` | `#[route("/approvals?:week")] Approvals { week: String }`. Root granted this. |
| `crates/horae/src/components/sidebar.rs` | `Route::Approvals { week: String::new() }`, the only caller. |
| `crates/horae/tests/approval_labels_ui.rs` | Stub signature updates, a `get_week_start` stub, a minimal `Routable` route stub and the new page tests (T-U1..T-U9). The three existing tests keep their assertions; the click-index test is updated for the pager buttons. |
| `.sqlx/` | The `list_approvals` descriptor is replaced. |
| `specs/017-approval-period-navigation/**` | These documents. |

Not touched: `server_fns.rs`, `build.rs`, `horae.css` and `horae-utils.css`,
`components/form.rs` and `controls.rs`, the timesheet page, migrations, the
015 contracts and `.specify/feature.json` (Spec Kit runs used a scratch
`SPECIFY_INIT_DIR`). The existing browser suites are also untouched:
`responsive-layout.cjs` still visits `/approvals` and stays a regression gate.

## Constitution Check

| Principle | Status |
|-----------|--------|
| I. Exactness | Totals stay integer sums of `total_minutes` / `billable_minutes`. No float. The bulk count is the server's integer. |
| II. Domain purity | The week-bounds rule lives in `horae-core`, with no I/O and its own unit tests. |
| III. Single datastore | No schema change. Org scoping is kept. |
| IV. Mutations via server fns | Mutations are unchanged. The UI calls only server functions. |
| V. Formatting gate | `nix fmt`, the regenerated `.sqlx` and an exact-head `nix flake check`. |

No violations, so Complexity Tracking is not needed. Re-checked after the
design: unchanged.

## Requirement → test map

| Requirement | Test (failing first) |
|-------------|----------------------|
| FR-001, SC-001 | T-U1: rows of three weeks render their own `period_label`; a cross-year week shows both years. |
| FR-002, FR-005, SC-002 | T-S1: W−1, W and W+1 for one user plus a foreign-org W. `list_week(W)` returns only own-org W. The status filter combines. |
| FR-003, FR-012, SC-004 | T-U2: an empty or garbage `week` requests today's aligned start; `week=2026-10-29` requests `2026-10-26`; a Sunday-start org requests a Sunday. |
| FR-004 | T-U3: Prev, This week and Next build `/approvals?week=…` for −7d, today and +7d (R7 harness risk). |
| FR-006 | T-S2: unparseable, non-aligned and overflowing (`NaiveDate::MAX` week) starts give `BAD_REQUEST`, and nothing is read. Core unit tests cover the `week_bounds` MIN/MAX edges. |
| FR-007, SC-003 | T-U4: shown totals equal the exact sum of the rendered rows (7:30 + 2:15 = 9:45). |
| FR-008, US3 | T-U5: the bulk click submits exactly the shown week's submitted ids. T-S3: `approve_periods` with W's ids leaves W−1's approval and entries unchanged. |
| FR-009, US3 | T-S4: `reopen_period(W)` leaves W+1's approval and approved entries. The existing isolation, submission, integration and preflight tests stay green unmodified. |
| FR-010 | T-U6: a response tagged with another start renders "Loading…", not its rows. T-U7: while an action is pending, every action button is `disabled`. |
| FR-011 | T-U8: a non-manager makes zero `list_approvals` calls. A `get_week_start` error shows its alert and makes zero list calls. |
| FR-013 | T-U9: the stub returns 2 for 3 ids, and the page shows "Approved 2 of 3 timesheets"; it is hidden after navigating to another week. |
| Boundary edge case | T-U2b: a `week` naming the max-representable week shows the inline error with both directions disabled and no list call. |

## Gates

1. Approval of this package by root. No code until then.
1. Implement test-first per [tasks.md](tasks.md). The core and UI tests need no
   DB.
1. The heavy slot, granted by root: a disposable DB for the `#[sqlx::test]`
   cases, `cargo sqlx prepare` (with the `touch` step), the offline
   all-targets build, clippy, the wasm check and `nix fmt`.
1. Root-run native smoke: bare `/approvals`, Prev, refresh, This week, and 320px
   header fit.
1. Two independent reviews, then exact-head `nix flake check`. No push or PR
   before root collects the work.
