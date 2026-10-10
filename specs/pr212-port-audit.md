# PR #212 port audit

## Current checkpoint — 2026-10-10

Status: source port accounting complete; recovery PRs prepared, not merged.
This records a new source-to-master audit after the original
57 functional extractions merged. It is not permission-policy activation or
full Harvest parity acceptance. No merges are authorized in this phase.

- Source base: `9301112c6a02ae3c92716273534f38889db241d1`.
- Published #212: `db3935db364f2a8aa193f0e938ce40ecc01a2f92` (still open/draft).
- Preserved tracked unpublished snapshot: `d364270a6c7684457734dff66ff54f54e1066f83`.
- Comparison master: `5336fbb1cbe93b72be63eefcea21cf33fb69bc8b`.
- The six original untracked files are included; the original worktree is unchanged.
- Compared all 1,214 original changed paths plus unpublished work: 1,223 unique
  paths, 1,157 exact object/mode matches (including matching deletions), 66
  differences requiring explicit reconciliation. These are file counts, not a
  feature-completion percentage.
- GitHub inventory and Git ancestry identify 72 merged PRs since the source base,
  including all 57 functional extractions, specifications, Clients and shared CI.
  All 72 commits in `9301112c..5336fbb1` map to this inventory; none is omitted.
  The original source range contains 145 commits, accounted for by its net diff.
  63 published-head trees match their merge trees exactly. For the other nine,
  `git merge-tree --write-tree <head> <merge>^` reconstructs the exact merge tree
  without conflicts: #210, #211, #214, #222, #225, #227, #239, #242 and #286.
  Their differences are intervening base changes, not dropped PR contents.

## Confirmed port gaps and deliveries

