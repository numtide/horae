# Implementation plan: project activity foundation

**Branch**: `feat/project-activity-foundation` | **Spec**: [spec.md](spec.md)
**Base**: `8975b881` | **Reference**: #208 worktree at `48a41560`,
`crates/core/src/project_activity.rs` and its historical F001–F003.

## Summary

Recover the pure reporting-period and weekly-activity module from #208 into
`horae-core`, adapted only to current core conventions, with its tests. This is
a prerequisite for later reporting work, not a visible feature.

## Technical context

- Rust 2024 in `crates/core` (`horae-core`); existing `chrono` and `thiserror`.
- Reuses `week::week_start` and `project::monthly_fee_date`, both unchanged on
  the base since #208 (same signatures and semantics).
- Testing: unit tests in the module, `cargo test -p horae-core`; Clippy and
  rustfmt in the pinned Nix dev shell with a private target directory.
- No storage, server, WASM, UI or generated-cache impact.

Allowed source files: `crates/core/src/project_activity.rs` (new) and
`crates/core/src/lib.rs` (module registration). Docs stay in this directory.
The Spec Kit scripts run with `SPECIFY_FEATURE_DIRECTORY` set to this
directory; they persist it to `.specify/feature.json`, which is restored to its
original `specs/016-expense-parity` value before every commit.

## Constitution check

| Principle | Status |
|-----------|--------|
| I. Exactness | PASS: integer minutes, checked addition, overflow is an error. |
| II. Domain purity | PASS: `chrono`/`thiserror` only; no I/O framework. |
| III. Single datastore | N/A: no persistence. |
| IV. Mutations via server fns | N/A: no mutation or surface. |
| V. Reproducible builds | PASS: Nix dev shell core tests, Clippy, fmt; full `nix flake check` is coordinator-scheduled and runs in CI before merge. |

Re-checked after design: unchanged. No complexity-tracking entries.

## Research and decisions

- **Reuse the #208 module verbatim in behavior.** Rationale: it was reviewed with
  its tests; re-deriving risks drift. Alternative (rewrite) rejected.
- **Convention adaptations only**: `use thiserror::Error` import style like
  `money.rs`/`duration.rs`; merge the two test modules into one `mod tests`.
- **Caller-owned bucket limit.** Rationale: the domain cannot know the server's
  resource budget; exceeding it fails before allocation instead of truncating.
- **Calendar periods via `monthly_fee_date`.** Rationale: reuses the existing
  month-end resolution instead of adding another month-length rule.
- **Excluded from #208's later work**: `project_breakdown`, budget rounding,
  readers, DTOs, permissions; they depend on unresolved G01/O1 decisions or
  non-core layers.

## Design (public API of `horae_core::project_activity`)

- `ActivityError`: `IncompleteRange`, `InvalidDate`, `ReversedRange`,
  `DateOutOfRange`, `TooManyWeeks { limit }`, `NegativeMinutes`, `Overflow`.
- `ActivityRange` (inclusive, nonempty, private fields): `parse_optional`,
  `new`, `month`, `quarter`, `year`, `from`, `to`.
- `DailyActivity { date, billable_minutes, non_billable_minutes }` input.
- `WeeklyActivity { from, to, billable_minutes, non_billable_minutes,
  cumulative_minutes }` output, `from`/`to` clipped to the range.
- `weekly_activity(range, first_day, days, max_weeks) -> Result<Vec<_>, _>`.

## Quickstart (validation)

```sh
nix develop -c env CARGO_TARGET_DIR=<private dir> cargo test -p horae-core
nix develop -c env CARGO_TARGET_DIR=<private dir> cargo clippy -p horae-core --all-targets -- -D warnings
nix develop -c cargo fmt -p horae-core --check
```

Expected: all core tests pass (including the new `project_activity::tests`), no
Clippy warnings, no formatting changes. No browser, database or Harvest check
applies; none is claimed.

## Delivery record

### Spec Kit scripts (2026-10-26)

With `SPECIFY_FEATURE_DIRECTORY=specs/004-project-dashboard/activity-foundation`:

- `setup-plan.sh --json`: exit 0, "Plan already exists … skipping template
  copy"; `FEATURE_SPEC`, `IMPL_PLAN` and `SPECS_DIR` all in this directory.
- `setup-tasks.sh --json`: exit 0, `FEATURE_DIR` this directory,
  `TASKS_TEMPLATE` `.specify/templates/tasks-template.md`.
- `check-prerequisites.sh --json --require-tasks --include-tasks`: exit 0,
  `FEATURE_DIR` this directory, `AVAILABLE_DOCS ["tasks.md"]`. Research, data
  model and quickstart are sections of this plan, not separate files.
- The scripts rewrote `.specify/feature.json` to this directory; it was restored
  to `specs/016-expense-parity` with the edit tool before committing.

Earlier spec/plan/tasks drafts were written before these scripts ran; the
scripts' resolved paths match the files as written. Checklists: requirements
8/8, exactness 15/15, so implementation proceeded without a waiver.

### Analysis

No critical or constitution findings; AFR-001–006 and ASC-001/002 each map to
a task. U1 (medium): the reference invalid-date test only asserted `is_err()`;
it now asserts the exact error for each case. C1 (low): `nix flake check` is
coordinator-scheduled before merge, not run here.

### Verification

Filled in after T006; see [tasks.md](tasks.md).
