# Implementation plan: project budget remaining tile

**Branch**: `feat/project-budget-summary` | **Date**: 2026-10-22 | **Base**: `0243a386`
**Spec**: [spec.md](spec.md) | **Research**: [research.md](research.md) |
**Data model**: [data-model.md](data-model.md) |
**Contract**: [contracts/budget-summary.md](contracts/budget-summary.md) |
**Validation**: [quickstart.md](quickstart.md)

## Summary

Add one Budget remaining tile to Project Detail. Load it through the existing
bound `get_project_detail_view`, inside one REPEATABLE READ snapshot that holds
the canonical project-read authority. Reuse the list's configured budget query,
legacy spend rules and `allocated_totals`. One small exact headline projection
carries the tile's states; the list API is unchanged. Hour values use an
integer-only `format_hours2`.

## Technical context

Rust 2024; Dioxus fullstack (SSR + WASM); Axum; PostgreSQL via sqlx macros;
`horae-core` for pure rules. No new dependency, migration, table, column,
permission or route. Changed queries need regenerated `.sqlx` metadata.
Tests: `cargo test -p horae-core`; `#[sqlx::test]` + `#[serial]` integration;
the SSR harness `crates/horae/tests/detail_navigation.rs`; a Playwright suite
under `crates/horae/tests/browser/`. Scale: one project per request; the
existing 5 s statement limit applies.

## Constitution check (before and after design)

| Principle | Status |
|-----------|--------|
| I Exactness | Pass. Integer minutes/cents; `format_hours2` loses its float; overflow is `Unavailable`. |
| II Domain purity | Pass. `headline` lives in `horae-core` and reuses `allocated_totals`. |
| III Single datastore | Pass. No schema change; org-scoped joins are unchanged. |
| IV Server functions | Pass. Read stays in the existing typed server function; no client fetch. |
| V Builds/format | Pending until the coordinated build: `.sqlx` regen, offline tests, clippy native/WASM, `nix fmt`. |

READ WRITE mode exists only to take authority locks, as in
`crates/horae/src/server_fns/snapshot.rs`; no business row is written. No
violation, so no complexity tracking.

## Design

The states and derivation rules are in [data-model.md](data-model.md); the
wire shape and authorization in [contracts/budget-summary.md](contracts/budget-summary.md).

### Server (all `cfg(feature = "server")`)

1. **`ReadAccess::begin_snapshot`** in
   `crates/horae/src/server_fns/projects/read_access.rs`.
   - Move `begin`'s body after isolation into a shared loader. `begin` keeps
     READ COMMITTED via `configure_administration`, so its callers are unchanged.
   - `begin_snapshot` sets `REPEATABLE READ, READ WRITE`, then
     `configure_transaction_limits`, then the loader.
   - On SQLSTATE 40001 it rolls back and begins a new transaction, at most 3
     attempts. After that it returns the existing
     `conflict("Access changed while reading. Please try again.")`. Other errors
     keep `begin`'s mapping.
   - It does not use `snapshot::manager` or its role check.
   - The retry wraps only the begin. That suffices because the downstream read
     graph has no locking read or write (R3; audited in T016). There is no
     generic retry helper.
