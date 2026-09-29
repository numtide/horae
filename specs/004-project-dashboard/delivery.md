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
