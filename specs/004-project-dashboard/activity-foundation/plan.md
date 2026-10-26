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
- `WeeklyActivity { from, to, billable_minutes, non_billable_minutes, cumulative_minutes }` output, `from`/`to` clipped to the range.
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

### Verification (2026-10-26, Nix dev shell, private `CARGO_TARGET_DIR`)

- Red: US1's 9 tests failed on `todo!()` stubs; then US2's 16 tests failed while
  the 9 US1 tests passed. Both went green after restoring the reference bodies.
- `cargo test -p horae-core`: 226 passed, 0 failed (201 existing + 25 in
  `project_activity::tests`); doc-tests 0.
- `cargo clippy -p horae-core --all-targets -- -D warnings`: no warnings.
- `cargo fmt -p horae-core --check` and `nix fmt -- --ci` on the changed files:
  clean (0 changed).
- Production code differs from the #208 reference only in the `thiserror::Error`
  import. The tests are the reference's 25, merged into one module; the invalid
  input test now asserts exact variants and adds a non-ISO date case.
- Covered errors: `IncompleteRange` (either bound missing), `InvalidDate`
  (empty, 2026-02-30, `07/09/2026`), `ReversedRange`, `DateOutOfRange` (first
  week before `NaiveDate::MIN`), `TooManyWeeks` (limit 4, limit 0, and
  MIN..MAX before allocation), `NegativeMinutes` (either split), `Overflow`
  (within a bucket, across the split, cumulative across weeks); `i64::MAX`
  exact total accepted; final week at `NaiveDate::MAX` clipped.
- Not run: full app build, `nix flake check`, database or browser checks; no
  Harvest or native UI behavior is newly verified. Coordinator schedules the
  heavy checks.

The analysis and verification above describe head `d83fb4c7`, before review.

### Review corrections (T009–T012, 2026-10-26)

Two independent reviews of `d83fb4c7` requested:

- **M1:** `parse_optional` used chrono's lenient `FromStr`, so `26-09-01` became
  year 26. Bounds must now equal their canonical `%Y-%m-%d` text, mirroring
  `server_fns/project_creation/validation.rs` without trimming. Signed and
  expanded years that round-trip (`-0001-01-01`, `+12026-09-07`,
  `NaiveDate::MIN`/`MAX`) stay valid. AFR-001 states the contract.
- **L1:** the error depended on input order (`[MAX, 1, -1]` gave `Overflow`).
  All included days are now checked for negatives before aggregation, and
  out-of-period entries are still ignored. The spec edge cases and the doc
  comment state the precedence.
- **Test gaps:** non-leap February (2026, 2100), and in the full-year test every
  interior boundary falls on the configured week start and every bucket spans
  1–7 days.
- **L2:** T007 ran before implementation. It moved to a pre-implementation
  phase with its original ID kept, and this re-analysis is T012.

Production code now differs from #208 by the import, `parse_date` and the
negative pre-pass. Raw logs (`set -o pipefail`, Nix dev shell rustc 1.96.1,
private `CARGO_TARGET_DIR`) are in `/tmp/horae-activity-logs-20261026/`:

| Step | Log | Result |
|------|-----|--------|
| Red (new tests, old code) | `red.log` | exit 101: 28 passed, 2 failed (`rejects_noncanonical_iso_dates_in_either_bound` got `Ok` for year 0026; `negative_minutes_win_over_overflow_in_any_input_order` got `Overflow`) |
| Targeted `cargo test -p horae-core --locked project_activity` | `targeted.log` | exit 0, 30 passed |
| Core `cargo test -p horae-core --locked` | `core-test.log` | exit 0, 231 passed, doc-tests 0 |
| `cargo clippy -p horae-core --all-targets --locked -- -D warnings` | `clippy.log` | exit 0, re-checked `horae-core`, no warnings |
| `cargo fmt -p horae-core --check` | `cargo-fmt.log` | exit 0 |
| `nix fmt -- --ci` on changed files | `treefmt.log` | exit 0, 0 changed |
| `check-prerequisites.sh --json --require-tasks --include-tasks` | `check-prerequisites.log` | exit 0, child `FEATURE_DIR`; selector restored to 016 |

Re-analysis (T012): spec AFR-001/edge cases, the code docs and tasks agree; each
AFR and ASC still maps to tasks; no constitution findings. Still not run: app
build, `nix flake check`, database, browser; no Harvest or native UI behavior is
newly verified.

### Next action

Coordinator's delta re-review (T008). No push, PR or merge until then.
