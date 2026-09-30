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

## Current gap review — 2026-09-30

The implementation is still on `feat/project-dashboard`, not accepted as a whole.
The following are current source findings, not additional exclusions from scope:

- **Project Detail:** grouped chart-week controls, applicable hour-budget
  references, the current-week marker, contextual reports and selected-period
  exports, chart month labels and weekly bar emphasis are implemented and tested
  below. The uninvoiced tile, invoice
  subject/payment-date persistence and recent entries remain incomplete. Additional
  action semantics are still the open decisions recorded in `spec.md`.
- **Clients:** `ClientList` has basic CRUD and active/inactive rows, but no designed
  search/filter/selection toolbar. `ClientDetail` still only renders its UUID.
  The project's client link therefore does not yet complete the intended journey.
  The independent `feat/clients-design` branch now contains the feature 012 draft
  specification and Harvest comparison in PR #209. Contact/payment-term scope
  and client archive/reactivation policy await user decisions; no client code is
  implemented by that specification.
- **Personal Settings:** `Settings` only implements theme and plugin display;
  profile, assigned people/projects, actual permissions and notifications are not
  supplied by this page.
- **Workspace:** routes currently provide People and Harvest Importers under the
  admin shell. The requested general preferences, invitations, permission matrix,
  backup/export and audit surfaces still require their specifications and work.

Harvest's inspected Tasks links preserve project/task/report dates. Horae's
contextual Reports navigation now preserves those dimensions and its existing
manager-only boundary; project-progress access does not grant access to people's
detailed entries. See `harvest-reference.md` for the observed behavior and
limitations of the browser evidence.

Next independent dashboard work: Tasks/Team/Invoices and full-run acceptance gaps.
Billing policy decisions must
be resolved before representing unknown external billing as a known amount. The
Clients and Settings/Workspace slices remain required, not replaced by this work.

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

## Reciprocal task and team breakdowns — 2026-09-29

Tasks and Team use the same authorized task/person intersections and the
activity chart's reporting interval. Both directions retain enabled zero-time
entities and historical contributors, sort exact minutes with stable identity
ties, expand into reciprocal child rows and reconcile their totals. Costs use
per-entry rounding in workspace currency; missing rates remain unknown and
private costs are absent from unauthorized payloads. Queries recheck current
progress permissions in a read-only repeatable-read transaction. Explicit
5,000-group/contributor limits reject excessive results rather than truncating.

Existing task/team management remains available in native disclosures. Successful
changes refresh both breakdowns and summary tiles. Loading and failed reads clear
previous-project/period rows and provide a retry. Tabs and sorting reuse the
loaded projection instead of making another request.

Evidence collected for this increment:

- Core: 143 tests passed, including three reciprocal aggregation tests covering
  exact reconciliation, missing/private costs, invalid quantities and overflow.
- Projects: 78 PostgreSQL tests passed, including five new breakdown tests for
  inclusive intervals, zero-time/inactive contributors, permissions, foreign
  organizations, multiple tasks/people, workspace currency and excessive groups.
- Detail navigation/rendering: 44 tests passed after adding the shared Avatar,
  including pending/failed route changes without stale breakdown rows.
- SQLx metadata regenerated with server/all-targets and incremental compilation
  disabled; existing tracked query metadata retained. No migration is added.
- Chrome MCP DOM-event checks verified reciprocal expansion, ascending hours
  sorting, tab selection and ArrowRight focus movement without another request.
  Native Playwright click timed out waiting for stability with Chrome hidden;
  these checks do not establish native pointer/keyboard acceptance.
- Read-only demo SQL and Chrome agree: Development 390, Design 330 and Meetings
  60 minutes, totaling 780 (13h). September filters to 150, 210 and 60 minutes,
  totaling 420 (7h) in the chart and both tabs. Switching tabs preserves the
  selected interval; the interval change makes one new breakdown request.
- An initial mobile screenshot exposed an undefined `sr-only` class and unpadded
  footer cells. Replace the caption with a table accessible name and use existing
  spacing/border utilities for semantic row headers and footer cells. Shared CSS
  defaults are unchanged.
- The correction's rendering regression failed before the fix and passes after
  it; all 44 detail tests pass on the final markup. Core tests, all-target offline
  server Clippy, WASM compilation and formatting also pass.
- A deliberately held September breakdown request removes the previous table
  and shows loading; releasing it shows the correct 7-hour period. The original
  browser fetch function is restored after the check.
- Computed layouts at 320/390/768/1440px show no page overflow. At 320px the table
  scrolls inside its existing container; controls remain at least 44px tall.
  Post-correction desktop/mobile screenshots are saved under the root checkout's
  ignored `.scratch/playwright-windows/project-breakdown-{desktop,mobile}-verified.png`.
  Native pointer/keyboard, 200% zoom and cross-screen acceptance remain open.

