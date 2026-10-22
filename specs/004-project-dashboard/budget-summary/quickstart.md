# Quickstart: validate the budget remaining tile

States and rules: [data-model.md](data-model.md). Wire and UI contract:
[contracts/budget-summary.md](contracts/budget-summary.md).

## Resources

- Disposable resources only. Do not use `.data`, ports 8080/8094 or an
  existing preview.
- The coordinator schedules one heavy build and a disposable migrated
  database.
- The browser runner keeps its port guard (`8093`, socket under
  `/tmp/horae-browser.*`).

## Commands (inside `nix develop`)

The dev shell exports its own `DATABASE_URL`, so set the disposable one inside
the shell, for example `nix develop --command env DATABASE_URL=… <cmd>`.
Final gates use CI's `CARGO_PROFILE_TEST_DEBUG=line-tables-only`.

```sh
cargo test -p horae-core
cargo test -p horae --features server --bin horae -- detail_budget canonical_read_tests::concurrency
cargo test -p horae --features server --test detail_navigation
touch crates/horae/src/main.rs crates/horae/tests/*.rs
cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo test -p horae --features server -- --test-threads 8
SQLX_OFFLINE=true cargo clippy -p horae-core --all-targets -- -D warnings -W clippy::perf
SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf
cargo clippy -p horae --features web --target wasm32-unknown-unknown -- -D warnings
nix fmt
```

## Core scenarios

`format_hours2`:

- Equal to the old float output over −100 000..=100 000 only.
- Exact strings for `9_007_199_254_740_991 → "150119987579016.52"` (the float
  gives `.53`) and for `i64::MIN`/`i64::MAX`.
- `-30 → "-0.50"` and `1 → "0.02"`.

`headline`, as independent arithmetic-state cases:

| Input | Expected |
|-------|----------|
| `[]` | `Unallocated` |
| `[(None, 120)]` | `Unallocated` |
| `[(None, 0)]` | `Unallocated` (visible all-blank fee shape) |
| `[(Some(i64::MAX), 0), (Some(1), 0)]` | `Unavailable` (allowance overflow, usage fits) |
| `[(Some(0), i64::MAX), (Some(0), 1)]` | `Unavailable` (consumption overflow) |
| `[(Some(i64::MIN), 1)]` | `Unavailable` (remaining overflow; unreachable from stored data) |
| `[(Some(60), 30), (None, 1000)]` | `Exact { 60, 30, 30 }` (blank excluded) |
| `[(Some(0), 120)]` | `Exact { 0, 120, -120 }` (explicit zero counts) |
| `[(Some(120), 150)]` | `Exact { 120, 150, -30 }` |

`allocated_totals`' and `used_percent`'s existing tests stay unchanged.

No remaining-percentage helper or tests exist yet: O1, the exact-half rule,
is user-deferred. Harvest's non-tie samples (2,300/8,000 → 29; −5,764.50/50,000
→ −12) stay reference data for that later work.

The bar (UI, not yet built) will reuse `used_percent`: `used_percent(570000, 800000) == Some(71)`, `used_percent(150, 120) == Some(100)`, and `None` for a
zero budget (O3: no bar).

## Integration scenarios (`canonical_read_tests/detail_budget.rs`)

- **Legacy (policy 0) without settings:** hours use raw lifetime minutes, fees
  use spend cents, and kind `none` gives `None`.
- **Configured project scope:** rounding, and `include_nonbillable` on and off.
- **Monthly reset** at dates 2028-02-29 and 2028-03-01: first and last day
  count, adjacent months do not.
- **Task/person scope:** blank vs explicit zero, all-blank `Unallocated`, and a
  negative overrun.
- **Allowance overflow:** two task allowances summing past `i64::MAX` with
  fitting usage give `Unavailable`.
- **Kind `none`:** configured kind `none` with leftover legacy columns gives
  `None`.
- **Authorization pairs:** one fee project read by an authorized viewer
  (`Exact` or `Unallocated`) and by an unauthorized viewer (`Unavailable`, no
  number in the JSON):
  - a policy-1 member with progress visibility only;
  - `BillableRateReadAll`;
  - `BillableRateReadManaged`, as manager and as non-manager;
  - the same pairs for an all-blank fee budget and a legacy fee.
- **Not found:** without read access, for a foreign-org project, and for a
  project whose client is in another organization.
- **Archived** project.
- **Serialized view** has no scope, labels or per-scope values.
- **Parity:** `Exact` equals `progress_for_viewer` / legacy spend for the same
  date; the other states are checked by state.
- **Read-only:** unchanged `xmin` and row counts, including
  `project_budget_notifications` and `horae_outbox`.

## Concurrency scenarios (`canonical_read_tests/concurrency.rs`)

- **Authority writer-order fences:** the existing tests that run with
  `Reader::DetailView` keep passing against `begin_snapshot`:

  - `project_readers_observe_grant_revocation_after_waiting_for_organization`;
  - `project_readers_observe_deactivation_after_waiting_for_actor`;
  - `project_readers_hold_authority_until_materialization_before_deactivation`;
  - `cancelling_project_readers_releases_organization_and_actor_fences`.

  Writer-first revocation now surfaces as 40001, then a fresh retry, then
  denial.

- **Data-snapshot coherence (new):** this test touches only unlocked tables,
  never the organization or actor rows.

  1. A writer holds `LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE`.
  1. The reader takes its snapshot and authority locks, then blocks
     (`wait_for_blocked`).
  1. The writer renames the project, changes its budget allowance, inserts a
     qualifying entry and commits.
  1. The reader returns the old name, old allowance and old consumption
     together.

  The identity-and-labels variant (locking `project_tasks`) failed red on the
  READ COMMITTED baseline. The budget variant was added afterwards and is
  green-only.

- **Retry exhaustion (new):** three 40001s map to the conflict error.
  Deterministic writers chained with `wait_for_blocked` (C3, resolved): red
  on READ COMMITTED, green after `begin_snapshot`.

- **Lock-only gate (new):** the production `remove_assignment`, paused after
  the access gate, removes the viewer's membership while a reader waits. The
  reader must not return the project. Red before the `AccessChange` fix.

- **Inherited limits:** `statement_timeout` and
  `idle_in_transaction_session_timeout` are preserved.

## SSR and browser

- **SSR (`tests/detail_navigation.rs`):** the fixtures gain `budget`. Cases: the
  exact no-budget sentence, the link only when `can_edit` with
  `return_to=/projects/<id>`, and no stale budget after navigating between
  projects.

- **Browser (`tests/browser/project-budget-tile.cjs`):**

  - seeded no-budget (editor and non-editor), hours, fee, monthly ("October
    2026"-style caption), overrun (negative percent, danger colour, full bar)
    and unavailable projects;
  - both themes at 1440/768/390/320, with no horizontal scroll;
  - a short screen at 200% text;
  - keyboard reach of "Set up a budget" and return after Cancel.

  Existing `project-detail-header`, `projects-design`, `project-edit` and
  report suites keep passing.
