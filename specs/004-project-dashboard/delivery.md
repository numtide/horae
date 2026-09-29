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

## Activity panel integration — 2026-09-29

Project Detail now consumes the authorized activity projection. The panel has
cumulative and weekly SVG charts, all-time/month/quarter/year presets, explicitly
applied custom dates, and an expandable semantic table of exact weekly minutes.
Mode changes reuse the same response. Pending requests compare their interval
identity before rendering; project navigation resets the keyed page. Invalid
custom dates preserve the currently applied report and show an associated error.

Framework review: new chart-prefixed rules only for grid geometry and SVG paint;
spacing, typography and controls reuse utilities. The additive `min-h-control`
utility uses the existing form-control-height token and is used only by this
panel. No existing CSS selector, shared component behavior, SQL query, migration,
dependency, billing rule or imported record changed.

Evidence:

- Seven initial period/geometry tests failed before implementation, then passed.
  Two additional tests verify the rendered accessible table and empty states.
- `cargo test -p horae --features server --test detail_navigation`: 35 passed,
  including all nine panel tests and a new pending/failed project-navigation test.
- Offline server Clippy, including all targets and performance warnings: passed.
- App WASM check: passed without the previous unused-activity-DTO warnings.
- Formatting and whitespace checks passed.
- Windows Chrome through Playwright MCP: real sign-in and project navigation,
  normal clicks between chart modes and calendar presets, unchanged activity
  request count on mode switch, and September filtering verified against SQL.
  Seeded all-time minutes were 780 (13h); September minutes were 420 (7h).
- Custom form checks used DOM input/click events in the real browser: reversed
  dates show validation and retain the previous applied report; 2026-09-29 alone
  returns 210 minutes (3.5h), matching SQL. These do not constitute a full native
  date-picker or keyboard audit.
- The weekly table opens with a real Enter key. At 390px its wider columns scroll
  inside the table container; at both 390px and 320px the page does not overflow.
  Desktop and mobile screenshots remain in the root checkout's ignored
  `.scratch/playwright-windows/project-activity-{desktop,mobile}.png`.
- Browser response-order check: temporarily delayed one activity fetch in the
  test tab, observed loading with no stale chart, selected This year, then
  released the old Last month response. The current year's 13h remained intact.
  The original fetch function was restored after the check.
- After the accessibility adjustment, Chrome measured the chart-view buttons and
  period trigger at 44px tall; the final 320px page still did not overflow.
- Normal flows reported no application console errors. The log also contains
  expected development-WebSocket failures during the intentional preview restart
  and warnings from an unrelated browser extension. These were not hidden.

The full detail layout is not accepted: weekly chart navigation, applicable
hour-budget overlays, the five summary tiles, Tasks/Team/Invoices breakdowns,
actions, exports and full cross-screen/Nix acceptance remain outstanding.
Clients and Settings/Workspace remain required and unimplemented delivery slices.

## Project identity and lifecycle — 2026-09-29

Replace the generic detail title/card with the handoff's client link, code/name,
type and manager-only Edit/Actions controls. Show actual active/archived status
and currency; retain planning dates, tags and authorized administrator notes in
an expandable information section. Keep Projects highlighted on detail routes.
The existing scoped details query supplies client ID, project type and status;
its access predicate and private-field policy are unchanged.

Archive/reactivate reuses the existing server action with native confirmation,
pending protection, visible error/retry and success feedback. Update status from
the successful server response without unmounting the header: unmounting it for
a refetch removed the dialog's focus-return target. No additional action policy,
schema, dependency or shared CSS behavior was introduced. Responsive layout uses
only existing utilities; a 320px check caught and corrected title compression.

Evidence:

- Identity UI and SQL tests failed before implementation. Final detail navigation
  suite: 37 passed, including administrator/manager/member links, archived/no-code
  identity and pending/failed routes without stale edit links.
- Two PostgreSQL details tests passed, covering type/rate combinations, current
  roles, private notes, archived records, foreign organizations and inactive users.
- The complete Projects server regression passed again on the final code:
  66 tests, including activity, privacy, assignment, mutation and bulk operations.