F016 remains open until the final browser acceptance is complete. The invoice
tab, financial tiles, exports and remaining dashboard requirements are still
required, followed by Clients and Settings/Workspace. This is not final visual
acceptance or permission to merge PR 208.

## Invoice history increment

Add project-attributable invoice history from stored after-discount, before-tax
line amounts, including time and fee sources. A shared invoice contributes only
this project's lines. Keep currencies and lifecycle states separate; exclude void
invoices from totals without losing void/reissue history. Current rate or project
status changes do not reprice historical invoice lines.

The read checks current organization-manager/admin authority and project access
within a read-only repeatable-read transaction. Foreign/inactive/unauthorized
viewers are rejected, and more than 5,000 invoices fails without partial totals.
The UI adds a permission-gated Invoices tab, real counts and invoice links, loading
and retry states, and all-history labels independent of the chart's work dates.
It reuses the shared table, status badges and utilities without changing CSS.

Evidence so far: all 146 core tests pass; all four new PostgreSQL billing tests
pass (mixed-project discounts and fees, void/reissue and currencies, current
permissions and foreign organizations, excessive history). SQLx metadata was
regenerated for server/all-targets with incremental compilation disabled; no
tracked metadata was deleted. No migration or business-data mutation was made.

Invoice subject and payment date are not stored yet and are explicitly shown as
not recorded instead of inferred. Contextual creation, financial tiles, the
metadata decision and full browser acceptance remain required. Chrome shows the
new tab and its real empty count; the native click again timed out waiting for
stability. This is not native interaction acceptance or completion of F019.

The final detail suite passes all 47 tests, including currency-labelled invoice
links, HTML escaping, empty history, role-gated requests and switching back from
Invoices without changing the work period. All 82 Projects PostgreSQL tests pass.
Chrome MCP's DOM-event check opens the real empty invoice history, preserves the
activity period and makes no additional invoice request. A first attempt crossed
a development reload and was inconclusive; the repeated single-evaluation check
above succeeded. Populated-history browser and native pointer checks remain open.
Offline server Clippy (all targets, warnings denied), the application WASM check,
`nix fmt` and whitespace checks also pass on this increment.

## Contextual invoice preparation

The Invoices tab now opens `/projects/:id/invoices/new`, reusing the existing
preparation form and account-scoped recovery gate. The gate completes before
project identity is requested. Client and project are fixed to the authorized
source; preview, source refresh and generation retain the single-project filter.
The generic invoice client/project picker is unchanged. Dates remain explicit,
and the initial prefilled context is clean rather than an unsaved user edit.
Cancel returns to the project, with the existing dirty/pending navigation guard;
successful generation opens the actual invoice. Route keys reset source state.

Verification:

- The new route/navigation test failed before implementation; all four route
  tests and 48 detail/rendering tests now pass. A direct-route test verifies that
  sources are not requested while recovery/authorization is unresolved.
- Offline all-target server Clippy with warnings denied and the WASM check pass.
  No SQL query, cache, schema, billing mutation API or shared CSS was changed.
- The new `project-invoice` browser suite runs against the runner's disposable
  database. Two projects share a client; preview and generation select only the
  originating project, and the other project's entry remains open. It checks
  clean and dirty cancellation without mutation, failed preview/retry, actual
  draft creation, populated history navigation and 320/390/768/1440px form bounds.
- The suite also verifies identity-load failure/retry and revoked billing access:
  a member cannot load source identity, see the form or create another invoice.
- The full existing `invoice-preparation` browser suite passes, including partial
  fees, excess confirmation, source refresh, lost responses, reload recovery,
  account-scoped replay, storage failures and fee-balance permissions. Its old
  literal `Project` heading assertion was updated to the actual project identity.
- Windows Chrome MCP opens the new route with the expected Acme client/project
  and clean state. Financial browser mutations occur only in the disposable
  Chromium runner, not in the live preview, imported data or Harvest.
- The complete `new-project` browser suite also passes: responsive validation,
  autosave/recovery, finalized budgets, reports/exports and invoice defaults.
  Its detail assertions now use the real heading, expand Project information
  and distinguish saved tags from the project-type badge. No assertions were
  removed and no editor behavior was changed to satisfy obsolete selectors.

This finishes F020, not dashboard acceptance. Invoiced/uninvoiced tiles, metadata,
additional action decisions, exports and remaining visual/accessibility coverage
are still required, followed by Clients and Settings/Workspace.

## Invoiced summary

The fourth summary tile shares the keyed page's authorized invoice response with
the history table. It shows stored project-attributed amounts after discounts and
before tax, separate currencies and non-void invoice counts. Labels make lifetime
scope, draft inclusion and void exclusion explicit. Missing, private, pending and
failed results do not masquerade as zero. A work-summary failure does not hide
the invoiced tile; switching projects clears previous amounts.

