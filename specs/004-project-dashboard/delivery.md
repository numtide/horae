# Design delivery: Project Detail, Clients, Settings and Workspace

Requested 2026-09-29. Baseline: `9301112` (`master`, after #207).

This coordinates the complete outcome, not a replacement for feature specifications. Completing Project Detail alone does not complete the delivery. No slice is accepted yet.

| Slice | Required outcome | Current evidence | Next gate |
|---|---|---|---|
| Project Detail | Dashboard, real summaries, charts/periods, Tasks/Team/Invoices, actions and exports | Metadata/tasks/assignments/fee balances exist | Reconcile specification, plan/contracts/tasks, implementation and acceptance |
| Clients list/detail | Designed list, search/filter/selection/actions, currency-safe summaries, client projects/invoices and editing | Basic CRUD; detail placeholder; default client rates already exist | Separate Spec Kit feature including contact/billing metadata and archival semantics |
| Personal Settings | Profile/photo/timezone, rates/assigned people/actual permissions and notifications; preserve theme/plugins | Theme/plugins exposed | Separate Spec Kit feature; OIDC ownership and notification delivery contract |
| Workspace | People/invitations, fixed-role matrix, general settings, full exports/backups and audit UI | Basic People/Importers shell | Separate Spec Kit feature; delivery, backup storage/retention and destructive safeguards |

## Delivery rules

- Isolated worktrees/branches, scoped PRs and unsigned commits as authorized. Do not auto-merge new work.
- Preserve existing roles, exact arithmetic, privacy, imported development data and previews.
- No fake totals/actions, notification switches without delivery, backup promises without storage or empty destinations count as complete.
- Unresolved later-slice decisions do not block independent work, but remain required before their acceptance.
- Reuse shared UI and verify cross-screen regression. No CSS framework replacement or new crate solely for these screens.
- Final acceptance requires evidence for every slice, real responsive/keyboard browser flows, full Nix checks and scoped PR delivery.

## Progress

- [x] Inspect current master/specs; preserve unrelated `.playwright-mcp/` files.
- [x] Create `.worktrees/project-dashboard` / `feat/project-dashboard` from `origin/master`.
- [x] Locate/read Spec Kit specify/clarify/plan skills in `.claude/skills/`.
- [ ] Complete Project Detail specify/clarify/plan/tasks/analyze, implementation and acceptance.
- [ ] Specify, implement and accept Clients list/detail.
- [ ] Specify, implement and accept Personal Settings.
- [ ] Specify, implement and accept Workspace.
- [ ] Verify cross-screen journeys, publish scoped PRs and reconcile the historical design inventory.

## Verified implementation increment — 2026-09-29

`crates/core/src/project_activity.rs` now implements inclusive custom/calendar
ranges and bounded weekly/cumulative tracked-minute series. It reuses the existing
calendar/week helpers, adds no dependencies and changes no billing rules, schema,
CSS or imported records. Configured week starts, empty/partial weeks and checked
overflow are supported.

Evidence:

- Baseline: 115 core tests passed before the change.
- TDD: the initial 16 new tests failed against unimplemented functions, then passed
  after implementation; six additional boundary/reconciliation tests were added.
- `nix develop --command cargo test -p horae-core`: 137 passed, zero failures.
- `nix develop --command cargo clippy -p horae-core --all-targets -- -D warnings`:
  passed without warnings.
- The changed Rust source was formatted using the Nix-pinned rustfmt.

This is a tested domain foundation, not a delivered dashboard. Authorized SQL,
screen integration, exports, browser acceptance and full application/Nix checks
remain required, as do all Clients and Settings/Workspace deliverables. The full
Spec Kit planning/acceptance gates remain open for the documented policy decisions.

## Authorized activity projection — 2026-09-29

`server_fns/projects/activity.rs` now exposes actual weekly tracked minutes through
a session-authenticated server function. It checks current progress authority in
the same repeatable-read, read-only transaction as the date bounds and aggregate.
The payload has no entry notes, identities, rates or financial values. All-time
empty projects return no invented dates; selected intervals retain empty weeks.
The existing organization weekday conversion is shared with the timesheet.

Evidence:

- The initial six PostgreSQL tests failed at the unimplemented projection, then
  all passed after implementation.

- Two additional tests cover invalid stored weekdays and inactive contributors /
  tasks with same-organization project isolation. Permission tests also cover
  configured member visibility and both project-management assignment roles.

- `cargo test -p horae --features server --bin horae server_fns::projects`:
  **66 passed, zero failures**, including all eight activity tests and existing
  detail, privacy, bulk-action and mutation tests.

- Tests ran with Nix on an isolated, migrated PostgreSQL cluster; no imported
  development or Harvest data was used or changed.

- `cargo test -p horae --features server --bin horae models::project::tests`:
  both wire-format tests passed, including exact `i64::MAX` round-trip and an
  empty all-time response.

- `cargo sqlx prepare --workspace -- --features server --all-targets`: passed;
  ten added query-cache records and no removed/modified existing cache records.

- `SQLX_OFFLINE=true cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf`: passed, including integration-test targets.

- `cargo check -p horae --features web --target wasm32-unknown-unknown`: passed.
  The three activity DTOs still produce dead-code warnings on the web target
  until their dashboard consumer is connected; no lint suppression was added.

- `nix fmt -- --ci` and `git diff --check`: passed.

This is not UI or full application acceptance: dashboard integration and all
remaining delivery slices are still required. Full `nix flake check` remains
pending; the PR must stay in draft.
