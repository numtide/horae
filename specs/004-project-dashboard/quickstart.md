# Quickstart & Validation: Project Detail Dashboard

End-to-end validation that the dashboard is correct and **data-honest** (every figure traces to real data and reconciles). No schema migration is involved.

## Prerequisites

- Nix dev shell (`nix develop`) with a running PostgreSQL; `DATABASE_URL` set (default `postgres://localhost/horae`).
- Schema up to date and seeded:
  ```sh
  cargo run -p horae --features server -- migrate run
  cargo run -p horae --features server -- seed
  ```
- If the new read queries changed, refresh the offline cache (no schema change, cache only):
  ```sh
  cargo sqlx prepare --workspace -- --features server --all-targets
  git add .sqlx/
  ```

## Build, test, run

```sh
cargo test -p horae-core                                   # pure rollup fold + progress_pct + reconciliation invariants
DATABASE_URL=… cargo test -p horae --features server       # #[sqlx::test] integration for the new read fns
cargo clippy -p horae --features server
cd crates/horae && DEV_LOGIN=1 DATABASE_URL=… dx serve      # http://localhost:8080
```

Sign in as Admin (`DEV_LOGIN=1`), open **Projects**, click a project → the detail route (`Route::ProjectDetail { id }`) now shows the dashboard instead of the stub.

## Manual validation (maps to user stories / success criteria)

1. **Budget & progress — hours project (US1, SC-001)**: open an hours-budgeted project with tracked time. Confirm hours spent, hours budget, remaining, and a `proj-bar` at spent ÷ budget. Header shows name/code, client, type badge, currency, active/archived, start/end when set.
1. **Budget & progress — fee project (US1)**: open an amount-budgeted project. Confirm fee spent / fee budget / remaining and the same bar style, all in the project's currency.
1. **No budget (US1, edge)**: open a `budget_kind = none` project. Confirm totals show with **no** progress bar and no "remaining".
1. **List parity (SC-002)**: note the project's "Spent" on the Projects list, then compare with the dashboard "Spent" — they must be **identical** (both use the shared `fold_entries`).
1. **Hours split (US1)**: confirm total hours = billable + non-billable hours.
1. **Breakdowns reconcile (US2, SC-003)**: for a project with two tasks and two people (some non-billable), confirm the by-task and by-person tables each sum (hours and amount) to the headline totals, and each row's billable + non-billable hours = its total.
1. **Invoiced vs uninvoiced (US3, SC-004)**: for a project where some billable entries are invoiced, confirm invoiced = sum of that project's invoice line-item amounts and invoiced + uninvoiced = total billable.
1. **Team, tasks, recent (US4)**: confirm the assignments table (unchanged), the enabled-tasks list with billable flag and rate (or "—"), and a newest-first recent-entries list (date, person, task, hours, billable, note snippet).
1. **Unresolvable rate / hidden money (FR-014)**: a billable entry with no rate anywhere counts hours but adds 0 to amount, and the section flags money may be incomplete; a viewer without money visibility sees all amounts as "—" with hours still shown.
1. **Empty & over-budget (SC-005)**: a project with no tracked time renders zero totals / empty tables without error; an over-budget project caps the bar at 100% while the numeric % and remaining convey the overage.

## Automated checks

- **`horae-core` unit tests** (`project_rollup.rs`): `billable + nonbillable == total`; Σ by_task == Σ by_person == total (minutes and cents); `progress_pct` clamps at 100 and returns `None` for zero budget; all-`None`-rate rows add 0 cents and increment `unresolved_rate_minutes`; money stays integer cents (no float).
- **Integration (`#[sqlx::test]`, `#[serial]`)**: seed a project with billable/non-billable entries across two tasks/two people, invoice some entries, then assert `get_project_dashboard` + `list_project_breakdowns` reconcile and `invoiced + uninvoiced == total billable`; assert `get_project_dashboard.spent_*` equals `list_project_spend` for the same project (SC-002); assert `NOT_FOUND` for a foreign-org project id; assert empty-state project returns zeros/empty vectors.

## Definition of done

- Dashboard replaces the stub; all sections render and reconcile per the checks above.
- No migration added (verify: `git status crates/horae/migrations/` shows no new file) — FR-016 / SC-007.
- `nix fmt` and `nix flake check` green; `.sqlx/` cache committed if queries changed.