Verification:

- All 52 detail navigation/rendering tests pass, including pending/failed route
  transitions, independent summary failure, currency separation and void counts.
- Offline server Clippy (all targets, warnings denied) and WASM compilation pass.
- The disposable Chromium `project-invoice` suite passes with native clicks:
  generated draft amounts reconcile with the tile and history; both use one
  request. An aborted read hides stale amounts, and a single retry restores both
  views. Tile and page bounds pass at 320/390/768/1440px.
- No SQL, schema, shared CSS, financial mutation API or dependency changed.

This completes F021 only. Uninvoiced amounts, invoice metadata, remaining chart
and action/export requirements and full acceptance are still open. Clients and
Settings/Workspace remain required.

## Weekly chart-window navigation

The chart now has previous/next week controls and a return to this week action.
It shows up to 26 configured weeks, clipped to the selected reporting dates, and
does not navigate past the current week. This is an independent viewport over
the authorized response: Tasks/Team, summary values, the full weekly table and
their reporting interval remain unchanged. Changing the reporting period resets
the viewport to its last non-future week. The keyed project route resets it too.

Cumulative geometry carries earlier selected-period hours into the visible
window rather than restarting at zero; its filled area closes against the full
baseline. Empty intersections and zero-activity windows are labelled separately.
No extra query, schema, dependency or shared CSS rule is introduced.

Verification:

- TDD first failed the new window and cumulative-origin assertions; all 56
  detail/rendering tests pass after implementation. Includes configured weekdays,
  leap/year/date bounds, cropped cumulative origin and preservation of the full
  report table. The navigation fixture now uses today's date so it cannot age
  out of the current viewport.
- The disposable Chromium `project-activity` suite passes with 31 weeks of real
  authorized data and a Sunday workspace week start. Native keyboard/pointer
  actions move the chart, enforce the current-week limit and make no extra
  activity/breakdown/invoice reads. Custom reporting selection changes the report
  while moving its viewport does not. Returning through Projects clears the old
  period; an injected read failure hides the chart/controls and retry recovers.
- Page bounds and 44px control heights pass at 320/390/768/1440px, including a
  600px-tall viewport. The existing `project-invoice` browser suite also passes.
- Offline all-target server Clippy with warnings denied, the application WASM
  check and repository formatting pass.
- The browser runner includes the new suite in its default list. All fixture
  writes are confined to its disposable database; no Harvest or preview record
  is changed.

Calendar picking, applicable budget overlays, full chart visual/zoom acceptance
and the other dashboard requirements remain open. This increment completes F022,
not the Project Detail slice or Clients/Settings/Workspace.

## Chart calendar selection — 2026-09-30

The chart week label now opens the shared DatePicker in a native calendar
popover. It respects the configured week start, allows the whole current week
and disables later days/months. Reopening resets the browsed month to the selected
week. Selection changes only the chart viewport; it does not request data again
or change the reporting interval. Existing New Project and Timesheet callers
omit the optional upper bound and retain future-date selection.

The shared calendar now uses scalable tokens for its nominal 308px width, 30px
month buttons and 36px days. Heading/footer wrapping, constrained width and
responsive padding avoid enlarged-text clipping. The planning-date popover class
was renamed for shared use; its appearance and native positioning script are
reused, not duplicated. No schema, query, dependency or financial rule changed.

Verification:

- 58 detail-navigation/rendering tests and 45 New Project screen tests pass.
  Calendar tests verify inclusive day bounds, next-month bounds and unrestricted
  defaults. Offline all-target server Clippy with warnings denied and the WASM
  application check pass.
- Native Chromium verifies opening and selection with Enter, Escape/focus return,
  outside dismissal, month reset, Sunday week starts, disabled future weeks and
  unchanged report/request counts. The calendar fits 320/390/768/1440px viewports
  at 600px height. Its inner surface, header, footer and buttons pass horizontal
  clipping checks at 200% text size at 320px and 1440px.
- The stronger inner-surface zoom assertion initially failed: outer-popover
  bounds alone had hidden clipped month controls. The shared dimensional fix
  makes that assertion pass without reducing text size.
- Computed layout/typography comparisons against the pre-zoom-fix build pass on
  eight other screens at 320/768/1440px. This is a sampled shared-style regression
  check, not complete visual acceptance of all those screens.
- The complete New Project browser suite passes after the activity suite,
  including calendar use, validation, recovery, finalized budgets and invoicing.
  The activity fixture now removes only its own disposable project/entries and
  restores the original week start; its previously retained project broke a
  subsequent suite's expected row count. No imported or Harvest records changed.

F023 is complete. The period-control presentation, report toolbar/links/exports,
budget overlays and other gaps above remain required. The PR stays in draft;
this is not completion of Project Detail or the full delivery.

## Tasks/Team reporting toolbar — 2026-09-30