1. **`headline`** in `crates/core/src/budget.rs`. One call to
   `allocated_totals` plus an explicit has-allocation fact; no second summation.
   See [data-model.md](data-model.md#derivation).
1. **`project_budget`** in `crates/horae/src/server_fns/budgets.rs`, inside
   the caller's snapshot, after its authority locks:
   - A non-arithmetic metadata preflight reads kind, currency, settings
     existence, monthly reset, the unchanged money-visibility predicates, and
     whether any applicable allowance is non-null (same tenant and membership
     joins as the consumption queries).
   - Precedence: no budget → `None`; fee without money → `Unavailable`; no
     allocation → `Unallocated`; otherwise read consumption.
   - Consumption runs in one sqlx nested transaction (savepoint). It uses
     `fetch_progress(…, allocated_only = true)` when settings exist, otherwise
     the legacy spend helper. Success releases the savepoint. Only SQLSTATE
     22003 rolls back to it and yields `Unavailable`. Any other error, rollback
     error or missing expected row stays a server error.
   - `fetch_progress` gained `allocated_only`; the existing callers
     (`progress_for_viewer`, alerts) pass `false`.
   - `progress_for_viewer` and `ProjectBudgetOverview` are unchanged.
1. **Legacy spend helper.** `spend_in_transaction(connection, org, viewer, permissions, project, include_money) -> Result<_, sqlx::Error>` in
   `crates/horae/src/server_fns/projects.rs`.
   - `include_money` gates money visibility, so neither the per-entry amount
     nor the final sum is computed without it.
   - `fetch_project_spend` passes `true`; its signature and error mapping are
     unchanged.
   - The legacy hour fallback passes `false`.
1. **Access gate.** `db::lock_organization(AccessChange)` writes a same-value
   row version (review finding P1, below).
1. **Detail view** in `crates/horae/src/server_fns/projects/detail_view.rs`.
   - `fetch` gains `today: NaiveDate` and uses `begin_snapshot`.
   - It loads the budget in the same transaction, before commit.
   - `get_project_detail_view` passes `chrono::Utc::now().date_naive()`, as
     `list_project_budget_progress` does.
   - Its 22 test callers pass a fixed date (`today()`, 2026-09-07).

### UI (`crates/horae/src/pages/projects.rs`)

- A pure `budget_tile(&ProjectDetailBudget) -> BudgetTile` (strings plus
  accessible labels), with unit tests and one exhaustive `match` over
  `BudgetHeadline`.
- A `ProjectBudgetTile` component sits between the header card and Project
  tasks, as in the handoff. The handoff's chart is not part of this increment.
- Copy per state follows the [contract](contracts/budget-summary.md#ui-contract).
  The UI pattern awaits review: O1 is user-deferred (no percentage) and O2 is
  a pending proposal.
- The bar reuses `used_percent` and the list's `<progress class="proj-bar">`;
  a zero budget has no bar (O3).

Styling (R5): `card rounded-xl p-5` with utilities. The container uses the
`grid gap-4` utilities in markup. The only new structural rule is
`.pd-summary { grid-template-columns: repeat(auto-fill, minmax(min(15rem, 100%), 1fr)); }` in `crates/horae/assets/css/horae.css`, because no utility
expresses those columns. No inline styles or hex, and no shared default
changes.

### Display formatting (`crates/core/src/duration.rs`)

Integer `format_hours2` (R4). `format_decimal`, `format_hhmm` and
`format_cents` are unchanged.

## Validation

All cases, commands and resource rules are in [quickstart.md](quickstart.md).

## Risks

- P1/P4 rest on official documentation screenshots, not a live configured
  project: the five Harvest projects read so far have no budget. Hour budgets,
  lifetime row wording and tie rounding have no direct evidence (P2, O1).
  These affect T006, T011, T012 and T015, not the server contract.
- Database-side valuation overflow (SQLSTATE 22003) shows `Unavailable` on the
  detail page through the savepoint. The Projects list keeps its existing
  error for the same data; it is not changed here.
- The access gate now writes a row version per access change. Concurrent
  project-editor saves can therefore see the existing reload conflict more
  often (P1).
- Changing `format_hours2` touches reports and exports. Output is unchanged in
  the tested ordinary range. For very large values (the counterexample is below
  2^53) it may differ, which corrects the float's wrong digits. Existing report
  suites cover the callers.
- REPEATABLE READ can retry under write contention; it is bounded to 3 attempts
  and returns a retryable error.
- Legacy fee consumption uses the Projects list spend valuation, including
  attached invoice-line amounts when present; it is not an invoiced-total
  metric.

## Implementation record

Local and uncommitted, awaiting coordinator review. This covers foundation and
server only; it is not delivery of the tile or of 004.

**Resources.** A disposable PostgreSQL 17 created with `mktemp`, `initdb` and
`pg_ctl`, listening on a free non-default TCP port, with all migrations
applied (`sqlx migrate run --source crates/horae/migrations`). The dev shell
exports its own `DATABASE_URL`, so pass the disposable one inside it:
`nix develop --command env DATABASE_URL=postgres://horae@127.0.0.1:<port>/horae <cmd>`.
No `.data`, ports 8080/8093/8094 or existing preview were used.

**Gates run** (dev profile; `<db>` stands for the command prefix above):

| Command | Result |
|---------|--------|
| `cargo test -p horae-core format_hours2` before T003 | failed as expected (float gives `.53`) |
| `cargo test -p horae-core` after T003 | 193 passed |
| `cargo test -p horae-core headline`, then the whole core suite | 4 failed (`todo!`), then 197 passed |
| `<db> cargo test -p horae --features server --bin horae detail_view_` on the READ COMMITTED baseline | 6 passed, 2 failed (coherence mixed; exhaustion wait timed out); exit status not captured |
| `<db> … --bin horae canonical_read_tests` after T005 | 64 passed; exit status not captured |
| Same, with the budget stubbed | exit 101; 12 failed (the new budget tests), 65 passed |
| Same, after T009 | exit 0; 77 passed |
| `<db> … --bin horae -- server_fns::projects server_fns::budget server_fns::invoices server_fns::importers::authorization_tests::project_reads server_fns::snapshot reports::` | exit 0; 447 passed, 2 ignored |
| CI profile from here (`CARGO_PROFILE_TEST_DEBUG=line-tables-only`): `<db> … --bin horae -- detail_budget::archived_project_keeps_its_configured_budget detail_budget::foreign_client_parent_hides_configured_and_legacy_budgets` | exit 0; 2 passed |
| `cargo sqlx prepare --workspace -- --features server --all-targets` (after `touch`) | exit 0 (superseded below) |
| P1/P2 reproductions before the fixes | exit 101; 3 failed (stale membership read; two 500 "bigint out of range") |
| `<db> cargo test -p horae --features server -- --test-threads 8` with the gate fix, before the overflow fix | exit 101; `--bin horae` 1523 passed, 2 failed (the two known overflow tests), 11 ignored. Cargo stopped there, so the integration targets did not run. Partial evidence only. |
| `<db> … --bin horae -- server_fns::projects server_fns::project_creation server_fns::permissions server_fns::budget` after both fixes | exit 101; 519 passed, 1 failed. The new editor-visibility test's fixture had no project rate (400 "Project rate is required"); production validation is unchanged. |
| `<db> … --bin horae -- server_fns::projects::canonical_read_tests` after fixing that fixture and adding the canonical-editor and reader-first cases | exit 0; 92 passed |
| final `cargo sqlx prepare` (after `touch`) | exit 0; 37 query files added, 2 removed |
| `SQLX_OFFLINE=true cargo test -p horae-core` | exit 0; 197 passed |
| `<db> SQLX_OFFLINE=true cargo test -p horae --features server -- --test-threads 8` | exit 0; `--bin horae` 1535 passed, 11 ignored; all 15 integration targets passed, including `detail_navigation` (72) and `integration` (36) |
| `SQLX_OFFLINE=true cargo clippy -p horae-core --all-targets -- -D warnings -W clippy::perf` | exit 0 |
| `SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf` | exit 0 |
| `cargo clippy -p horae --features web --target wasm32-unknown-unknown -- -D warnings` | exit 0 |
| `nix fmt`, then `nix fmt -- --ci` | 10 files reformatted (rustfmt and mdformat only; SQL literals unchanged), then 0 changed |

The tests and clippy ran on the source before that formatting pass. The
post-rebase checks rerun on the formatted source. No browser suite has run,
and none exists yet for this tile.

**Post-begin statement audit (T016).** After `begin_snapshot`,
`detail_view::fetch` runs:

- `project_details_in_transaction`;
- the context query;
- `assignment_rows`;
- the team and task queries;
- `budgets::project_budget`: the metadata preflight, then `SAVEPOINT`, one of
  `fetch_progress` or `spend_in_transaction`, then `RELEASE` (or
  `ROLLBACK TO SAVEPOINT` on 22003 only).

All data statements are plain SELECTs: a source scan found no `FOR SHARE`,
`FOR UPDATE`, `INSERT`, `UPDATE` or `DELETE`. The savepoint statements are
transaction control. They neither lock nor write business data and keep the
REPEATABLE READ snapshot. Only the begin's two authority locks can raise
40001, so the begin-only retry is sufficient.

**Review finding P1 — stale authority after a lock-only gate (fix applied).**

- **Defect:** `db::lock_organization(AccessChange)` took `FOR NO KEY UPDATE`
  without writing the row, and 41 mutation paths use it.
  - A REPEATABLE READ detail reader takes its snapshot at its first statement,
    then waits on the gate.
  - When a lock-only writer (for example `remove_assignment`) commits, the
    reader's `FOR SHARE` succeeds without 40001. It then authorizes from a
    snapshot that still contains the removed membership.
- **Reproduction:**
  `detail_view_observes_membership_removed_behind_the_lock_only_access_gate`
  pauses the production writer after the gate with a table lock on
  `assignments`. It failed red on the old gate: the reader returned the
  project.
- **Fix (coordinator-approved, `db.rs` only):** `AccessChange` now runs
  `UPDATE organizations SET access_revision = access_revision WHERE id = $1 RETURNING id`.
  - The lock strength is the same (NO KEY UPDATE), so FK `KEY SHARE` checks
    stay compatible.
  - The update writes a new row version. A REPEATABLE READ reader whose
    snapshot predates the change therefore gets 40001 on its `FOR SHARE` and
    retries with a fresh snapshot. This is PostgreSQL 17's documented
    behaviour for rows updated after a REPEATABLE READ snapshot (`transaction-iso.html`,
    `explicit-locking.html`).
  - The logical revision, key and stored values are unchanged.
  - `Shared` mode is unchanged.
- **Cost:** one HOT-eligible row version and its WAL per access change, and
  more dead tuples on `organizations` for vacuum. The review found no trigger,
  `updated_at` column or `xmin` consumer on `organizations`.
- **Behaviour change:** the project editor's save runs SERIALIZABLE and takes
  this gate after its first statement. If another access change commits while
  the save waits, the save now ends with the existing "The project changed
  while saving. Reload before retrying." conflict instead of proceeding on its
  pre-wait snapshot. No other gate caller runs REPEATABLE READ or
  SERIALIZABLE.
- **Regressions added**, all in `canonical_read_tests/concurrency.rs` and
  green in the 92-test gate:
  - **Writer-first, production paths:**
    `detail_view_observes_membership_removed_behind_the_lock_only_access_gate`
    (`remove_assignment`) and
    `detail_view_observes_report_visibility_revoked_by_the_project_editor`.
    The latter is a policy-1 editor, so `editor.access` is present, managers
    are unchanged and the gate is re-entered through manager authorization.
  - **Reader-first:**
    `membership_removal_waits_for_a_reader_that_already_holds_authority`.
    The reader returns the pre-removal project, and the removal completes
    afterwards.
  - **Stored values:** `access_change_gate_keeps_the_stored_organization_unchanged`.
    Rollback leaves the row and its version unchanged. After commit every
    stored value, including `access_revision`, is unchanged; only the row
    version is new.
  - The existing delegation/foreign-key, editor, replay, no-op and
    cancellation suites pass in the 519-test gate.

**Review finding P2 — budget overflow failed the whole detail (fix applied).**

- **Reproduction:**
  `configured_fee_valuation_overflow_is_unavailable_not_a_failed_detail` (rate
  `i64::MAX`, 120 minutes) and
  `legacy_hours_budget_ignores_unrelated_money_overflow` failed red with 500
  "bigint out of range" (SQLSTATE 22003), which aborts the transaction.
- **Fix:** the metadata preflight, the 22003-only savepoint,
  `allocated_only`, and `include_money` (design steps 3–4).
- **Tests** (`detail_budget.rs`, green):
  - configured line overflow and sum overflow;
  - legacy fee line and sum overflow;
  - legacy hours with unused money overflow, while the list still errors;
  - unallocated configured scopes (empty and all-blank) keep kind and month
    and value nothing;
  - blank-scope overflow cannot hide an exact allocated total;
  - explicit-zero overflow is `Unavailable`;
  - a non-22003 failure (statement timeout) still fails the detail with
    `INTERNAL_ERROR`;
  - the shared-member person budget is aggregate-only.

**Known test gaps:**

- T014 SSR cases;
- the whole UI (T011/T012/T015/T017).

## Spec Kit execution

`check-prerequisites.sh --json --paths-only` and `setup-plan.sh --json` ran
with `SPECIFY_FEATURE_DIRECTORY=specs/004-project-dashboard/budget-summary`.
`setup-plan` rewrote `.specify/feature.json`; it was restored to its committed
`016-expense-parity` value. No extension hooks are installed. The
`update-agent-context` script is absent, and no substitute was added.
Phase 1 artifacts: [data-model.md](data-model.md),
[contracts/budget-summary.md](contracts/budget-summary.md) and
[quickstart.md](quickstart.md). Plan sections moved there are linked, not
duplicated.
