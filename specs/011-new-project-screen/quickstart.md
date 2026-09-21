# Validation Quickstart: New Project

## Safety and prerequisites

Use `.worktrees/new-project-screen` and the Nix dev shell. Never point migrations, seeds or browser mutations at the real imported `horae` database or an existing user's preview. Keep ports 8080/8092 and their processes untouched.

The existing browser runner `crates/horae/tests/browser/run-design-checks.sh` creates its own temporary PostgreSQL cluster and refuses an occupied port. Extend that runner for this feature rather than silently reusing a running database. For integration development, provision a separate throwaway PostgreSQL cluster/database and set DATABASE_URL explicitly; role needs CREATEDB.

## Build and focused checks

From the worktree, inside `nix develop`:

```sh
cargo test -p horae-core
cargo test -p horae --features server project_creation
cargo test -p horae --features server invoice
cargo test -p horae --features server time_entries
cargo test -p horae --features server budget
cargo sqlx prepare --workspace -- --features server --all-targets
SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings
cargo check -p horae --features web --target wasm32-unknown-unknown
nix fmt
git diff --check
```

Run database commands only with the isolated DATABASE_URL. Regenerate the SQLx cache with server features; omitting them can delete it.

## End-to-end scenarios

1. Admin → Projects → New project. Create a client, choose it, add name/code/dates/tags; save and reopen details. Verify one project and all relationships.
1. Exercise every type/rate/budget branch with real catalog tasks/team, including Add everyone, restricted tasks and zero rates. Verify independent expected totals and invoice eligibility.
1. Change draft values, await real save acknowledgement, reload; simulate failed save/conflicting tab/repeated final submit. Verify no lost acknowledged input or duplicate project.
1. Test managers, leads, assigned members and foreign-org users directly against server/compatibility/export paths. Verify absent confidential fields and unchanged personal timesheets.
1. Prepare fixed-fee single/milestone/monthly invoices; verify no synthetic time/double charge. Override terms/PO/taxes/discount; confirm snapshot isolation and PDF/CSV/XLSX totals. Void and regenerate to test occurrence claims.
1. With isolated mail stub, cross a budget threshold twice and retry a failed delivery. Verify one logical notification, stable identity and no retry after acknowledgement. Without configuration, verify disabled email control and explanation.
1. Inspect real application at widths 390/768/1440 and keyboard-only operation. Exercise each modal, error, empty and pending state. Do not execute/render prototype files.
1. Run existing Projects selection/bulk recovery, Clients, Timesheet, Invoices, navigation and menu regression suites, then full flake CI.

## Completion evidence

Record commands/results and outstanding limitations in the PR. Check off tasks only after their acceptance checks pass. Requirements checklist completion is not implementation completion. Final gate: complete workflow, adversarial review, supported build/test checks and a scoped PR ready for human review; no automatic merge.

## Execution log — 2026-09-21

- Spec Kit prerequisites/plan/tasks setup executed in the isolated worktree. Requirements checklist 16/16; analysis mapped 20 functional requirements and 7 success criteria to 54 tasks without material gaps.
- Nix dev shell verified Rust 1.96.1, Cargo 1.96.1 and PostgreSQL 17.10 tools. Existing ignore rules cover target, worktrees and local database state. Browser runner inspected: creates its own temporary cluster; it has not been run for this feature yet.
- Core TDD red: 14 new checks failed at their unimplemented functions while 88 existing checks passed; a second increment had 3 expected new failures.
- Core green: `cargo test -p horae-core --quiet` passed 105 tests; `cargo clippy -p horae-core --all-targets -- -D warnings` passed; `cargo fmt --all` applied. Commands ran inside `nix develop` with the shared temporary target cache.
- Draft model tests passed (3). Schema tests first failed compilation against the baseline schema, then passed against migration 0030. An additional adversarial test reproduced a nullable fee-shape CHECK accepting an incomplete schedule; the CASE-based constraint fixed it. The combined focused `project_creation` run passed 8 tests (3 model + 5 database).
- The development PostgreSQL cluster is `/tmp/horae-new-project-db.XqwJiR`, socket-only, database `horae_new_project`. Its initial migration 0030 predates the fee CHECK correction; sqlx test databases receive the corrected current migration. Before applying subsequent migrations in development, create a fresh isolated database rather than rewriting the migration ledger or resetting real data.
- Explicit no-cache formatting covered all 13 new Rust/Spec Kit targets and passed without changes. Shared formatter configuration was not changed.
- Full server Clippy, WASM, browser and complete flake checks remain pending. The not-yet-connected DTOs currently produce dead-code warnings; do not suppress them instead of wiring the feature. No production/import database was changed.
- Backend increment: 26 focused tests passed, covering actor/client locks, private draft ownership, bounded raw input, concurrent saves, exact final-form validation, atomic creation/retry/rollback, project-only costs, task restrictions, currency mismatch, paginated catalogs and explicit client creation. An adversarial response-code test reproduced a 409 instead of 404 for unavailable foreign drafts; the corrected response now passes without disclosing ownership.
- A fresh database `horae_new_project_current` was created in the same isolated socket-only cluster and all 30 current migrations applied successfully. Use this database for subsequent commands; the earlier base is no longer needed for compilation.
- Cache regeneration on a warm shared target also requires recompiling the integration-test entry points: `touch crates/horae/tests/integration.rs crates/horae/tests/cli_restart.rs` changes timestamps only, then run `CARGO_INCREMENTAL=0 cargo sqlx prepare --workspace -- --features server --all-targets`. Verify no unrelated cached queries were deleted before committing.
- The draft persistence dependency was implemented before finalization so atomic/retry tests exercise a real saved draft. UI tasks remain pending; the new creation settings are not yet integrated into reporting, invoicing or task mutation enforcement. Project-created outbox records are written transactionally; their consumer remains pending.
- Verification of the backend checkpoint: all 26 focused checks passed; `SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings` passed after simplifying two nested conditionals. `cargo check -p horae --features web --target wasm32-unknown-unknown` passed with 18 unused-model warnings because the new screen is not wired yet. SQLx regenerated with zero deletions of existing cache entries. No shared CSS, utilities, existing pages or prototype files changed.