The reporting menu now sits below the breakdown tabs with its period heading,
matching the handoff's placement. Tabs and controls stay mounted during reads;
pending/failed requests clear old rows/counts without resetting Team to Tasks.
Invoices remain independently accessible and hide the reporting toolbar without
resetting its period. The chart still uses that interval, but week navigation
remains local and performs no additional reads.

Verification:

- The placement assertion first failed because the toolbar was missing; all 59
  detail-navigation/rendering tests now pass, including the four breakdown tests.
- Native Chromium initially caught lost focus after cancelling custom dates.
  Apply and Cancel now return focus to the existing menu trigger. The final
  `project-activity` and `project-invoice` runner exits successfully.
- The activity suite verifies invalid dates do not dispatch requests, cancellation
  leaves All time selected, and delayed/failed reads preserve Team and the chosen
  period. Invoice history opens while the breakdown read is pending; retry
  restores the correctly filtered one-hour table. Invoice reads remain unchanged.
- Toolbar heading/menu reflow and internal clipping checks pass at 200% text
  size at 320/768/1440px, alongside the existing chart/calendar checks.
- Offline server Clippy on all targets with warnings denied, the application
  WASM check and `nix fmt` pass. No SQL, schema, dependencies or shared CSS changed.

F024 is complete. Scoped report links and exports are still required; this does
not complete Project Detail, Clients or Settings/Workspace. PR #208 remains draft.

## Task-scoped report foundation — 2026-09-30

Reports now accepts an optional task filter across grouped rows, detailed rows,
streamed CSV and bounded XLSX. The XLSX size preflight uses the same predicate as
the payload query. The Reports page exposes that filter through its existing
selector component; labels are associated with controls and unavailable selected
values do not silently display All. Catalog failures have a visible retry.

The current report/export manager gates, organization scope, rate calculations
and private-cost rules are unchanged. Malformed UUID arguments now return
`BAD_REQUEST` from the shared parser instead of an internal-server error.

Verification:

- The two new PostgreSQL tests initially failed: task filtering returned 210
  minutes instead of 90, and XLSX counted an oversized field in an excluded task.
  Both now pass. The full report/export module run passes 49 tests with two
  pre-existing manual measurement tests ignored. Two UUID parser tests pass.
- All 59 Project Detail navigation/rendering tests pass. All-target offline
  server Clippy with warnings denied and the application WASM check pass.
- Native Chromium verifies catalog failure/retry, project/task/date conjunction,
  grouped/detailed minute reconciliation, filtered CSV contents, real XLSX
  download, invalid task IDs (400), and revoked-role denial (403) for both report
  endpoints and both exports. Layout fits 320/768/1440px. Project activity and
  its independent chart/report/invoice state regression pass in the same runner.
- The full New Project browser suite also passes after those suites, including
  tagged report/export flows, validation, recovery, budgets and invoice defaults.
  The combined disposable-database runner exits successfully.
- SQLx metadata was regenerated with server/all-targets against the migrated
  local database. The three changed query snapshots are replaced. Ninety-two
  unchanged snapshots omitted by cached compilation were restored only after
  verifying their exact SQL still exists in the source; all-target offline
  compilation validates their continued availability. No migration was added.

F025 completes the task-filter prerequisite, not the contextual Project Detail
journey. F026 still requires reproducible report route context, dashboard links,
the export menu, PDF summary and bounded-download explanations. The broader
Project Detail/Clients/Settings/Workspace delivery and full Nix acceptance remain
open; the PR remains draft.

## All-time report period prerequisite

Reports and its CSV/XLSX downloads now support an actual unbounded work-date
period, without fabricated date limits. Both absent bounds mean all time; an
inclusive custom range must contain two valid, ordered dates. Core range parsing
is shared by report server functions, CSV streaming and XLSX preflight. Invalid
input returns 400 instead of widening the report or looking like a server error.

The Reports period selector retains the current-month default and preserves
custom dates when switching to All time. Date controls are disabled for all time;
invalid custom dates hide old rows and disable downloads. Existing form utilities
and controls are reused; no shared CSS, permissions, financial rules, migrations
or dependencies changed.

Evidence for this prerequisite:

- 149 core tests pass, including absent, inclusive, partial, empty, malformed and
  reversed period cases. The new parser tests first failed before implementation.
- All 52 report/export tests pass (two pre-existing manual measurements ignored).
  They cover 1990 and 2090 entries, project/task/person conjunction,
  foreign organizations, matching CSV/XLSX scope and invalid-period rejection.
- Native Chromium passes custom/all-time switching, invalid-date recovery,
  grouped/detailed/CSV reconciliation, XLSX download, active filters, 400 errors
  and revoked-role 403 responses. The Project Detail activity regression passes
  alongside it, including independent reporting/chart/invoice state.