1. [#287](https://github.com/numtide/horae/pull/287): restore seven unpublished
   specification updates. FR-035/036 existed in the merged client contract but
   were missing from the main requirements, task list and operation matrix.
   Commit `b72aaad0`; formatting and whitespace checks pass. GitHub run
   `38062897977` and Nixbot build `644` passed on that head. Historical test
   process references are explicitly labelled, not revived. T238 stays open.
1. [#288](https://github.com/numtide/horae/pull/288), commit `d9682d7c`:
   restore #212's `CurrentUser` and
   `UserListItem` in `tests/scoped_reports_ui.rs`. The extracted test doubles
   used internal `User`, unlike the real endpoints. Two-line test-only change;
   native Nix tests pass: 191 core, 1,481 server and 334 auxiliary tests (2,006
   total), zero failures, 11 existing ignored measurements. All 20 Reports
   component tests pass. Formatting passes. No runtime payload or endpoint change.
1. Preserve the original [constitution proposal](015-scoped-permissions/constitution-proposal.md)
   for separate review. The original 1.1.0 text was only retained on #212;
   copying it as a clearly labelled proposal does not amend the authoritative
   1.0.0 constitution or approve migration.

## Reconciliation

- All original SQLx descriptor paths agree with the preserved unpublished
  snapshot. The three old client descriptors were replaced by #281's three
  descriptors; its reader, registered HTTP tests and contract are exact matches.
- #220 adds the separately developed #217 legacy report/invoice snapshot guard;
  #232 preserves its financial snapshot integration. The removed pool-level
  invoice wrapper is replaced by transaction-bound `fetch_invoice_from`.
- Snapshot race tests use PostgreSQL row versions instead of permission-schema
  revisions, preserving independent legacy authorization verification. The
  organization SHARE/NO KEY UPDATE lock relationship remains unchanged.
- #241's explicit transaction configuration preserves the same isolation and
  timeout statements as the original shared permission helper, without imposing
  an extraction dependency. #227 additionally fixes last-admin counts under
  inherited REPEATABLE READ and covers all four demotion/deactivation pairs.
- #263 explicitly rolls back a denied export before deferred pool cleanup;
  its new regression verifies immediate authority release. This correction must
  not be reverted while copying old source.
- Permission module/test ordering and crate-local DTO module visibility are
  extraction adaptations, not missing behavior. Preflight's unused-code
  annotation is narrowed to its actual unexposed entry point.
- The directory HTTP function moved intact to `scoped_directory::check`;
  time-entry invoice-redaction assertions moved to `time_entry_payload::check`.
  Both remain invoked by the registered-session integration test. Shared route
  discovery now matches exact endpoint names plus generated numeric suffixes.
- Import cancellation, permission-management race barriers and Timesheet
  cancellation tests now wait for the actual lock/rollback boundary. Export
  backend-close observation allows both existing five-second cleanup deadlines.
- #216 contributes Clients routes, validation, navigation protection and
  contextual project/invoice selection. It preserves permission editor and
  Timesheet navigation protection. Existing project permission wiring remains
  in the default route; client context only initializes the existing filter.
- #259 keeps historical Calendar editing available without current creation
  choices, with added move/resize/reorder regressions. #279 prevents individual
  task-name words breaking inside the scrolling table and adds a browser check.
- Browser registration retains every original suite and adds seven suites.
  Browser fixtures preserve subsequent Clients API/control names, stricter
  response readiness, exact source lists and fixture-owned receipt cleanup.
  Project detail now tests the actual combined page DTO; legacy reader behavior
  remains covered by native privacy and registered-session tests. This is source
  comparison, not a new visual acceptance claim.
- Expense specification #214 extends feature 011 without replacing its permission
  reconciliation. `.specify/feature.json` is a workflow cursor for feature 016,
  not feature 015 implementation; it must not be reset as part of a port.

## Verification and delivery boundary

- Difference accounting covers all 66 paths. Read-only checks confirm the moved
  directory body and invoice-redaction loop are identical, all 22 original HTTP
  check calls remain among the current 25, and all 27 original browser suites
  remain among the current 34. All 238 source task IDs and checkbox states are
  preserved by #287. The archived proposal matches the original source bytes.
- The #288 native check completed successfully with disposable PostgreSQL and
  the committed offline SQLx cache. Derivation:
  `/nix/store/fzd2llwanww15mjdiv53018hg8mw0pfn-horae-tests-0.1.0.drv`.
  The tested source file matches published `d9682d7c` byte for byte. This is not
  a new browser/VM acceptance claim; remote CI and human review remain pre-merge
  gates, and no auto-merge is enabled.
- This audit and the preserved proposal are delivered in
  [#289](https://github.com/numtide/horae/pull/289). All 66 differences have an
  explicit disposition; the confirmed missing content is carried by #287–#289.
  The three PRs are independent over master `5336fbb1`; they touch disjoint files.
  No additional already-written runtime feature was found missing from master.
- Next, assess unfinished original requirements. The original open tasks,
  canonical Clients writes, approvals/locks, financial report families and policy
  cutover are not implemented merely because their existing increments are ported.

## Difference accounting

Each path below differs from the preserved #212 source. Paths not listed have
identical Git objects/modes or matching deletions. The dispositions preserve
later accepted behavior instead of copying old files over it.

| Path | Disposition |
| --- | --- |
| `.specify/feature.json` | Workflow cursor; keep feature 016 selection |
| `.specify/memory/constitution.md` | Recover text as a proposal; do not ratify |
| `crates/core/src/lib.rs` | Clients module registration (#216) |
| `crates/horae/assets/css/horae.css` | Clients-only tokens and structural selectors (#216) |
| `crates/horae/assets/js/project-edit-navigation.js` | Clients guard added; permission/Timesheet guards preserved |
| `crates/horae/src/importers/harvest/engine_tests.rs` | Observe PostgreSQL session-lock release before import retry |
| `crates/horae/src/importers/harvest/engine_tests/csv_streaming.rs` | Observe PostgreSQL session-lock release before import retry |
| `crates/horae/src/models.rs` | Crate-local DTO module visibility |
| `crates/horae/src/pages.rs` | Clients legacy admin presentation helper |
| `crates/horae/src/pages/new_project.rs` | Client context; saved draft retains precedence |
| `crates/horae/src/pages/projects.rs` | Client context initializes existing scoped list/filter |
| `crates/horae/src/pages/tasks.rs` | Task-word wrapping correction with browser regression |
| `crates/horae/src/pages/timesheet.rs` | Historical drag editing independent of creation choices |
| `crates/horae/src/reports.rs` | Transaction-bound invoice reader replaces pool wrapper |
| `crates/horae/src/reports/limits/tests/time_authorization.rs` | Regression for immediate denied-export lock release |
| `crates/horae/src/reports/limits/time.rs` | Explicit rollback on denied export |
| `crates/horae/src/reports/streaming/database_tests/authorization.rs` | Allow both existing backend cleanup deadlines |
| `crates/horae/src/route.rs` | Clients contextual routes and navigation tests |
| `crates/horae/src/server_fns.rs` | Remove broad unused-code suppression |
| `crates/horae/src/server_fns/importers/authorization_tests.rs` | 25 checks retain original 22; exact route-name matching |
| `crates/horae/src/server_fns/importers/authorization_tests/scoped_time.rs` | Invoice-redaction loop moved verbatim to time_entry_payload |
| `crates/horae/src/server_fns/importers/authorization_tests/time_reports.rs` | Module/check ordering only |
| `crates/horae/src/server_fns/importers/authorization_tests/user_directory.rs` | Scoped-directory function body moved verbatim |
| `crates/horae/src/server_fns/invoices.rs` | Current-authority readers plus Clients filter |
| `crates/horae/src/server_fns/invoices/snapshot_tests/editor.rs` | Equivalent row-version/lock tests without permission-schema dependency |
| `crates/horae/src/server_fns/invoices/snapshot_tests/races.rs` | Equivalent row-version/lock tests without permission-schema dependency |
| `crates/horae/src/server_fns/invoices/snapshot_tests/settings.rs` | Equivalent row-version/lock tests without permission-schema dependency |
| `crates/horae/src/server_fns/permissions.rs` | Comments/test ordering; no command removal |
| `crates/horae/src/server_fns/permissions/preflight.rs` | Narrow unused-code annotation to unpublished entry point |
| `crates/horae/src/server_fns/permissions/tests/profiles.rs` | Module ordering only |
| `crates/horae/src/server_fns/permissions/tests/project_management.rs` | Deterministic actor/project lock barrier |
| `crates/horae/src/server_fns/permissions/tests/timesheet_context.rs` | Release test barrier before cancelled-query rollback |
| `crates/horae/src/server_fns/project_creation.rs` | Reuse equivalent client name/currency/rate validation |
| `crates/horae/src/server_fns/projects/privacy_tests.rs` | Additional Clients-filter privacy regression |
| `crates/horae/src/server_fns/reports.rs` | Legacy detail reader uses current-authority transaction |
| `crates/horae/src/server_fns/snapshot.rs` | Same organization SHARE lock without schema dependency |
| `crates/horae/src/server_fns/time_entries.rs` | Equivalent explicit isolation/timeouts; original helper retained |
| `crates/horae/src/server_fns/users.rs` | Recheck last-admin count at READ COMMITTED |
| `crates/horae/src/server_fns/users/tests/concurrency.rs` | Four last-admin race cases under inherited REPEATABLE READ |
| `crates/horae/tests/browser/action-errors.cjs` | Current Clients controls; retained mutation failure assertions |
| `crates/horae/tests/browser/editor-navigation.cjs` | Additional Clients guard and Timesheet scroll cases |
| `crates/horae/tests/browser/new-project-permissions.cjs` | Combined detail DTO; native legacy privacy coverage retained |
| `crates/horae/tests/browser/new-project.cjs` | Current Clients form, focus readiness, exact unfiltered ID set |
| `crates/horae/tests/browser/project-bulk-actions.cjs` | Server-backed assertion timeout only |
| `crates/horae/tests/browser/project-editor-permissions.cjs` | Remove only fixture-owned audit receipts |
| `crates/horae/tests/browser/project-read-permissions.cjs` | Avoid unrelated Timesheet mount; preserve failure diagnostics |
| `crates/horae/tests/browser/project-task-rates.cjs` | Preserve original error before bounded diagnostics |
| `crates/horae/tests/browser/projects-design.cjs` | Preserve original error before bounded diagnostics |
| `crates/horae/tests/browser/reports-permissions.cjs` | Server readiness and additional expand/collapse icon assertions |
| `crates/horae/tests/browser/responsive-layout.cjs` | Current Clients reader name |
| `crates/horae/tests/browser/run-design-checks.sh` | All 27 original suites retained; seven added |
| `crates/horae/tests/browser/shared-style-audit.cjs` | Current Clients reader name |
| `crates/horae/tests/browser/task-catalog.cjs` | Additional task-word wrapping regression |
| `crates/horae/tests/browser/task-read-permissions.cjs` | Observe identity request before navigation; bounded diagnostics |
| `crates/horae/tests/browser/timesheet-errors.cjs` | Historical Calendar move/resize/reorder without creation choices |
| `crates/horae/tests/detail_navigation.rs` | Additional Clients routes/fixtures; original tests retained |
| `crates/horae/tests/scoped_reports_ui.rs` | Original minimal identity types restored by #288; native check passes |
| `specs/011-new-project-screen/spec.md` | Expense budget extension (#214), not permission replacement |
| `specs/015-scoped-permissions/contracts/dependent-spec-reconciliation.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/contracts/operation-matrix.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/plan.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/progress.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/quickstart.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/research.md` | Historical labels and unratified governance wording |
| `specs/015-scoped-permissions/spec.md` | Recovered unpublished client requirements/history in #287 |
| `specs/015-scoped-permissions/tasks.md` | Recovered unpublished client requirements/history in #287 |

## Merged PR inventory

The merge commits below are ancestors of the comparison master. Full-tree equality
is a preservation check, not a substitute for reviewing source differences.

| PR | Delivery | Merge commit | Head tree equals merge |
| --- | --- | --- | --- |
| [#209](https://github.com/numtide/horae/pull/209) | Specify client list and detail workflows | `30dd25ba` | Yes |
| [#210](https://github.com/numtide/horae/pull/210) | Specify workspace administration workflows | `6115d238` | Reconcile base |
| [#211](https://github.com/numtide/horae/pull/211) | Specify personal settings journeys and decision gates | `17ab31a5` | Reconcile base |
| [#213](https://github.com/numtide/horae/pull/213) | Plan Harvest parity specification delivery | `e6de584f` | Yes |
| [#214](https://github.com/numtide/horae/pull/214) | Specify expense tracking and category workflows | `386cb586` | Reconcile base |
| [#215](https://github.com/numtide/horae/pull/215) | Stabilize project picker checks after viewport changes | `e1400c16` | Yes |
| [#216](https://github.com/numtide/horae/pull/216) | Add the Clients MVP workflow | `02f7b58a` | Yes |
| [#218](https://github.com/numtide/horae/pull/218) | Document permission PR separation and recovery inventory | `5336fbb1` | Yes |
| [#219](https://github.com/numtide/horae/pull/219) | Extract pure permission scope and catalog rules | `6d5dd32f` | Yes |
| [#220](https://github.com/numtide/horae/pull/220) | Recheck report and invoice authority in consistent snapshots | `7b85d2c3` | Yes |
| [#221](https://github.com/numtide/horae/pull/221) | Extract permission rules for rates, management and approvals | `7212fc89` | Yes |
| [#222](https://github.com/numtide/horae/pull/222) | Add inactive permission profile storage | `c4aa27c1` | Reconcile base |
| [#223](https://github.com/numtide/horae/pull/223) | Prevent lock inversion during import report conversion | `d79a7d4e` | Yes |
| [#224](https://github.com/numtide/horae/pull/224) | Recover interrupted import sessions before releasing reservations | `2d3e6721` | Yes |
| [#225](https://github.com/numtide/horae/pull/225) | Recheck current authority before saving organization branding | `eb8eae55` | Reconcile base |
| [#226](https://github.com/numtide/horae/pull/226) | Extract internal reusable permission profile commands | `ff3fde10` | Yes |
| [#227](https://github.com/numtide/horae/pull/227) | Recheck administrator authority during user changes | `3df38af8` | Reconcile base |
| [#228](https://github.com/numtide/horae/pull/228) | Coordinate project access changes before locking resources | `cea013c9` | Yes |
| [#230](https://github.com/numtide/horae/pull/230) | flake: remove darwin support | `1b8fa4f5` | Yes |
| [#231](https://github.com/numtide/horae/pull/231) | Prepare durable CSV batches before opening transactions | `2b441a92` | Yes |
| [#232](https://github.com/numtide/horae/pull/232) | Recheck current authority for financial snapshot readers | `483be1cd` | Yes |
| [#233](https://github.com/numtide/horae/pull/233) | Serialize invoice writes before administrator revocation | `82c11cda` | Yes |
| [#234](https://github.com/numtide/horae/pull/234) | Extract atomic person permission profile commands | `b50ec27e` | Yes |
| [#235](https://github.com/numtide/horae/pull/235) | Revalidate administrator authority for Harvest connection changes | `82e85160` | Yes |
| [#236](https://github.com/numtide/horae/pull/236) | Revalidate import job commands and error downloads | `aa8e9ef9` | Yes |
| [#237](https://github.com/numtide/horae/pull/237) | Retain the original requester of import jobs | `81194134` | Yes |
| [#238](https://github.com/numtide/horae/pull/238) | Revalidate budget email recipients before delivery | `b60ed44f` | Yes |
| [#239](https://github.com/numtide/horae/pull/239) | Restrict approval changes to the current organization | `27b27ad5` | Reconcile base |
| [#240](https://github.com/numtide/horae/pull/240) | Limit identity responses and resolve approval labels directly | `1998388f` | Yes |
| [#241](https://github.com/numtide/horae/pull/241) | Fence time-entry writes against account deactivation | `919b1e7b` | Yes |
| [#242](https://github.com/numtide/horae/pull/242) | Keep invoice identities out of time-entry responses | `40434acd` | Reconcile base |
| [#243](https://github.com/numtide/horae/pull/243) | Preserve scoped project manager delegation | `bf144b7f` | Yes |
| [#244](https://github.com/numtide/horae/pull/244) | Show current own permissions in Settings | `6c301447` | Yes |
| [#245](https://github.com/numtide/horae/pull/245) | Add administrator permission change history | `f2389524` | Yes |
| [#246](https://github.com/numtide/horae/pull/246) | Add read-only legacy permission diagnostics | `5b7a1bc4` | Yes |
| [#247](https://github.com/numtide/horae/pull/247) | Recheck access before delivering spreadsheets and invoice PDFs | `34ef9f81` | Yes |
| [#248](https://github.com/numtide/horae/pull/248) | Separate scoped permission specifications and verification history | `036aeebf` | Yes |
| [#249](https://github.com/numtide/horae/pull/249) | Recheck current access during CSV downloads | `6d19a88f` | Yes |
| [#250](https://github.com/numtide/horae/pull/250) | Expose requester-bound permission editor operations | `a06ac539` | Yes |
| [#251](https://github.com/numtide/horae/pull/251) | Use ThinLTO and parallel code generation for release builds | `35dc414f` | Yes |
| [#252](https://github.com/numtide/horae/pull/252) | Cache Rust dependencies with Crane for builds and checks | `ed558f62` | Yes |
| [#253](https://github.com/numtide/horae/pull/253) | Expose scoped people directory reads | `e41250e1` | Yes |
| [#254](https://github.com/numtide/horae/pull/254) | Expose identity-only project team choices | `0d3b0ef0` | Yes |
| [#255](https://github.com/numtide/horae/pull/255) | Add scoped time-entry reads without financial metadata | `bbdfb8f7` | Yes |
| [#256](https://github.com/numtide/horae/pull/256) | Add scoped Timesheet person discovery | `2165320f` | Yes |
| [#257](https://github.com/numtide/horae/pull/257) | Bind Timesheet page reads to requester and subject | `888240f6` | Yes |
| [#258](https://github.com/numtide/horae/pull/258) | Extract person-bound Timesheet commands | `5e3b241f` | Yes |
| [#259](https://github.com/numtide/horae/pull/259) | Bind Timesheet views and commands to the selected person | `03dc0bbb` | Yes |
| [#260](https://github.com/numtide/horae/pull/260) | Add scoped People directory and permission editor UI | `b114011c` | Yes |
| [#261](https://github.com/numtide/horae/pull/261) | Add scoped detailed time reports with full-period totals | `94585bf2` | Yes |
| [#262](https://github.com/numtide/horae/pull/262) | Add scoped time report grouping with exact totals | `2216446f` | Yes |
| [#263](https://github.com/numtide/horae/pull/263) | Apply scoped authorization to time report spreadsheets | `4c824d8b` | Yes |
| [#264](https://github.com/numtide/horae/pull/264) | Scope streamed time exports and shared download filters | `7f9d42c8` | Yes |
| [#265](https://github.com/numtide/horae/pull/265) | Bind time report downloads to the current permission mode | `f48d674e` | Yes |
| [#266](https://github.com/numtide/horae/pull/266) | Export scoped time groups as bounded spreadsheets and CSV streams | `29242324` | Yes |
| [#267](https://github.com/numtide/horae/pull/267) | Preserve active-project and billability filters in time downloads | `80014914` | Yes |
| [#268](https://github.com/numtide/horae/pull/268) | Connect scoped time reports to the Reports screen | `7abdb3a8` | Yes |
| [#269](https://github.com/numtide/horae/pull/269) | Preserve protected project fields in scoped editing | `21452bdd` | Yes |
| [#270](https://github.com/numtide/horae/pull/270) | Scope project overview and detail reads to current permissions | `40883344` | Yes |
| [#271](https://github.com/numtide/horae/pull/271) | Scope project exports and revalidate monetary access | `37370514` | Yes |
| [#272](https://github.com/numtide/horae/pull/272) | Scope Harvest-compatible project reads and counts | `a2102fa6` | Yes |
| [#273](https://github.com/numtide/horae/pull/273) | Separate task catalog access from tracking identities | `aadabd4c` | Yes |
| [#274](https://github.com/numtide/horae/pull/274) | Wait for emulated VMs before connecting the test shell | `8b3cc257` | Yes |
| [#275](https://github.com/numtide/horae/pull/275) | Enforce task creation and explicit rate-edit permissions | `6b5691c1` | Yes |
| [#276](https://github.com/numtide/horae/pull/276) | Preserve task archive state across projects and imports | `da8c19d8` | Yes |
| [#277](https://github.com/numtide/horae/pull/277) | Authorize project task links and preserve rate currency | `909b8a7f` | Yes |
| [#278](https://github.com/numtide/horae/pull/278) | Create tasks and initial rates in one authorized transaction | `9b2ef230` | Yes |
| [#279](https://github.com/numtide/horae/pull/279) | Expose the permission-aware task catalog and editor | `2ae55815` | Yes |
| [#280](https://github.com/numtide/horae/pull/280) | Expose project task archive and restore controls | `176ff152` | Yes |
| [#281](https://github.com/numtide/horae/pull/281) | Enforce scoped reads for Harvest-compatible clients | `81b7c5cf` | Yes |
| [#282](https://github.com/numtide/horae/pull/282) | Stabilize import cancellation and emulated recovery checks | `c9ea1f39` | Yes |
| [#286](https://github.com/numtide/horae/pull/286) | Stabilize CSV transport and Linux import checks | `f29e72b4` | Reconcile base |