- Three navigation-highlighting tests passed.
- SQLx cache regenerated with server/all-targets: replace the details-query cache;
  no migration or other query changed.
- Final all-target server Clippy (warnings/performance), app WASM check and
  formatting passed.
- Chrome MCP on the disposable demo verified confirmation/cancel, a deliberately
  failed transport, visible retry, a held retry with duplicate-submit suppression,
  disabled controls and cancellation prevention while pending, and successful
  archive/reactivate. Restore the original fetch function after fault injection.
- SQL before/after the lifecycle cycle retained five entries, 780 actual minutes,
  three enabled tasks and one assignment, and returned the project to active.
- Keeping the header mounted restored focus to the Actions trigger after both
  archive and reactivation. The edit link opened the shared editor with the
  saved project name and code, without saving any edit.
- Computed Chrome layout checks at 320/390/768/1440px showed no horizontal page
  overflow, a usable title width and a 44px Actions trigger. These measurements
  are not a substitute for foreground screenshots or 200%/keyboard acceptance.

Browser checks in this increment used DOM events: Chrome reported the page
hidden and stopped delivering animation frames; normal Playwright clicks and
screenshots timed out. This is not full visual/keyboard acceptance. The broader
dashboard, Clients and Settings/Workspace delivery remains incomplete, and the
PR stays in draft.

## Hours, budget and internal-cost tiles — 2026-09-29

Add three real summary tiles beneath activity, using the handoff's auto-fitting
240px columns and existing card, typography, spacing and progress utilities.
The only new CSS is a project-prefixed grid template; shared defaults, migrations
and dependencies are unchanged. Tile headings explicitly use the existing
sans-serif utility rather than inheriting the global heading face.

The summary reads one authorized, repeatable-read, read-only database snapshot.
Lifetime hours use actual stored minutes. Current configured budgets reuse the
overview's scope/period/rounding query; legacy amount budgets reuse its spend
query, now optionally restricted to one project. Core aggregation preserves
missing allocations, zero budgets, negative remaining balances and individual
scope overruns even when their combined allowance is positive.

Internal costs use actual minutes, per-entry rounding and current cost rates in
workspace currency. A missing rate makes the complete total unknown rather than
showing a partial sum. Members receive no cost payload; managers receive none
when any contribution uses an administrator-only project cost override. These
restrictions are enforced before the cost query, not only in markup.

Evidence:

- Core: 140 passing tests, including checked scope aggregation and overflow.
- Project server regression: 73 passing PostgreSQL tests, including seven
  summary tests. Coverage includes empty/archived/foreign projects, current and
  revoked permissions, monthly rounded budgets versus lifetime actual hours,
  overview parity, task allocations and missing/zero/private/mixed cost rates.
- Detail rendering/navigation: 40 passing tests, including loading/error and
  route changes without previous-project totals, period/currency labels and
  missing/private costs without a fabricated zero.
- SQLx metadata regenerated with server/all-targets and incremental compilation
  disabled: incremental preparation omitted unchanged integration-test queries.
  The complete cache retains those queries and replaces only the changed spend
  query, alongside the new summary/test queries.
- Final all-target offline server Clippy and app WASM checks passed.
- Chrome MCP and read-only SQL agree on the isolated demo's five entries:
  780 actual minutes, 720 billable and 60 non-billable; its 200-hour budget leaves
  187 hours. Missing cost rates show N/A rather than zero.
- Computed Chrome layouts at 320/390/768/1440px have no page or tile overflow.
  A desktop screenshot exposed the heading-font mismatch corrected above.
  Post-correction desktop and 390px screenshots were captured and inspected;
  all three titles use Instrument Sans and the tiles remain within the viewport.
  Evidence lives in the root checkout's ignored `.scratch/playwright-windows/`
  as `project-summary-{desktop,mobile}-verified.png`. This is a tile-level check,
  not full-page, keyboard, zoom or cross-screen acceptance.

This is not completion of the dashboard. Invoiced/uninvoiced tiles, breakdown
tabs, remaining chart controls/actions/exports, full keyboard/zoom/cross-screen
acceptance and Nix checks remain open. Clients and Settings/Workspace remain
required subsequent delivery slices. PR 208 remains draft.