- SQLx metadata was regenerated for server/all-targets: three changed query
  snapshots replaced and two test-query snapshots added; unrelated cache retained.
- Offline all-target server Clippy and the app WASM check pass. Full Nix
  acceptance remains pending.
- The complete New Project browser regression passes, including its tagged
  Reports/CSV/XLSX journey, draft recovery, validation and invoice defaults.
  Formatting and whitespace checks pass.

F026 is still open: contextual routes/links, the project export menu and PDF
summary are not supplied by the period prerequisite. Clients and
Settings/Workspace remain required delivery slices, and PR #208 remains draft.

## Contextual project report navigation — 2026-09-30

Task, teammate, reciprocal breakdown and total hours now open Detailed time with
the project, applicable task/person and all-time/custom period preselected. The
links reuse Reports and its current manager gate; zero hours and readers without
report authority retain plain text. No new CSS, dependencies, schema or report
endpoint is introduced.

Project URLs retain the reporting dates and selected tab, so reload and Back
restore the same view. Same-project query navigation updates the period heading
as well as the rows. Invalid or explicitly empty filter/date query values show an
error before mounting report resources instead of silently widening the scope.

Verification:

- All 61 detail navigation/rendering tests and 45 New Project screen tests pass.
- Native Chromium passes contextual task/person/reciprocal/total links, scoped
  CSV contents, reload/Back, invalid-context rejection and role gates. The
  activity, report-task-filter and project-invoice suites pass in the same
  disposable-database run, followed by the complete new-project, project-edit
  and new-project-navigation suites. The combined runner exits successfully.
- The combined run exposed a fixed three-project assumption in New Project's
  filter-reset test after invoice fixtures added two projects. It now asserts
  exactly one more project than the initial rendered list, retaining its
  specific created-project identity checks. The full combined rerun passes.
- The final WASM, all-target Clippy with warnings denied, formatting and
  whitespace checks pass after removing a redundant signal-init closure.
  Eight route tests and seven Reports UI tests pass, including malformed and
  empty context rejection. No shared CSS, SQLx cache or migration changed.
- Windows Chrome MCP confirms Harvest's nonzero task/person/total report links,
  plain zero-hour rows and tab query state. Its project Export menu exposes Excel
  and CSV, whereas Horae's handoff also requires PDF summary. Chrome reports a
  hidden document, so menu/tab inspection uses DOM events; this is not native
  click acceptance. No Harvest records were modified.
- Windows Chrome also renders Horae's project/task/all-time links after reloading
  the preview following its development rebuild. Native interaction acceptance
  comes from the isolated Chromium run above, not from hidden-window DOM clicks.

F026 remains incomplete: the project export menu and PDF summary are still
required. The whole Project Detail, Clients and Settings/Workspace objective is
not complete, and PR #208 remains draft.

## Project export menu and PDF work summary — 2026-09-30

The Tasks/Team toolbar now offers CSV, Excel and PDF summary for the selected
project and all-time/inclusive custom period. The existing report gate controls
visibility; pending/failed breakdowns disable export. A shared native dialog
explains each format's bounds before downloading without replacing the dashboard.
This dialog is an intentional handoff deviation to make FR-016's limits explicit.

CSV/Excel reuse the detailed report endpoints. The read-only PDF route reuses
current report/progress authorization, organization scope and the dashboard's
cost-safe breakdown. It includes task/team rows, historical/zero contributors,
exact minutes and formatted hours, and distinguishes restricted/missing costs.
Lifetime budgets, invoices, notes and rate values are not part of this summary.
Labels are literal Typst inputs. Existing fonts, render admission, deadline and
output cap are reused; label/row limits reject the complete document before
rendering rather than truncating it. Text limits are post-fetch checks, not SQL
preflight checks; metadata and work totals are separate reads.

Framework review: Menu, Modal and existing spacing/reflow utilities are reused;
no shared CSS/defaults, dependencies, migrations or SQL macro text changed.
Enlarged-text browser checks exposed overflowing modal actions and intrinsic
date-field widths in the existing fee panel. Local wrapping, padding and minimum
width utilities address both without changing shared form components. The browser
test excludes only Dioxus's injected development toast from layout measurement.

Verification:

- Three PDF renderer tests pass: deterministic output with literal user labels,
  row/text bounds without truncation, exact minutes, period and restricted costs.
- The query-validation unit test rejects partial/empty/reversed periods and
  unsupported filters while accepting absent all-time bounds.
- All 62 detail-navigation/rendering and 45 New Project screen tests pass on the
  final code, including export URL scope and unchanged shared input defaults.
- Native Chromium passes real CSV/Excel/PDF downloads for all-time and one-day
  scopes, keyboard/focus, 320/390/768/1440px layout, 200% text, short-viewport
  download access, foreign projects, invalid queries, oversize fields and revoked
  report authority. The suite is included in the default browser runner.
- The same final run also passes project-report-links, project-activity and
  project-invoice against one disposable database. Imported data is unchanged.
- Final all-target server Clippy with warnings denied, WASM compilation and
  formatting pass. No schema or SQLx metadata changed in this increment.

F026 is implemented and its focused journeys are verified. This is not full
Project Detail acceptance: the remaining dashboard gaps and all Clients and
Settings/Workspace delivery slices remain open; PR #208 stays draft.

## Grouped chart navigation and hour-budget reference — 2026-09-30

Previous/calendar/next now share the existing pager frame, with a readable week
range and a separate return-to-current-week action. The shared calendar and its
configured-week/future bounds are unchanged. The central control uses the system's
inset focus token locally so the frame cannot crop its keyboard indicator.

The cumulative chart shares the summary tiles' existing authorized resource.
Complete hour allocations supply a labelled dashed reference and scale; money,
missing allocations, loading/errors and incompatible periods do not. Lifetime
requires all-time reporting, and monthly requires the exact month returned by the
current summary. Combined task/person allowances are labelled explicitly. The
reference is a current allowance, not configured rounded consumption; actual
tracked minutes and the budget tile's accounting remain unchanged. Weekly charts
do not inherit the reference or cumulative scale.

Verification:

- Native Chromium passes one shared summary request, exact budget/actual totals,
  cumulative/weekly switching, matching monthly/lifetime scope, loading/error/retry,
  money/permission gates, keyboard focus, 320/390/768/1440px widths and 200% text.
- The same final disposable-database run passes project-activity, project-exports
  and project-invoice, including calendar focus/dismissal, report independence,
  actual downloads and invoice preparation. The new budget suite runs by default.
- A focused keyboard assertion first reproduced the central control's outward
  focus ring; after the local inset correction, the full combined run passes.
- Final detail-navigation/rendering (66) and New Project screen (45) tests pass,
  as do all-target server Clippy with warnings denied, WASM and formatting checks.
- Framework review adds only project-scoped SVG paint and focus rules using
  existing tokens. No shared selector/default, dependency, SQL, wire type,
  migration or imported record changes.

F027 is implemented. This does not complete Project Detail visual acceptance:
the current-week highlight and broader chart fidelity still need review, alongside
the previously recorded billing/action gaps and full application/Nix acceptance.
Clients and Settings/Workspace remain required work; PR #208 stays draft.

## Current-week chart marker — 2026-09-30

The chart now highlights the current configured week in both modes using the
same integer bucket geometry as the plotted data. Partial buckets and year
crossings retain their real position. Historical windows, old reporting periods
and empty series do not label the latest recorded week as current. Tracked totals,
report scope and request counts are unchanged; the accessible chart description
also identifies the highlight.

The marker uses existing primary tokens and a local badge variation. Unlike the
prototype's overhanging centered label, the badge stays inside the trailing plot
edge. Its decorative dot is omitted locally and its background is opaque over
the series. Enlarged-text inspection exposed character-by-character badge wrapping
and overflowing endpoint dates; existing minimum-width/wrapping utilities fix
both without changing shared defaults. The browser checks whole-word readability
as well as bounds, and captures can be saved with `HORAE_TEST_SCREENSHOT_DIR`.

Verification:

- The initial geometry test failed against the missing implementation; the final
  69 detail-navigation/rendering and 45 New Project screen tests pass. Coverage
  includes Monday/Sunday boundaries, clipped/year-crossing buckets, historical,
  future and empty series, zero-current-week work and unchanged totals in both modes.
- Native Chromium verifies configured-week positioning, marker removal/restoration
  through calendar/navigation and historical reports, unchanged request counts,
  keyboard behavior, mobile/desktop bounds and 200% text. Local desktop/mobile
  chart captures, including the corrected 200% mobile layout, were inspected.
- One invoice regression run timed out waiting for the page `load` event after
  navigation had committed; a complete rerun passed. The fault-injection reload
  now waits for DOM readiness and the existing awaited application error/retry
  assertions, without removing those assertions or changing billing behavior.
- The final combined run passes project-activity, project-chart-budget,
  project-exports and project-invoice after that readiness adjustment.
- All-target server Clippy with warnings denied and WASM compilation pass. The
  change adds no query, schema, dependency or shared style override.

F028 is implemented. Broader chart/dashboard visual acceptance, billing and action
decisions, full application/Nix acceptance, Clients and Settings/Workspace remain
open. This marker is not evidence that the full requested delivery is complete.

## Native lifecycle and complete-run regression — 2026-09-30

The new `project-lifecycle` browser suite uses only the runner's disposable
PostgreSQL database. Normal keyboard input exercises confirmation/cancel, failed
transport, a held retry, duplicate-submit/cancel guards, focus return,
archive/reload and manager reactivation. It compares full fixture records for
time, tasks, assignments, invoice header/lines and project configuration; only
status and one editor revision increment per transition may change. A member
cannot see management/invoice controls or replay a previously authorized mutation.
Arrow/Home/End tab navigation and confirmation bounds pass at 320/390/768/1440px,
including 200% text at desktop and mobile widths.

The initial complete browser run stopped in `action-errors`: one assertion
matched a raw detail URL without the router's empty query delimiter, and another
looked for assignment controls before opening the new team disclosure. Both
cases now pass using the real disclosure and exact origin/path/empty-query
checks. The same stale URL parsing in three New Project suites is updated,
retaining database identity and permission assertions. The subsequent complete
run passed those first nine suites, then exposed the same detail-URL assumption
in `project-task-rates`. That suite now normalizes exact project destinations,
matches project-row links with or without the empty query delimiter, and opens
task management before exercising its controls in every billing mode. These
changes retain the existing rate, currency, persistence and recovery assertions.

A later complete run passed the first sixteen suites, then exposed a fixture
isolation bug: `new-project-permissions` left a second active administrator in the
disposable database. Dev login could choose that account while subsequent tests
revoked the seeded administrator's role. The permissions suite now signs in
before creating that auxiliary administrator and deactivates only its own
auxiliary account in cleanup. Permission assertions remain intact; no application
authentication behavior changes.

The new lifecycle fixture initially used an incorrect Actions accessible name
and assumed the project edit revision never changed. It now uses the stable
trigger ID and asserts the expected revision increment. Viewport/focus scrolling
is allowed to settle for two animation frames before keyboard opening because
the shared menu intentionally dismisses on scrolling. No forced clicks or fixed
sleep delays are used. The focused lifecycle and action-error suites pass.

The 69 detail-navigation/rendering tests, 45 New Project screen tests, strict
all-target server Clippy, WASM check and full formatting check pass. No production
Rust, CSS, query, migration or dependency changes are involved. After the fixture
cleanup fix, the chained permissions, invoice, activity, task-filter, report-link,
export, chart-budget and lifecycle suites all pass, including the actual Cancel
button and revoked-member mutation replay. The complete 23-suite rerun and final
Nix acceptance remain pending at this checkpoint.

## Chart month labels and weekly emphasis — 2026-09-30

The chart labels the actual visible months, including the first year and January
year changes. Narrow containers retain the first/last month; exact range dates
remain in the chart description and weekly data table. Three weekly bar groups
preserve the existing integer geometry and emphasize the latest visible buckets,
not a fabricated current week. The actual current-week marker stays independent.
Project-only color tokens preserve the handoff's dark palette and provide distinct
light-theme shades without changing shared primary/pine colors.

Verification: 74 detail and 45 New Project tests, strict all-target server Clippy,
WASM compilation and formatting pass. Chromium passes activity/navigation,
chart-budget, project-invoice and invoice-preparation suites. Month bounds, both
themes, mobile/desktop, enlarged text, unchanged totals/reads and historical
navigation are covered. Desktop and enlarged-mobile captures were inspected.
The budget suite now selects the summary loading status explicitly: the invoiced
tile can legitimately be loading at the same time.

An earlier complete run used a live preview build while its watcher replaced
assets, causing a WASM HTTP loading failure. Browser verification now uses a
frozen server/public copy with a disposable database; the user preview is left
running. The full 23-suite rerun is pending, separately tracked by F029. Broader
dashboard acceptance, billing/action decisions, Clients and Settings/Workspace
remain open.

The frozen-build complete rerun subsequently stopped in `action-errors` with an
unhandled response-wait timeout during navigation. The response and DOM-ready
navigation are now awaited together, preserving the successful-response check
and ensuring navigation failures enter the scenario's diagnostic handler. All
three action-error scenarios pass in isolation; a new full rerun is still needed
before F029 can be marked complete.

## Tasks/Team native acceptance — 2026-09-30

The report-link browser suite now creates its own three-task/two-person project,
including equal-hour tasks. Both tabs reconcile their parent totals with SQL,
show matching row counts, preserve deterministic ties through ascending/descending
sorts and expand/collapse reciprocal rows through native keyboard input. Child
minutes reconcile exactly with their parent. Existing period, reciprocal report,
download, navigation and permission checks remain in place. F016 is verified.

The first attempt depended on the seed having multiple contributors, which it
does not; the fixture now owns its contributor and removes its time, task links,
project and user in foreign-key order. It passes followed by project-exports,
project-chart-budget and project-lifecycle on the frozen build and disposable DB.
No production code or real/imported records changed.

The complete run passed its first nineteen suites before the new fixture's
precondition failed in project-report-links. The four final suites pass together
after the fixture correction, but this is not yet one complete 23-suite pass.
F029 and full Nix acceptance remain open.

## Complete browser regression — 2026-09-30

The default runner completed all 23 suites successfully against the frozen
server/public build and a fresh disposable database, including the corrected
Tasks/Team fixture. The run covers shared styles/layout/menu/navigation, bulk
actions/recovery, action failures, the full New Project/edit/permission/transport
flows, invoice preparation, project invoice/activity, report filters/links,
exports, chart budgets and project lifecycle. F029 is complete.

This supersedes the earlier partial-run checkpoints, not the wider feature gates.
Full Nix acceptance, remaining dashboard billing/action decisions, Clients and
Personal Settings/Workspace are still required. The separate
`feat/workspace-design` branch now contains a feature 013 specification draft;
it adds no application behavior and does not resolve the open policy decisions.

## Offline chart label regression — 2026-09-30

PR #208's Flake Check failed in `project-activity` at 320px with 200% text.
The previous native pass did not reproduce CI's fallback-font environment.
Blocking Google Fonts and using only CI's DejaVu font directory reproduces the
failure: the plot is 120px wide, the badge is 88px, and `week` spans two lines.
The temporary Fontconfig file omits host configuration and profile font paths.

The chart overlay now uses existing `px-1 py-2` utilities, retaining vertical
spacing while giving the label more horizontal room. Shared badge CSS, font
sizes, chart data and controls are unchanged. The browser test blocks remote
fonts and reports each word's rectangles without weakening its readability check.

The fullstack build and strict all-target server Clippy pass. With the isolated
fallback fonts, `project-activity`, `project-chart-budget`, `project-invoice` and
`project-lifecycle` pass together. Desktop and 320px/200%-text chart captures were
inspected: words remain whole and inside the plot. Evidence is in the worktree's
ignored `.scratch/ci-label/` directory. F031 is complete.

The complete runner passed its first six suites, then remained in `action-errors`
after its client/archive checks. Its live process and disposable Chromium were
confirmed before closing only that browser after several minutes without progress.
The runner exited unsuccessfully and shut down its disposable PostgreSQL. The
cleanup error is not proof of the original wait's cause; F032 tracks investigation
and a fresh complete run. This revision is not a 23-suite pass. Full Nix acceptance
and the remaining dashboard and Clients/Settings/Workspace requirements stay open.

## Bounded browser regression — 2026-09-30

The original `action-errors` wait did not recur when running that suite alone
or after the six preceding suites with Playwright API tracing. All three action
failure/recovery scenarios passed without changing their assertions. A subsequent
full attempt failed earlier, while `responsive-layout` waited for the login page's
load event; that separate timeout does not establish the original wait's cause.

The runner now prints each suite's name and limits it to 15 minutes, terminating
the process group and escalating after ten seconds if needed. A timeout is a
failed run, not a retry or success. This bounds waits such as Playwright's
`response.finished()` without removing readiness or recovery checks. Coreutils
is already an explicit browser-check input; no dependency or application code
changes are needed.

The fresh default run completed all 23 suites with exit status zero against the
same frozen fullstack build, isolated DejaVu fallback fonts and a new disposable
database. Assignment recovery, enlarged chart labels and all existing assertions
remain covered. Shell syntax, Nix formatting and whitespace checks pass; a short
timeout probe confirms exit status 124 rather than success. Standalone ShellCheck
was unavailable in the dev shell, so no standalone ShellCheck pass is claimed.

F032 is complete as investigation, bounded execution and fresh regression
evidence, not a proven root-cause fix for the unreproduced wait. Full Nix CI and
the remaining feature requirements are still open; no new permission policy has
been activated, and no imported data or preview instance was changed.

## Task/team management belongs in the editor — 2026-09-30

The detail handoff uses Tasks/Team for reporting; task and team configuration
belongs in Edit project. Removed the legacy management accordions, their local
state, redundant catalog/assignment reads and the unused breakdown refresh input.
The shared editor, reporting tabs, fee balances, server APIs and authorization
rules remain unchanged. No shared CSS, migrations or SQL queries changed.

The new browser regression first failed against the old build because both
accordions were present. Coverage now exercises the detail-to-editor link,
task/team removal and re-addition, failed-save preservation and retry, and task
rates through the shared editor rather than the retired controls. Navigation
tests use the reporting projection and still reject stale project data.

Server/WASM builds, strict all-target server Clippy and all 74 tests in
`detail_navigation` pass. Browser verification uses the runner's disposable
database; the imported account and database are not test fixtures. An intermediate
browser run was invalidated by concurrent bundle regeneration and is not counted
as passing evidence.

The final stable-bundle run passed `project-edit`, `project-task-rates`,
`action-errors`, `project-report-links` and `project-lifecycle` together with exit
status zero. This includes three editor widths, enlarged-text lifecycle checks,
exact stored rates, task/team persistence and error recovery. F033 is complete;
this is a five-suite targeted regression, not a new full 23-suite or Nix CI pass.
