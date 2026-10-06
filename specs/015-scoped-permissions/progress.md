# Scoped permissions investigation progress

## 2026-10-06 — Project task lifecycle consumer

- Previous turn was a status-only answer (no progress). Revalidated `dcadcee`
  and the remaining T234 gap; no live verifier was assumed. Spec Kit prerequisites
  identify feature 015. The seven local checklists pass 7/7; full requirements
  remain 12/16, under the existing authorization for closed increments.
- Extended the disposable `project-editor-permissions` browser fixture with
  retained active, project-archived and globally archived tasks. Corrected two
  fixture mistakes (the association has a composite key, and cleanup must remove
  its rows before the project). Corrected RED `9221` exited 1 on the missing
  Restore control; existing ordinary-save preservation passed first.
- The editor now stages project-link activity separately from its saved form,
  includes only changed retained links in its existing receipt-bound save,
  tracks dirty/cancel state and preserves the original request for uncertain
  retries. Global archive remains distinct, and new unsaved rows still remove.
  Archived rows disable field editing; bulk billable controls skip them.
  No draft payload, server contract, SQL, dependency or shared CSS changed.

Next: verify the current build and extended browser regression, then strict
native/WASM lint and focused tests. T234/T230 remain open pending evidence;
full permissions integration, review and activation gates remain unchanged.

- Both build targets passed in `89298`. Its browser run passed lifecycle,
  undo/cancel, lost-ack replay and revoked save, then rejected the old hours-only
  test because the added fixture contained dormant task money: the server
  correctly requires financial authority before clearing those values. Changed
  the added task fixture to integer hour budgets, keeping the existing dormant
  project-money test intact; no authorization predicate was weakened.
- Corrected `7595` exited 0 across `project-editor-permissions`, `project-edit`,
  `new-project-task-errors` and `project-task-rates`. Native/all-target and WASM
  strict lint `22206` exited 0. Desktop dark/mobile light captures were inspected
  in one batch; no further visual edit. This is Linux Chromium, not Windows MCP.
- Added a real running timer before the archive save: require rejection with
  input and lifecycle intent preserved, then successful save after removing only
  that disposable test timer. Focused run `45830` is in progress. The actual PR
  query `77347` confirms #212 remains open/draft on the expected branch.

Next: obtain `45830`, format and publish this increment unsigned without merging.
Review covered canonical versus legacy state, saved versus new row identities,
independent global activity, dirty/undo, field preservation, pending receipts,
keyboard focus and current-access rejection. This is scoped self-review, not
independent full-feature acceptance. Remaining reports, approvals, lock/inventory,
activation and full Nix gates still prevent completing the goal.

- `45830` exited 0, including running-timer rejection/recovery and all preceding
  canonical editor assertions. No SQL or migration changed, so no new SQLx
  preparation was required. No verifier remains live. Formatting/publication
  remain the immediate next steps before returning to remaining integration.

- Formatting and formatting-CI `60949` exited 0 with zero changes; diff checks
  and the inline-style/hardcoded-color sweep passed. Publish the six scoped files
  to existing draft #212. Next substantive integration: T203 report-filter
  candidate discovery, beginning with the unresolved universe recorded in
  `contracts/people-directory.md` and `contracts/time-reports.md`; do not infer
  eligible people from existing report rows or expose a whole directory.

## 2026-10-06 — Catalog creation form

- Previous goal turn made progress: atomic initial-rate creation and its real-
  session/transaction tests were implemented. Revalidated the existing verifier;
  `3061` exited 0 after 159 project tests, registered sessions, SQLx preparation,
  offline all-target compilation and strict native/WASM lint. Cache changes are
  five new descriptions and replacement of the old four-column task insertion;
  no unrelated cache deletion. Formatting `87009` passed with zero changes.
- Reuse the existing catalog editor and native modal for creation. There is no
  Tasks-specific handoff; the established Workspace form components/tokens remain
  the visual authority. Added actual-component tests for allowed/denied creation,
  exact rates, validation preservation, duplicate-submit exclusion, successful
  refresh and requester/permission invalidation. RED run `77661` is in progress.

Next: observe RED, wire the shared editor's creation state, then run component
and disposable-browser acceptance. Project-editor lifecycle, prior narrow-table
limitation and the full permissions goal remain open; no merge or activation.

- RED `77661` failed on the three absent creation controls while the seven
  existing component/transport tests passed. Connected creation through the same
  keyed native dialog, preserving bound identity and shared controls. Empty
  initial rates remain absent; explicit zero is exact money. Creation success
  resets to the first active page; permission loss discards the form. Unknown
  save errors advise checking persisted state before retrying.
- Initial verification `24574` found a Dioxus key syntax error; corrected the
  formatted-string key without changing its identity semantics. Corrected run
  `4106` is active. Extended the guarded disposable browser suite with real
  creation, validation, exact rates, missing financial grants and revoked writes.
  Its new assertions/captures have not run yet; no browser acceptance claimed.

Next: poll `4106`, then run strict native/WASM checks and the extended browser
suite. Keep Rust stable while the current compile runs.

- `4106` exited 0 with all ten component/transport tests passing. Strict verifier
  `34629` exited 101 on the new harness response's type complexity. Introduced a
  named response alias rather than suppressing the lint. Corrected verifier
  `29161` is running native/WASM lint followed by the extended disposable-browser
  suite. No browser result yet, and the previous table-polish limit is unchanged.

- `29161` exited 0: native/WASM strict lint, current server/WASM build and the
  extended `task-catalog` browser suite passed. Inspected the two new creation
  validation captures at 1440px dark and 390px light together. No visual fixes
  were needed; shared CSS and the previously recorded mobile table limitation
  are unchanged. This is one creation-form inspection, not another table-polish
  loop or Windows Chrome/full-suite acceptance.

- Added component assertions for blank financial-editor rates and creation from
  the archived filter returning to the first active page. `20096` passed all ten
  component/transport tests and strict test lint. Scoped self-review checked
  optional versus zero amounts, requester lifetime, duplicate submit, denied
  cleanup, success refresh, transaction rollback and post-commit events. No new
  high/critical finding identified; independent full-feature review remains open.

Next: format and publish this verified creation increment to the existing draft
PR #212 without merging. Then wire explicit project task archive/restore into
the existing project editor; T234, T230 and the full permissions goal stay open.

- Formatting and formatting-CI `30758` passed. Published unsigned commit
  `dcadcee` (`Add atomic task creation to the task catalog`); push `41853` exited
  0 on `feat/scoped-permissions`. Worktree was clean before this publication note.
  No merge, real data change, policy activation or running verifier remains.
- Next concrete consumer gap: `new_project` still builds `ProjectEditRequest`
  with empty `task_activity` and passes only global `inactive_task_ids` to its
  Tasks section. Connect retained project-link activity to explicit controls and
  the existing save/revision/recovery flow, preserving hidden configuration and
  saved drafts. Do not implement a competing Project Detail management panel.

## 2026-10-06 — Atomic task creation

- The preceding estimate turn made no implementation progress. Revalidated
  `5561f14` and the existing worktree; only the prior publication note was dirty.
  Available disk space is 152 GiB, so no cleanup or unrelated process stop is
  needed. The next safe action remains T234 initial-rate creation.
- Rechecked Harvest's task-management guide: account-level creation accepts a
  default rate. Added registered-session regressions for independent rate-write
  authority, requester identity, stale currency, invalid amounts, project failure
  rollback and response redaction. RED verification is running as `2521` against
  a disposable socket-only PostgreSQL; no real data or policy was changed.

Next: observe the RED result, extend the existing creation transaction and reuse
the edit-rate validator; verify both creation and existing edits before wiring
the catalog create form. The full permissions goal remains open.

- RED `2521` exited 101 as expected: the old endpoint ignored the initial-rate
  input and returned success without requiring its authority. Extended the
  existing endpoint with required identity/rate intent; reused the edit validator
  and kept creation, initial rate and optional project link in one transaction.
  Adapted every helper caller and the legacy form, with no unbound fallback.
- Added production tests for task versus global/managed financial grants, exact
  zero/positive values, event payload, project-currency rollback, and current
  authority/currency after actual lock waits. Corrected an unexecuted test premise:
  the normalized catalog requires global rate read when granting global rate
  write, so a valid writer is not a write-only actor. No permission rule changed.
- Rust formatting passed (`82097`). Verifier `3061` is running: project tests,
  registered sessions, SQLx preparation, offline all-target compilation and
  strict native/WASM lint. Source is held stable during verification.

Next: poll `3061`, fix any demonstrated failures, then connect and browser-test
the catalog create form. Project-editor lifecycle and full permissions acceptance
remain open. No merge, policy activation or real-data mutation.

- `3061` passed all 159 project tests and the registered-session matrix, including
  the new atomic creation cases. It is still running SQLx preparation, offline
  compilation and lint; do not infer a terminal green result from the functional
  tests alone. Scoped source review checked every creation caller, organization-
  first authorization, current financial intent, rollback and post-commit events.
  No independent review or new browser acceptance is claimed.

## 2026-10-06 — Task consumer verification

- Previous goal turn made progress: implemented the bound reader and actual
  Tasks consumer; production catalog and real-session tests passed. Revalidated
  the dirty worktree and polled `91625`: terminal exit 101. All 12 Workspace
  tests passed; five consumer tests failed in the harness's event lookup, not
  authorization assertions. Static DOM IDs are stored in Dioxus templates rather
  than attribute mutations. Reused the mounted-target lookup pattern from the
  existing Reports/permission-editor harnesses; no production ID workaround.

Next: rerun component tests, SQLx/native/WASM checks, then the disposable-browser
suite. Atomic task creation and project-editor lifecycle integration remain open.

- `36640` passed the two production catalog tests, registered-session matrix,
  12 Workspace tests and seven Tasks/transport tests. It exited 101 at strict
  lint because the existing detail-navigation harness imported the expanded task
  model privately. Matched that harness's public model-module convention; no
  production dead-code suppression. SQLx's cached-target omission was corrected
  by invalidating the test entry points before regenerating, as in the existing
  full task verifier.
- `88712` exited 0: those functional/component checks, complete SQLx preparation
  and strict native/all-target and WASM lint passed. The cache contains three new
  descriptions and no deleted descriptions. Browser suite syntax and whitespace
  checks pass. No browser execution result or full-feature acceptance yet.
- Resolved pinned browser tools from the Git flake. Cancelled only the initial
  path-flake evaluation (`14133`, exit 1) to avoid copying build artifacts; the
  Git-source evaluation (`98830`) passed. No unrelated process was stopped.

Next: build the app and run `task-catalog` with
`.scratch/run-task-catalog-browser.sh`, inspect its batched desktop/mobile
evidence, then address creation and project-editor lifecycle integration.

- `96671` exited 0: built the current server/WASM bundle and passed the guarded
  disposable Chromium task-catalog suite. It verified 1440px dark / 390px light
  keyboard dismissal/focus, exact zero currency, hidden-rate preservation,
  archive/restore and loss of editing authority. Four captures under
  `.scratch/task-catalog-browser/` were inspected together. The narrow table
  scrolls inside its container; monetary cells need the existing nowrap utility.
  Applied that one local utility and added a browser assertion. Confirmation is
  pending; no shared CSS change, no claim of Windows Chrome or full T234 parity.

- `63085` exited 0: component checks and the second disposable-browser round
  passed after the money-cell utility change. Inspected the confirmation mobile
  capture; horizontal scrolling is contained, monetary units stay together, and
  narrow task-name wrapping remains a visual limitation. Stopped the bounded
  visual passes; do not call the mobile design final. Registered the verified
  fixture in the default suite (no full-suite result implied).

- Scoped self-review checked requester binding, fresh task/rate authorization,
  SQL-side omission of protected fields, name/ID pagination and preserved hidden
  rate intent. No extra permission loader, dependencies, migrations or shared
  CSS changes. This is not independent full-feature review or T234 sign-off.

Next: format and publish this verified consumer increment on draft PR #212,
then implement atomic catalog creation and project-editor lifecycle integration.

- Formatting and formatting-CI (`7186`) passed. Published unsigned commit
  `5561f14` (`Add permission-aware task catalog management`) on the existing
  branch; push `30431` exited 0. Worktree was clean before this publication note.
  No merge or policy activation. No process remains from local verification.

Next: extend the existing task creation transaction with bound requester and
independently authorized initial rate, add failing atomicity/preservation tests,
then connect the create form. Keep T234, T230 and the full permissions goal open;
project-editor task lifecycle and the recorded narrow-name wrapping still need
completion before full consumer sign-off.

## 2026-10-06 — Bound task catalog delivery

- The preceding estimate turn was no progress toward implementation. Revalidated
  the existing worktree and polled inherited verifier `83689`: terminal exit 101,
  with the intended missing `load_task_catalog` registration failure. No restart
  was inferred from silence.
- Added the typed, bounded catalog reader using the existing `ReadAccess`
  transaction, independent financial grants and current-requester binding.
  Canonical access is explicit; legacy roles do not grant catalog access.
  No schema, dependencies, activation or real-data changes.

Next: verify the registered delivery and pagination/isolation, then connect the
actual Tasks consumer. T234 and the full permissions goal remain open.

- `52201` passed the real-session matrix with the new catalog reader (exit 0).
  Added production pagination/activity/isolation and lock-wait revocation tests.
  Connected Workspace/rail navigation and the Tasks list/edit/archive/restore
  consumer using existing controls, exact money parsing and bound commands.

- Actual-component tests cover hidden/pending/denied/read-only/financial states,
  account switches, editor disposal and explicit archive confirmation. Initial
  UI verification `24310` exited 101 on an input callback type annotation; fixed
  that and the explicit child-module path needed by external component tests.
  Removed an unused binding and an undefined footer class during local review.
  Browser acceptance, atomic creation and project-editor restoration remain open.

- Corrected verifier `91625` is running. Its two production catalog tests and
  real-session matrix passed; it is now compiling/running the actual Tasks and
  Workspace component tests, then SQLx preparation and native/WASM lint. Keep
  Rust unchanged until its terminal result. The preceding formatter passed.

- Added `tests/browser/task-catalog.cjs`, guarded to the existing disposable
  browser runner. It exercises desktop/mobile keyboard focus, stored currency,
  zero rate, hidden-rate preservation, archive/restore and revoked writes. It has
  not run yet and is not registered in the default suite until validated.

Next: poll `91625`; address any terminal failures, then build the pinned browser
artifact and run `task-catalog` against the disposable runner. Complete atomic
initial-rate creation and project-editor lifecycle controls before T234 sign-off.
No commit, push, merge, policy activation or real-data mutation in this iteration.

## 2026-10-06 — Task-management consumer integration

- Previous turn made progress: published `979a594`, closed verified backend T233,
  and left only its publication note dirty. No live verifier was inherited.
- Read the design/Rust/testing/Ponytail skills and existing Workspace/Settings
  sources. Impeccable's context launcher could not install its engine outside
  permitted cache paths; followed its direct-context fallback without installing
  anything. No task prototype exists; preserve the existing Workspace system.
- Traced the task readers, commands, People consumer and Workspace/sidebar gates.
  The active-only unbound task list cannot deliver archived recovery or correct
  currency editing. Record the needed bound catalog projection and actual UI
  acceptance under T234 rather than treating backend completion as delivery.

Next: reproduce the missing bound catalog delivery through the real session
route, implement its current-authority projection, then wire and test the Tasks
consumer and project lifecycle controls. No activation, live data or merge.

## 2026-10-06 — Link currency boundary review

- The preceding estimate turn was a verified wait: `75534` was polled live and
  its registered-session matrix passed. It did not complete implementation.
  The same handle remains active; SQLx preparation passed and offline all-target
  compilation is in progress. Do not restart or edit Rust before it terminates.

- Read-only review found `enable_project_task` checks copied default currency
  only when `project_settings` exists. Canonical-policy organizations can still
  contain older projects without that row. Cover unknown and incompatible
  denominations, including zero, through the authorized link transaction before
  correcting the shared helper. Keep policy-zero inheritance and existing stored
  overrides unchanged; an authorized explicit project-currency override remains
  the recovery path. This follows the existing exact-money invariant, not a new
  currency-conversion feature.

- Spec Kit skills remain absent from the available local skill directories;
  continue against the existing reviewed T233 contract without claiming a skill
  invocation. Rust/testing/async/Ponytail instructions were read for this change.

- `75534` exited 0: affected API/project/time/editor/import and registered-session
  tests, SQLx preparation, offline all-target test build and native/WASM lint
  passed. Rust stayed frozen until terminal. No full Nix/browser gate is claimed.

- Added a production-link regression crossing absent/task/person settings,
  unknown/USD/EUR catalog denomination and zero/nonzero amounts. It checks
  atomic denial, authorized explicit recovery and preservation on repeated
  linking. RED verifier `10613` is running against unchanged currency logic.

- RED verifier `10613` exited 101 with the intended failure: absent project
  settings, unknown default currency and a zero amount incorrectly returned
  success. The shared query now requires a known matching default denomination
  for canonical policy as well as configured projects. No new helper, dependency
  or migration; existing-link no-ops and policy-zero behavior are untouched.

- Formatting `19757` exited 0. Expanded task verifier `11565` is running against
  the corrected snapshot; keep Rust frozen until its terminal result. Its 29 API,
  two transport, 154 project, 57 time-entry, 102 editor, 187 import and registered
  session tests have passed (eight existing import scale tests remain ignored).

- `11565` exited 0. SQLx preparation, offline all-target test compilation,
  strict native lint and WASM lint passed after the functional suites. The cache
  has seven new descriptions and one replaced description, with no unrelated
  removal. Scoped self-review checked current requester/grants before no-ops,
  independent managed-rate scope, organization-before-resource locks, retained
  archived state and exact currency preservation. The reproduced currency issue
  is fixed; no additional high/critical finding remains in this scoped review.
  This is not an independent review or full-feature sign-off.

- T233's backend lifecycle/link requirements are now verified and marked done;
  `quickstart.md` maps each boundary to executable regressions. T234's actual
  consumer/browser work and the encompassing T230 remain open. Final formatting
  and publication follow; no policy activation, real-data mutation or merge.

- Final formatting and formatting-CI checks (`69128`) passed. Published unsigned
  commit `979a594` to the existing branch; push `4782` exited 0 and GitHub confirms
  the exact head `979a594e74de7f0a94aba321293f1dcb757ce2eb` on open draft PR #212.
  No merge. The worktree was clean before this publication record was added.

Next: implement T234's bound task-management UI using the existing design
components and browser acceptance.
The full permissions objective remains active and incomplete.

## 2026-10-06 — Existing-task linking authority

- Previous goal turn made progress: published and verified `0591407`. Revalidated
  the worktree; only its publication record remained dirty. No live verifier was
  inherited. T233/T234 and the full permissions goal remain incomplete.

- Rechecked Harvest's current permissions and billable-rate guides and traced
  the direct link endpoint, shared link helper, editor authority and pure rate
  evaluator. Project edits, catalog writes and project-rate writes are distinct;
  use the existing project editor's authorization rather than a new role hierarchy.

- Added nine registered-session cases for all/managed project scope, missing
  designation, global-task-only denial and independently scoped explicit rates.
  Repeated calls also test current authorization, changed requester identity,
  anonymous requests and equal-rate intent without financial authority.

- First verifier `48688` exited 101 before tests because a new fixture referred
  to a nonexistent `created_by` column. Corrected the fixture against migration
  0044; production authorization remains unchanged for the RED verification.

- Corrected RED verifier `55532` exited 101 with the intended failures: current
  canonical Members were rejected, legacy administrators without the applicable
  project/rate grants were accepted, and denied associations persisted.

- The existing endpoint now binds the requester and uses session authentication;
  its transaction reuses `editing::lock_editor_actor` under the access-changing
  gate. Explicit rates use the pure project-owned rate evaluator before link
  no-ops. No new permission loader, dependency, schema or legacy-role remapping.

- Added production-helper tests for project/rate/designation revocation after
  real waits, retained authority while waiting for the task, unavailable current
  state before no-ops, archived association preservation and foreign targets.
  Formatting passed; the expanded task verifier is now running on this snapshot.

- Expanded verifier `2810` exited 101 after compatibility, 153 project tests,
  time-entry, 102 editor and 187 import tests passed. The sole HTTP failure was
  a new test expecting 409 for a changed requester; the existing task/project
  requester contract correctly returns 403. Corrected that expectation without
  changing the production denial. Added managed-rate designation revocation
  independently from all-project editing to the real lock-wait test.

- Corrected verifier `75534` is running. An initial invocation was denied access
  to the Nix daemon socket before starting; the authorized retry uses only the
  same disposable `/tmp` database. No live verifier was duplicated. Freeze Rust
  until the active handle's terminal result.

- Scoped review checked requester equality against the authenticated identity,
  the shared strict editor loader before resource locks, project-owned financial
  scope before no-op detection, unchanged legacy behavior and existing archive/
  foreign-parent validation. No new consumer calls this direct endpoint, so the
  new requester argument does not silently strand an existing browser control.
  Browser delivery and full-feature acceptance remain outstanding.

Next: collect `75534`, resolve any failures, record completed gates and publish
the verified increment. No policy activation, live data or merge.

## 2026-10-06 — Retained project-task activity

- The last estimate reply made no implementation progress. Revalidated the
  existing `ac4c90c` worktree and collected verifier `82493`: terminal exit 101
  from ambiguous SQLx `begin` traits. Removed the redundant `Acquire` import;
  no verification process was restarted while still live.

- The retained-link implementation adds migration 0048, explicit editor activity
  intent, archive metadata and both tracking-source predicates. Global archive
  invalidates affected project editors and global restore leaves links archived.
  Activity-only edits preserve rates, restrictions, budgets and recorded time.

- Harvest's unlocking-time documentation resolves the restoration order: restore
  the task globally before restoring its project link. The source and applicable
  contract are recorded in `contracts/task-permissions.md`; no browser validation
  is claimed. Earlier RED verifier `98213` exposed implicit project restoration
  and missing editor invalidation before these changes.

- Import batches now join the organization access gate before resource locks.
  Review found two additional paths needing regression coverage: new links for
  globally inactive imported tasks, and activity omitted during preview snapshot
  restoration. Added failing assertions for canonical/legacy imports and old/new
  snapshot formats. Verifier `84187` is compiling/running the import suite on an
  isolated disposable database; Rust remains frozen until its terminal result.

- T233/T234/T230 remain open: direct-link authorization, UI lifecycle controls,
  broader consumers and complete acceptance are not finished. No real data,
  live policy activation or merge is part of this iteration.

- `84187` exited 101 with exactly the two intended archival failures (185 passed,
  two failed, eight existing scale measurements ignored). Import insertion now
  derives new-link activity from the current policy/global task, and checkpoint
  restoration retains explicit state or derives it for pre-migration snapshots.
  Existing links and their rate overrides remain untouched on reimport.

- Formatting passed. Expanded verifier `72241` is running against the corrected
  snapshot, with Rust frozen until its terminal result. The check includes the
  new populated-schema migration test and the real-session explicit-restore flow.

- `72241` terminated with exit 101 after the compatibility, project and time
  suites passed. Editor tests had one invalid new fixture: it assigned both an
  hours and a money budget to the same task, violating the schema's exclusivity
  constraint before reaching the implementation. Corrected the test to exercise
  hours and money separately, preserving all archive/restore/replay assertions.
  The other 101 editor tests passed; later import/cache/lint stages did not run.

- Corrected verifier `87785` is live; Rust is frozen until the handle terminates.

- Its compatibility, project, time-entry and editor suites passed, followed by
  all 187 import tests (eight pre-existing scale measurements ignored) and the
  registered-session matrix, including explicit project restoration before a
  timer can restart. SQLx preparation/offline compilation/native/WASM lint are
  still running; their completion is not inferred from the passing tests.

- Scoped review traced both import adapters through the shared organization gate,
  checked absence of organization-lock upgrades in their production writes, and
  checked editor request binding, bounded duplicate-free activity intent, receipt
  compatibility, revision invalidation and retained protected fields. The direct
  `link_project_task` endpoint still calls `require_manager`/`lock_creation_actor`;
  its next increment must use current project-write scope independently from
  global task-write and project-rate grants, including no-op/revocation cases.
  This is not an independent review or full-feature acceptance.

- `87785` exited 0. SQLx preparation, offline all-target test compilation (3m10s),
  strict native/all-target Clippy (1m36s) and WASM Clippy (15.76s) passed. Scoped
  self-review identified no additional high/critical finding in this increment;
  this is not independent review, a browser run or the full Nix gate. All verifier
  handles from this iteration are terminal. GitHub confirmed #212 remains
  OPEN/DRAFT at `ac4c90c` before publication; no merge was requested or performed.

- Final formatting, CI-format and diff checks passed. Published unsigned commit
  `0591407`; push exited 0 and GitHub confirmed #212 OPEN/DRAFT at
  `05914078ae1bc8277b4632e115503dda537e5bbe`. Publication handles also terminated.
  SQLx preparation replaced obsolete generated descriptors, recoverable in Git;
  no business data or user files were removed.

Next: finish direct-link authority with session-bound project scope and independent
rate intent, then lifecycle UI acceptance. T233/T234, full permissions and the goal
remain incomplete. No merge or live policy activation occurred.

## 2026-10-06 — Task activity authority and timer exclusion

- The preceding estimate reply made no implementation progress. Revalidated
  `facfb49` and the existing worktree; only its publication log was dirty.
- Added registered-session regressions for canonical task writers with legacy
  Member roles, legacy administrators without task grants, independent rate
  projection and rejection of archival while a timer is running. The isolated
  verifier `40406` is running against the unchanged lifecycle implementation.
- T233 remains the full lifecycle integration: independent retained project-task
  state, explicit project restoration, editor/tracking integration, existing-task
  linking and races. The activity authority boundary alone does not close it.
- `40406` exited 101 with the intended session failures: canonical Member denied,
  legacy Admin accepted without grants, protected rates returned, and a running
  task archived. Replaced the legacy wrapper with authenticated requester binding
  and current transactional task authority. The activity path takes the exclusive
  access gate from the outset; create/edit retain their existing gate modes.
- Canonical archival rejects running entries without returning their identities.
  Session rate projection is independent from the bounded service-event payload,
  and events are dispatched only after a committed actual transition. No schema,
  UI, live policy or business-data change. Policy-zero behavior is preserved.
- Added five production-helper cases covering strict state and no-op authorization,
  full events versus redacted responses, preserved history/rates, revocation after
  a real lock wait, both sides of the tracking-writer gate and a committed timer.
  Session coverage includes anonymous/mismatched/foreign requests, real timer
  start/stop/archive/start rejection, restore and repeated-response redaction.
- `33965` is running compatibility, project, time-entry and registered-session
  regressions, followed by SQLx preparation, offline all-target compilation and
  strict native/WASM lint. Rust is frozen until the verifier terminates.
- `33965` passed 29 compatibility tests, both rate-transport tests, all 147 project
  tests and all 56 time-entry tests. It then exited 101 because the new HTTP test
  fixture omitted the required `project_tasks.billable` column. Corrected that
  fixture to create an explicitly billable link; no production rule or assertion
  was weakened. Cache and lint stages had not run.
- Corrected run `16237` passed the compatibility/transport/project/time-entry
  suites and the full registered-session matrix, including the new real timer
  workflow. SQLx preparation and offline/native/WASM gates are still running.
- Adversarial self-review checked actor/requester binding, strict policy before
  no-ops, independent rate projection, internal events after commit, tenant-scoped
  generic timer conflicts and final gate acquisition before task locks. Both
  interactive timer paths hold the conflicting shared organization gate; imports
  insert historical entries without running state. No additional high/critical
  issue is identified in this limited boundary. This is not independent review
  or acceptance of the unfinished retained-link restoration/UI work.
- Recorded the remaining editor/draft, retained configuration, sorted parent
  locks, revision invalidation and dual tracking-source integration checklist.
  Harvest documentation confirms the two restoration levels; the edge case of
  restoring a project link while its global task is still archived remains to
  reconcile before exposing that control. No loaded browser tools were available.
- `16237` exited 0. All-target SQLx preparation, offline test compilation (3m23s),
  strict native/all-target Clippy (1m46s) and WASM Clippy (15.78s) passed after the
  source/session suites. No new browser or full Nix check is claimed. T233 and
  T230 remain open; this verifies the activity authority/timer boundary only.
- Format, CI-format and diff checks passed without source changes. Published
  unsigned commit `ac4c90c`; push exited 0 and GitHub confirmed #212 OPEN/DRAFT
  at `ac4c90c95f1f093602adacba0eeeda43e09c4855`. No merge occurred. All verifier
  and publication handles in this iteration have terminated.

Next: collect verification and review the activity boundary, then complete the
retained-link state and editor/tracking integration still required by T233.
No real-data mutation, policy activation or merge.

## 2026-10-06 — Explicit task-rate editing

- Previous goal turn made progress: published and verified creation increment
  `8dd61d4`. Revalidated the worktree; only its publication log remained dirty.
- Added nine real-session edit scenarios against the retained endpoint. `78533`
  exited 101 with the intended failures: canonical Member rejected, legacy Admin
  allowed without current task authority, preserve intent clearing a hidden rate,
  and explicit rate changes allowed without the global financial grant. No
  implementation was changed while the RED verifier was running.
- Replaced nullable edit input with required tagged preserve/clear/set intent
  and an expected-requester identity. The existing helper now takes authenticated
  actor identity and reuses the creation transaction's current task authority.
  Pure `RateEdit` gates explicit financial intent; set validates integer amount
  and current organization currency. Preserve leaves unknown denominations
  untouched, clear removes both fields, and no-op comparisons include currency.
  Redacted session results stay separate from unchanged service-event payloads;
  dispatch remains after commit. No schema, policy activation or UI change.
- Adapted existing mutation/no-op/concurrency tests to explicit intent rather
  than retaining an unchecked helper. Added production tests for hidden/no-op
  projection, unknown currency, unchanged project overrides/time history,
  financial no-op denial, invalid inputs/foreign tasks, task/rate revocation,
  stale currency after organization waits, held authority and actor deactivation.
  Added transport round-trip/rejection cases and session identity/anonymous checks.
- `37561` is compiling/running the full compatibility/project/session/cache/lint
  verification on this snapshot. Keep Rust frozen until terminal output. The
  new model transport tests also need an explicit test filter after compilation;
  merely compiling them does not prove their assertions passed.
- `37561` passed all 29 API tests, 142 project tests and the registered-session
  matrix. The separately executed transport tests (`34556`) found that a tagged
  unit variant accepted extra amount fields despite the enum's strict annotation.
  Kept the rejection assertion and changed preserve/clear to empty struct
  variants. No persistence or permission rule was relaxed to make it pass.
- After that confirmed failure, stopped the isolated verifier process group;
  `37561` terminated with exit 130, not a completed green gate. SQLx preparation
  had passed before interruption; offline compilation/lint was not accepted.
  Rust was edited only after terminal confirmation. Added the transport filter
  directly to the local verification runner so this assertion gates later checks.
- Corrected snapshot verifier `28245` is running. Rechecked official Harvest
  lifecycle references and inventoried the missing independent project-task
  activity state, both tracking sources, editor saves and revision triggers.
  Recorded the evidence and integration boundary without adding a migration or
  changing runtime lifecycle while the edit snapshot is being verified.
- `28245` has passed 29 API tests, both strict rate-transport tests and all
  142 project tests on the corrected snapshot. These include the eight new edit
  transaction cases, preserved creation tests and adapted legacy mutation tests.
  Remaining session/cache/lint stages are not yet accepted as complete.
- The corrected run also passed the complete registered-session matrix and
  all-target SQLx preparation. Offline test compilation is running. Refined
  T230 into traceable creation (T231, accepted), edit (T232), lifecycle/link
  (T233) and actual consumer/browser (T234) work without removing its full scope.
- Adversarial self-review checked strict policy/active actor, required requester,
  explicit financial intent before no-op detection, current currency after lock
  waits, global versus managed/report grants, SQL no-op semantics, tenant IDs,
  event/session payload separation and post-commit dispatch. The strict-transport
  defect is corrected and verified. No additional high/critical issue is identified
  in this edit increment; no independent sign-off or UI/lifecycle acceptance is
  claimed. Used existing `RateEdit` and transaction helpers, with no new dependency.
- `28245` passed offline all-target test compilation (3m01s) and native all-target
  Clippy (1m45s), then exited 101 on WASM's unused transport warning: the task
  editor has no browser caller yet. Added the repository's narrow, explained
  `expect(dead_code)` pattern to this type only, excluding server and test builds.
  It does not change transport/runtime behavior and will become an unfulfilled
  expectation when the browser caller is wired. No global lint suppression.
- WASM verification `36639` exited 0 (15.73s). Server/test runtime and query
  semantics were unchanged by its conditional lint annotation. T232 is accepted
  with the recorded source/session/transport/cache/native evidence; T230 remains
  open for lifecycle and real consumer delivery.
- Format, CI-format and diff checks passed without source changes. Published
  unsigned commit `facfb49`; push exited 0 and GitHub confirmed #212 OPEN/DRAFT
  at `facfb4914fd9f3480d93315841b6108846989c8c`. No merge occurred.

Next: finalize the retained-link lifecycle contract and its failing tests under
T233. Existing-task
linking, bound catalog UI and full permission/Nix gates remain required.
No merge or real-data mutation.

## 2026-10-06 — Current task-creation authority

- The preceding estimate reply made no implementation progress. Revalidated the
  existing scoped-permissions worktree and resumed its unfinished creation
  increment without creating another branch or restarting a verifier.
- The earlier failing-first run `44017` exited 101 with all four intended
  regressions: canonical Member denied, legacy Administrator inherited authority,
  unknown policy accepted, and task authority alone permitted a project link.
  The working correction replaces the wrapper's legacy role guard with an
  authenticated actor and checks current stored policy/grants inside the existing
  transaction. Composite creation also reuses project edit scope; its exclusive
  organization gate precedes actor/resource locks. Policy-zero behavior remains.
- Expanded coverage for all/managed project authority, absent designation,
  project authority without task creation, nonbillable project defaults,
  archived/missing/foreign destinations, corrupt client parents, unavailable
  actor state, and task/designation revocation across a real organization wait.
  Added real registered-function session coverage for allowed/denied creation,
  foreign project IDs, current grant loss, inactive sessions, response privacy
  and rollback. Fixtures are disposable; no live policy or business data changed.
- Verification `4964` is running the complete task-read runner against the new
  snapshot: compatibility/project/session tests, SQLx preparation, offline test
  compilation, native/WASM lint. Keep Rust unchanged until its terminal result;
  the newly added cases are not yet claimed as passed.
- `4964` has now passed all 29 compatibility and 134 project tests (including
  all eleven creation cases), plus the complete registered-function session
  matrix with the new task writes. SQLx/cache/native/WASM checks remain running.
  Source review confirms no new financial default or plugin event on a rolled
  back creation; events are dispatched only after the existing helper commits.
- SQLx preparation completed with the expected replacement of the project-link
  query descriptor and new creation/test queries. Offline all-target test
  compilation passed (3m13s). Native/WASM lint is the remaining live stage.
- Scoped adversarial self-review checked strict stored state, the active actor,
  organization-before-actor/resource locking, managed designation versus legacy
  role, destination parent isolation, atomic rollback, nonbillable link defaults,
  rate-free responses and post-commit plugin dispatch. No additional defect was
  identified in this creation increment. This is not independent review or
  approval of the still-unintegrated update/activity/link endpoints and canonical
  catalog UI. Rust/testing/concurrency guidance led to real database lock-wait
  tests and reuse of existing authorization/transaction helpers, without new
  dependencies or a parallel task service.
- Recorded the retained edit helper's concrete gaps and required explicit-rate
  regression cases in the task contract. Rate editing, lifecycle, UI binding and
  activation remain open; the creation increment is not substituted for T230.
- `4964` exited 0. Strict all-target native Clippy (1m43s) and WASM Clippy
  (15.68s) passed after the successful source/session/cache/offline stages.
  This snapshot is ready for formatting checks and publication. No new browser
  acceptance is claimed because this increment changed no UI/CSS consumer.
- Formatting, CI-format and diff checks passed without source changes. Published
  unsigned commit `8dd61d4`; push exited 0 and GitHub confirmed PR #212 OPEN/DRAFT
  at `8dd61d4436b6266ae5d58c1091b11ada0315f337`. No merge occurred.

Next: add failing direct-edit authority/preservation tests, then implement the
reviewed explicit rate-edit transport and transaction. Continue task lifecycle, project
association effects and actual catalog controls under T230. No full permissions
completion, activation, merge or full Nix acceptance is claimed.

## 2026-10-06 — Task read acceptance and write integration

- Previous turn made progress: added source/browser coverage, corrected the
  real-pointer resize interception and completed all six affected browser suites.
  Revalidated `93182` without restarting it. It exited 0: 29 API tests, 123 project
  tests, registered-session matrix, all-target SQLx preparation, offline test
  compilation and strict native/WASM Clippy passed on the final Rust snapshot.

- Completed the scoped adversarial self-review against the actual task reader,
  compatibility query, session tests, actor fences and consumer wiring. No
  additional high/critical finding remains identified for this read increment;
  this is not independent review or acceptance of unimplemented writers.

- T228/T229 are complete. The contract now separates historical baseline,
  accepted reads and remaining write/UI requirements. Recorded the next concrete
  write sequence: ordinary global creation, composite project checks, explicit
  preserve/clear/set rate intent, response redaction and separate lifecycle
  effects. Existing editor and global-default semantics remain authoritative.

- Published unsigned commit `f6e8bf1` after formatting and diff checks. Push
  succeeded; GitHub confirms #212 remains OPEN/DRAFT at that exact commit.

- Added four failing-first creation regressions against the existing production
  helper: canonical Member with TaskWriteAll, legacy Administrator without it,
  unknown policy, and global-task authority without destination project authority.
  They reuse the existing canonical fixture and verify denied requests leave no
  task. `44017` is compiling/running them on disposable PostgreSQL; no RED result
  or implementation is claimed until its terminal output is collected. Keep
  Rust unchanged while this verification snapshot is running.

Next: collect `44017`, then implement the reviewed task-creation checks and
registered-session coverage. Reconcile composite linking with the editor before
enabling its canonical branch; do not bypass project/field checks to make a
negative test green. T230, full permissions activation and complete Nix acceptance
remain open. No merge or real-data changes.

## 2026-10-06 — Task browser delivery and adversarial checks

- The preceding estimate turn was a verified wait: it polled live `65693` and
  collected successful compatibility/project/session tests and SQLx preparation.
  Resumed the same handle; it exited 0 with offline all-target test compilation
  and strict native/WASM Clippy also passing. No duplicate verifier was started
  while that source snapshot was active.

- Reviewed task SQL projections, strict actor loading, compatibility totals,
  actual timer consumers and the editor's separate minimal catalog. Added two
  source regressions for current manager designation and corrupt cross-tenant
  client parents in both historical and membership identities. `93182` is
  verifying the extended source snapshot with the full task runner; collect its
  result before claiming those cases passed.

- Added the real Chromium `task-read-permissions` suite and included it in the
  default browser runner. It checks archived own-timer labels, global catalog
  denial, rate-free tracking despite financial grants, enabled-task selection,
  and persisted start/stop with disposable fixtures only.

- Browser build `90382` passed. Browser run `86817` reached all permission and
  timer assertions, then failed a real Cancel click: the sibling sidebar resize
  handle intercepted the button. No forced click or skipped assertion was used.
  The localized stylesheet fix suspends that handle only while a timer/account
  popover exists, without changing menu components, geometry, tokens or layering.
  Added assertions that closing restores dragging and that account menus obey
  the same boundary. Rebuilt bundle `23475` passed; `29067` is confirming task,
  mobile/menu, project-editor/rate and Timesheet browser regressions.

- Rechecked official task management and running-timer documentation. Recorded
  separate global/project restoration, running-timer archive refusal and the
  difference between public API versus web task permissions in the task contract.
  These findings constrain T230; no global restore shortcut or implicit public
  API policy change is authorized by this read increment.

- Extended verification `93182` has now passed all 29 API and 123 project tests,
  including both new adversarial cases; the remaining session/cache/lint stages
  are still running. Browser `29067` has passed the new task suite, mobile
  navigation, all menu checks, scoped editor and task-rate suites; its Timesheet
  suite is still running. Do not restart either live handle.

- `29067` exited 0: all six browser suites passed, including Timesheet delegated
  commands, revocation and owner-only recovery. `93182` also passed the complete
  session matrix and has moved on to SQLx/cache/native/WASM checks. T228 is
  complete; T229 awaits the final source snapshot gates and publication review.

Next: collect `93182`, address actual failures and record exact
acceptance before publishing. Continue direct task writes and canonical catalog
controls under T230 afterward. Full permissions activation remains incomplete;
no merge or real-data changes have occurred.

## 2026-10-06 — Task compatibility delivery integration

- Previous turn made progress: corrected the ordinary reader and passed all
  121 project tests, then started independent compatibility regressions.
  Revalidated and collected the same `59836` handle; it exited 101 after
  2m41s compilation with all five expected API failures. The unchanged API
  rejected canonical readers, inherited legacy scope/rates and accepted unknown
  policy. No duplicate verifier was launched.
- Replaced the separate task count/list/detail SQL with one authorized
  count/page query, reusing the existing strict organization/actor read fence.
  Canonical catalog visibility and global-rate projection are independent;
  legacy policy, filters, stable ordering and direct-ID not-found semantics
  remain intact. Empty and exhausted pages retain the authorized total.
- Added actual-router tests for financial-only versus whole-catalog revocation
  after an organization wait, deactivation after session lookup and malformed
  state. Added registered Dioxus session-cookie coverage for catalog/tracking
  fields, anonymous/foreign/restricted users, revocation and inactive sessions.
- `65693` is running the compatibility suite, project regressions and complete
  session matrix, followed by all-target SQLx preparation, offline test
  compilation and strict native/WASM Clippy. Its PostgreSQL is temporary and
  socket-only. Keep Rust unchanged while this verification snapshot is running;
  no new result or completed API gate is claimed yet.

Next: collect `65693` and fix actual failures. Finish adversarial task-source
coverage and affected browser consumers, then publish only after the cache and
lint gates pass. Direct task writes, catalog controls and full activation remain
required; this read increment does not complete the permissions goal.

## 2026-10-06 — Task catalog authority regressions

- Previous turn made progress: reviewed and published project read delivery in
  `2631186`, followed by acceptance bookkeeping in `1b81680`. Revalidated the
  clean worktree at that head before beginning this increment; no old verifier
  was restarted.
- Traced catalog, tracking, retained project-task endpoints, the existing editor
  catalog, compatibility API and direct mutations. Reopened official Harvest
  permissions/rate guidance and recorded sources, scope and limitations in
  `contracts/task-permissions.md`. No interactive restricted-user Harvest
  observation or Spec Kit command execution is claimed; the local skill search
  still exposes no `speckit-*` instruction files.
- Added six production-reader regressions. `38679` completed compilation in
  2m56s and exited 101: five intended failures reproduce rejected canonical
  readers, inherited legacy catalog/rate authority and unknown-policy fallback;
  the existing archived-own-history behavior passed. T227 is complete.
- Reused the current read transaction and strict permission loader for task
  readers. Canonical global catalog scope now requires `TaskReadAll`, global
  default rates require all-rate read and tracking remains rate-free. Project
  labels retain independent linked-project/membership scope. Policy-zero SQL is
  preserved and both paths hold current actor/organization authority through
  materialization; no new policy, migration or dependency was added.
- Expanded tests to project-versus-catalog scope, private-progress membership,
  foreign/inactive actors and financial revocation after a real organization
  lock wait. `78824` is running formatting and all project tests against a new
  socket-only PostgreSQL cluster. No successful result is claimed yet. The new
  SQL cache has not been regenerated and this increment is not published.
- `78824` exited 0 after 2m44s compilation: all 121 project tests passed in
  13.07s, including the ten task regressions and unchanged legacy privacy,
  membership, financial and lifecycle tests. This does not prove equivalent
  compatibility delivery. Added five real-router task API regressions for
  catalog scope, count/direct IDs, global-rate revocation, archived filtering
  and unknown policy before replacing its independent legacy queries.
- The unchanged-API verification is live in `59836`, using the same isolated
  runner. Keep its Rust snapshot unchanged until compilation finishes; do not
  launch a duplicate build or claim those five tests have passed.

Next: collect `59836` for the new regressions against the unchanged API, then
integrate its count/page/direct-ID query with current authority and extend
session/race coverage before preparing SQLx and native/WASM gates. Direct writes
and actual catalog consumer affordances remain required; no merge or real-data
activation.

## 2026-10-06 — Project read delivery review

- The preceding estimate-only turn was no implementation progress. Revalidated
  the existing worktree and `2497dbe` base; no verifier is being resumed or
  restarted from a stale handle.

- Completed the scoped adversarial self-review across ordinary project reads,
  current-authority transactions, source/release export checks, compatibility
  count/page projection and actual list/detail consumers. Checked the financial
  revocation tests, unchanged billing-value assertions, minimal label DTOs,
  requester continuity and pending/error focus behavior against their code.
  No additional high/critical defect was identified; this is not an independent
  reviewer sign-off or evidence for unimplemented canonical consumers.

- Clarified the read contract's historical baseline versus current acceptance
  and added reproducible verification commands and snapshot limitations to
  `quickstart.md`. T226 is complete with the prior recorded test/browser/lint
  evidence and this review; full activation and Nix gates remain open.

- Final `nix fmt -- --ci` passed without changes and `git diff --check` passed.
  The removed SQLx descriptors belong to replaced queries; prior all-target
  offline compilation verified the regenerated cache. No migration, dependency
  or shared stylesheet was changed in this increment.

- Published unsigned commit `2631186` to `feat/scoped-permissions`; GitHub
  confirms #212 remains OPEN/DRAFT at that exact head. No merge or real-data
  activation. The normal sandbox denied the Git index write; the authorized
  escalated commit/push succeeded.

- Reconciled stale T161 bookkeeping with the existing editor evidence: `2994`
  had failed the mail test, but its exact frozen-source repeat `52467` passed
  all 1,751 tests and source/cache identity was checked before `2497dbe` was
  published. Its browser, lint, SQLx, formatting and both VM gates had passed.
  T161 is therefore complete for the existing editor. This does not claim
  the transient mail failure was fixed or full Nix acceptance of `2631186`.
  No new verification was started merely because the old handle is gone.

Next: inventory OP14/15 task catalog reads and direct mutations against the
existing editor/identity paths, then implement the settled global task and
independent financial predicates with regression tests. Keep project linking,
membership effects and any unresolved lifecycle/creation rules explicit; do
not infer answers to pending product questions. Full activation remains open.

## 2026-10-06 — Detail regression suite and fee-session acceptance

- The preceding estimate/status turn was a verified wait: it polled the live
  `69006` verifier without restarting it. This continuation collected exit 0:
  185 core, 1,350 app and 295 integration/component tests passed (1,830 total),
  with 11 existing manual measurements ignored. That snapshot includes the
  bound detail reader/consumer, but predates the following test additions.

- Removed the unused production role helper and obsolete inline-editor test
  doubles. Added a controlled late 401/403 fee response to the actual routed
  detail test: loaded labels and editing disappear, diagnostics remain hidden,
  and retry retains the original requester. All 34 navigation tests passed.

- Added legacy fee requester checks to the registered HTTP session matrix:
  matching identity succeeds, anonymous access is denied, and account or
  organization mismatch is forbidden. The complete session-matrix test passed
  against a new socket-only temporary PostgreSQL cluster. Canonical fee scope
  and policy activation remain separate, incomplete gates.

- The native all-target Clippy pass found four complex controlled-response
  field types in the test harness. Reused one test-only type alias; did not
  suppress the lint. `1599` exited 101 at that lint, not at an acceptance test.
  `83784` now runs native/WASM Clippy and the browser build after the correction.

- Browser dependency paths resolve to the pinned Nix Playwright installation.
  Added a disposable-database project-read scenario for minimal labels,
  withheld fields, managed/edit revocation, inactive actors, pending state and
  policy continuity at desktop/mobile widths. Migrated task-rate and team-error
  checks to the existing editor rather than dropping their validation/recovery
  assertions. Updated other project browser fixtures to the overview response
  and separate budget summary/breakdown contract. Browser execution is pending;
  syntax checks alone are not browser acceptance.

- `83784` passed native all-target Clippy, then failed the WASM lint on one
  server/test-only re-export and two retained endpoint DTOs no longer constructed
  by the UI. Scoped the re-export and documented those two client-side dead-code
  expectations without removing endpoints or changing their wire contracts.
  `74539` passed strict WASM Clippy and built both browser client and server.

- Browser `39197` exposed a fixture assumption: seeded projects legitimately
  remain visible through shared membership. Added a separate unrelated private
  project and checked management/edit scope independently of that membership.
  `60210` then passed all new project-read scenarios and the existing canonical
  editor-permission suite, including real revocation and recovery. Inspected
  desktop/mobile captures under `.scratch/project-read-browser/`; no overflow.
  Its later task-rate scenario failed on an outdated expected message, not a
  missing rejection. Corrected the editor-specific message and teammate-picker
  selector; `42839` runs the remaining affected browser suites. These browser
  fixes do not yet prove full-suite compatibility or bulk focus recovery.

- Follow-up browser runs corrected two test selectors (the save button changes
  its label while pending; the teammate selector includes a disabled empty
  option). `11168` passed task-rate/editor-mode/currency/budget/keyboard acceptance
  at all three widths. `95312` passed action-error recovery, including team
  removal/re-addition through the editor, and the complete projects-design
  suite, including shared-control defaults, responsive budgets and bound links.

- After distinguishing the success live region from progress loading, `1583`
  reproduced a real regression: bulk success remounted the list but lost the
  keyboard focus target. Added mounted-state focus restoration for loading,
  failed and successful refreshes, preserving deliberate navigation focus and
  the non-sensitive operation receipt while stale rows/actions stay unmounted.
  Impeccable hardening guidance informed this scoped correction; no CSS or
  shared component defaults changed. Extended the browser regression to cover
  deliberate focus movement during a pending refresh. `84428` runs component
  tests and rebuilds the browser for confirmation; the focus fix is not yet
  verified.

- `84428` passed 34 component tests and rebuilt both targets. `50897` exited 0:
  all four bulk recovery cases passed, including pending/error receipts and
  deliberate navigation focus; real bulk status/session/tenant/history checks,
  menu/popover tests, modal recovery and responsive-layout checks also passed.
  The layout suite covered projects and the existing client, invoice, report,
  people, approval, settings, import and timesheet screens at five widths.

- T225 is complete: refreshed SQLx/offline, current row/field scope, malformed/
  unknown-policy and real revocation tests are green. T226 remains open for
  final cross-delivery review/publication, not a claim of full permissions or
  activation. Final native/WASM Clippy `4791` exited 0 with warnings denied.
  Formatting `58850` and `git diff --check` passed after the focus correction.

Next: review the complete project-read
delivery diff before publishing the verified increment to existing draft #212.
Continue the outstanding lifecycle/creation/invoice and full activation gates;
no merge or real-data change is authorized by these results.

## 2026-10-06 — Team suite passed; bound detail consumer implemented

- The preceding turn made implementation progress. `9696` exited 0 after
  all-target SQLx preparation (1m14s), offline test compilation (3m15s), 185
  core, 1,347 app and 291 integration/component passes (1,823 total). Eleven
  existing manual measurements remained ignored. This covers the assignment
  reader and download additions, not the following detail-view changes.
- Added a requester-bound detail response holding the existing organization
  and actor fences across saved metadata, current edit affordance and minimal
  team/task labels. Reused the same detail and assignment query paths inside
  that transaction; no general people/task directory permission is required
  merely to show the readable project's identities. Protected rates, emails,
  costs and global/legacy role labels are not included in these label DTOs.
- Connected the actual detail route to that response. Pending, denied and
  mismatched project/requester/policy responses hide retained content. Refresh
  keeps the initial binding and remounts dependent state. Replaced its duplicate
  legacy task/assignment forms with the existing authorized project editor link;
  no task, assignment or business records were deleted. Shared CSS is unchanged.
- Existing policy-zero fee reads now carry the displayed requester; access
  denial suppresses the parent view. Canonical fee/invoice delivery retains its
  separate unresolved integration gate and is not inferred from project editing.
  The legacy fee endpoint and remaining task/membership writes still require
  their full canonical enforcement before activation.
- Added production detail-view scope/identity/edit tests, extended the registered
  session matrix and all four reader concurrency/cancellation cases, and added
  four actual routed UI tests for labels/editor access, pending/denied payloads,
  refresh identity/policy/project mismatch and edit revocation. Updated old
  navigation assertions for the single editor and sanitized error states.
- Formatting and diff checks passed; no inline styles or literal colors were
  introduced. Started `69006` for all-target SQLx preparation and the complete
  offline suite. New detail acceptance is pending, not established by `9696`.
- The initial check reports obsolete navigation doubles/imports and the now
  unused production `pages::is_admin` helper. Remove these after the current
  compilation finishes; do not restart the verifier for warnings. Browser tool
  discovery is still empty, so no browser acceptance is claimed.

Next: collect `69006`, fix actual failures and obsolete test doubles, then verify
fee-session denial, native/WASM and available browser behavior. Keep the whole
permission goal and lifecycle/creation/invoice/activation gates open.

## 2026-10-06 — Team reader uses current project and rate scope

- `51878` exited 101 with the expected four regression failures: canonical
  project readers were rejected, legacy Administrator access leaked team rows,
  managed scope was ignored and a missing canonical state returned success.
  The tenant/inactive-reader case passed. The test binary compiled in 2m36s;
  these are observed failures, not inferred test outcomes.
- Reused `ReadAccess` for assignments through materialization. Explicit
  project-read scope controls the team; ordinary membership exposes only the
  actor's own retained row. Assignment-rate fields follow the independent
  project-owned rate grant. Tenant-consistent client and person joins exclude
  foreign targets, including malformed legacy relationships. Inactive teammates
  remain visible to authorized project readers. Legacy policy remains separate.
- Added optional requester continuity to the registered assignment endpoint;
  the detail page still omits it pending its parent-loader integration. Extended
  the real-session matrix for matching/changed/foreign/anonymous requesters,
  forged identity, same-session rate/read revocation and inactive actors.
- Extended all four database-observed reader race/cancellation cases to team
  reads, plus unknown-policy/malformed-grant and cross-tenant-target checks.
  Formatting and diff checks passed. No CSS, membership write, invoice or task
  authority changed. The added reader coverage does not complete detail UI
  integration, browser acceptance or full-policy activation.

`2132` exited 1 during all-target SQLx preparation: the navigation mock's newly
added argument used an unqualified `PermissionRequester` outside its import
scope. Qualified that test-only type using its existing module; no production
authorization change was needed. This run did not execute the regression suite
and does not prove cache completeness. Repeat preparation and offline tests.

Next: collect `9696`, the replacement full SQLx/offline verifier, resolve actual failures,
then continue requester-bound detail and workflow identities.

## 2026-10-06 — Download verification passed; team-read regression started

- The preceding user-status turn made no implementation progress. Revalidated
  the specific `41827` process rather than restarting it: it exited 0 after
  the download/session/export/component checks. Two core, ten parameter, one
  registered-session matrix, two privacy, 53 streaming, 29 detail/overview and
  19 report-component tests passed (116 total); one existing manual measurement
  remained ignored. This covers the export identity addition, not browser or
  whole-policy acceptance.
- Inspected the actual detail consumer and assignment reader. The latter still
  trusts legacy team/rate columns, denying canonical readers with a legacy
  Member role and disclosing team/rates to a legacy Administrator without the
  equivalent current grants. Recorded the own-membership versus project-team
  boundary and its official reference evidence in the read contract.
- Added five production-reader tests for current team scope, inactive retained
  teammates, own-only membership, separate rate grants, tenant/actor denial and
  malformed state. Started `51878` against a disposable PostgreSQL cluster to
  establish the failure before changing the reader. No policy activation or
  real-data mutation; the detail UI and task/invoice/membership-write gates remain.

Next: collect `51878`, implement the team reader under the existing permission
fence, cover revocation and registered sessions, and verify the new SQL cache.

## 2026-10-06 — Full overview suite passed; download verification started

- `14350` exited 0: 185 core, 1,338 app and 291 integration/component tests
  passed (1,814 total); 11 pre-existing manual measurements stayed ignored.
  The app tests finished in 165s. This includes the three new routed overview
  cases and the expanded identity-bound auxiliary/lifecycle session checks.
  It covers the prepared/offline overview snapshot, not the later export binding.
- The known unused test import is removed in the subsequent source. No failure
  was hidden or ignored. Browser, native/WASM/Nix acceptance, detail integration
  and full canonical lifecycle/creation enforcement remain open.
- Started `41827` (`.scratch/verify-project-consumers.sh`) after the full verifier ended.
  It reuses the disposable-cluster reader verifier, checks download parameters,
  the registered session matrix, project privacy and streaming regressions,
  then runs actual detail/overview and report components. The export additions
  introduce no new SQL; this pass need not regenerate the already complete cache.

Next: collect `41827`, resolve actual failures,
then continue the project detail/workflow identity boundary. Preserve OP13's
unanswered decision and the broader permission/activation gates.

## 2026-10-06 — Project download identity implementation

- The preceding turn made implementation progress. Revalidated the live `14350`
  run: all-target SQLx preparation completed in 1m17s, 185 core tests passed and
  the offline all-target test build completed in 2m57s. Its compiled tests are
  still running; the following export additions are not covered by that snapshot.
- Added optional flat `expected_org_id`/`expected_user_id` bindings to project
  CSV/XLSX downloads, matching the existing time-report convention. Both formats
  reject changed identities before reserving generation capacity; partial pairs
  and malformed/repeated UUID query values fail rather than disabling the check.
  Unbound legacy links retain current authenticated authorization, not arbitrary
  requester selection. Existing row/financial release fences are unchanged.
- The actual project export link now supplies its displayed requester. Added
  three parser/identity tests, real-session CSV/XLSX matching/same-tenant/foreign/
  anonymous/malformed checks and rendered-link identity assertions. No new SQL
  queries or transport implementation were added.
- Removed the obsolete test-harness `Client` re-export after `14350` finished
  compiling, without restarting its compiled suite. Formatting and diff checks
  passed for the additions. Their own compile/test verification is pending.
- Tool discovery still exposes no browser/MCP tools. Do not claim browser
  acceptance from the routed component tests. Inspection of the detail page
  confirms the remaining unbound global people/assignment/task/fee dependencies;
  those must be reconciled independently of the now-bound list.

Next: collect `14350`, resolve any actual consumer failures, then verify the
new download binding and rendered links. Continue the detail/creation/lifecycle
integration under the full goal; do not activate policy or infer the unanswered
OP13 choice. No merge or real-data change.

## 2026-10-06 — Overview consumer connected; identity-bound auxiliaries

- The preceding turn made implementation progress. `36912` exited 0 after a
  4m35s build: two core arithmetic, 102 project, 18 budget, 88 invoice, five
  display, one registered-session matrix, four canonical XLSX, 12 legacy export
  and 53 streaming tests passed (285 total). One pre-existing manual measurement
  stayed ignored. This snapshot predates the following consumer changes.
- Project detail, tags, spend, budget and single/bulk lifecycle endpoints now
  accept an expected requester and reject identity changes before reading or
  mutating records. The overview reuses the same identity check. Expanded the
  real-session matrix to verify matching, same-tenant/foreign mismatch and no
  lifecycle effects from a changed requester. This is not a replacement for
  lifecycle authorization under OP13.
- `ProjectList` now loads the requester-bound overview before mounting its
  content. It retains the initial requester/policy across refreshes and remounts
  dependent resources, selection and dialogs for each accepted response. Client
  groups/search/options use only workflow client identities; the directory and
  independent current-user resource were removed from this consumer.
- Auxiliary reads and existing lifecycle requests carry the displayed requester.
  Authentication/authorization failures suppress rows and actions until access
  is refreshed. Per-project edit controls use the server affordance; creation
  and import flags use current capability/administrator identity. Legacy status
  controls remain explicitly limited to legacy policy: OP13's canonical rule
  is still awaiting the user's decision and has not been silently inferred.
- Added actual routed-component tests for client labels, bound auxiliary calls,
  pending/denied access and same-page refresh changes of user, organization or
  policy. The UI framework and CSS/tokens are unchanged. This does not constitute
  browser acceptance. Canonical project creation enforcement, detail consumer
  integration and requester-bound export intent remain separate open work.
- Formatting and diff checks passed. `14350` is running full SQLx preparation
  plus the offline core/app/integration suite. The initial check reported an
  unused `client::Client` re-export in `tests/detail_navigation.rs`; remove that
  obsolete test-harness import after collecting this frozen run. No new test
  result or complete cache acceptance is claimed yet.

Next: collect `14350`, address failures and the known test-harness warning,
then complete the remaining project consumer/export identity and lifecycle
contracts. Preserve the full permission goal, browser/native-WASM/Nix gates
and activation review. No merge, policy activation or real-data modification.

## 2026-10-06 — Full read suite passed; requester-bound overview implementation

- `58485` exited 0. SQLx preparation and the offline full build passed; 185 core,
  1,332 app and 288 integration/component tests passed (1,805 total). Eleven
  existing manual measurements remained ignored. The app tests took 230.90s.
  This proves the frozen read/export/compatibility snapshot, not the subsequent
  overview addition or remaining browser/WASM/Nix/full-policy gates.
- Added `get_project_overview`, binding expected session identity to the current
  authenticated requester, including empty results. Unavailable actors and
  mismatched identities fail closed. Reused `ReadAccess` and the existing project
  query rather than fetching the client directory or adding a policy engine.
- The query now also supplies only the visible project's client ID/name/active
  context and its current edit affordance. Canonical editing uses project write
  scope plus current management designation; legacy editing uses the stored
  role under the actor fence. Existing list/tracking calls map back to their
  unchanged `Project` wire shape. No lifecycle authority is inferred.
- Registered the six prepared database tests and extended the real-session
  endpoint matrix with anonymous denial, expected-requester checks across
  same-tenant/foreign accounts, actor spoofing, protected fields and revocation.
  Tests were authored before implementation; no pre-implementation assertion
  failure is claimed for this new endpoint.
- Formatting and `git diff --check` passed. `36912` is running
  `verify-project-reads.sh`: all project cases plus budget, invoice, display,
  real-session, materialized export and streaming regressions. The two pure
  arithmetic cases passed; app compilation/test outcomes remain pending.

Next: collect `36912` without launching a duplicate build; resolve actual
failures, then connect the real overview consumer and requester-bound auxiliary
reads/actions. The latest query still needs SQLx regeneration before offline
acceptance. T225/T226 remain open for that latest state. No new UI/CSS changes,
merge, policy activation or real-data modification in this iteration.

## 2026-10-06 — Overview consumer contract and verification continuation

- The preceding user-facing turn was a status estimate, not implementation
  progress. Revalidated the worktree and continued the live `58485` verifier;
  no replacement build was started. All-target SQLx preparation completed in
  2m03s, 185 core tests passed, and the offline app/test build completed in
  5m21s. The app suite is still running; its final result is pending.
- Defined the next overview response in `contracts/project-read-permissions.md`:
  authenticated requester continuity, minimal visible client identities and
  per-project current edit affordances. Existing tracking payloads and separate
  lifecycle/financial/creation authority stay unchanged.
- Prepared six database tests in the currently unregistered
  `projects/canonical_read_tests/overview.rs`: minimal client projection,
  empty-result requester mismatch, scoped edit authority, inactive actor denial,
  current legacy role and concurrent edit revocation. They are not compiled or
  claimed as passing yet; registration and implementation follow the frozen
  full-suite result.
- The lifecycle investigation found that `set_project_active` and its bulk
  sibling still use legacy manager authority. The user question about authorizing
  archive/restore with current project editing permission remains unanswered.
  Do not infer a decision or let it block independent read integration.

Next: collect `58485`, resolve any actual failures, then register the overview
tests and implement the requester-bound projection using the existing project
read fence and query. Keep the actual consumer/auxiliary-resource and browser
acceptance gates open. No merge, policy activation or real-data modification.

## 2026-10-06 — Compatibility verification passed

- `94252` exited 0: compile 4m25s, two pure arithmetic tests passed, then
  218 tests matching `harvest::` passed in 18.56s; eight pre-existing manual
  scale measurements stayed ignored. This filter includes both compatibility
  API and import/connection regressions, not 218 API-only tests.
- The five reproduced compatibility failures now pass, as do the expanded
  malformed-state checks, current project-management scope, filtered counts
  beyond the last page, shared hour budgets without money, tenant-consistent
  client parents and actual-router concurrent permission/actor revocation.
  Existing compatibility filters, pagination, historical reads and imports also
  passed in the same run.
- No UI code was changed during this verification. The interface inspection
  documented requester continuity and unrelated directory prerequisites under
  T226; it does not substitute for component or browser acceptance. Browser/MCP
  tools are not currently loaded; existing local browser test assets remain
  available for the later UI verification workflow.
- Formatting and diff checks passed. Session `58485` is running
  `verify-scoped-permissions-all.sh`: all-target SQLx preparation followed by
  core and complete app/integration tests with `SQLX_OFFLINE=true`. This run is
  not yet green; collect this handle rather than starting another build.

Next: collect `58485` for the full offline suite and SQLx regeneration. Keep source frozen
for that snapshot, then continue requester-bound project consumer integration.
No merge, production migration, real-data modification or policy activation.

## 2026-10-06 — Compatibility project read integration

- Previous turn made implementation progress and passed export verification.
  Continued `79370`, which compiled in 4m30s and exited 101: all five new
  compatibility cases failed their intended authorization assertions. Canonical
  legacy-Member reads were empty; legacy Admin and unassigned managed reads
  disclosed projects; malformed canonical state returned 200; financial-only
  revocation retained the budget. No compilation or fixture failure occurred.
- Both compatibility project handlers now use one reader, reusing the existing
  `ProjectReadAccess` organization/active-actor fence and strict stored grants.
  The legacy `AuthUser.org_role` no longer supplies canonical project authority.
  Missing/foreign direct IDs still return 404; unavailable current actors or
  malformed policy return a closed authorization error.
- Count and page share a single filtered materialized set and statement
  snapshot, including empty/out-of-range pages. Scope includes current project
  management and explicitly shared membership. Tenant-consistent client parents
  are required, monetary budgets have independent ordinary-field authority,
  and existing archive/date/client filters and pagination links are preserved.
- Added route-level concurrent organization/actor revocation cases, filtered
  out-of-range pagination, shared hours without money, private report loss and
  foreign project/client-parent denial. Expanded invalid-state tests to missing
  state, unknown grants and unknown policy with explicit 403 expectations.
- `94252` is running all `harvest::` tests against disposable PostgreSQL after
  the focused RED run. Its compile and test results are pending. The shared read
  fence's visibility was widened only enough for its named internal re-export;
  no authority logic, legacy policy activation or real data was changed.
- Inspected the actual ProjectList/ProjectDetail resources using the existing
  design context and the interface-hardening guidance. Recorded the concrete
  global-client/global-user prerequisites, legacy action gating and requester
  continuity gaps in `contracts/project-read-permissions.md`. No UI/CSS edit or
  browser validation is claimed for this inspection. The previously unavailable
  context launcher was not retried within the same session.

Next: collect `94252`, correct actual failures, then regenerate all SQLx targets
and run the complete offline suite. Requester-bound project UI, browser and
native/WASM/full-Nix acceptance remain part of T226 and the full permission goal.

## 2026-10-06 — Project export verification passed; compatibility regressions

- `68138` exited 0 after a 4m04s compile. Passed: two core arithmetic tests,
  96 project tests (12.08s), 18 budget/alert tests (0.84s), 88 invoice tests
  (19.13s), five display tests, the real-session HTTP matrix (16.08s), all four
  canonical materialized-export regressions (1.01s), 12 existing project-export
  authorization tests (4.77s), and 53 streaming database tests (35.60s).
  Total: 279 passed; one existing manual large-volume measurement stayed ignored.
- The four previously reproduced XLSX failures are now green. All five new CSV
  cases passed, including financial-only revocation after backpressure and the
  invalid empty-source snapshot. Existing time/invoice streaming, cancellation,
  size bounds and legacy project authorization also passed in this run.
- Reconciled T224 against the executable canonical read, managed designation,
  shared person/task budget and independent historical tracking cases. That
  reproduction task is complete; T225 still requires refreshed SQLx and T226
  retains actual consumer, compatibility, browser and whole-tree gates.
- Prepared and registered five compatibility-route regressions in
  `harvest/pagination_tests/project_permissions.rs`: canonical reader access,
  legacy-Admin denial, financial-only revocation, current managed designation,
  and malformed state. These use signed-in real router requests and pagination
  helpers. They were registered only after `68138` ended, so are not covered by
  its green result. The API implementation remains unchanged for their RED run.
- Formatting and diff checks passed. Session `79370` is running those five
  compatibility tests against disposable PostgreSQL; do not start a duplicate
  build or count these tests as passed before collecting its result.

Next: collect `79370`, then enforce canonical list/detail
projection with count and page sharing one snapshot. Keep the source/cache and
consumer acceptance gates open; no policy activation, merge or real-data change.

## 2026-10-06 — Canonical project export capture and release

- The preceding conversational turn was a status estimate, not implementation
  progress. Revalidated the worktree and the incomplete delivery patch before
  resuming; that failed patch had not changed its targets.
- `13095` finished with the two arithmetic tests, all 96 project tests, 18
  budget/alert tests, 88 invoice tests, five display tests and the real-session
  HTTP matrix passing. Its four new export cases all reproduced the intended
  authorization failures; none failed in compilation or fixture setup.
- Materialized project exports now use canonical project and financial grants,
  managed-project designations and shared-member visibility, retaining legacy
  behavior only under policy zero. Source size limits and payload remain one
  statement. Workbook release separately checks projects whose rendered cells
  actually contain money, including explicit zero, after rendering completes.
- CSV delivery records monetary project IDs as well as row IDs and revalidates
  both after acquiring channel capacity. The shared project authority check
  owns the organization/actor/parent fences; no authority lock spans backpressure.
- Replaced the legacy project cursor with a canonical source snapshot. Stored
  permission metadata is captured and strictly decoded with the rows, including
  an empty-source sentinel, so repairing malformed authority before fetch cannot
  legitimize an invalid captured source. Monetary masking uses the declaration
  snapshot rather than initial or subsequent grants.
- Added five CSV cases covering authorized legacy-Member export with blank
  money, no inherited legacy-Admin scope, monetary snapshot changes, invalid
  captured state for empty and populated sources, and financial-only revocation
  while a channel is full. These additions have not yet been claimed as passing.
- Formatting and `git diff --check` passed. Verification session `68138` is
  running against a disposable socket-only PostgreSQL cluster. It includes the
  existing project, budget, invoice, display and HTTP stages, materialized export
  regressions, legacy export authorization and all streaming database tests.

Next: collect `68138` and resolve actual failures. Expand export scope/field
coverage where needed, then integrate compatibility list/detail/count and
requester-bound UI under T226. SQLx cache regeneration and full-tree gates remain
pending. No merge, real-data change or permission-policy activation.

## 2026-10-06 — Budget regression correction; export release cases

- Previous turn made progress implementing budget projection and its consumers.
  Continued `81487` until its terminal result instead of starting a duplicate.
  It compiled in 4m34s, then passed 95 of 96 project tests, including the seven
  executable budget authorization/overflow cases and the five-reader races.
- The remaining test failed during setup, not in the reader: its task settings
  supplied both `budget_minutes` and `budget_cents`, violating migration 0030's
  explicit mutual-exclusion check. Corrected both initial and zero-allowance
  writes to populate only the active unit. No production constraint or assertion
  was weakened. Because the verifier stops on failure, this run did not reach
  legacy budget/alert, invoice, display or registered-session stages.
- Traced current materialized/streamed project exports and the compatibility
  API. Their legacy predicates remain a real integration gap. In particular,
  workbook release and CSV blocks retain only project IDs: they cannot yet
  distinguish captured monetary data from retained non-financial project access.
  Keep capacity waits outside authority locks and revalidate protected fields
  after rendering/backpressure, not just at initial query time.
- Added four export regressions in `reports/limits/tests/project_canonical.rs`:
  canonical legacy-Member read, legacy-Admin scope denial, monetary withholding,
  and financial-only revocation while an actual XLSX renderer is paused through
  explicit channels. Registered them only after `81487` had finished. They are
  not yet claimed as executed or reproduced; production exports are unchanged.
- `13095` is live. One compile first checks the corrected project/budget,
  legacy alert, invoice, display and registered-session stages, then runs the
  four next-increment export cases (RED expected). Formatting and diff checks
  passed. SQLx cache still predates the budget/export-test queries.

Next: collect each stage of `13095`, distinguish fixture/compiler failures from
real authorization failures, then integrate captured project/monetary scope into
XLSX and CSV with bounded release checks. Compatibility list/detail/count and
requester-bound UI integration remain part of T226. No merge, real-data change,
policy activation or verified-whole-goal claim.

## 2026-10-06 — Configured-budget authority and summary implementation

- Previous turn made progress by adding the configured-budget regressions.
  Collected both live processes rather than restarting them.
- `35327` exited 0: SQLx preparation and the offline all-target test build
  succeeded; 183 core, 1,305 app and 288 integration/component/CLI tests passed
  (1,776 total). Eleven pre-existing manual probes stayed ignored. The app
  tests took 228.47s. This snapshot predates the budget implementation below.
- `13714` compiled successfully in 4m43s and exited 101. Four assertions
  reproduced actual gaps: authorized legacy-Member budget missing, unmanaged
  legacy-Admin access, revoked allowance/consumption disclosure and another
  person's allocation disclosure. The task-budget case failed in fixture setup
  because `project_tasks.billable` was omitted; corrected that required column.
  Do not count that setup failure as a reproduced authorization defect.
- Added current read-fenced budget projection, reusing the same stored grants
  and managed-project designations as list/detail/spend. The shared raw query
  still serves trusted per-scope alerts; its service call has no user projection.
  Ordinary monetary authority gates both budget and consumption, with arithmetic
  skipped when hidden. Tenant-consistent invoice parents are now explicit.
- `ProjectBudgetOverview` separates whole-project totals from permitted
  breakdown rows. Canonical shared members retain their own person allowance
  but not other-person or individual task allocations. Totals are calculated
  before filtering; hidden money is omitted, including the entire breakdown.
  Legacy policy retains its existing row/field visibility until cutover.
- Added pure `allocated_totals` arithmetic: blank scoped allowances exclude
  their usage, explicit zero includes it, and overflow remains unavailable.
  The two new core tests have passed. The overview consumes the server summary
  without summing permitted details. Existing display components/CSS are unchanged.
- Extended deterministic authority/cancellation coverage to the budget reader
  and the real-session matrix to budget reads, forged requester fields,
  monetary-only revocation, no-project access and inactive/anonymous sessions.
  Adapted legacy budget tests to inspect permitted breakdowns through the real
  new reader; their trusted-alert assertions are unchanged.
- `19433` passed the two new pure arithmetic tests, then exited 101 during app
  compilation: the budget disclosure control still referenced the removed
  `budget_rows` variable. Updated that actual consumer to use the overview's
  permitted breakdown and hide an empty disclosure; no CSS/control defaults
  changed. No app test execution is claimed for this failed build.
- Added cases for hidden monetary overflow, managed financial scope independent
  of project-read-all, and blank-versus-zero task allocations for both hours and
  money. The latter also checks the unchanged trusted alert's per-scope values.
  Unknown/malformed canonical-state coverage now includes configured budgets.
- `81487` is the corrected focused verification against disposable PostgreSQL:
  all project readers, legacy budgets/alerts, invoice arithmetic, display and
  registered sessions. It is still live; formatting and diff checks passed.
  SQLx regeneration and full current-source native/WASM, consumer, export/API
  and browser gates remain open.

Next: collect `81487`, resolve actual failures, add remaining adversarial budget
edge cases and refresh SQLx once the query settles. Continue T225/T226 without
confusing this read increment with full scoped-permissions completion. No real
data mutation, policy activation, merge or new publication occurred.

## 2026-10-06 — Offline cache built; configured-budget regressions added

- Previous turn made progress by collecting the successful spend/session run
  and starting the full verifier. Revalidated `35327`; it was compiling, not
  stalled. SQLx preparation completed in 2m25s, all 183 core tests passed, and
  the complete offline app/test-target compilation succeeded in 5m40s.
- `35327` is now executing the 1,316-test app binary and will subsequently run
  the separate integration/component/CLI binaries. Its final result remains
  pending. The existing 11 manual/large-volume ignores are unchanged.
- Added five configured-budget regressions in
  `projects/canonical_read_tests/budgets.rs`: canonical reads with a legacy
  Member role, current managed-project designation, monetary allowance and
  consumption after financial-only revocation, other-person allowances and
  individual task allowances for shared members. Keep authorized project
  context after monetary revocation; absence is not a numeric zero.
- Wrote the new file without registering it while the full build was compiling;
  registered it only after `35327` reported compilation complete and started
  its already-built test binary. Therefore that full run does not prove these
  five next-increment cases. Their separate RED run is live in `13714`, using
  the existing focused verifier and a distinct disposable database.
- Formatting passed for both budget test files. No production configured-budget
  query or DTO has changed yet. SQLx cache generation predates these new test
  queries and must be refreshed again after the budget implementation settles.

Next: collect `35327` and `13714`; preserve exact failure evidence, then implement
the configured-budget projection under current read authority with a complete
summary independent of allowed detail rows. Preserve trusted alert calculations
and verify monetary withholding, monthly periods and unallocated-versus-zero
budgets. No full-goal completion, publication, merge or activation is claimed.

## 2026-10-06 — Spending regression and session verification passed

- The preceding estimate turn was a verified wait: `49640` was still live.
  Continued that process without restarting compilation.
- `49640` exited 0: 88 project tests passed (10.00s), 88 invoice tests
  passed (18.86s), five budget-display tests passed and the complete
  registered-session authorization matrix passed (16.09s). This includes
  spending's eight new regressions, the four-reader concurrency/cancellation
  cases, exact invoice arithmetic and same-session financial-only revocation.
  Compilation took 6m36s. These results do not cover configured-budget
  permissions, exports, compatibility delivery or the whole permission goal.
- Started `35327` using the existing `verify-scoped-permissions-all.sh`:
  prepare every server/test SQLx target, then run the core and complete app
  suite with offline SQL checking against a disposable socket-only database.
  This also compiles the adjusted navigation-test DTO. No real data is used.
  The run is not yet claimed green; no new source changes will be mixed into
  its verification snapshot.
- Traced configured-budget consumers: the overview is the only interactive
  caller, while `configured_progress` independently supplies trusted alerts.
  Preserve that service path and its per-scope threshold tests when adding
  current-user projection. The present flat DTO cannot safely represent a
  complete project summary by summing a member-filtered person/task breakdown.

Next: collect `35327`, fix any actual cache/regression failures, and continue
T224/T225 with configured-budget authorization and separate summary/breakdown
semantics. Native/WASM lint, actual consumers and export/API verification remain
required before publishing the read increment. No merge or policy activation.

## 2026-10-06 — Project read delivery verified; spending regressions

- Previous turn made progress by adding actual-session and deterministic
  concurrency coverage. Revalidated `11584` and `52467`; no duplicate build.
- `11584` exited 0: all 80 project tests passed (11.57s), including the foreign
  client-parent case and the four concurrency/cancellation tests across all
  three readers. The complete registered-session matrix also passed (37.04s),
  including the new project list/detail/tag and protected-field revocations.
  Compilation took 5m53s. This is not evidence for spend/budget/API/export or
  full permissions completion; their tasks stay open.
- Added five next-increment spending regressions for canonical legacy-Member
  reads, financial-only revocation, managed project designation, no-entry
  zero-versus-withheld values and report-only/shared-member separation. `40226`
  is compiling their RED verification against the unchanged spend reader in a
  disposable PostgreSQL cluster. No successful execution is claimed yet.
- Clarified OP09's ordinary spending projection from existing FR-008/021 and
  the operation matrix, without treating it as a new report-family grant.
  Reuse current project visibility and preserve exact rate/rounding/invoice
  precedence. UI must distinguish unavailable money from an actual zero.
- Impeccable context launcher is unavailable because its engine is not
  installed in a writable cache. Read the existing DESIGN.md and hardening
  playbook directly; no engine install or design-system change. PRODUCT.md
  is absent. UI changes will retain the incumbent utility classes/components.
- `40226` exited 101 after successful compilation (5m47s): all five new
  spending tests failed on their intended assertions. Current legacy Member
  lost an authorized row; legacy Manager disclosed unmanaged progress; legacy
  Admin and shared/report-only grants disclosed 12,345 cents; an empty project
  had no explicit zero/withheld projection. No setup failures were substituted
  for RED evidence.
- Implemented current spending authority under the existing read fence,
  tenant-consistent contributors and SQL aggregation. `spent_cents` is now
  optional and omitted when withheld; monetary arithmetic is conditional so
  hidden overflow cannot suppress authorized hours. Authorized empty projects
  now receive explicit totals. The UI preserves absent money rather than
  defaulting it to zero; no CSS, tokens or shared control defaults changed.
- Extended coverage to financial-only concurrent revocation, foreign entry
  parents, hidden/authorized overflow, all four-reader fence orderings and real
  session spend responses. Kept invoice/rate arithmetic expectations exact while
  adapting their DTO assertions. `49640` is compiling the combined project,
  invoice, display and registered-session verification; no GREEN claim yet.
- Exact Nix repetition `52467` exited 0: 183 core, 1,280 app and 288 separate
  integration/component/CLI tests passed; 11 pre-existing manual probes remain
  ignored. The original mail failure was not reproduced and no fix is claimed.
  Together with the previously passed frozen-snapshot browser, Clippy, SQLx,
  formatting and both VM checks, the editor verification gates are satisfied.
- Verified the frozen Nix Rust/core source and SQLx blobs match commit
  `2497dbe` (`30792`, exit 0), then pushed only that commit to the existing
  `feat/scoped-permissions` branch (`23781`, exit 0). PR #212 remains a draft;
  new read/spend work is uncommitted and unpublished. No merge or activation.
- Reopened the official Harvest budget and Projects overview guides. Shared
  members retain project-wide hour totals but only their own person allowance;
  individual task/other-person budgets stay private. Recorded the need to
  separate summary and permitted breakdown so filtering rows cannot distort
  project totals. Also recorded the existing blank-allocation summary gap:
  unallocated work must not consume allocated budget; preserve its detailed
  row and the independent alert calculation. No new product rule was guessed.

Next: collect `49640`, resolve genuine spend/invoice regressions, then continue
configured-budget projection independently of trusted alerts. Regenerate SQLx
and complete current-source consumer/export/API verification before publishing
this next increment. The complete permissions goal remains unfinished.

## 2026-10-06 — Project read concurrency and registered-session coverage

- The preceding user-facing estimate was a status-only turn, not progress.
  Revalidated the worktree and live `32222` before continuing. Its project run
  passed 75 tests, including 11 canonical regressions; the later foreign-client
  parent case was compiled by its next stage but had not yet executed.
- The isolated mail loop was recompiling on successive Cargo invocations.
  Stopped that diagnostic intentionally (`32222`, exit 130), rather than
  waiting through 50 rebuilds. Its disposable database exited with the runner.
  Running the already-built binary instead passed all 50 isolated repetitions
  (`67261`, exit 0). No production notification code or assertion changed.
- Thirty traced batches of the three sendmail tests also passed (`6667`,
  exit 0; 90 executions). Expanding the trace to all nine non-database mail
  tests produced a different failure: the verbose-output test exceeded its
  five-second deadline under instrumentation (`26407`, exit 1; eight passed).
  This does not reproduce or explain Nix's original `Transport` result.
  The next trace excludes only that instrumentation-sensitive volume case;
  its ordinary full-suite assertion remains unchanged.
- Added deterministic lock-dependency tests for list/detail/tag reads:
  organization-first grant revocation, actor-first deactivation, reader-first
  materialization with organization/actor fences and cancellation release.
  They use actual PostgreSQL waits, not timing sleeps.
- Added a registered-session matrix to the existing single-AppState HTTP
  harness: canonical reads despite legacy Member, restricted legacy Admin,
  foreign direct IDs, anonymous/inactive sessions, forged viewer fields,
  current monetary-field revocation and subsequent project-read revocation.
  These additions are not yet claimed as passed.
- Verification `11584` compiles once against a fresh disposable PostgreSQL,
  then executes all project tests and the complete registered-session matrix
  using that binary. This includes the previously unexecuted foreign-client
  parent regression. Formatting passed; SQLx regeneration and broader delivery
  integration are still pending.
- The earlier independent Nix snapshot also passed both VM checks: deployed
  application (88.16s) and OIDC (43.57s). Its overall result remains failed due
  to the recorded notification test; there is no full-green Nix claim.
- `98041` finished with exit 0: 30 batches of eight concurrent non-database
  mail tests (240 executions). Traces are in
  `/tmp/horae-mail-trace.Hf4VLT`; original three-test traces are in
  `/tmp/horae-mail-trace.DpCO75`. The original transport failure remains
  unreproduced, not fixed. A single controlled repeat of the exact failed Nix
  derivation is live in `52467`; its frozen editor source is unchanged and
  excludes the new project-reader work. Do not retry repeatedly until green.

Next: collect `11584` and the exact Nix reproduction `52467`; resolve genuine
failures without weakening acceptance. Continue scoped spend/budget projections,
SQLx and actual consumers/export/API integration. Keep `2497dbe` unpublished until
its independent gate failure is resolved. No real data/policy activation or merge.

## 2026-10-06 — Canonical project reads corrected; independent mail-test failure

- Previous turn made progress: saved `2497dbe`, traced the next read boundary
  and reproduced five real-reader failures (`40067`). Continued from the same
  worktree and live verification; no duplicate full Nix run.
- Implemented current project-read loading under organization SHARE followed
  by active actor SHARE, using strict stored grants and bounded transaction
  settings. List/detail/tag queries now distinguish canonical managed/all and
  shared-member visibility from legacy roles. Private notes use explicit
  administrative identity; project rates and monetary budgets require current
  project-owned financial grants. Tracking keeps its separate identity path.
- `59405` passed all five original RED regressions (exit 0; compilation 4m13s,
  tests 1.28s). Added six further tests for managed designation/revocation,
  scoped tags, private fields, financial projection against the pure ownership
  evaluator, invalid policy/grants and shared-report versus historical-tracking
  identity. The expanded all-project run is live in `32222`, followed by 50
  isolated executions of the failing mail test. These further tests are not
  claimed as passed yet; complete SQLx and cross-surface acceptance remain open.
- Nix `2994` terminated with exit 1. Browser, Clippy, SQLx cache validation and
  formatting passed, but the app suite reported 1,279 passed, one failed and 11
  existing ignored. The failure is
  `notifications::tests::sendmail_failure_does_not_expose_program_output`:
  expected `Rejected`, received `Transport`. Its script already uses the Nix
  shell path and consumes stdin, so neither a missing `/bin/sh` nor an early
  script exit is established as the cause. No assertion was weakened and no
  speculative transport fix was applied. Core passed all 183 tests; the app
  failure prevents claiming the later separate test binaries in this Nix run.
- Nix continued through its other gates with `--keep-going`; the deployed
  application VM test finished successfully. Preserve the failed check and
  diagnose it before publishing `2497dbe`; no full Nix success is claimed.

Next: collect `32222`, fix any genuine expanded-reader regression and obtain
evidence for the mail transport failure. Finish project spend/budget projections,
registered-session/revocation tests, SQLx and real consumers before considering
T224–T226 complete. The editor commit remains local and no real policy/data is
changed.

## 2026-10-06 — Editor checkpoint and next project-read regressions

- Previous goal turn was a verified wait: polled live Nix handle `2994` while
  answering the estimate. Revalidated that handle; no build was restarted.
- The complete independent release-package browser suite passed, including
  permission recovery, scoped Reports, editor revocation/replay and hidden
  inactive-budget preservation. Nix Clippy also passed. `2994` has moved to
  SQLx preparation; remaining cache/tests/VM/format gates are not yet complete.
- Saved the locally verified editor increment as unsigned commit `2497dbe`
  (`Enforce scoped permissions in the project editor`). No push or merge.
  Its application source matches the frozen editor increment under Nix
  verification; subsequent project-read tests are separate uncommitted work.
- Traced the actual `/projects/:id` route, overview resources, configured-budget
  service reuse, project CSV/XLSX release and compatibility API list/detail.
  Added `contracts/project-read-permissions.md` and T223–T226 with current
  official sources and explicit gaps. Shared-member budget limits and
  financial-only export revocation need more than changing the editor guard.
- Added five production-reader regression tests for canonical Member reads,
  legacy Manager overexposure and legacy Administrator financial disclosure.
  `40067` exited 101: all five ran and failed on their expected assertions after
  successful compilation (5m23s), not on setup or build errors. Canonical
  `ProjectReadAll` with legacy Member returns an empty list/missing detail;
  own-only canonical grants with legacy Manager disclose both; canonical
  project-read-only with legacy Admin discloses rate and fee-budget values.
  The disposable PostgreSQL fixture was stopped by its exit trap. No runtime
  read implementation or real policy/data changed. T224 remains open for
  managed/shared projection and full delivery-path acceptance.
- Nix SQLx preparation/cache validation passed; `2994` is now building the
  independent full Rust test derivation. Its verified snapshot predates these
  deliberately RED next-increment tests. Do not report the current uncommitted
  tree as green or publish those tests as a completed read integration.

Next: implement the scoped list/detail/tag read boundary against these five RED
cases, including current authority and protected-field projection; collect
`2994` independently and resolve any actual editor gate failure before publishing
`2497dbe`. Do not include unfinished reader tests in that publication or infer full
permissions completion from either increment.

## 2026-10-06 — Full local regression passed; rebuilt browser verification

- Previous turn made concrete progress by repairing complete offline SQLx
  generation and recovering Nix source evaluation. Re-polled the existing live
  handles `75514` and `2994`; no duplicate builds were started.
- `75514` completed the offline app suite: 1,280 passed, zero failures, 11
  pre-existing ignored tests (not claimed as executed). All separate test
  binaries passed too: AdminShell 11, approval labels 3, CLI imports 5, CLI
  restart 1, detail navigation 25, import jobs UI 35, integration 36, new-project
  form 73, own permissions 11, audit UI 5, permission editor UI 55, scoped Reports
  UI 19, timer widget 1 and shared trigger utilities 8. Core's 183 tests passed
  earlier in this same run. No offline query error remains.
- The existing chain is now building the corrected Dioxus app before running
  every default browser suite. The server-test result does not substitute for
  post-fix browser acceptance. The independent Nix check `2994` has completed
  its client build and remains live; package/server and all later gates are not
  yet reported as passed.
- Read-only GitHub check confirms PR #212 remains OPEN/DRAFT on
  `feat/scoped-permissions`, published head `de8f9ad839879c35cdec19c0f40888cb4b948df8`.
  Local editor integration and cache changes remain unpublished. No merge or
  real-data/policy activation.
- `75514` exited 0. Complete offline Rust verification, corrected Dioxus build
  and all 24 default browser suites passed in one chain. The new hours-only
  budget browser assertion is GREEN against the rebuilt app after RED `54183`
  against the old binary. Explicit zero/reset, exact response-loss replay,
  canonical Member access, revocation/reload and historical storage preservation
  all pass alongside legacy forms, shared controls, Timesheet and Reports.
- Totals: 183 core + 1,280 app + 288 separate integration/component/CLI tests =
  1,751 passed. The 11 ignored tests are existing explicit large-volume/manual
  measurements; none were silently skipped or changed for this increment.
  `2994` is still running its independent release-package browser suite and
  later Nix gates. Keep T161 open until those gates are resolved; do not restart
  the completed local chain.

Next: collect `2994`, resolve any actual Nix failure, then
publish the verified existing-editor increment without claiming the remaining
permissions surfaces, creation contracts or full policy activation are complete.

## 2026-10-06 — Full offline build exposed incomplete query metadata

- The previous turn made concrete progress: fixed inactive parent-money loss,
  verified 99 editor/creation tests and repaired the timer-date browser fixture.
  Recovered the existing `92817` handle. Core tests passed (183), but the full
  server build exited 101 with 108 missing-query errors in `tests/integration.rs`.
  Its chained Dioxus build and full browser run did not execute.
- This contradicts completeness of the earlier SQLx preparation, not its exit
  code: `28776` really passed prepare and lint, but fresh offline integration
  compilation proves those gates were insufficient. There were 94 tracked cache
  removals, including still-used integration queries. Do not publish that cache
  or restore unrelated stale descriptors merely to make compilation pass.
- Inspected the pinned CLI (`sqlx-cli-sqlx 0.9.0`), local SQLx macros (0.8.6),
  Cargo targets and source timestamps. SQLx is optional behind `server`, while
  the CLI's [metadata reader](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-cli/src/metadata.rs)
  does not forward feature flags. Its [selective recompile step](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-cli/src/prepare.rs)
  discovers macro dependents from that default graph. Unchanged integration
  targets can stay cached while prepare removes their query descriptors.
- The scratch verifier now refreshes modification timestamps of the existing
  app/test roots before the standard all-target, server-feature prepare. No
  source content, dependency, application behavior or real database is changed
  by that refresh. It uses a fresh migrated disposable PostgreSQL cluster, then
  verifies the generated cache through offline core/server tests.
- New run `75514` is active: full regeneration, complete tests, rebuilt Dioxus
  binary, then every default browser suite. `92817` is terminal; do not resume or
  duplicate it. No successful full offline or post-fix browser result is claimed
  yet. Nix gates and publication remain pending.
- Default-feature Cargo metadata confirms no SQLx edge for Horae (`70326`,
  exit 0). Forced preparation completed in `75514`; only the three superseded
  production-query removals remain, instead of 94. All 183 core tests passed
  again and the complete offline server test build is compiling.
- Documented the timestamp precaution in `AGENTS.md` and updated the existing
  scratch editor verifier so subsequent iterations cannot repeat this omission.
  Registered the new editor/query files as intent-to-add solely for Nix's source
  snapshot. No commit or publication. Repository-wide `nix fmt -- --ci` passed
  before the final documentation update; source formatting is unchanged.
- Nix's initial no-build evaluation failed on an absent filtered-source store
  path, including with refresh/evaluation-cache disabled. Materializing
  `packages.x86_64-linux.default.src.outPath` succeeded (`7476`), without changing
  Nix definitions. The repeated no-build evaluation then passed every native
  derivation (`35539`); this proves evaluation only, not the build/tests.
- Full native Nix check `2994` is now running with `--max-jobs 1 --cores 2`.
  Dependencies were fetched and package compilation started. Its source snapshot
  includes all new files through intent-to-add; source implementation is frozen
  while both verification paths run. `75514` remains the separate offline test,
  Dioxus rebuild and default-browser chain. Neither has a terminal result yet.
- Rechecked the next project-read boundary against the current
  [Harvest permission reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
  and [budget guide](https://support.getharvest.com/hc/en-us/articles/360048686811-How-to-set-project-budgets).
  The new guide separates project and rate permissions; the budget guide still
  describes legacy roles and separately shared member progress. Current
  `projects_for_viewer`, `fetch_project_details` and tags still consume the legacy
  `project_read_access` view. Editor verification therefore cannot establish
  OP08/09 completion or justify activation. No new custom-grant inference or
  production behavior change was made from this source recheck.
- `75514` completed the full offline test build in 4m19s and began its 1,291
  app tests. This verifies compilation of the previously failing integration
  target against the regenerated cache; test execution and subsequent browser
  results are still pending. The Nix chain remains independently live.

Next: collect `75514` and `2994`, confirm all offline targets compile/run, then
complete browser/Nix gates before publishing the
integrated editor. Preserve the full permissions scope beyond T159/T160.

## 2026-10-06 — Inactive budget preservation and timer test date

- The previous user-facing estimate was a status-only turn (no implementation
  progress). Revalidated the existing worktree and recovered orchestration cell
  `5616`: regression `44552` exited 101, reproducing Hours-to-Hours erasure of
  `Some(0)` parent monetary storage. No process restart was inferred from silence.
- Canonical non-monetary budget edits now preserve inactive parent cents using
  the existing monetary-mode predicate. Legacy behavior and authorized monetary
  transitions are unchanged. Added a fee-to-hours control for denial without
  financial authority, allowed conversion, exact retry and denied replay after
  financial revocation. Verifier `28776` is running against a disposable database;
  its success is not yet claimed.
- Browser `5435` exited 1 in the delegated timer assertion after passing the
  earlier suites. The fixture selected PostgreSQL CURRENT_DATE in Europe/Madrid
  (October 6), while production timer commands use UTC (October 5). The test now
  reads the timer's actual stored date and opens that sheet, asserting its running
  entry is present before testing disabled controls and direct-request denial.
  No timer behavior or permission assertion was relaxed. Continuation `31978`
  runs Timesheet and remaining permission suites on a new disposable database;
  its editor binary predates the pending parent-budget fix.
- Reused the existing read-only reviewer for the parent-money fix and remaining
  existing-editor field/effect review. No new agents, real-data writes, policy
  activation, publication or merge.
- `31978` exited 0: Timesheet (including both delegated denial and owner-only
  terminal recovery), permission recovery, scoped Reports and canonical editor
  browser suites passed. The timezone mismatch was confirmed by the same run in
  the Madrid/UTC date boundary, without changing production timer behavior.
- The reviewer found no remaining high/critical existing-editor effect defect.
  `28776` passed the parent-budget regression, all four concurrency tests, the
  registered HTTP matrix and all 99 creation/editor tests, including the
  authorized monetary-to-hours/revoked-replay control. Cache/lint gates remain
  live and are not yet reported as passed.
- Extended the real editor browser fixture with hidden inactive parent cents
  and an explicit hours edit. Pre-fix binary run `54183` exited 1 exactly at the
  storage assertion: expected `86753`, got `null`; earlier form phases passed.
  This is an intentional additional RED reproduction, not a failure of the new
  server source. Rebuild and GREEN verification are still required.
- `28776` exited 0: all 99 editor/creation tests, registered HTTP and concurrency
  checks, complete SQLx preparation, offline native/all-target and WASM Clippy,
  and Rust formatting passed. Targeted repository Markdown formatting also
  passed. T159/T160 are closed for existing-project integration; T161's full
  regression/browser/Nix verification remains open.
- Started `92817`: complete core and server tests on a new disposable PostgreSQL
  cluster, then the existing pinned Dioxus build script, then every default
  browser suite against that rebuilt binary. It initially waited on the existing
  verifier's Cargo lock; `28776` has now completed. Keep this handle and inspect
  its next output rather than launching another build. No full-suite success or
  post-fix browser success is claimed until it completes.

Next: collect `92817`, resolve any real regression and complete the Nix gates
before publishing the integrated editor. Keep the full permissions scope open, including creation,
remaining consumers, approval/lock integration and activation review.

## 2026-10-06 — Concurrency verified; browser regression continuation

- The interrupted goal turn made concrete progress: added both concurrency
  orderings, corrected reviewer-identified assertion confounds and ran the
  expanded checks. Recovered the existing `35845` handle rather than restarting:
  exit 0, all four concurrency tests (12 cases), registered HTTP matrix, all 97
  creation/editor tests, complete SQLx preparation, offline native/all-target
  and WASM lint, and Rust formatting. No production change was needed for those
  race cases; the real transaction protocol passed.
- Default browser run `55125` exited 1 in `new-project-permissions`: its exact
  cost-field label still expected `· admins only`, removed by the scoped-cost
  integration. Confirmed the production control retains its person/currency
  accessible label. Updated both stale test locators without weakening legacy
  role visibility, response redaction, draft ownership or mutation assertions.
- Started the corrected failing suite and every unexecuted successor on a fresh
  disposable database, using the same verified production build. Earlier default
  suites passed up to this failure; this continuation is not yet a complete
  single-run default-suite success. No real data or Harvest mutation.
- Browser continuation `5435` passed the corrected real-session permission
  matrix and has progressed through task-error and keyboard/billing branches.
  It is still live; later suites remain unverified.
- Existing-editor effect review found one real gap: explicit Hours/None budget
  intent can clear inactive parent `projects.budget_amount_cents` without a
  billable write grant. Child cents already have an effect check, but the parent
  value is neither represented in that intent nor protected by its persistence
  flag. Added a save/replay regression across Hours/None transitions and
  NULL/zero/positive parent amounts. RED `44552` is compiling; production is
  unchanged pending reproduction. The proposed fix preserves inactive parent
  cents for canonical non-monetary-to-non-monetary intent, while preserving the
  existing authorized fee-transition semantics and legacy behavior.

Next: collect `44552`, fix the parent-money preservation gap with a fee-transition
control, and collect `5435` without restarting it. Then finish existing-editor
field/effect review and the full default/Nix gates. The
full permissions goal, creation contracts and remaining surfaces stay open.

## 2026-10-05 — Canonical editor revocation races

- Previous goal turn made concrete progress: canonical Member HTTP access was
  corrected with legacy denial preserved; `77415` passed HTTP, 93 creation/editor
  tests, native/WASM lint and Rust formatting. Browser `56210` passed twice with
  disposable fixture cleanup. Revalidated the current worktree and reused it.
- Added `editing/tests/canonical_concurrency.rs`: read, save and receipt replay
  each race against project-write grant loss, designation removal and financial
  access loss. Retaining project-read on write revocation preserves the accepted
  designation retention rule. Designation loss uses the production composable
  command; grant fixtures publish their change with the organization revision
  under the AccessChange gate.
- Tests observe PostgreSQL blocking with the existing helper, commit revocation,
  require the old repeatable-read/serializable snapshot to fail, then retry with
  current authority. Financial-only loss still permits a freshly redacted read;
  writes and receipt replay must deny. Project rows, task rates and editor
  receipts must remain unchanged after the rejected operations.
- `72129` is compiling the nine cases, then HTTP and complete creation/editor
  regressions, SQLx preparation, offline native/WASM lint and formatting.
  Requested a focused read-only review from the existing reviewer. No runtime
  change or passed result is claimed for this increment yet.
- Review found that project-grant/designation loss could also remove effective
  financial authority, masking the intended guard. Those branches now retain
  all-project financial authority; only FinancialAccess removes it. The saved
  snapshot now includes `project_tasks`, where task rates actually live, as well
  as settings. Initial `72129` passed the original three tests, then exited 101
  while compiling this stronger snapshot: `project_tasks` has a composite key,
  not `id`. Corrected its ordering to `task_id`; no production defect is inferred.
- Added reverse-order coverage for read/save/replay: hold a project table lock,
  observe the editor waiting after its org gate, then observe the production
  designation command waiting on that editor. Releasing the first blocker must
  let the authorized editor finish before revocation; later access must deny.
  This uses actual lock dependencies and JoinSet cleanup, not timing sleeps.
- Reviewer confirmed the authorization/snapshot confounds are fixed and found
  no confirmed reverse-order defect. `54444` exited 101 before execution because
  `pg_stat_activity.pid` is nullable in SQLx metadata; the test now explicitly
  requires the observed blocked backend ID. The corrected full runner is started
  again; results are pending, not inferred from the earlier three-test snapshot.
- Complete default browser runner `55125` is live against the already verified
  production build (only Rust tests changed since that build). Early design,
  responsive/shared-control and bulk-action suites pass; remaining suites are
  not yet verified. This adds no browser access to real data or Harvest.

Next: collect the corrected runner, resolve any test/review findings before
concluding the editor concurrency audit. Full effect review,
other permission surfaces, creation contracts and activation/Nix gates remain
open. No publication, merge or real-data changes.

## 2026-10-05 — Browser acceptance and public editor authority

- Previous user-facing turn was an estimate, not implementation progress. The
  live browser handle `24434` has now been collected: exit 0, including revocation
  and explicit reload. Build `35638` completed before that run.
- Adversarial review found a medium test defect: the cost-read-only assertion
  used an archived teammate, whose fieldset independently disables editing.
  Added an active retained teammate with a populated cost override, checked its
  withheld response and disabled read-only control, then an authorized explicit
  reset alongside a project-rate zero and exact lost-acknowledgement replay.
- Corrected browser run `80064` passed all four phases against the freshly built
  server in headless Chromium and a disposable database. Historical records and
  protected storage comparisons pass. This is not a Windows Chrome/MCP test or
  evidence for the complete permission matrix.
- The registered editor endpoints still reject legacy Members before their
  canonical grants are evaluated. Added public-route coverage for a canonical
  Member, legacy Member denial/Manager compatibility, anonymous reads and revoked
  reads, retaining existing requester-switch/retry tests. RED runner `60259`
  reproduced the canonical Member rejection: exit 101, HTTP 403 with `Manager access required`, after the legacy compatibility checks passed.
- Replaced only `load_project_editor`/`save_project_editor` public role prechecks
  with session authentication. `lock_editor_actor` still evaluates legacy roles
  for policy 0, and current project grants/scope for policy 1 under transaction
  locks; creation stays unchanged. GREEN `77415` runs the registered HTTP matrix,
  creation/editor suite, native/WASM lint and formatting. Browser fixture now
  uses a legacy Member with explicit grants and restores/removes only its
  disposable fixtures on exit. Fresh build/browser verification is pending.
  Requested focused read-only review from the existing reviewer. No deployment
  or real policy activation.
- `77415` passed the registered HTTP/session matrix and all 93 creation/editor
  tests; native/WASM lint is still running. The independent reviewer found no
  confirmed issue in the public guard adjustment or active-cost assertions.
- Fresh build/browser `65206` passed all four phases with a legacy Member, but
  exited 1 during fixture cleanup because assignments do not cascade from the
  project. Corrected teardown to remove only this fixture's assignments first.
  `56210` then passed the suite twice in the same disposable database, including
  both teardowns (exit 0). Added this suite to the default browser runner. No
  complete default-suite or Windows Chrome acceptance is claimed.
- `77415` completed with exit 0: registered HTTP/session matrix, all 93
  creation/editor tests, native/all-target and WASM lint, and Rust formatting.
  Shell syntax and `git diff --check` also pass. No SQL queries or migrations
  changed in this increment; it reused the previously prepared offline cache.
- Next audit evidence: `project_creation/locking_tests.rs` currently establishes
  concurrent legacy role loss, while canonical editor tests establish sequential
  grant/designation loss. Add explicit canonical grant/designation revocation
  races for the composite read/save/replay transaction; catalog-only races do
  not prove that scope. Project list/detail still contain legacy role-based
  controls and must not be counted as fully integrated by this editor result.

Next: add and run the canonical editor concurrency cases above, then finish the
effect/cross-surface audit before closing T159/T160/T161. Keep creation predicates
and full-feature cross-surface/activation/Nix gates open. Work remains local and
unpublished; no merge or real-data changes.

## 2026-10-05 — Retained responsibility controls

- Previous goal turn made concrete progress: corrected real-link navigation,
  passed all 69 component tests and started `6246`. Revalidated that same runner
  as live; it has now passed all 93 creation/editor database tests and the actual
  registered HTTP/session matrix. SQLx preparation/native/WASM checks remain.
- Added three production-page regressions for visible/removable outside-team
  managers, archived in-team manager removal without unlocking tracking fields,
  and a manager checkbox surviving removal of its tracking membership. These
  reuse the approved independent-selection contract and existing backend tests;
  no new eligibility rule, membership promotion or visual redesign is proposed.
- RED runner `79530` is queued behind `6246` on the shared build directory. The
  last 69-test success predates these three new cases; do not call them verified.
- `79530` exited 101 with exactly the three new regressions failing; seven
  existing manager cases passed. Implemented session-local identity retention,
  responsibility-only rows in the existing Team panel and a shared toggle that
  updates canonical selection independently of tracking membership. Unchecking
  and rechecking remain possible before saving; no new modal, endpoint or CSS.
- Archived team rows now keep only tracking controls disabled; canonical
  retained responsibility can be removed or locally restored. Archived people
  without an original designation remain disabled, including synthetic-event
  handling; the legacy archived checkbox remains disabled too. Added coverage
  for that negative case and undo before save.
- `6246` has passed SQLx preparation and native/all-target lint, and is queued
  for its final WASM check. `82168` is compiling the manager GREEN component
  suite. Late changes (negative test, fieldset label) may postdate its snapshot;
  final verification must cover the complete current tree. Requested a focused
  read-only review from the existing transaction reviewer, with no new agents.
- `6246` completed successfully, including WASM lint. `82168` passed all 73
  component tests, including the archived-negative case. The final current-tree
  rerun `51876` also passed all 73 tests plus native/all-target and WASM lint.
  Independent source review found no confirmed high/medium finding; it did not
  run tests or claim browser evidence. No SQL change was made in this UI increment.
- Added `tests/browser/project-editor-permissions.cjs` for actual canonical saves
  in the established isolated runner: withheld/read-only finance, archived/outside
  manager removal, keyboard/narrow viewport, explicit zero, lost-acknowledgement
  replay and same-session revocation/explicit reload. It checks protected storage
  and historical records and refuses non-disposable DBs. Node syntax check passed;
  runtime acceptance is still pending, and it is not in the default suite yet.
- Browser build `35638` is live, using the existing `.scratch/build-report-browser.sh`.
  The last checked server/WASM files predate these changes and must not be used
  as proof for the new browser suite. 167 GiB is available; no cleanup is needed.

Next: collect `35638`, then run `run-design-checks.sh project-editor-permissions`
with the freshly built `target/dx/horae/debug/web/server`, pinned Playwright
`/nix/store/i6xnc927g8yrwq0fws8wbmjvj4y36fq5-playwright-test-1.60.0/lib/node_modules/@playwright/test`
and browsers `/nix/store/2br1n8v0cx3zlqs4bik69d5fm2qm0z3m-playwright-browsers`.
Use the Nix shell and a fresh disposable cluster; no preview DB or Harvest writes.
Full-form browser acceptance, public guard cutover and feature-wide gates stay open.

## 2026-10-05 — Project navigation boundary verification

- Previous turn produced new evidence: `78694` exited 101 with 67/68 component
  tests passing. Its database/HTTP/cache/lint commands did not execute.
- Corrected the navigation regression: calling `MemoryHistory::push` directly
  does not notify the router. The harness now clicks a real `Link` in a test
  layout. `44238` still reproduced the failure (67/68): the link became current
  but the previous project's unavailable screen remained. Thus the corrected
  regression establishes the production defect, unlike the earlier history-only
  probe.
- Inspected the pinned Dioxus reconciliation code: identity keys are handled in
  keyed fragments, not a lone component. `EditProject` now places the loader in
  a one-element keyed fragment, keeping each project's resources and invalidation
  state separate without changing shared controls or CSS.
- Added a second real-link case that navigates while a save response is pending;
  the old handler must be cancelled and cannot navigate the new form away.
  `2209` passed the 68-test snapshot before this final added regression.
- `6246` passed all 69 component tests, including the pending-save navigation
  case, and is now compiling the disposable-database suite. It also includes
  the actual registered HTTP/session matrix, all-target SQLx preparation and
  native/all-target plus WASM lint. Those later stages are not yet verified.
- No Playwright/browser MCP navigation tools are registered in this turn;
  component evidence is not browser acceptance. Reuse the documented direct
  browser client when performing that acceptance rather than assuming a loaded
  MCP connection.

Next: collect the live `6246` database/session/cache/lint runner without restarting
it, then finish archived/outside-team manager controls. Existing
T159/T160/T161 and full-feature activation/review gates remain open. No merge,
publication, policy activation or real-data mutation.

## 2026-10-05 — Requester-state invalidation and explicit reload

- `51401` finished successfully: 93 creation/editor tests, registered HTTP/session
  matrix, complete SQLx preparation and native/all-target plus WASM lint. This
  closes the verified budget-preservation increment, not all form acceptance.
- `68612` reproduced three UI failures: first/later-page people mismatches and
  save-time session/access loss kept the prior form visible. The ordinary
  400/409/500 preservation control passed. Terminal exit 101.
- Added one shared session-change error reason for backend save/catalog failures
  and local response mismatch checks. No requester identities are returned.
  A local editor boundary invalidates on 401/403 or that explicit 409 reason;
  ordinary conflicts and ambiguous failures keep their existing recovery path.
- Invalidation cancels/clears the parent resource and unmounts local form,
  options and pending request state. Reload is explicit; generation-bound
  callbacks and a local latch prevent stale results crossing the boundary. The
  message does not claim that an earlier uncertain save failed. No CSS changes.
- `32544` passed all 65 production-page tests. Review found no confirmed
  high/medium issue but requested route and delayed-response verification.
- Added controlled pending-read cancellation and cross-route cases. `39598`
  passed ten cases, including cancellation, but reproduced a route defect:
  unavailable state from project A prevented opening B. Keyed the loader itself
  by project identity. Added delayed-save-response cancellation coverage too;
  it postdates `39598`. No browser evidence is claimed from these component tests.

Next: collect `78694`, the final component/database/HTTP/cache/native/WASM runner, then
review the loader boundary and finish archived/outside-team manager controls.
The full permissions goal remains active; no activation, publication or merge.

## 2026-10-05 — Retained member budget preservation

- Previous iteration made concrete progress: reproduced and patched stale parent
  budget totals, added boundary/authority tests and obtained independent review.
  Revalidated `23212` as live; no duplicate runner or restart.
- Added real-save/replay coverage for a retained inactive member budget during
  a canonical name-only edit, plus a control proving an explicit hour-budget
  mode change still clears inactive member budgets. These tests postdate the
  first compilation in `23212`; its subsequent `project_editor` filter includes
  both new tests and will compile them before running. The preservation case is
  expected RED until the member-budget write honors the original keep marker.
- `23212` passed all 91 creation/editor tests, including the four new parent
  aggregate regressions and their nonzero/zero/None plus revocation cases. It is
  now compiling the second filter with the new member preservation/control tests.
- Corrected the planned verification scope: the `project_editor` filter does not
  select the registered HTTP matrix; that helper runs inside
  `job_endpoints_enforce_session_role_and_organization`. The final GREEN runner
  must select that actual test name explicitly; do not claim HTTP coverage from
  a matching module filename or the narrower editor filter.
- `23212` exited 101 after reproducing the exact new failure: the retained
  budget row was absent instead of 125 minutes. The explicit mode-change control
  and three other editor-filter cases passed. Cache preparation/lint did not run.
- Extracted only the existing member-budget write tail and call it after the
  assignment/cost save when Budget is not kept. Canonical preservation skips that
  write; explicit edits and legacy requests retain their previous behavior.
  The member-removal loop and its FK/history/authority checks are unchanged.
- GREEN runner `51401` is live: complete creation/editor suite, actual registered
  HTTP/session matrix, all-target SQLx preparation and native/WASM lint.
- Its 93 creation/editor tests and the registered HTTP/session matrix have now
  passed. SQLx preparation and lint remain live. Post-patch independent review
  found no high/medium issue in the member-budget extraction.
- Began the requester-state gate with production-page regressions: both initial
  and continuation-page people mismatches must discard the editor, and a failed
  retry reporting changed session/access must discard the pending request until
  explicit reload. A control retains input for ordinary 400/409/500 responses.
  These UI cases are intentionally not implemented yet; RED verification follows
  the running SQLx preparation. Existing retry-preservation tests remain intact.
- UI RED runner `68612` is waiting on the same build directory while `51401`
  prepares SQLx metadata. Keep both handles; do not restart on silent compilation.
  Error handling review requires one session-change discriminator for both
  server and locally detected response mismatches, without requester identities
  in the error. Cancel/clear the parent's cached success and scope invalidation
  callbacks to a generation; do not just restart into the new account.

Next: collect `51401` and the UI RED runner. Implement typed session-change
classification, generation-scoped invalidation, parent resource cancel/clear and
explicit fresh reload; classify Add everyone errors before string conversion.
Do not discard ordinary conflicts/ambiguous pending intent or let late responses
cross generations. Complete manager controls remain after requester-state work;
the full permissions goal is active.

## 2026-10-05 — Protected controls and original edit intent

- Previous goal iteration made concrete progress: context-bound catalogs and
  identity-only team selection passed 56 component tests and native/WASM lint.
  No runner was left live; reused the same isolated branch and local changes.
- Added three actual-page regressions: preservation on save/retry, independent
  read-only costs without private-note access, and withheld finance with editable
  hour budgets. `2803` reproduced all three failures (exit 101).
- Wired separate billable/cost/note metadata through the existing controls.
  Withheld numeric controls are absent; read-only values and monetary budgets
  cannot be edited; project type/rate mode/billable controls retain their write
  boundary. Hour budgets remain independently editable. No CSS changes.
- Added cost zero/reset, hour-budget payload and monetary task-row preservation
  tests. `16119` passed all 62 component tests and native/all-target plus WASM
  lint (exit 0). That binary predates the following review correction.
- Adversarial review found two medium defects in permission-only preservation:
  untouched editable blank task rates could become Reset and copy a global
  default; untouched None/hour budgets could unnecessarily require financial
  writes for stale stored task money. Both need interaction-based intent.
- Added a typed edited-field set to the existing editor state. Untouched scalar
  fields and retained row overrides are preserved regardless of write access;
  actual same-value/blank/zero input marks explicit intent and makes the editor
  dirty. New writable row defaults retain the existing addition/inheritance
  behavior; unwritable new fields remain preserved/unset. The pending request
  captures this once for exact retries. Search/focus alone is not an edit.
- Added the untouched-versus-explicit-empty existing-task regression and
  strengthened the cost reset test to use a retained nonempty override. `76680`
  passed all 63 component tests and native/all-target plus WASM lint (exit 0).
- Follow-up review found a remaining medium defect: deleting a budgeted task or
  person while keeping Budget deletes the child but retains the old parent sum.
  Marking the whole group edited would clear unrelated inactive amounts. The
  correction must recalculate only the active affected aggregate while retaining
  the original preservation intent for settings, retained rows and inactive units.
- Added three real-save PostgreSQL regressions for per-task hours, per-person
  hours and per-task fees, including unchanged retries and inactive task money
  preservation without financial authority. RED runner `1488` exited 101 with
  all three intended stale-total failures (1500 versus 300 minutes; 2500 versus
  500 cents). Its binary predates the fix and following strengthened cases.
- Separated parent active-unit preservation from child/settings preservation.
  Only removing a populated budget contributor (including zero) releases that
  active total; a blank/unrelated association removal cannot repair stored money.
  Strengthened cases cover nonzero/zero/None remainder, fee denial before save
  and after replay revocation, and untouched stale monetary parent totals.
  Independent source review found no further high/medium issue; runtime and
  regenerated SQLx verification remain pending.
- Adjacent preservation review confirmed a separate medium issue to fix next:
  `load_members` retains inactive stored hour budgets, but `save_member` deletes
  them on an ordinary save outside HoursPerPerson even when Budget is kept.
  Forward the canonical keep decision and skip only that member-budget write
  block; preserve assignment/cost writes, explicit budget edits, legacy behavior
  and intentional assignment deletion. Add a real-save/replay regression with
  an inactive retained member-hour budget. This issue is not fixed or covered by
  the currently running binary.

Next: collect live runner `23212`: all project creation/editor database tests,
registered editor/session cases, complete all-target SQLx preparation and
native/WASM lint. Fix any failures before accepting aggregate preservation.
Then fix the confirmed inactive member-hour budget preservation gap above.
Then finish requester-state invalidation and archived/outside-team manager
controls before browser acceptance and public guard cutover. No publication,
policy activation, merge or real-data mutation.

## 2026-10-05 — Editor catalog UI integration

- Revalidated runner `46713`: all 91 project creation/editor tests passed,
  including the seven catalog cases. The registered HTTP matrix and complete
  all-target SQLx preparation also passed. Its native lint later encountered the
  newly introduced Dioxus key syntax error; terminal exit 101, no WASM result.
- The active foreign client/task fixture strengthening postdates the first
  compilation. Fresh runner `91998` passed its final pagination test and exited
  0 against disposable PostgreSQL; active foreign and archived local matching
  candidates are now independently covered at the page boundary.
- Added production-page regressions for project-first loading, no canonical
  fallback to creation catalogs, mismatched catalog requester rejection and
  atomic rejection of Add everyone after a session switch. `34586` could not
  complete offline because the full cache preparation was still running, but
  its newly compiled `new_project_screen-5748ffb5af2ca2c6` harness was executable.
  Running that exact binary reproduced all three intended failures (legacy
  helper passed). The older `61dda3a91063c6b5` binary selected zero cases and is
  not evidence. Component tests use controlled responses, not Chrome.
- Connected project-first loading, client/task refresh, identity-only people and
  all-page Add everyone to the captured editor context. Each canonical response
  is checked without falling back to creation catalogs. Retained projections
  remain separate from new candidate identities; no rates are reconstructed.
  The editor key now includes requester and access revision. Full stale-state
  purge and protected-field controls remain pending.
- Source review found no additional high/medium issue in this increment and
  recommended second-page requester switching. Added that regression plus
  50/51 pagination, default-rate absence and 501-person atomic rejection.
- `9304` stopped on the same RSX key syntax issue (no runtime tests). Corrected
  the key to the required formatted string. `89444` passed all 56 real-page
  component tests, including the six new integration cases and legacy controls.
  Its native/all-target and WASM lint stages also passed; terminal exit 0.
  Removed now-obsolete WASM dead-code expectations for the project picker types
  used by the UI. No live runner remains from this iteration.

Next: integrate protected-field controls and original
unchanged/reset/zero intent together. Separate costs from the legacy private-notes
flag, protect billing type/rate mode/billable toggles and monetary budgets, retain
hour-budget editing and revalidate actual save/retry payloads. Complete requester
state invalidation, archived/outside-team manager controls and Chrome acceptance
remain open. No publication, policy activation, real-data change or merge.

## 2026-10-05 — Context-bound editor catalogs

- Collected `5827`, terminal exit 0: 25 navigation and 50 real-page component
  tests passed, followed by native/all-target and WASM Clippy. This verifies the
  preceding manager-selection increment, not the complete canonical form.
- Traced the remaining creation-catalog calls. The canonical editor needs its
  own operation-bound client/task reads; teammate selection already has the
  identity-only `project_people` contract and must not be folded into a response
  containing rates or permission metadata.
- Added strict editor context/search/response models and an authenticated
  `project_editor_catalog` wrapper. Four database regressions first exercise a
  temporary legacy-catalog adapter to reproduce financial leakage, legacy-role
  coupling, absent requester/project binding and implicit legacy fallback.
  `57477` finished with all four intended failures. Replaced that adapter with
  the canonical reader and added three scope/pagination/lock-wait cases plus
  registered HTTP coverage; runtime verification is recorded above.
- Independent contract review confirmed the proposed separation. In particular,
  `lock_editor_actor` alone does not prove project existence for an all-project
  editor: the catalog must explicitly verify tenant/project identity. Both task
  amount and its currency require global-task rate Read. Client defaults remain
  withheld under the unresolved separate predicate, including for Administrator.

Independent source review found no high/medium issue in the canonical reader;
its active foreign pagination fixture suggestion was applied. UI wiring and
full verification remain separate gates. No deployment, public editor cutover,
policy activation or publication.

## 2026-10-05 — Independent manager intent in the real editor

- Collected `2482`: all 84 project creation/editor tests, all 32 manager tests
  and the registered HTTP/session matrix passed. Full all-target SQLx preparation
  also succeeded. The detail-navigation harness passed all 25 tests.

- Its new-project harness compiled the first local manager-state regressions:
  46 passed, two deliberately reproduced missing selection updates and missing
  designation restoration when adding a retained manager. `2482` is terminal
  exit 101; its subsequent native/WASM Clippy stages did not run. This is not a
  backend regression or complete form acceptance.

- Began connecting an independent manager selection signal to the existing
  checkbox, team additions and the original pending edit request. No CSS or
  layout changes. The selection retains the complete authorized snapshot,
  including outside-team and archived designations.

- Independent review identified that dirty/leave checks must also inspect this
  selection: adding, designating and removing a temporary teammate can leave the
  form unchanged but delegation intent dirty. Added real-page interaction tests
  for that sequence and exact original-intent retry after an uncertain response.
  These two page tests postdate `2482`'s binary.

- `43410` reproduced all four intended RED cases: missing manager updates,
  missing retained designation on addition, clean state for manager-only edits,
  and incomplete actual save payload. The legacy toggle control passed. Terminal
  exit 101. The tests drive the production page with controlled responses, not
  a real browser or database.

- Checkbox updates now retain unrelated designations; additions/readditions use
  the current independent selection, preserving explicit unchecks as well as
  retained managers. Dirty display/navigation and Cancel/Back share one memoized
  check including set-wise manager comparison. The page snapshots current
  manager intent into the existing pending request without rebuilding retries.

- The cancel regression records the native-dialog open request; merely finding
  its always-mounted text would be insufficient evidence. Independent source
  review found no additional defect in this increment, with full acceptance
  explicitly still pending.

- `5827` finished with exit 0: all 25 detail-navigation and all 50 new-project-screen
  tests, including the four corrected regressions and native-dialog request
  verification, followed by native/all-target and WASM lint. These component tests use synthesized Dioxus
  events and controlled server responses, not Chrome acceptance.

Next: implement requester-keyed reset and
context-bound catalogs. Protected-field UI intent,
outside-team/archived manager controls and browser acceptance remain unfinished.
No public guard cutover, publication, policy activation or merge.

## 2026-10-05 — Canonical managers in the project form transaction

- `8140` ended with successful complete SQLx preparation and native/all-target
  Clippy, but its zero-test selection is not runtime evidence.

- Corrected `29949` passed all 32 `project_management_tests`, including joint
  commit/rollback and the held-FK no-upgrade regression. The shell then failed
  because its ignored verification script was edited while Bash was executing
  it; its second planned test did not run. The script now passes `bash -n` and
  accepts additional filters in the same disposable database; do not edit a live
  script. `29949` is terminal exit 2.

- Separate `72224` reproduced the real editor defect: the legacy Lead checkbox
  still claimed canonical responsibility despite only an outside-team archived
  person being designated. The test failed at that assertion, terminal exit 101.

- Added the complete retained manager snapshot to canonical editor metadata and
  an explicit manager selection/access revision to edit intent. Reads share the
  form transaction. Internal comparison flags and response flags both use actual
  designations; canonical membership writes preserve legacy roles independently.

- The form now calls the shared delegation command in its own transaction even
  when only outside-team managers change. Original form replay precedes the
  independent delegation-receipt conflict check. Delegation and project receipts
  commit together, with self-removal last and current authority still required
  for replay. Missing/stale/duplicate/mismatched selection rejects without a
  partial project save. Legacy omitted fields retain their wire shape.

- Added form regressions for combined edits, unchanged-form manager changes,
  historical replay, stale/invalid intent, incompatible receipt reuse, archived
  retained members, self-removal and failure of the final form receipt. Independent
  pre-implementation review identified the internal-before flag and unchanged-form
  hazards; both are included in these cases. Runtime results are pending.

- The page constructor only retains the original manager snapshot. Interactive
  checkbox/independent selection state, catalogs, financial intent and browser
  acceptance remain unfinished; no canonical public guard cutover or publication.

- Code review caught an invalid transaction prelude: administration setup queried
  limits before the editor switched to SERIALIZABLE. Split the limits helper and
  set SERIALIZABLE, READ WRITE before its first query. The reviewer confirmed the
  correction and no additional high/medium finding in this integration slice.
  This correction and the archived-manager removal assertion postdate the initial
  `25569` compile; collect it, then verify final source rather than treating that
  run as acceptance of the corrected prelude.

- `25569` ended exit 101: 55 tests passed and 29 writer-dependent cases failed
  on its pre-correction transaction prelude. Its later manager/HTTP/SQLx/lint
  stages did not run. The reader reproduction now passed. A fresh combined run
  on the corrected source includes the archived-manager removal assertion and
  real-page harnesses; do not restart the terminal `25569`.

Next at that checkpoint (now collected above): collect `2482`, the fresh combined project-creation, manager and registered HTTP runner,
then its full SQLx/component/native/WASM checks. Correct failures before connecting the
actual permission-aware form controls. The whole feature remains active.

## 2026-10-05 — Shared manager transaction

- Collected `42587`, terminal exit 0: the corrected registered HTTP/session
  matrix passed (including project-editor requester binding), complete all-target
  SQLx preparation succeeded, and native/all-target plus WASM Clippy passed.
  Only the three previously superseded query descriptors are removed.
- Extracted the existing delegation command into a transaction-taking operation.
  The standalone endpoint still owns configuration, commit and explicit rollback;
  project editing can now compose the same authorization, revision, relationship
  and audit/receipt writes without a second commit. No new policy or duplicate
  manager implementation is introduced.
- Added SERIALIZABLE parent-transaction tests with the real AccessChange gate before
  project UPDATE. They check joint commit and joint rollback of the
  project name, manager set, access revision and receipt, plus safe original-intent
  retry. Runtime verification is pending; this does not yet wire form selection.
- Independent source review identified a composition precondition: the current
  editor takes organization NO KEY UPDATE, while delegation requires FOR UPDATE.
  A follow-up review confirmed that NO KEY UPDATE preserves the required exclusion
  of permission writers and SHARE readers, while allowing unrelated FK KEY SHARE.
  Reused the existing AccessChange helper, preserving the missing-org forbidden
  outcome, and added a held-FK regression against a late lock upgrade. No other
  extraction defect was reported. Runtime verification of this correction remains
  pending.
- `8140` compiled the initial extraction but selected zero tests because its
  filter was `project_management::` instead of `project_management_tests::`.
  Do not count this as a passing regression suite. Its later prepare/lint stages
  may see newer source; a correctly filtered run on the final source is required.

Next: collect live `8140` (SQLx preparation and native lint), run the corrected
`project_management_tests::` filter on final source, then connect the
complete retained manager selection and access revision to the existing form's
read/save transaction. Preserve independent tracking membership and legacy mode;
do not publish or activate the partially connected canonical editor.

## 2026-10-05 — Existing editor requester binding

- Collected `87545`: 73 project-creation/editor tests passed, followed by native
  all-target and WASM Clippy, terminal exit 0. That runtime binary predates the
  extra NULL-task-override reset case; `65212` subsequently passed all 73 tests
  including that case, terminal exit 0.
- The next concrete integration boundary is the identity captured by the editor.
  Added optional `expected_requester` transport using the existing
  `PermissionRequester` type; legacy requests omit it. Request constructors copy
  the authorized reader's identity, not a profile or client-supplied grant.
- Added database regressions for missing/mismatched requester, same-organization
  switching between two independently authorized people, no write/receipt on
  rejection, and original-intent replay. Runner `26187` compiled before the
  server guard and reproduced both actual unwanted writes: 14 existing canonical
  tests passed and both new tests failed at their denial assertions (exit 101).
- The save transaction now checks requester binding after current actor/project
  authorization but before private form loading, validation or receipt replay.
  Canonical saves require an identity; supplied mismatches also reject in legacy
  policy. Canonical original intent includes the bound identity. Added a wire
  roundtrip proving omitted legacy fields remain omitted, and changed the rate
  denial fixture to open its form under the canonical policy it is testing.
- `91206` passed all 76 project-creation/editor tests, with no failures or skips:
  both requester regressions now pass, including their valid save/replay controls,
  and the legacy wire roundtrip passes. Its real-page harnesses also passed:
  24 detail-navigation tests and 44 new-project-screen tests. Native/all-target
  and WASM Clippy both passed; `91206` is terminal, exit 0.
- Focused independent source review found no high/medium defect in requester
  binding, original pending retry, legacy wire/receipt compatibility or the
  new test controls. This is source review only, not HTTP/browser acceptance
  or approval of the still-unfinished whole form integration.
- No visual/CSS changes or public canonical guard cutover. The form still needs
  requester-keyed reset, context-bound catalogs, protected-field intent and
  canonical manager integration; this increment is not full UI acceptance.
- Added a registered HTTP-route regression to the existing shared session
  harness (`authorization_tests/project_editor.rs`): anonymous and switched
  cookies, missing/foreign requester, unchanged saved state, original retry,
  independently opened second-session form and current-authority revocation.
  This source addition postdates `91206`'s binary compilation and needs the full
  session harness run plus all-target SQLx preparation, not merely component SSR.
- `54233` reached the new HTTP check: switched/missing/foreign requester denials,
  unchanged state, both valid sessions and original replay passed. Its last
  assertion failed because the revocation fixture stored an empty array, which
  violates the immutable own-work floor and correctly returns sanitized 500,
  not ordinary 403. Fixed the fixture with `PermissionSelection::new(&[])` and
  kept an explicit malformed-empty-policy check. `54233` is terminal exit 101;
  its SQLx/lint stages did not run.

Next: collect corrected live `42587`, running
`job_endpoints_enforce_session_role_and_organization`, full all-target SQLx
preparation and offline native/WASM lint for the HTTP addition. Continue requester-keyed form state,
context-bound catalogs and canonical manager selection. `65212`, `87545` and
`26187`, `91206` and `54233` are terminal; do not restart them.

## 2026-10-05 — Protected editor write intent and replay

- The preceding goal turn collected new terminal evidence from `47946`: all 26
  selected read/legacy tests passed, including stale task-money redaction, but
  all-target SQLx preparation failed because two component harnesses omitted
  `permission_editor`. Fixed both real-model imports. Its incomplete cache
  generation must be replaced by a successful complete prepare, not committed
  as removal of unrelated query descriptors. `47946` is terminal.
- Added typed `ProjectEditRequest.unchanged` fields. Canonical saves validate
  original intent, load current field grants under the existing transaction
  gates, merge protected values from storage, and require write permission for
  explicit edits even when equal, empty or zero. Costs no longer borrow the
  actor's legacy Admin role. Legacy empty-intent requests keep their wire shape.
- Canonical receipts record original input and required write effects, never
  the merged hidden values. Replay reauthorizes the recorded effects so removal
  and mode-change permissions cannot vanish merely because the first save
  already changed the project. The public mutation still returns only a UUID;
  its full Project is server-side plugin context, not a client payload.
- Runner `11761` passed all 30 editor tests with no skips, including the former
  read-only-write failure and new keep/retry, equal-value and zero/reset/revoked
  retry cases. It is still running full all-target SQLx preparation. No duplicate
  build is needed while that handle is live.
- `11761` subsequently exited successfully after complete all-target SQLx
  preparation (2m27s). The cache recovered: its only three tracked removals are
  the superseded project UPDATE, settings UPDATE and task-settings UPSERT, each
  replaced by the new preservation-aware query. No unrelated cache deletion
  remains. Both component harnesses now compile in the all-target prepare.
- Independent source review then identified two additional defects: an inline
  New task alias could resolve to a retained task after authorization, and
  validation normalization could clear inactive-mode values despite keep intent.
  Rejected that alias in canonical saves; persistence now preserves project
  rate/budget, fee/invoice settings and milestones explicitly. Added the alias
  regression and expanded keep/retry coverage with inactive rate/fee/milestone,
  second-tax and stale task-cents values. The reviewer accepted these specific
  fixes and withdrew the alleged response leak after tracing the UUID wrapper.
  Those final changes postdate the 30-test binary and still need runtime tests.
- A further source check found explicit reset of an already-empty task override
  could be skipped by whole-form equality. Canonical active task resets now
  execute their default-copy semantics even when raw input equals the loaded
  form; the zero/reset/revoked-retry test now begins from a NULL override with a
  populated task default. This change postdates the `87545` test compilation;
  collect that runner before starting a final runtime check of the added case.
- No CSS, schema, dependency, public guard cutover, publication, merge or real
  database mutation. The page constructor only supplies the legacy default
  empty keep list; actual permission-aware UI/catalog/manager integration is
  still required, not silently replaced by this internal transaction increment.

Next: collect live final runner `87545`: all project-creation/editor regressions
on the reviewed final source, then offline native/all-target and WASM Clippy.
`11761` is terminal and must not be polled or restarted. Then wire permission-aware form state,
context-bound catalogs and canonical manager selections. The actual EditProject
still loads the legacy catalog first, and its session/requester binding and
canonical manager access revision must be connected as part of that cutover.
Add remaining indirect
effect/concurrency/session/browser cases before publishing the combined form.
The goal and T159/T160/T161 remain incomplete; no product answer is needed for
this existing-project step.

## 2026-10-05 — Existing-project financial ownership approved

- The user answered A. FR-034 now requires project access plus the applicable
  project-scoped billable Read/Write grant for monetary budgets, fixed fees and
  invoice defaults. Hour-only budgets and independent cost grants are unchanged.
  This is an approved Horae rule, not verified Harvest enforcement. The historical
  dependency stops below no longer block this existing-project work.
- The internal editor now reuses canonical project authorization and projects
  rate, cost, private-note and monetary-setting visibility independently. Added
  explicit withheld/read-only/editable response metadata. Public form/catalog
  integration and protected write intent remain unfinished; do not publish or
  activate this partial path.
- The monetary-visibility regression failed at its intended assertion in runner
  `13869` before the projection change. Its refined fixture now establishes
  monetary settings through the real legacy save transaction. Current source
  needs fresh verification; earlier legacy checks do not verify this increment.
- Runner `49520` exposed an invalid fixture, not a product failure: FixedFee
  does not support TotalFees budgets. Split the setup into TimeAndMaterials
  monetary budgets and FixedFee hourly budgets/fees, using real saves in both.
  `22403` then passed 25 editor tests (17 legacy and 8 canonical reads/scope
  cases); the sole failure remains explicit rate-write authorization. It still
  accepts the forbidden write, so this is not a green canonical editor suite.
- Read tests now cover withheld/read-only/editable financial metadata, ordinary
  hour-budget visibility, independent costs/notes and actual managed-project
  designation without disclosure of global task defaults. Internal snapshot
  inclusion uses an explicit private-field flag, not a synthetic Admin actor.
- Independent source review found stale task cents could be formatted under a
  nonmonetary parent budget mode. Made task-budget loading unit/mode-aware and
  added `canonical_editor_never_projects_stale_task_money_as_hours`. The reviewer
  accepted the source fix; `22403` predates it and does not verify this regression.
- Final runner `47946` is live: editor regressions including the stale-money
  case, then complete all-target SQLx preparation in disposable PostgreSQL.
  Its explicitly named `editor-read-boundary` mode excludes only the known
  unfinished write-denial test, not any read or legacy test. The failing test
  remains enabled in source and must pass before completion/publication. Do not
  rerun `22403`, which is terminal, or claim the live runner has passed.

Next: collect `47946`, then implement original protected-field
intent and transactional preservation/authorization before wiring the actual
form and catalogs. Preserve revision/replay semantics and test indirect effects.
Creation designation and client-default ownership remain separate open contracts.
No merge, policy activation or real database mutation is authorized here.

## 2026-10-05 — Integration dependency revalidation

- The preceding goal turn completed verification: `94881` passed 56 legacy
  project-creation/editor tests and native/WASM Clippy; final gate `81837` passed
  formatting and `git diff --check`. All named runners are terminal. Repeating
  these checks without implementation changes is not the next action.
- Rechecked the worktree, plan readiness table and open integration contracts.
  T159/T160 still require a decision on project non-rate monetary fields. The
  approved hourly-rate/cost predicates and the six canonical failing fixtures
  are unchanged; legacy compatibility is not the requested final state.
- Checked a distinct implementation alternative: the person-management writer.
  FR-027/028/029/031 and its pure prerequisites are implemented, but
  `contracts/person-management-validation.md` explicitly leaves the activity of
  new relationship endpoints unanswered. The project-manager active-addition
  rule does not settle that person-management choice. Do not copy it silently.
- Full report candidate integration also retains its documented pending choice;
  company-lock scheduling and migration retain their own open contracts.
  No new independent ready implementation was established by this review.
  Further speculative intent helpers would not complete the blocked real form.

Next: obtain the pending project-field choice first, one decision at a time:
either project access plus existing scoped billable-rate Read/Write controls
monetary budgets, fixed fees and project invoice defaults, or project Read/Write
alone controls those fields. Hours-only budgets remain project-governed and
costs remain independent in both options. Neither option is approved or claimed
as verified Harvest enforcement. Then implement the actual reader/editor/save
contract together and rerun the six RED cases against the changed implementation.
This is the first dependency-only stop after the completed compatibility work;
the goal remains active and incomplete, with no new build, merge or publication.

Second consecutive dependency-only review: the worktree and pending contracts
are unchanged and no product answer has arrived. The preceding dependency review
did not implement functionality; do not count its log update as implementation
progress. There is no live verification to poll or new evidence warranting an
identical test/reference run. Keep the goal active at this second stop; obtain
the existing field-ownership answer rather than inventing a default or another
compatibility-only increment.

Third consecutive dependency-only review: no intervening product answer or
implementation change resolves the same blocker. The previous turn was no
implementation progress, not a verified wait. Revalidated the worktree and the
existing-project, person-management and report-candidate entry gates; no safe
independent ready implementation has emerged. Mark the goal blocked, not
complete. Preserve all local work and the open canonical acceptance cases.
Resume from the field-ownership decision above when the user supplies it;
do not rerun already-terminal builds or infer consent from automatic continuation.

## 2026-10-05 — Rate-intent compatibility and final verification

- The preceding user-requested cleanup reclaimed 3.94 GiB of unused build
  artifacts in two old worktrees. It preserved source, worktrees, databases and
  the active compilation. The observed free space afterward was 179 GiB in WSL;
  storage is not the current implementation blocker.
- Resumed `89497`, without duplicating its live build. It completed successfully:
  all 17 legacy editor tests passed, including zero/reset/default preservation,
  followed by complete all-target SQLx preparation. The seven new descriptors
  correspond to the editor regression and canonical RED fixtures; no existing
  descriptor was deleted. This is not a passing canonical editor or full suite.
- Independent review found no material change to legacy behavior and recommended
  removing the redundant write-authorization call after role-based intent
  construction. That simplification is applied. Core authorization remains for
  independently supplied intent/authority, not as a claim of enforced canonical
  form permissions. No wire, schema, dependency or CSS changes were introduced.
- The simplification landed while the earlier runtime build was in flight, so
  its test result alone is not claimed as verification of the final dispatcher.
  Session `94881` passed all 56 tests in the wider project-creation regression
  group on the final source in disposable PostgreSQL (5m02s build, 7.46s tests).
  It explicitly excluded the same six canonical RED cases. Offline native/
  all-target Clippy then passed with warnings denied (3m53s), followed by WASM
  Clippy (37.87s). The session exited successfully; neither it nor `89497` remains
  live. Format gate `15241` passed with 567 unchanged files before this result-only
  documentation update. No identical compatibility run needs restarting.
- Rechecked the dispatcher against its original implementation and actual
  parser/validation path: no dependency, amount parsing, SQL mutation or caller
  authority change beyond representing the same three legacy cost outcomes.
  Wider tests retain draft, finalization, catalog, import and revocation checks.
  This verifies the internal preparation, not a protected canonical wire form.

Next: implement the complete protected-field projection/original-intent merge
and actual form integration under T159/T160 after settling the pending non-rate
monetary-field ownership; internal work on approved rate semantics does not waive
that decision. Retain the six canonical failures as open acceptance work. Do not
reopen confirmed rate rules or publish/activate a partial canonical form. No
merge, real data mutation or new publication occurred in this iteration.

## 2026-10-05 — Internal explicit rate intent

- The previous turn confirmed the reviewed RED run, not an implemented form.
  Continued independent intent preparation rather than repeating those failures
  or treating the unanswered financial-field question as consent.
- Added three pure tests first; `40125` failed compilation on the missing
  `RateEdit`/`RateEditDenied` symbols. Implemented the narrow parsed intent in the
  existing core rates module: Unchanged, Reset, Set(minor units), with explicit
  edits requiring a server-derived field-write decision even for equal values
  or zero. This is not currency validation or full operation authorization.
- Wired the intent to the actual member-cost save dispatcher while preserving
  the existing legacy role, parser and SQL behavior. No public payload, route,
  schema or policy change. Added a real-editor regression for zero versus reset
  that also checks the person's default cost is untouched; the existing hidden
  private-cost preservation/removal test remains the unchanged-intent baseline.
- `34148` passed all 183 core tests and core all-target Clippy with warnings
  denied. Disposable session `89497` now runs the legacy editor regression suite
  and complete SQLx preparation. Its explicitly named `legacy-editor` mode skips
  the six known canonical RED cases for this compatibility check only; they are
  not ignored in source or removed from full tests, and remain incomplete work.
- Requested independent read-only review of the core intent, actual dispatcher
  and new legacy regression. No new agent or publication. Poll `89497` next;
  the pure-test/lint run is terminal, with no duplicate build to start.

Next: finish compatibility/cache verification and review, then connect original
intent to the complete protected-field editor transaction once its pending field
ownership is resolved. Do not claim the legacy-preserving internal step fixes
the six canonical failures or finishes T159/T160, policy activation or the goal.

## 2026-10-05 — Reviewed editor RED confirmed

- Resumed `55014` rather than starting another build. It terminated with all
  six intended failures after 2m52s compilation and 1.55s tests. This is the
  final reviewed fixture, including coherent designation state, populated task
  defaults, retained task identity and the additional write/receipt assertions.
  No compiler or fixture-setup error occurred. Assertions after an initial RED
  failure are compiled but not executed; their success is not claimed.
- Rechecked actual load/save and catalog call paths. `load_editable_project`
  loads a repeatable-read snapshot through the legacy actor guard; saving uses
  the same raw-string form with complete-set comparison and stored replay
  payloads. A reader-only redaction would turn withheld rates into ambiguous
  empty edits. Do not wire that partial change into the public form.
- The shared project-people reader already implements the approved exact-project
  operation predicate; reuse its current grant/designation logic in the eventual
  editor authority boundary. Core rate predicates remain the approved financial
  rules. No new policy behavior or wire type was introduced in this iteration.
- The monetary-field product question is still unanswered. The recommended
  option is not consent. No further identical RED execution is useful before
  implementation changes. All test runners from these reproductions are now
  terminal; no build or database-test session remains to monitor.

Next: resolve the pending financial-field ownership before public form cutover;
internal projection/explicit intent preparation and unrelated closed-contract
work remain possible. Preserve the six RED cases and original replay intent;
do not hide failures, claim T159/T160 done, or publish a partial canonical form.
No merge, real data mutation, activation or additional publication occurred.

## 2026-10-05 — Canonical editor operation and field boundary

- Previous goal turn made progress: published the verified report filter and
  reproduced three actual editor failures. Resumed live `98783`, which then
  completed with the same three intended RED failures after the refined fixture
  (2m19s compilation, 1.74s tests); no production implementation is claimed.
- Tightened the read-only-rate case again: clear private notes in stored fixture
  state before loading the original revision, rather than sending a note clear
  alongside the rate change. This prevents an unrelated private-note denial
  from satisfying the financial-write oracle in a future implementation.
- Added operation-boundary cases: canonical project editor with legacy Member
  role is allowed; financial-only grants with legacy Admin role are denied;
  managed project editing requires an actual canonical designation, not a
  tracking-lead assignment. All reuse the real configured project/editor path.
- Session `83496` formats and executes these six cases in disposable PostgreSQL.
  It is confirmed live compiling; poll that handle rather than rebuilding.
  Reused the existing read-only transaction reviewer for adversarial fixture
  review. No new agent, schema, real migration, policy activation or publication.
- Current official rate/project/budget references still do not specify custom
  permission ownership for all non-rate form fields. Asked one product decision:
  A would gate monetary budgets, fixed fees and project invoice defaults with
  the existing billable-rate read/write grant in project scope, in addition to
  project access; B would use project read/write alone for those fields.
  Hour budgets remain project-governed and costs independent in either option.
  Clarified the exact existing grant in commentary. Neither choice is accepted
  yet; do not infer A from its recommended/preselected presentation.
- `83496` completed with six intended RED failures (2m58s compilation, 1.87s
  tests), including wrong legacy-role acceptance/denial and missing canonical
  project designation enforcement. This run precedes the final review fixes.
- Independent review identified three fixture weaknesses. The rate-write test
  now requires unchanged revision/no receipt on denial and a successful retry
  after adding only billable-write authority; it also seeds the existing manager
  designation so the unchanged form does not implicitly add one. The withheld
  test retains the task identity and checks a populated global task default,
  rather than relying only on a serialized string search. No runtime changes.
- The reviewer confirmed canonical grant normalization and non-admin custom
  identity are coherent. Internal rate projection/intent work can proceed on
  approved FR-021/022; publishing the composite form wire contract remains gated
  on unresolved field authority and complete consumer integration.
- Final reviewed-fixture check `55014` is running formatting and the focused
  disposable-database tests. Poll this exact handle next; `83496` is terminal.
  Uncommitted work is still tests and contracts only, with no new publication.

Next: confirm the final reviewed fixtures, then implement the internal protected
rate projection/intent path without publishing a partial form wire contract.
Retain RED cases until actual reader/editor/save integration passes them.
Non-rate field authority and creation decisions remain explicit
gates; the report-candidate question is also still unanswered. Continue safe
independent work without activating or publishing a partial canonical form.

## 2026-10-05 — Published report filter; project financial-form reproduction

- Cleanup revalidation found 179 GiB free in WSL and 432 GiB on Windows;
  no further removal was necessary. This verified the storage constraint but
  did not change application behavior.
- Resumed publication handle `28542`: it had stopped on a Markdown-only format
  correction, before creating a commit. Confirmed the exact diff, accepted the
  formatter output and reran the gate. `71048` passed with 566 unchanged files
  and created unsigned `de8f9ad`. Push `27516` succeeded; remote `64950` verifies
  #212 OPEN/DRAFT on that exact head. No merge or full-feature acceptance.
- Returned to T159's existing-project cases, independent of report candidates
  and unresolved creation predicates. Reopened current Harvest permission and
  budget guides; they still do not settle custom fee-budget write authority.
  The approved FR-021/022 rate/cost rules remain the test oracle, not a new
  restricted-account observation. Spec Kit prerequisites pass; unavailable
  command skills are not claimed as executed.
- Added three real-editor tests under `editing/tests/canonical_fields.rs`,
  reusing `configured_fixture`: withheld stored task rate, custom cost reader
  without legacy Administrator, and rejected task-rate update with read-only
  billable authority. They exercise existing load/save transactions rather
  than a new disconnected evaluator. No production policy or wire change.
- Disposable PostgreSQL test session `51844` is live compiling the RED cases.
  Its final read-only-write case was refined after launch to omit protected
  notes, avoiding an unrelated denial oracle; if that source was already read
  by rustc, rerun the focused command after the handle terminates. Do not restart
  a live build. These uncommitted tests are not published as passing coverage.
- `51844` terminated with the intended three RED failures (2m38s compilation,
  0.59s tests): stored task rate appears in the editor payload, authorized custom
  cost read returns empty, and the read-only task-rate mutation succeeds.
  This confirms missing canonical integration in the actual transactions, not
  a compile error or a claim about deployed policy. Recheck the refined write
  fixture after formatting; retain all three as failing acceptance cases.
- Confirmation session `98783` runs formatting followed by the same focused
  disposable-database command against the refined fixture. Poll this exact
  handle next; the earlier `51844` is terminal. The verified report increment
  remains published separately, while the project tests/contracts are local.

Next: finish the refined RED confirmation, then integrate explicit protected-field
intent and current rate predicates with the actual editor/read/save path as
required by T159/T160. Keep unsettled non-rate/create predicates explicit;
do not activate a partial canonical form or infer unanswered product choices.
Full feature 015, cross-surface gates and the implementation goal remain open.

## 2026-10-05 — Report candidates and active-project filtering

- Previous iteration is published as `ecac66b`; remote #212 is OPEN/DRAFT at
  that commit and the worktree was clean. Its final format gate `51708` passed
  with 565 files and no changes. This is completed navigation/delivery progress,
  not full T203 or feature acceptance.
- Reopened the current detailed-report, Member-report and permission guides.
  They confirm multiple filters and the distinction between retained archived
  results and archived candidate choices, but do not settle restricted
  zero-record teammate eligibility. Existing 2026-10-04 Chrome evidence remains
  an owner-only observation, not a restricted-actor test.
- Retried the reusable Windows MCP client (session `40431`). Initialization
  succeeded, but `browser_tabs` returned no browser data and the client request
  timed out after 240 seconds. Graceful exit did not finish; interrupted only
  that client (terminal exit 1), without a tab-close command or business writes.
  No new authenticated Harvest evidence is claimed. Asked one pending decision:
  whether the already-approved Timesheet zero-record project-participant rule
  should also apply to report filters. Do not infer the answer.
- Continuing independent T222: the documented Active projects only result
  filter. Contract and task now distinguish it from candidate discovery. Added
  a default-false typed/wire field and tests before SQL implementation. Initial
  `72068` ended with a test-only private-module import error; moved reader tests
  into their existing permission modules instead of widening production access.
- Revalidated after the cleanup interruption: 177 GiB free in WSL; preserved
  current changes and active build artifacts. Corrected the test sibling path
  after terminal `31547` reported the wrong module name. RED `58049` then ran:
  strict transport passed, detailed/grouped readers and workbook-source tests
  failed on retained archived rows (one passed, three failed). UI RED `3814`
  failed because the filter control was absent, as expected.
- Added source predicates to all six existing reader/export queries and bound
  the shared Checkbox to detailed/grouped/nested request keys and download URLs.
  No new dependency, migration, shared CSS or authority changes. Added filtered
  empty/denied export, frozen cursor/workbook-source tests, strict flattened
  grouped transport, late-response/cursor-reset component and actual-browser
  fixtures. Formatter `21187` passed; full regression/cache run is next.
- Re-ran Spec Kit's prerequisite script successfully (feature 015, all expected
  artifact categories present); unavailable command skills are not claimed as
  executed. Added T222 to the plan's contract/test matrix. Browser script syntax
  check `22181` passed. Independent static review identified one P2: flattened
  grouped URL values cannot deserialize directly as bool. Corrected only the
  URL field with strict string-to-bool parsing; JSON request DTOs remain bool.
  No other material static findings. Regression/cache session `19410` is live
  and began before that parser correction; any result from it must be followed
  by a fresh check of the corrected code.
- `19410` finished with 133 passing, one failed and two pre-existing manual
  measurements ignored. The only failure is the confirmed pre-fix flattened
  URL bool parser. All four new export/activity/snapshot/empty-authority tests
  passed, along with the existing export authorization regressions. This failed
  batch did not reach permission-reader tests or SQLx preparation. A corrected
  full retry is required; do not treat the preceding binary as current validation.
- Corrected retry `2099` is confirmed live compiling. The reviewer independently
  confirmed `query_bool` closes the P2 for direct/flattened URLs while preserving
  default/duplicate/invalid handling and JSON types. No current GREEN claim,
  cache completion, commit or push yet. Continue polling this exact session;
  do not start another build while it is live.
- Corrected `2099` has passed 134 report/export tests (two existing manual
  measurements ignored), all 25 scoped detailed/grouped-reader tests, 11
  permission-storage tests and the registered-session job authorization test.
  The grouped boolean regression now passes. SQLx all-target preparation is
  running in the same session; UI/build/browser/lint gates remain pending.
- `2099` completed successfully, including SQLx all-target preparation (3m54s).
  Cache changes replace six production query descriptions and add five test
  query descriptions. Format check `42971` passed with 566 files unchanged;
  its warnings name the six intentionally replaced cache files.
- Verification chain `1423` is live: 19 actual-component tests, then native
  all-target lint, WASM lint and a fresh fullstack browser bundle, sequentially
  with `set -e`. Poll that exact handle; SQLx/backend verification is complete.
- `1423` passed all 19 tests in `scoped_reports_ui` (2m44s compilation, 0.07s
  tests), including active-filter paging resets, download binding and all four
  nested late-response cases. Native all-target lint is the current live phase;
  the pipeline has not yet reported WASM lint or fullstack-build completion.
- `1423` completed successfully: native all-target lint passed in 4m19s,
  WASM lint in 40.85s, both with warnings denied. Fresh Dioxus client/server
  build passed in 164.17s. Started the isolated `reports-permissions` Chromium
  suite; evidence goes to `.scratch/active-project-reports-browser-evidence/`.
- Chromium `82589` passed against the fresh server/client and a disposable
  database (Chromium 148.0.7778.96). Real sessions verify active/archived mixed
  totals, keyboard checkbox changes from a later page, all grouped dimensions,
  nested three-filter detail, strict direct/grouped CSV/XLSX URLs, empty output,
  private-colleague exclusion and policy revocation. Inspected all six desktop
  dark/mobile light captures together: the new control fits the incumbent
  layout; tables retain contained horizontal scrolling. No visual repair or
  shared CSS change needed. T222 is complete; full T203/feature 015 is not.
- Final format gate `29584` passed (566 files, no changes), and `git diff --check` is clean. Remote `46340` confirms #212 OPEN/DRAFT on
  `feat/scoped-permissions` at `ecac66b` before publication. No full-flake or
  full-feature acceptance is claimed by this focused increment.

Next: final format check and publish this verified increment unsigned to draft
#212, without merging. Candidate decisions remain pending; no merge, real migration
or Harvest data mutation. Full feature 015 and the implementation goal stay open.

## 2026-10-05 — Individual time reports and breakdowns

- Revalidated the worktree at published `2b59b58`; the grouped CSV backend is
  already on draft #212. The interrupted RED run `89816` completed: 14 component
  tests passed and the three missing-link/navigation tests failed as expected.
  The disk-cleanup interruption was diagnostic progress, not a report change;
  existing build artifacts and databases were retained.

- Reopened Harvest's Time-report guide and ran Spec Kit prerequisites. Retained
  the design handoff's shared controls and table framework; its Reports screen
  depicts the custom builder, not ordinary Time results. Command skills remain
  absent; no unavailable specify/analyze invocation is claimed.

- Added bound grouped CSV links, all four individual report contexts, the
  client's project navigation and all four documented client/project inline
  breakdowns. Detail requests/downloads retain all selected dimensions; no
  legacy catalogs, dependency, migration or CSS change. Added two navigation
  glyphs to the existing icon component without changing existing glyphs.

- Initial run `45847` passed all 17 component tests, with a deprecated signal
  alias warning corrected afterward. Independent static review found no material
  defect and requested late-response, paging and denial coverage for the child.
  Added all four mappings, correct grouping cursors, collapse/late response,
  identity mismatch, denial/retry/empty and pending-page/date-change remount
  checks. `5872` passed all 18 tests without warnings in 67 seconds of build
  time. Follow-up review found no material defect; its browser synchronization
  recommendation is implemented with awaited context-filter links.

- Full server/WASM build `26737` passed in 74.16 seconds. Native/all-target
  and WASM Clippy `83106` passed with warnings denied (115 and 29.39 seconds).
  Disposable Chromium `86289` passed real scoped reads/downloads, all four
  individual reports and all four nested expansions. Desktop/mobile inspection
  found no viewport overflow; the mobile capture cropped the lower breakdown
  and transition timing made selection colors ambiguous. Updated only the
  capture assertions: disabled screenshot animations, verified exactly one
  selected group and captured the full nested page. Confirmation `79691` passed;
  the final desktop/mobile captures in `.scratch/nested-reports-browser-evidence/`
  were inspected together. Shared CSS and real data remain untouched. T221 is
  complete for grouped navigation/delivery; full T203 is not.

Next: format-check and publish this verified increment unsigned to draft #212,
then resolve full report-picker candidate eligibility against Harvest before
wiring the selectors. Financial families, protected project fields, approvals,
policy activation and full cross-surface verification remain required. No merge.

## 2026-10-05 — Grouped CSV delivery

- Revalidated clean `ca170c0` and completed the pending remote check: #212 is
  OPEN/DRAFT at that exact commit. The grouped workbook increment was published;
  the previous section's publication-pending note is now superseded.
- Reopened Harvest's Time-report guide and retained its viewed-report export
  semantics. Executed Spec Kit prerequisites successfully; command skills remain
  absent, so no unavailable specify/analyze execution is claimed.
- RED `32504` failed all three initial grouped CSV tests with the stub's 501.
  Added a canonical route, native pair-fragment cursor and bounded delivery.
  Reused the existing worker, admission, deadlines and authorization predicates;
  no dependency, migration or shared CSS change. Group totals do not escape until
  every original context is authorized under one gate lifetime.
- `25407` passed eight tests in 12.25 seconds: four dimensions and exact hours,
  empty canonical authority, 10,001 source entries and groups, oversized quoted
  Unicode, captured invalid state, frozen reassignment, last-of-129 scope
  revocation and backpressure denial. Independent static review found no material
  defects and requested explicit between-batch concurrency evidence.
- Added a database-local third-fetch pause for a 257-context group, real lock
  dependency assertions and normal/cancelled release with a one-connection pool.
  Independent review identified a test-only cancellation timing assumption;
  corrected it to release the artificial fetch blocker before waiting for cleanup.
  The already compiled `8948` run passed 127 regressions but failed that old
  cancellation wait, with two manual probes ignored; preparation did not run.
  Corrected run `60045` passed 128 report tests (two existing manual probes
  ignored, 54.57 seconds), all 11 storage tests and the existing registered
  HTTP suite (12.29 seconds). This includes normal/cancelled between-batch
  authority verification. Full SQLx preparation completed in 82 seconds with
  ten new descriptors and no deletions. Browser
  assertions now exercise the actual registered CSV route in all four dimensions
  with all five filters, empty results, invalid queries, anonymity, wrong identity
  and later policy changes. Disposable Chromium `24811` passed all of those
  checks and the existing report regressions (version 148.0.7778.96); captures
  are in `.scratch/grouped-csv-browser-evidence/`. No UI/CSS changed in this
  increment, and no new visual-parity claim is made.
- Fresh full server/WASM bundle `64061` passed in 75.52 seconds. `60662` passed
  native all-target Clippy in 110 seconds and WASM Clippy in 18.62 seconds,
  both with warnings denied. Format gate `80746` passed (564 files, zero changes).
  The final independent review found no material production defects; its
  cancellation-test concern was corrected and verified above. No full-flake
  or full-feature completion is claimed.

Next: publish the verified grouped CSV backend unsigned to existing draft #212,
then connect its bound UI link and nested reports as required T221 work. T203,
full pickers, financial families and policy activation remain open. Do not merge.

## 2026-10-05 — Grouped workbook delivery

- Previous iteration published `e2e66fb` to existing draft #212; verified remote
  HEAD and a clean worktree before continuing. This was implementation progress,
  not full feature completion.
- Reopened Harvest's Time-report guide: downloads represent the viewed report.
  Implementing grouped XLSX first; grouped CSV retains its streaming requirement
  and will not inherit an arbitrary source-entry or workbook row limit.
- Independent design review rejected unbounded per-group context arrays for CSV:
  the existing native fetch function does not truncate an oversized first record.
  XLSX uses a complete scoped relation, separate bounded group/context probes and
  a single statement for size/payload consistency. Tuple context and labels count
  toward its logical budget; current scope is rechecked after rendering.
- RED `96011` confirmed three failing grouped-workbook tests on the unavailable
  reader. Implemented the bounded source, exact-text workbook renderer and a
  separate canonical route; added group-count/field/filter and four-dimension
  tests. `39878` found a test-only reference to an unavailable direct dependency;
  reused Axum's existing query extractor instead of adding a dependency.
- `80208` passed the first six focused tests, 11 storage tests and the registered
  session suite (13.03 seconds), then completed full SQLx preparation in 85
  seconds. It adds eight descriptors and deletes none. Independent source review found no
  material defect; added its multi-context partial-revocation and exact combined
  budget boundary tests for the final regression run. The grouped XLSX button
  now carries dimension, dates, requester and policy only while data is ready.
  Browser assertions include all dimensions, invalid/ambiguous queries, wrong
  identity, unauthenticated access, empty results and later policy changes.
  Final export regressions `14710` passed 119 tests with two existing manual
  probes ignored (41.88 seconds). Fresh full Dioxus build `71419` passed in 87.95
  seconds. `2402` passed native/all-targets Clippy (135 seconds) and WASM Clippy
  (20.52 seconds), warnings denied, followed by all 15 component tests after a
  113-second compilation. Final format gate `16719` passed with zero changes.
  Static follow-up review confirmed the added tests and bound link;
  browser ZIP/200 checks establish transport, while Rust assertions cover cells.
- Disposable Chromium `44459` passed the new actual-route assertions and existing
  report regressions. Captures are in `.scratch/grouped-xlsx-browser-evidence/`;
  desktop and mobile were inspected together. No new shared CSS or dependency,
  no database migration and no real data change. Publication remains pending;
  no full-flake or full-feature claim is made.

Next: publish the verified grouped XLSX increment to existing draft #212 without
merging, then implement grouped CSV with bounded original-scope fragments (not
unbounded context arrays or workbook-derived row limits). Nested reports remain
required T221 work; full T203 and feature activation remain open.

## 2026-10-05 — Grouped report consumer

- Previous turn published verified `41ff137` to existing draft #212; remote HEAD
  matched and the worktree was clean. That was implementation progress, not full
  feature acceptance.
- Reopened Harvest's Time-report guide and read the Reports handoff: the latter
  depicts the custom report builder, not finished ordinary Time results. Preserve
  its shared tables/controls and current Horae utility system; do not fabricate
  builder controls, rates or financial-family data.
- T221 starts with the four ordinary grouping tabs and hour-total drilldown into
  the existing detailed reader. Dates, entity IDs and requester binding must
  follow through to detailed CSV/XLSX. Grouped downloads and nested entity-name
  reports remain required, and are not represented by detailed export links.
- Added actual-component tests for grouped navigation, full-period totals,
  pending/cancelled responses, identity rejection and bound drilldown. RED
  `34642` failed the two new navigation tests because the controls did not exist.
  The first build `99653` exposed the explicit-path child module layout; corrected
  it without moving the existing Reports entry point.
- Independent review found a stale-date risk in a singleton keyed child. Dates
  now enter the resource as reactive signals and participate in its response key;
  parent-owned pagination resets on date changes and preserves grouping. Added a
  Ready-second-page regression. `68632` passed all 15 component tests. Follow-up
  review confirmed the fix with no material findings; corrected its minor fixture
  cursor observation too.
- `64723` formatted the increment. Fresh full Dioxus build `37299` passed (73.97
  seconds). Disposable Chromium run `93847` passed all four grouping tabs,
  keyboard hour drilldown, entity-bound CSV/XLSX, cleared filters, empty/date
  transitions, scoped data and policy-change rejection. The fixture has 503 own
  entries plus a private colleague entry; no existing database was used.
- Batched desktop-dark/mobile-light inspection found grouped names breaking
  within words on mobile. Removed only that cell's `wrap-anywhere`, preserving
  the shared scrolling table and all global CSS. Added a browser geometry check;
  confirmation build `41625` passed (81.44 seconds). Confirmation Chromium
  `71066` passed with the added word geometry assertion; inspected its desktop
  and mobile grouped captures together, with no additional correction required.
  Evidence lives in `.scratch/grouped-reports-browser-confirmation/`.
- `55479` passed server/all-targets Clippy (93 seconds), WASM Clippy (17.16
  seconds), both with warnings denied, and all 15 component tests (75-second
  compilation). Final formatting `6138` completed with zero changes. No full
  flake check or completed feature acceptance is claimed.
- Actual Spec Kit prerequisites passed again; command skills remain absent, so
  this is not a claim that unavailable specify/analyze commands were executed.
  No SQL or migration changed, so the published SQLx cache remains applicable.

Next: publish this verified consumer increment unsigned to draft #212, then complete matching
grouped exports and nested entity-name reports. T221/T203, full pickers,
financial families and policy activation remain open; do not merge.

## 2026-10-05 — Scoped group aggregates

- Previous goal turn made implementation progress: published `7266abb` to draft
  #212 and verified remote HEAD; its component/browser/native/WASM checks passed.
  Revalidated a clean worktree before this increment.
- Reopened Harvest's current permission, contractor, profitability and Time
  report guides. Financial report families require distinct projections and
  missing domain sources, not relabeling the legacy monetary time grouping.
  Recorded dependencies without weakening C02 or assuming old administrator-only
  wording overrides the approved custom grants. Ordinary four-dimension grouping
  remains independent of the unanswered picker candidate question.
- Actual Spec Kit prerequisites pass; command skills remain absent. Added the
  grouped reader contract and T219–T221. Independent contract/DTO review found no
  blocking decision; it calls out 501-group probing, entry-versus-group counts,
  duplicate labels, zero-minute groups and totals before the cursor.
- RED `89604` completed: all three initial tests failed on the unavailable
  grouped-reader stub. GREEN `48973` then passed six SQLx tests against the real
  grouped query: four dimensions, own/managed/all union, duplicate labels,
  500/3 pagination, zero-minute groups, full-period totals, frozen rounding,
  archived history, tenant-qualified parents and malformed input.
- Following the requested cleanup, rechecked free disk space (187 GiB) and
  resumed this existing increment. Added deterministic revocation/cancellation
  and actor-deactivation waits, bigint sums, registered-session HTTP checks and
  all-five-dimension filter combinations. `73292` is running report regressions,
  storage checks, actual HTTP and SQLx preparation on disposable PostgreSQL.
- Independent static review found no material authorization, tenant or paging
  defect. Suggested explicit per-entry-before-group rounding and varied-label
  C-order cursor coverage; these checks are next. No grouped UI/export acceptance
  or full-feature completion is claimed. T221 and T203 remain open.
- `73292` passed 181 report-related regressions (three manual measurements
  ignored) and all eleven storage checks. HTTP caught a test expectation error:
  Dioxus 0.7.9 maps argument-decoding errors to 500 before invoking the handler
  (`ServerFnError` status conversion in fullstack-core and `magic.rs`). Kept
  strict typed rejection and recorded the framework limitation; domain-invalid
  ranges/cursors still require 400. No framework-wide patch is included here.
  Added both suggested rounding and C-cursor tests before the final rerun.
- Final disposable run `18050` passed 183 report-related regressions (three
  pre-existing manual measurements ignored), eleven storage checks and the
  registered-session HTTP suite (13.45 seconds), including all new grouped
  cases. SQLx preparation with `--workspace --features server --all-targets`
  completed successfully and added eight query cache entries without removing
  existing ones. Formatting CI `26220` checked 557 files with zero changes.
- The final read-only review also found no material issue in the extra tests
  or the documented decoding limitation. Native/all-target and WASM Clippy are
  running in `58545`; no full `nix flake check` or full-feature acceptance claimed.
- `58545` passed native/all-target Clippy (98 seconds), then WASM correctly
  reported the five not-yet-consumed grouped DTOs. Followed the existing
  `models/project_managers.rs` pattern with non-server `expect(dead_code)` on
  the query/page roots, explicitly tied to T221. The initial attempt covered
  nested types too; `75440` correctly rejected those redundant expectations,
  which were removed. These annotations do not relax
  authorization or hide native warnings; remove them when wiring the consumer
  (fulfilled expectations then become lint failures). No executable server or
  SQL changed after the passing tests/cache/native lint.
- Final WASM Clippy `44150` passed (15.55 seconds), warnings denied. T219/T220
  are complete; T221/T203 and full activation remain explicitly open.

Next: publish the verified reader increment unsigned to existing
draft #212 without merging, then implement T221's bound grouped consumer,
drilldown and equivalent exports against `design/`. Full pickers and financial
families remain T203 requirements. Keep unanswered candidate-policy decisions
distinct from the confirmed Timesheet choice; the implementation goal stays open.

## 2026-10-05 — Reports browser verification and disk recovery

- Previous turn made concrete maintenance progress at the user's request: removed
  only validated inactive build directories and two obsolete browser bundles,
  recovering approximately 190 GiB. Source, worktrees, pending diffs, evidence and
  databases were preserved; the active permission worktree retained its cache.
- Dioxus 0.7.9's `cli/build.rs::into_targets` detects the explicit dependency
  `fullstack` feature even with `--fullstack false`. Corrected the ignored runner
  to use the project's normal single fullstack build, without a preliminary
  native build. `50982` completed successfully in 223.69 seconds, including an
  explicitly successful fresh client bundle (16.59 seconds).
- Disposable Chromium run `38460` failed on a missing `billable` fixture value;
  `86456` reached the delayed-request case but exposed an interceptor/unroute
  race. Corrected both test defects without changing authorization behavior.
  `76285` then passed in Chromium 148.0.7778.96: 503 authorized entries versus a
  private other-person entry, full-period totals, 500/3 pagination, CSV/XLSX,
  invalid dates, hidden pending results, policy changes and inactive-user denial.
  It used the already-current native server and explicitly refreshed WASM.
- Inspected desktop-dark and mobile-light captures together. Mobile names and
  dates wrapped too aggressively; removed only the name cells' anywhere-wrap
  utility and kept dates on one line. No shared CSS changed. Added geometric
  word/date assertions and registered the browser suite in the default runner.
- `22181` passed all twelve actual-component tests, then failed Clippy on the
  nested Next-button condition. Applied the suggested let-chain without changing
  its readiness guard. Final bundle `1062` and component/native/WASM checks
  `82436` are running; second browser confirmation remains pending. These are
  not full-feature acceptance. T203 and activation remain open.
- Final bundle `1062` completed (125.73 seconds). Actual-component rerun `82436`
  passed all twelve tests; lint continues in that same invocation. Browser
  confirmation `23602` passed against both parts of the final Dioxus bundle,
  including the new geometric assertions. Inspected the second desktop/mobile
  pair: mobile dates stay intact, words wrap normally and horizontal overflow is
  contained by the existing table wrapper. No further visual iteration needed.
- Independent final static review found no material defect in the consumer/test
  delta, including fixture cleanup and the held-request lifecycle. Verification
  used disposable PostgreSQL and local headless Chromium, not authenticated
  Harvest or Windows Chrome. No new Harvest product behavior was inferred.
- `82436` completed successfully: all-target native Clippy (127 seconds) and
  WASM Clippy (20.90 seconds), warnings denied. Formatting CI `8206` checked 554
  files with zero changes. T216–T218 are complete; no new SQL or migration was
  introduced after the successful backend/storage/HTTP/cache run `11622`.
  Full `nix flake check` and full-feature acceptance are not claimed.

Next: publish this verified increment unsigned on `feat/scoped-permissions` to
existing draft #212 without merging. Continue T203's full candidate discovery,
grouping and financial-family integration using the existing contracts and
Harvest evidence. Do not infer Reports candidate policy from the separate
Timesheet decision or repeat the unanswered zero-record candidate question.
Independent C02/financial integration analysis remains available while those
picker decisions are open. Scoped approvals, company locks, migration and full
activation are still required; the goal remains active.

## 2026-10-05 — Ordinary Reports consumer

- Previous response only reconfirmed FR-033/B and made no implementation progress.
  Revalidated clean published `a23804f` and reused the existing worktree/draft #212.
- Actual Spec Kit prerequisites pass; command skills remain absent. Read Rust,
  testing, async and interface-hardening guidance. Closed T216–T218's consumer
  contract without deciding the pending Reports candidate question.
- RED `63621` reproduces all six legacy catalog/report reads before admission;
  three new isolation tests fail while existing monetary presentation tests pass.
  An earlier harness compile error was corrected before claiming RED.
- Independent review identified the unbound get-my-permissions/get-me handoff.
  Reused the existing export authority transaction for one access response with
  requester and mode. The route pins both across remount/retry; the scoped view
  connects bounded rows, exact full-period totals and bound downloads.
- Actual-component suite `71446` passes all twelve cases: legacy/canonical
  isolation, pending/error states, cursor/date changes, cancelled responses,
  escaped text, full-period totals and requester/mode continuity. HTTP `13191`
  passes the first access-endpoint matrix (12.52s).
- Review then identified a stale scoped link becoming a legacy-mode download.
  Added optional strict `expected_policy` transport and checks before XLSX rows
  or CSV DECLARE, preserving later release checks. Static re-review is clear.
  Added same-session HTTP transitions in both directions and a browser fixture.
- Export run `85126` passes 110 regressions but fails one new parser test because
  its URI omitted required dates; corrected the fixture, not production behavior.
  Final export/storage/HTTP/cache run is `54896`; no final result claimed yet.
- `54896` failed during compilation with ENOSPC, before any test. A separate
  font lookup used an unfiltered path flake and copied build artifacts into Nix.
  Interrupted that process, verified the exact unreferenced source copy, then
  removed only `/nix/store/p29g07awi53pcqpd7m2nfsgmw4wqgack-source` with Nix
  (18 GiB recovered). The original worktree/data remain intact. Use the existing
  Git-filtered flake's `checks.x86_64-linux.browser.FONTCONFIG_FILE` attribute
  for fonts, never an unfiltered path `getFlake` here. Retry is `11622`.
- `11622` passes 111 report/export regressions (43.28s; two existing manual
  measurements ignored), eleven storage cases (2.17s), the actual-session suite
  including both policy transitions (13.63s), and full SQLx preparation (96s).
  No cache descriptors changed. The font derivation is available. Fresh native
  browser server builds successfully (50.79s); current client build is `44838`.
- User noticed the slow terminal. Process inspection showed that `dx build --web --fullstack false` was nevertheless building `server-dev` with the
  `server` feature, duplicating the verified native build. Interrupted only
  those identified Dioxus/Cargo processes. `44838` returned zero after the
  interrupt, which is NOT evidence of a completed client build; the browser
  bundle still contains stale client files and must not be used for acceptance.
  Next: correct the client-only build invocation, then run the prepared browser
  fixture and final native/WASM checks. No build process is intentionally left
  running from this invocation.
- Next: complete those gates, build a fresh browser bundle, run the disposable
  report suite and inspect desktop/mobile captures. T203, full pickers, grouping,
  financial families and activation remain open. No real data or shared CSS changed.

## 2026-10-05 — Full-period report totals

- Previous goal turn was a verified wait on live Chrome client `81095` plus a
  reconfirmation of the already recorded FR-033/B decision, not new code.
  Revalidated clean published `93aaa68` and reused the existing worktree/#212.
- The pending Chrome `browser_tabs` call timed out. Sent the client's `exit`
  command without closing user tabs; no new authenticated Harvest evidence.
  Reopened the official Member and detailed-report guides. They document
  period-wide rounded reporting and archived/multiple selections, not the
  unresolved restricted-actor picker cases.
- Asked once whether Reports should include active, zero-record managed-project
  participants; the Timesheet decision does not implicitly answer this separate
  consumer. Historical candidates and filter narrowing remain separate questions.
- Actual Spec Kit prerequisites pass; its command skills remain absent locally.
  Read the constitution and Rust/testing/async/simplicity guidance. Closed the
  full-period aggregate refinement and T213–T215 without inventing picker rules.
  This is a prerequisite for exact paged reporting, not finished Reports UI.
- RED `64711` is compiling the existing 503-entry keyset test with full-period
  totals assertions, including the exhausted cursor. No result claimed yet.
- `64711` ended before testing: LLVM exhausted the filesystem. Removed 4.3 GiB
  of regenerable Horae-package artifacts only. Restart `89942` did not pick up
  the attempted package-specific environment setting; explicitly interrupted it
  (exit 130), cleaned its 1.1 MiB artifacts, and changed the ignored runner to
  pass `profile.dev.package.horae.debug=0` through Cargo configuration. No
  business data, dependency cache or source was deleted. Expanded scoped,
  filtered, archived/frozen, revocation, tenant and large-sum assertions before
  restarting RED. These infrastructure failures are not test results.
- RED `96539` completes compilation and runs eleven focused cases: nine fail
  specifically on missing `totals`, while invalid-query/policy and actor-denial
  cases remain green. Implemented a single scoped SQL statement with totals
  before the cursor, bounded text delivery and an empty-page sentinel. Extended
  actual-session assertions across 501 rows and added per-entry live/frozen
  rounding versus billable summation. Chrome client `81095` is now terminal after
  interrupting the hung shutdown; no user tabs were closed.
- GREEN `34358` passes 136 report-related regressions (44.87s; two existing
  manual measurements ignored), including all twelve scoped report cases, eleven
  permission-storage tests (2.03s) and the registered-session HTTP suite (12.18s).
  Complete SQLx preparation is running in the same temporary cluster. Independent
  static production/test review found no material defect. Snapshot coherence is
  established by the single statement, not a newly forced concurrent-edit test.
- `34358` finishes complete SQLx preparation (89s): four new descriptions replace
  the obsolete detailed-reader description; the other three describe test-only
  queries. Formatting `91995` updates only the two changed Rust test files.
  Offline native/WASM Clippy runs as `91426`. GitHub confirms #212 open/draft at
  `93aaa68` before publication. No schema, shared CSS or application data changed.
- `91426` passes offline all-target server Clippy (108s) and WASM Clippy (19.85s),
  warnings denied. T213–T215 are complete for full-period ordinary time totals;
  T203 and full-feature acceptance remain open. Review confirms all new cache
  descriptions match the changed SQL; no unrelated cache removals occurred.
- Final formatting CI `1692` checks 552 files with zero changes; diff checks pass.
  Publication target is the existing draft #212, unsigned and without merging.

Next: continue T203 with canonical/legacy Reports resource separation and
the ordinary consumer, resolving candidate eligibility before the full picker.
The Reports zero-record question is pending; do not ask it again or infer its
answer from Timesheet. Keep financial families, remaining writes, migration and
full acceptance open. This increment is not a delivered Reports screen or a
completed permissions feature.

## 2026-10-05 — Multi-ID download query transport

- The previous goal turn made progress: verified scoped CSV delivery was
  published as unsigned `09bd15f` on existing draft #212. Revalidated the clean
  worktree and actual Spec Kit prerequisites; command skills are still absent.
- Traced the existing scalar links, `ExportParams`, common query conversion,
  both source/release gates and the registered-session harness. Closed the
  multi-ID wire contract without changing report semantics or picker candidates.
- Independent design review confirms strict presence-based scalar/plural
  conflicts, wholly empty versus malformed lists and paired identity bindings.
  No new product decision, SQL authority, dependency or UI abstraction is needed.
- RED `79204` runs five extractor tests: legacy links pass; plural selections,
  requester binding, cursor rejection and ambiguous-filter checks fail as
  expected. Added strict conversion into the existing `TimeReportQuery`, reusing
  the current CSV/XLSX authorization and output mechanisms.
- Added registered HTTP tests for every dimension, AND/OR selection, duplicate
  IDs, own/managed union, unauthorized/foreign IDs, malformed/repeated parameters,
  legacy scalar links and actual account-switch binding. `35028` is compiling
  the report regression suite, followed by storage/HTTP and full SQLx preparation
  in a disposable PostgreSQL cluster. No result is claimed yet.
- `35028` passes 110 report/export tests (including the five new parser cases)
  and eleven storage tests. The HTTP fixture failed before requests because its
  new client omitted the required currency. Corrected that test-only insert to
  use EUR; no production change or waived assertion. Final HTTP/cache verification
  must be rerun before publication.
- Final `29148` repeats all 110 report/export and eleven storage tests in green;
  the actual-session HTTP suite now passes in 12.15s. Full SQLx preparation is
  running. Independent production/test review found no material defect. GitHub
  confirms existing #212 open/draft at `09bd15f` before publication.
- `29148` completes full SQLx preparation in 78s, adding five test-query cache
  descriptions without deleting or changing any existing description. Offline
  native/WASM Clippy is running as `29721`. No production SQL, schema, CSS or
  application data changed.
- `29721` passes offline server all-targets Clippy (101s) and the unchanged WASM
  target with warnings denied. Formatting `98173` checks 552 files with zero
  changes. T210–T212 are complete for the transport boundary, not T203 or the
  full permissions feature.

Next: publish this verified increment unsigned to draft #212 without merging,
then connect the ordinary Reports consumer under the canonical policy and
resolve its full candidate-discovery contract against Harvest before wiring
pickers. The Reports consumer/candidate universe,
financial families, remaining writes and migration/cutover gates are still
required; the full goal remains active.

## 2026-10-05 — Recorded-scope CSV delivery

- Reused published `cbc78a8`, existing worktree and draft #212. FR-033/B is
  already recorded and implemented; its repeated confirmation adds no policy.
- RED `94314` reproduced canonical Member rejection and legacy-Manager scope
  expansion. Implemented native cursor authority metadata and strict decoding,
  including the empty-source sentinel. The shared permission-state restore and
  XLSX current-authority helpers avoid a second authorization implementation.
- Delivery records private owner/project pairs for each bounded block, reserves
  channel capacity without authority locks, then rechecks current grants and
  pinned policy. Invoice/project exports and all CSV columns remain unchanged.
  No schema, dependencies, CSS or real-data changes.
- GREEN `30494` passes both initial tests. Independent production review finds
  no material defect; this is static review, not proof of all concurrency cases.
- Added DECLARE-versus-initial/FETCH authority and relationship tests, transient
  invalid source with/without rows, missing-identity sentinels, metadata bounds,
  blocked-body revocation, complete pending-context checks and clearing, empty
  header invalidation, captured reassignment/deletion and actual HTTP assertions.
  `9111` is running report/export regressions, strict storage tests, HTTP and
  complete SQLx preparation in a disposable PostgreSQL cluster. No result is
  claimed until that process finishes.
- `9111` now passes 105 report/export tests (two manual measurements ignored),
  all eleven strict storage tests and the actual-session HTTP suite. SQLx
  preparation is running in the same disposable cluster. Independent review of
  the new tests found no material defect; no fresh Harvest/browser claim.
- `9111` completes full SQLx preparation in 72s: ten new cache files, two obsolete
  time cursor/decoder descriptions removed, no unrelated deletions. Offline
  server all-targets and WASM Clippy are running as `22934`. Existing #212 is
  confirmed open/draft at `cbc78a8`; no new branch or duplicate PR is needed.
- `22934` passes offline server all-targets Clippy (87s) and WASM Clippy (17s)
  with warnings denied. Formatting checks 550 files with zero changes. T207–T209
  are complete for this CSV boundary, not T203 or the full feature.

Next: publish this verified increment unsigned to existing draft #212, without
merging, then continue T203 with multi-ID download transport and the ordinary
Reports consumer. Resolve remaining candidate discovery before full pickers.
Multi-ID download transport, Reports consumer/candidate
discovery, financial reports, remaining writes and migration/cutover stay open;
the goal is active and no merge or full-feature acceptance is authorized.

## 2026-10-04 — Recorded-scope XLSX delivery

- The preceding clarification only reconfirmed FR-033/B and was no progress.
  Revalidated clean `75f13a1`, the already published detailed reader, and actual
  Spec Kit prerequisites. Its command skills remain unavailable locally.
- CSV review ruled out declaring its source cursor inside a rolled-back
  authorization savepoint: that destroys the cursor. Preserve the existing
  bounded transport; capture/validate authority in the cursor snapshot, including
  empty-source metadata. This remains required follow-through, not implemented.
- Closed the XLSX boundary in `contracts/time-reports.md`. RED `70145` reproduces
  Member denial and canonical legacy-Manager overexposure in both initial tests.
- Replaced the XLSX two-query reader with one scoped size/payload statement and
  private captured owner/project pairs. Rendering runs outside authority locks;
  release reloads strict grants and active identity, pins policy and checks every
  recorded pair. The registered route now delegates authorization to this gate.
  Invoice gates, CSV, CSS, schema and real data are unchanged.
- Added tests for unions/filters, source reassignment in both directions during
  rendering, source deletion, one captured pair revoked, empty-file state and
  policy invalidation, cancellation/pool recovery and real-session XLSX delivery.
  Initial compile `93359` exposed the private permission module boundary; reused
  its strict loader through a crate-private re-export instead of duplicating it.
- Independent static review found no material production defect and requested
  direct coverage of the new release method, not the legacy manager renderer.
  Added that coverage before starting build/test `76042`.
- `76042` finished with all eight then-current scoped XLSX tests passing and one
  inherited snapshot assertion failing on a valid 413. Adjusted only Entries to
  accept its exact earlier snapshot or coherent size rejection, not oversized
  payload. Added source grant gains/losses, direct actor commit/rollback and
  in-render grant revocation coverage. Final review reports no material defect.
- `42927` passes 95 report/export tests (including eleven new XLSX cases); two
  manual measurements remain ignored. The actual-session HTTP suite also passes
  in 11s. Complete SQLx preparation passes with ten new cache files and one
  obsolete size-query cache removed. No full server-suite or Nix-suite claim.
- Offline all-target server/WASM Clippy passes with warnings denied (`60722`,
  83s/15s). T204–T206 are complete for this boundary, not T203 or full feature
  acceptance. GitHub confirms #212 remains open/draft at `75f13a1` before this
  increment's publication. Formatting `36144` checks 548 files with zero changes.
  Publication target remains that draft PR, with an unsigned commit and no merge.

Next implementation: continue T203 with canonical CSV source/release
authorization, multi-ID download transport,
Reports consumer and full candidate discovery. Retain private source authority
metadata in the CSV cursor snapshot, validate its empty-source case and recheck
captured scope before each bounded output block. Remaining financial families,
people/approval/project writes and policy cutover gates stay open.

## 2026-10-04 — Ordinary detailed report reader

- The previous turn only reconfirmed the already implemented FR-033/B decision;
  it was no implementation progress. Revalidated clean `1b41033` and successful
  Spec Kit prerequisites. Its command skills remain unavailable locally.
- Reopened the official detailed-report, Member-report and archiving guides.
  They confirm retained historical results and scoped ordinary time reporting,
  but do not resolve all custom-scope picker candidates. A new read-only Windows
  MCP connection initialized, then `browser_tabs` timed out; closed that client.
  No new authenticated Harvest observation or mutation is claimed.
- Traced `report_time`, `report_detailed`, shared SQL, CSV authorization/buffering
  and XLSX snapshot/release checks. Independent review confirms a nonfinancial
  detailed-reader boundary can proceed separately from picker discovery and C02.
  Recorded it in `contracts/time-reports.md`, including mandatory consumer/export
  follow-through and recorded-scope authorization for retained source snapshots.
- Added explicit query/page/entry projections and a canonical session endpoint.
  Reused the time reader's authority fence, not the financial legacy snapshot.
  SQL filters before pagination, qualifies tenant parents, retains historical
  labels and report rounding/billability, and never selects rates or invoice IDs.
- RED `82050`: both initial behavior tests fail on the denied placeholder.
  GREEN `62902`: all seven initial scope/history/filter/cursor/policy tests pass.
  The production placeholder is removed. No schema, CSS or dependency changes.
- Independent static review found no production defect; tightened cancellation
  testing to prove one-connection pool reuse after rollback. Added tenant-parent,
  billed/frozen-zero and direct-deactivation cases plus registered HTTP coverage.
  `98511` is verifying that final source and preparing SQLx; it is not yet a
  completed verification result. No full-feature or browser acceptance is claimed.
- `98511` now passes all ten final report tests, the real-session HTTP suite with
  the new endpoint and eight shared time-reader regressions. SQLx preparation is
  still running in the same disposable database; no live application was migrated.
- SQLx's first pass omitted fresh integration targets. Forced their metadata
  emission without changing test content; `75780` completes the full cache with
  13 added files and no removed entries. No schema/business data was changed.
- Offline all-target server Clippy passes (`71355`, 73s). WASM first identified
  the DTOs awaiting the T203 consumer; retain narrowly scoped expectations only
  on the unused query/page roots, consistent with the existing picker pattern.
  Removed redundant child expectations; final WASM `8425` passes in 14s.
  Formatting `15562` checks 546 files with no changes. T201/T202 are complete
  for this backend increment; T203 and full-feature acceptance remain open.

Next: publish this verified increment to existing draft #212 without merging,
then implement T203's recorded-scope CSV/XLSX authorization and connected
ordinary Reports consumer. Keep financial
report families, full picker semantics, remaining people/approval/project writes
and policy migration gates open. Do not replace picker authority with directory
grants or derive its candidates solely from report rows. The unrelated pending
person-management activity question is not reopened.

## 2026-10-04 — Canonical People consumer integration

- The previous clarification response was no implementation progress: FR-033/B
  was already recorded. Revalidated the existing worktree and ran the actual
  Spec Kit prerequisite script successfully; its command skills remain absent.
- Reused the test executable compiled before the shell change: both new tests
  fail on the original gate (valid canonical reader denied; legacy Admin without
  People scope admitted). The earlier linker failure was not a test result.
- Connected canonical directory reads, activity filters, cursor paging, shared
  table controls and initial `(target, requester)` editor binding. Policy 0 alone
  selects the legacy directory/tasks/writes; failed or unsupported projections
  cannot fall back. Sidebar entry follows People read scope, not legacy role.
- Independent review identified two P2 cases, now addressed: recovery remains
  mounted after returning to policy 0, and canonical projection failures cannot
  change the legacy Importers admission rule. Denied canonical People mounts
  only recovery, without directory/target reads. No shared CSS or SQL changed.
- Added actual-component paging/filter/error/requester tests and disposable
  browser scenarios for managed/all scope, canonical Administrator with legacy
  Member role, all-grants non-admin, denial, legacy isolation and recovery after
  complete People-grant revocation. Verification is still in progress.
- Native linking encountered a bus error with less than 1 GiB disk free. Removed
  debug symbols from this worktree's regenerable old native executable, retaining
  its runnable code and the reusable browser-build caches; no source/data removed.
  Fixed compile diagnostics in test module resolution and shell result typing.
  Nine shell tests passed before the additional Importers regression; final
  combined test invocation `82918` is live, not yet evidence of completion.
- `82918` ended on native-link disk exhaustion; direct test execution found two
  harness failures resolving static control IDs, not passing directory tests.
  Added traversal of actual mounted templates rather than changing production IDs.
  `89909` then passed all 11 shell and 55 editor/directory component tests. Official
  sequential Dioxus server/WASM packaging `88704` passed in 76s.
- Nix's automatic cache collection removed the browser font configuration.
  An incorrect path-based `getFlake` evaluation to restore it copied ignored build
  artifacts too, consuming 21 GiB and exhausting disk during a test-file write.
  Stopped that evaluation, recovered the complete modified test file from the
  evaluation's pre-failure snapshot, and removed only that verified store copy
  through Nix (19.9 GiB freed) plus three old regenerable test executables.
  Git, original data and other worktrees were not deleted. A Git-filtered flake
  evaluation restored the exact fonts configuration without copying `target/`.
- Clippy identified a complex test-only type and nested conditional; corrected
  both without suppression. `29224` repeats tests/native/WASM lint on the restored
  final source. Real disposable Chromium `14009` is now running. These live
  handles are not completed verification; no full-feature acceptance is claimed.
- `29224` passed all 66 component tests and both Clippy targets. Chromium
  `14009` passed the new scope/session cases, then exposed an outdated test focus
  expectation: the account-switch scenario now opens from the other administrator's
  row. Corrected the expected return-focus target; full suite `92963` passes.
  Independent final review closes both P2 findings with no new high/medium issue.
- The batched 1440-dark/390-light inspection found short fragments in narrow
  name/email cells. Removed only their `wrap-anywhere` utility and retained the
  shared scrolling table, without CSS changes. Extended requester tests to next
  page and filter changes as well as refresh/initial editor load. Final `67362`
  passes 11 shell + 55 editor/directory tests, server Clippy (71s), WASM Clippy
  (14s) and official server/WASM packaging (71s). Final browser confirmation is
  running on that bundle; no further visual redesign or policy change is planned.
- Final browser `98074` passes the complete suite on the current bundle with no
  JavaScript page errors. The second batched visual inspection confirms readable
  names/emails and local horizontal scrolling at 390px, with desktop preserved.
  Formatting CI `32073` checks 541 files and changes none. T198–T200 are complete
  for this connected consumer; no full permission feature or Nix gate is claimed.
  GitHub confirms #212 remains open/draft at `dab6885` before publication.
- Published unsigned `ee16165` to the existing `feat/scoped-permissions` branch
  for draft #212. No merge occurred; the scoped verification above does not
  assert complete CI/Nix or full-feature acceptance.

Next: continue T014/T015 by resolving the remaining Reports filter-candidate contract against
Harvest before replacing its `list_users(false)` resource. Investigate zero-record
managed-project participants, historical contributors and filter narrowing; do
not import Timesheet decisions or People directory grants as report authority.
The existing person-management activity question remains pending, not reopened.
Ordinary people writers, approvals, project financial integration, the remaining
cross-command/migration gates and full feature acceptance stay open.

## 2026-10-04 — Editor reload identity before directory integration

- Previous response only reconfirmed the already implemented FR-033/B rule; it
  was not implementation progress. Revalidated clean `774f60a`, the constitution
  and Spec Kit prerequisites. No pending timer clarification remains.
- Independent review confirms People read and canonical Administrator editor
  reachability can proceed independently of the open financial predicates.
  Directory integration must isolate legacy resources/writes and carry the page
  requester into initial editor selection. It is not implemented yet.
- Tracing that connection exposed a current defect: explicit reload clears the
  requester already fixed by the subject picker. RED `12334` reproduces both
  retry after a rejected subject load and initial-response reload adopting a
  different requester; the existing picker rejection test still passes. These
  are real Dioxus component tests, not browser evidence.
- Retain the first accepted response's requester and preserve it on reload.
  Close/save/recovery still release the binding. T195–T197 document the repair;
  verification is pending. No design/CSS, endpoint, schema or grant changes.
- GREEN `83067`: all 52 actual-component tests pass, including both user/org
  mismatch loops and the existing draft, template, recovery and cleanup cases.
  Independent review finds no material defect in the two-line change. Added a
  disposable-browser regression with a real session-cookie switch in a second
  tab, rejected reload, and restoration of the original account.
- Browser packaging `50384` built the native executable but Dioxus also started
  a separate `server-dev` profile despite the attempted client-only flag; disk
  exhaustion prevented packaging. The handle is terminal, not a pending wait.
  Removed only this worktree's two regenerable `server-dev` trees (2.1 GiB) and
  native incremental cache (3.5 GiB), no source or database. Official sequential
  packaging `4188` now limits build jobs to two, disables incremental compilation
  and omits server debug symbols. Browser/lint verification remains pending.
- Independent browser review found a stale-error false-positive risk. Held each
  tested read until pending/stale-error suppression is observed, then verified
  the actual 200 response carries the other Administrator's identity before
  requiring local rejection. No fallback or server denial substitutes for this.
- Official packaging `4188` passes in 279s. Full disposable-browser suite `83813`
  passes in Chromium 148.0.7778.96, including the new real-session reload case,
  audit history, lost-response recovery and acknowledged self-demotion cleanup.
  The runner stops its owned services. Formatting `13071` passes (one test file
  reformatted). T196 is complete; native/WASM lint `39182` remains live.
- `39182` completes: offline all-target server Clippy passes in 75s and WASM
  Clippy in 14.34s, warnings denied. Final independent review has no material
  finding. T195–T197 are complete for this reload repair only; no SQL/cache,
  endpoint or policy activation changed. Publication to the existing draft is
  next; no full feature or full Nix acceptance is claimed.
- Published unsigned `1879b8a` to draft #212 and appended the verified scope and
  remaining directory work to its existing description. Final formatting
  `50092` changes zero files. No merge occurred. The next action remains the
  canonical directory integration below, not another reload investigation.

Next: verify the fix and recovery regressions, then connect canonical People,
shell/navigation and the initial page-requester binding. These are still required;
this repair does not complete directory integration, T018 or the feature.

## 2026-10-04 — Cross-command invoice authorization gate

- Previous turn made progress: authenticated project delegation and verification
  were published to draft #212 (`c4e83c8`). Revalidated that clean worktree and
  the constitution, plan, tasks and remaining project-form predicates. Spec Kit
  prerequisite script passes; its command skills remain absent locally.
- Current public permissions/rate/budget documentation still does not resolve
  initial creation scope or all non-rate form effects. A fresh read-only attempt
  through the existing Windows Playwright client (`97024`) initializes MCP but
  `browser_tabs` times out; no browser outcome or account mutation is claimed.
- Independent T042 review identifies a concrete invoice actor/organization-FK
  inversion against the real user-revocation command, distinct from the existing
  T068 assignment test. T192–T194 refine that cross-command requirement without
  changing invoice grants or relaxing the unresolved full-feature gates.
- Applied Rust, async, testing and simplicity guidance. Added a real-command
  regression for both invoice/user-revocation orders, using PostgreSQL waiters
  and the authenticated generation request. RED execution `75652` is pending;
  no production locking change has been made yet.
- RED `75652` reproduces `deadlock detected` (500) through those real commands
  in 1.19s. Added one common writer prefix: explicit READ COMMITTED/READ WRITE,
  organization SHARE, then the existing invoice advisory lock. Generation,
  editing and lifecycle transitions all use it before their actor checks.
  No financial/lifecycle/grant predicate changed. Verification is pending.
- Follow-up clarification did not change implementation: FR-033/B was already
  recorded. Polling `17952` established the original regression now passes
  (both command orders, 0.23s). This is focused evidence, not full acceptance.
- Added persisted invoice/receipt counts and actual edit/transition races in
  both orders, denied replays and inherited read-only/repeatable-read tests.
  `28313` exposed a test result-type mismatch; corrected it. `94723` passes all
  three non-cancellation tests. Its cancellation assertion incorrectly required
  PostgreSQL rollback before releasing an in-flight advisory wait and times out.
  Independent review confirms SQLx queues rollback; cancellation is not a query
  CancelRequest. Corrected the test to release the blocker, verify rollback and
  reuse a one-connection pool, matching the existing transaction contract.
  No new immediate-cancellation guarantee or production timeout was added.
- Final-source `72281` rebuilt and passed the original regression, but its
  temporary runner still selected the default single test and then hit a driver
  syntax error. It is not full-suite evidence. Validated the corrected runner
  before starting `11887`; the sandbox initially denied Nix's daemon socket, and
  the authorized retry uses the same disposable-database workflow.
- `11887` passes the complete server-binary suite: 1,154 passed, zero failures,
  11 pre-existing ignored in 121.31s, including all four new authority tests and
  registered-session, user/project, invoice and financial regressions. Independent
  review approves the final test/production delta. T192/T193 are complete.
  Removed only this worktree's regenerable Horae dev artifacts (220 files,
  1.7 GiB) for complete SQLx preparation. SQLx/offline lint phases remain live;
  T194 stays open. No source, database, policy or real-account data was removed.
- `11887` completes successfully: SQLx preserves all 1,520 existing descriptions
  and adds four; offline all-target server Clippy passes in 1m12s and WASM Clippy
  in 14.51s, with warnings denied. Formatting `36426` changes zero files. T194
  is complete for this bounded invoice/user-revocation integration. No full Nix,
  canonical invoice policy, full permission feature or activation claim follows.
- Published unsigned `a25e544` and updated draft #212 with the regression,
  verification and remaining boundaries. GitHub confirms that exact head on the
  open draft. Final formatting `19348` changes zero files; the implementation
  worktree is clean after publication. No merge or real-data operation occurred.

Next: resume the full T014/T015 integration inventory. T159/T160's project-form
effects remain gated by the already-pending initial-designation and non-rate
financial predicates; investigate resolving Harvest evidence before connecting
the real form, without repeating settled FR-021/022/033 decisions. Continue
independent resolved integration if those predicates remain open. Scoped invoice
policy, approval lifecycle and full activation remain required, not completed by
this repair. Do not rerun the closed deadlock investigation without new evidence.

## 2026-10-04 — Authenticated project-manager delivery

- Previous turn made progress: published verified `5f7895c` to draft #212.
  Revalidated the clean worktree. Rechecked current Harvest permissions/budget
  guides; they still do not close all composite project-form financial effects.
  No new product decision, restricted-user observation or account write follows.
- Independent review identified the closed FR-026 existing-project delegation
  contract as implementable without creation, financial or person-management
  activity decisions. T189–T191 connect its existing transaction, not a new
  policy engine or ad-hoc UI. Checked Spec Kit prerequisites; command skills are
  absent locally, so no slash-command execution is claimed.
- Added exact retained-set and authority/revocation/cancellation tests. RED `2150`
  fails the real reader test with `Forbidden` against the placeholder. Shared
  DTOs preserve the command's exact serialized tag/fields and receipt format.
- Implemented minimal reader and authenticated read/save wrappers. Both reader
  and existing writer share current actor/project-edit authorization under their
  organization gates. Requester binding stays outside durable intent; current
  authorization still precedes replay. Retained inactive/incompatible people
  are returned without private fields or permission-state reads.
- Added registered HTTP cases for identity binding, exact payloads, scoped
  authority, stale/duplicate/invalid batches, replay, self-removal and audit
  denial to non-administrator authors. Reader-first/revocation-first checks use
  real PostgreSQL blockers, including direct activity changes and cancellation.
- Applied Rust/testing/async and simplicity guidance. Removed only this thread's
  two redundant Dioxus native caches (1.7 GiB) after confirming they had no live
  compiler; sources, tested native/client artifacts and databases were preserved.
- User reconfirmed FR-033/B: terminal timer recovery stays owner-only. This
  repeats the recorded decision and requires no broader delegated exception.
- Focused `43174` passes all 29 project-management tests, including both direct
  activity lock orders. Its registered HTTP phase fails before authentication.
  Diagnostic `48468` confirms the writer's serialized `kind` is rejected by the
  derived struct deserializer. Replaced the serialization-only struct tag with
  an explicit, single-variant discriminant: durable JSON is unchanged and input
  remains strict. Added exact round-trip, wrong/missing kind and extra-field tests.
- Independent static review approves the activity/audit test delta. Final
  `49140` runs the server-binary regressions, complete SQLx preparation and
  offline server/all-target and WASM lint. Results are pending.
- Independent review also approves the strict discriminator fix. `49140` now
  passes all server-binary regressions: 1,150 passed, zero failures and the 11
  pre-existing ignored cases in 114.00s. This includes the corrected registered
  HTTP matrix, 29 delegation tests and the DTO round-trip test. SQLx/offline
  phases remain in progress; T189/T190 are complete, T191 remains open.
- `49140` ends with WASM dead-code lint for the five new DTOs, not a failing
  runtime test. Applied the existing pending-UI convention only to this shared
  DTO module and only outside the server build. Its cached SQLx preparation
  also omitted 91 existing descriptors; do not publish those deletions.
  `52861` regenerates after cleaning only the validated worktree's Horae dev
  package artifacts (dry-run: 330 regenerable files, 7.9 GiB), then reruns both
  offline lint targets. No sources or databases are removed by that cleanup.
- Clean preparation `52861` preserves all 1,512 previous SQLx descriptors and
  adds eight. Offline all-target server Clippy passes in 1m18s. WASM rejects
  the module-level lint expectation as unfulfilled; moved that expectation to
  each of the five pending DTOs, following the existing project-picker model.
  This is compile-time annotation only; `76301` verifies the final WASM build.
- `76301` identifies two redundant expectations on nested types. Kept only
  the three top-level DTO expectations; final WASM Clippy `69854` passes with
  warnings denied (13.84s). No runtime code changed after the full test pass.
  T189–T191 are complete for authenticated backend delivery, with the independent
  review closed and all existing SQLx descriptions preserved.
- Published unsigned implementation commit `84d5352`; GitHub confirms the exact
  head on open draft #212. Its description retains prior evidence and adds this
  increment. Final formatting changes zero files; the worktree was clean after
  publication. No merge or policy activation occurred.

Next: resume the remaining permission integration, resolving the composite
project-form field predicates before connecting canonical manager editing to UI.
This delivers no form/browser integration or policy activation;
remaining field predicates, new-relationship activity and approval decisions are
unchanged. The full permission goal remains active.

## 2026-10-04 — Delegated Calendar mutation acceptance

- Revalidated published `e29f4d8` and reused draft #212. Extended the isolated
  scoped Timesheet fixture with real creation, Calendar drawing/movement/resize,
  two-entry ordering, modal deletion and one-command Week row deletion (T188).

- RED `21196` moved a timed block from Monday 03:00 down one hour into Tuesday
  but persisted 04:15 instead of 04:00. Native mouse offsets were relative to
  nested text. Four noninteractive Calendar labels now ignore pointer hit-testing;
  the event container and separate resize handle retain their behavior. No
  shared utility, token, visual layout or authorization change.

- Independent review required a real two-entry order inversion and multi-ID
  row deletion; both now assert exact payloads and persisted rows. It finds no
  unintended selector effects. Successful joint deletion alone is not proof of
  transactional rollback; the existing mutation-error suite covers stale locks.

- RED `83821` exposed a second defect while creating the Tuesday sibling:
  Day's Add entry chose Monday. Single-day views now use their displayed date;
  week defaults are unchanged. The regression checks Day's dialog and persisted
  date, plus Calendar Day view's Tuesday dialog before cancellation.

- Reused Rust/testing, simplicity and UI-hardening guidance. No new dependency,
  schema, real-data write or permission activation. FR-033/B remains owner-only.

- Browser `6400` confirmed corrected Day creation, then failed on the test's
  exact menu-trigger name, which includes a caret. Reused its stable trigger ID
  and bounded browser actions to 15 seconds; no application change for this.

- Final combined Chromium `48723` passes all eleven scoped scenarios plus
  modals, mutation errors and complete permission recovery/history. Exact SQL
  assertions retain Tuesday 04:00, 90-minute resize, both sort orders and
  two-ID deletion. Existing error coverage confirms stale-lock rollback/retry.
  No JavaScript page errors; scoped fixtures clean up before neighboring suites.

- Client assets regenerated with Dioxus; its redundant native build was stopped,
  not counted as a full successful Dioxus build. Native `49421` passes (54.88s),
  asset processing `38094` passes, server Clippy `72570` passes with warnings
  denied, WASM check `49082` passes and formatting `91491` changes zero files.
  Independent review also approves the date correction; its requested Calendar
  single-day assertion passes. T188 is complete for this bounded acceptance.

Next: publish the verified increment to draft #212 without merge, then continue
the remaining connected permission integration. Outstanding new-relationship
activity and approval questions are unchanged. These four Linux Chromium suites
are not Windows Chrome, full Nix acceptance, real activation or MVP completion;
the full goal remains active.

## 2026-10-04 — Scoped Timesheet browser navigation and recovery

- Reused published `68bbaae`, the existing worktree and draft #212. Added a
  runner-only scoped-policy fixture with distinct requester/subject sessions;
  no real policy, account or data was changed.
- Initial fixture executions rejected a missing explicit billable value and a
  timed entry extending past midnight. Corrected the fixture, retaining its
  hidden 1,200-minute entry as untimed; neither failure tested application behavior.
- Browser `36981` reproduces a real navigation defect: selecting the teammate
  changes the URL but retains Admin User's sheet and sends no replacement read.
  Independent review traced it to singleton component keys not remounting in
  Dioxus 0.7.9. The existing key now lives in a keyed dynamic fragment; the parent
  still pins the requester across navigation. No CSS or authorization change.
- Upstream context: [Dioxus issue #4688](https://github.com/DioxusLabs/dioxus/issues/4688)
  reports the same inline-key behavior; the local reproduction, not issue status,
  is the acceptance basis. Browser history and zero-entry-person checks retain
  this regression. T187 remains open until the complete new suite passes.
- Independent test review strengthened shell-timer identity verification and
  draining of second-session reads before navigation/closure. The fixture also
  exercises the confirmed FR-033/B owner-only recovery rule.
- Client `83179` completed in 68.34s after correcting RSX key syntax; interrupted
  only the redundant native profile. Separate native build and browser acceptance
  followed. Native `40249` failed during linking with signal 7 at under 1 GiB free.
  Verified all compilers stopped and removed only the two regenerable redundant
  native caches (2.8 GiB). Native retry `46585` passes in 56.64s; asset processing
  `39860` passes. Sources, client assets and all databases were preserved.
- Browser `2345` passes navigation/history, Day/Week writes and gated revocation,
  then reveals an incorrect test expectation: loss of tracking eligibility is
  the existing explicit 409 conflict, not 403. Confirmed `eligible` and retained
  an exact error-message assertion together with unchanged running state.
- Final combined browser `83994` passes all seven scoped scenarios, all modal
  and mutation-error cases, and the complete permission recovery/history suite.
  The latter starts after scoped-fixture cleanup in the same disposable database.
  No JavaScript page errors; distinct real owner session proves FR-033/B.
- Independent review approves the minimal keyed-fragment fix; strengthened
  history checks also assert previous rows disappear for the empty teammate.
  Server Clippy `27895` passes with warnings denied; WASM check `11713` passes.
  T187 is complete and the new suite joins the default runner (22 scripts).
  This is four-suite Chromium acceptance, not a fresh complete Nix/full-browser
  gate or every delegated Calendar/create/delete flow.

Next: publish this verified repair on draft #212 without merge, then continue
remaining connected permission work. New-relationship activity and the separate
approval decision remain unanswered; no inferred product decisions. Full-feature
enforcement, reviewed activation and MVP acceptance remain open.

## 2026-10-04 — Timesheet mutation browser acceptance

- Previous turn only reconfirmed FR-033/B; no implementation progress. Revalidated
  clean `2f5357f` and reused the isolated feature worktree and draft #212.
- Adapted existing `timesheet-errors.cjs` to requester-bound atomic commands,
  current page reads and runner-only database validation. Fixtures are created
  transactionally, collision-refusing and removed after the browser closes.
- Chromium `12819` passes all six cases: stale-client submitted lock rejects the
  entire row deletion, unlock/reload permits retry, failed move/resize/reorder,
  committed reschedule with lost response reconciles actual data, and timer-start
  transport error. This does not settle final submitted-editing parity.
- Adapted `modals.cjs` to shared selectors and current commands. Chromium `5141`
  passes keyboard/inertness/backdrop/short-viewport checks for Export, Add entry
  and Add row, then reproduces lost focus after a save attempt: the authorized
  refresh disables the opener before native dialog closure.
- Reused the shared Modal's existing `app-main` focus fallback in Timesheet;
  no shared component, CSS, permission guard or persistence change. Independent
  review also identified a transient-empty-grid deletion false positive; the
  test now verifies the refreshed complete DTO and Ready controls before absence.
- Follow-up `91539` clarified that the first failure was Cancel immediately after
  a failed retry, while tracking authority was still refreshing. Normal Cancel
  now waits for an available opener, separately from fallback assertions after
  successful create/update/delete. `92882` passes all ten scenarios in both suites
  against the rebuilt client/server.
- Native build `35587` passes (72s), asset processing `35262` passes, server
  Clippy `39842` passes and WASM check `54166` passes. Dioxus `11558` regenerated
  the client but also compiled a duplicate native profile despite `--fullstack false`; interrupted that redundant build and verified its processes stopped.
  Removed only the two caches it created this turn, retaining the tested native
  and client artifacts. Its interrupt is not a successful full Dioxus build.
- Both suites are registered in the default runner. Combined run `46759` passes
  the first 18 suites, then exposes real horizontal overflow in both Timesheet
  dialogs after earlier suites create long project names. It exits 1; not a green
  full gate. Standalone RED `76196` reproduces with 200-character catalog labels,
  keeping real eligible IDs and all mutation authority unchanged.
- Replaced TrackingPicker's implicit grid track with existing column-flex
  utilities, locally shared by both Timesheet dialogs. No CSS/global selector
  change. The modal suite now always supplies long labels and tests their actual
  presence and bounds. Initial edit focus is required inside the native dialog,
  rather than on a particular asynchronously arriving control.
- Client rebuild `16869` reports completion in 38.95s; stopped only its redundant
  native build after that message. Separate native build `18834` passes (56.75s),
  followed by asset processing `21606` and server Clippy `70801`.
- Final Chromium `23232` passes modals with long labels, all six mutation/error
  cases and the complete permission-editor recovery/history suite, including its
  loaded-CSS guard. Independent review approves both local UI changes and closes
  the deletion false positive. All 21 default scripts have passed across the
  complementary `46759`/`23232` blocks, not an uninterrupted green full gate;
  no full Nix acceptance or full permission completion is claimed.
- Independent research identifies the next connected person-management writer
  and its mandatory audit/recovery consumers. Read official Harvest assignment,
  teammate API and archive documentation; none settles activity of new relation
  endpoints. Asked one scoped question and recorded sources/limits in the
  person-management contract; no answer or archival behavior is inferred.

Next: deliver this regression repair on draft #212 without merge, then implement
the connected person-management command/editor once the new-relationship activity
question is resolved. Reuse existing persistence, replay and profile-loss logic;
extend strict audit decoding/history together. Full permission implementation,
activation and the separate outstanding approval decision remain open. No real
data or Harvest account was changed.

## 2026-10-04 — Permission history browser acceptance

- Previous turn only acknowledged the already-recorded FR-033/B clarification;
  it was not implementation progress. Revalidated clean `2078a13`, reused the
  isolated feature branch and extended the existing disposable Chromium suite
  rather than creating another fixture or changing application behavior.
- T185 exercises empty history, 29 real person/template receipts, exact 25+4
  paging, native keyboard details, deleted-template snapshots, pending-content
  suppression, revocation/restoration and canonical Administrator access from
  Settings with a legacy Member role. Existing recovery assertions remain.
- Independent static review identified a possible false positive in the held
  history request. Added an explicit interception barrier before assertions;
  independent re-review confirms closure. No additional agent was created.
- Dioxus `build` unexpectedly created implicit native `server-dev` caches. At
  low disk space, stopped the owned build (`37758`; its interrupt reports exit 0,
  not a completed build) and verified no child compiler remained. Removed only
  its newly created `target/server-dev` and `target/x86_64-unknown-linux-gnu`
  directories, recovering 5.7 GiB. No prior build, worktree or database was removed.
- The client bundle completed before interruption. Native `cargo build` on the
  retained default target passed in 57.92s (`6259`). Linked its adjacent `public`
  directory to that current client output without copying another binary.
- First Chromium run `46457` passed functional checks, but inspecting both
  captures showed missing CSS. This is **not visual acceptance**. Added a guard
  requiring loaded theme tokens and generated utilities. Used the official
  `dx tools assets` command to process the current native executable and assets
  (`52256`, exit 0); no application/CSS change or stale bundle substitution.
- The first styled run `80901` stopped on the new guard's uppercase color
  expectation: Dioxus minification returns lowercase. Corrected the test, then
  `12852` passed all recovery/history assertions. Inspected both styled history
  captures. Follow-up review made the guard palette-independent: nonempty theme
  token plus computed `display:flex`, rather than a particular hex value.
- Complete browser gate `65390` exposed stale waits for `list_time_entries` in
  Timesheet navigation. Updated the responsive/menu/style-audit readers to the
  actual `load_timesheet_page` endpoint. The New Project reporting check now
  awaits `apply_timesheet_command`, then verifies the refreshed scoped DTO's
  unique entry, project, integer minutes and date rather than expecting the old
  mutation's returned entry. No behavior assertion was removed. Rerun `55544`
  passed through responsive/menu/mobile/bulk/action checks, then found an obsolete
  native-select locator in New Project's Timesheet scenario. Updated it to use
  the actual shared Project/Task selectors and assert the fixture's refreshed
  page is complete. Continued the remaining default suites in `3987`, starting
  from New Project, without claiming the interrupted complete runs passed.
- `3987` completed with exit 0, including real project → Timesheet → reports and
  both exports, invoice preparation, rate/permission/error/keyboard matrices,
  interrupted transport, editing/navigation and the final history/recovery suite
  with its palette-independent CSS guard. All 19 default scripts have therefore
  passed across the complementary blocks (`55544` before its New Project
  failure, then `3987`), **not a single uninterrupted green full-gate run**.
  The optional shared-style baseline script was reviewed and syntax-checked but
  not executed; no baseline comparison is claimed.
- Independent review of the endpoint and selector adaptations found no material
  issue. Its page-completeness suggestion is included in the passing New Project
  scenario. Syntax checks for all five changed scripts passed (`64620`), and
  format-CI passed (`17386`, 535 files, zero changes). No Rust, CSS, schema or
  dependency changed; no fresh server-unit, SQLx or full Nix gate is claimed.

Delivery remains the isolated feature branch and draft #212, unsigned and without
merge. Next: update and execute the remaining standalone Timesheet modal/mutation
browser scenarios (`modals.cjs`, `timesheet-errors.cjs` still reference old APIs),
then continue the approved permission consumers/person-management flow. The
complete implementation, cross-surface enforcement and activation remain open;
the outstanding approval product question is not answered by this work.

## 2026-10-04 — Connected permission history

- The prior clarification response was not implementation progress: FR-033/B was
  already recorded. Revalidated published `b8b1c60` and the existing draft #212,
  then continued the independent FR-013 history flow. The future-project approval
  question remains unanswered; no approval behavior or activation is inferred.

- Reused the existing authorized specialist for backend/DTO/session tests in this
  worktree. Main owns the consumer, route/shell, UI tests and documentation; all
  builds are serialized. No new agent, branch reset, schema change or real-data
  operation. The complete feature and existing high-level acceptance remain open.

- Extended the audit contract, plan, operation matrix and T182–T184. Source-read
  the Workspace Audit log, shared subnavigation, design-system/table references
  and production shell. Its example events, names, counts and contradictory
  retention copy are not runtime requirements. The consumer displays actual
  permission receipts only, with recorded UUID/operator attribution and UTC.

- Backend RED `84646` failed the production paging assertion with `Forbidden`
  from the intentional placeholder. An earlier attempt `42695` only exposed a
  test-module path error and is not behavioral evidence. Backend now reuses the
  existing transaction gates and strict decoder for tenant-first 25+1 paging.

- UI RED `23494` showed no initial history request from the placeholder. Added
  the connected `/admin/audit` consumer, captured requester, replacement paging,
  stale-content suppression, empty/error/retry/end states and native historical
  details. Workspace Data and canonical Administrator Settings expose the route.
  Only this route uses the canonical shell gate; legacy People/Importers gates
  remain intact. Shared grant/profile descriptions are reused; no shared CSS
  defaults, dependencies or prototype controls changed.

- Independent static review found a P2: the new shell authorization path could
  render an internal authentication error. Replaced that path's error copy and
  added a passing sentinel regression. Intermediate runs verified shell/Settings
  and requester/paging behavior. The detail test initially expected named HTML
  entities; inspection proved Dioxus correctly emits decimal entities, and the
  assertion now checks the actual escaped literal rather than weakening safety.

- Full server plus affected UI regression `94206` completed with exit 0: the
  complete 1,154-case server binary (including its existing ignored probes),
  seven shell tests, eleven own-permission UI tests, five history UI tests and
  fifty editor UI tests. Registered HTTP and new PostgreSQL concurrency tests
  pass. Formatting
  `82785` passed (535 files; eight changed). Complete SQLx preparation `15682`
  passed in 1m27s, adding five descriptors and preserving every existing one.
  Offline all-target Clippy `66983` flagged one nested conditional; corrected it
  without an allowance. `88926` then passed native/all-target Clippy (1m28s),
  WASM Clippy (16.85s) and all five history UI tests. No browser/MCP tools
  are loaded. The checked-in Spec Kit command skills are absent from this
  worktree; no fresh specify/clarify/analyze invocation is claimed.

- Followed the disclosure finding to its origin: `get_my_permissions`, now used
  by the audit shell gate, still propagates `require_user` storage diagnostics in
  its HTTP response. Extended the real-session cancellation probe to the lookup,
  paged history and own-permission gate. RED `54837` reproduced a 500 containing
  PostgreSQL's cancellation diagnostic from `get_my_permissions`. Sanitized that
  wrapper's authentication failure while preserving 401; the final HTTP matrix
  in `25607` passes (10.98s), including all three injected failures and recovery.
  Final offline all-target native/WASM Clippy also passed in `25607` (1m21s and
  17.14s), with warnings denied. No SQL/schema change was needed
  for this fix. Formatting `8154` passed (535 files; three changed).

- Final formatting and format-CI `82090` passed (535 files, zero changes), and
  whitespace checks pass. GitHub confirms the existing open draft #212 on this
  branch; publication preserves the draft and documents the design deviations.

T182–T184 are verified within their documented scope. Publish unsigned on draft
#212 without merge. Next: browser acceptance on an isolated fixture when available,
then continue the approved person-management/permission consumers while the
whole-timesheet/future-project approval decision remains outstanding. Full
T018/T041, cross-surface enforcement, reviewed activation and the implementation
goal remain open; this is not MVP readiness.

## 2026-10-04 — Captured own-week submission and approval discriminator

- Previous response only acknowledged the already-recorded FR-033/B decision;
  classify it as no implementation progress. Revalidated clean `a0632a8` before
  this iteration; reused the existing branch/worktree and draft #212.
- Read the approval coverage/visibility boundaries and current Harvest flexible
  approval and submission documentation. A targeted official-source search did
  not settle future-created project coverage or post-withdrawal submission state.
  Asked one product question: whole-timesheet approval protects future projects
  too (A), or only the existing project set (B). **No answer yet; neither rule is
  implemented.** Do not re-ask while this question is outstanding.
- Browser/MCP tools are not loaded. No account mutation, browser evidence or
  actual Spec Kit invocation is claimed. Impeccable context could not install
  its engine in the restricted cache; read existing DESIGN.md and the hardening
  guidance instead. PRODUCT.md is absent; no design-system repair was attempted.
- Independent concrete gap: the remaining own-week submission omitted the
  captured requester/subject/policy used by ordinary Timesheet commands. Added
  a real-session regression: RED `16764` exited 101, returning 404 for the new
  session's empty week instead of rejecting the old page's identity with 403.
- Bound `submit_week` to the existing Timesheet context, retaining its legacy
  own-only contract. The production transaction now takes the organization gate,
  checks policy 0 and active local owner before the exclusive time-write barrier.
  Added both access-change orders and commit/rollback checks. No flexible
  coverage, submitted-editing or delegated submission rule is inferred.
- Full server regression `87811` completed successfully: **1,137 passed**, zero
  failures and 11 pre-existing ignored cases, in 137.12s. This includes the new
  registered-session checks, both access-change orders and existing submission,
  rounding, cancellation, time, invoice and UI regressions. Formatting `47827`
  completed successfully (532 files, two formatted).
- The existing authorized specialist completed its read-only review with no
  material finding. It inspected the organization → owner → Timesheet barrier →
  approval → entry order against adjacent writers and the UI context handling;
  it did not run tests or claim browser acceptance.
- Offline native/all-target Clippy and WASM Clippy `99074` passed with warnings
  denied (1m11s and 13.96s). SQLx `91933` returned success but removed 92
  descriptors, including still-used cached integration queries. Invalidating
  only integration's source timestamp in
  `17730` regenerated that target but omitted the cached binary descriptors.
  Neither incomplete generated cache is a deliverable. `99835` regenerated both
  after touching their entry-point timestamps together; inspection then found
  one still-omitted descriptor from `tests/cli_restart.rs`. Native/all-target
  Clippy `39525` passed, but reused that unchanged target and is not proof of its
  descriptor. `15539` invalidated main and **all** test entry-point timestamps
  together and completed successfully. The resulting cache adds five descriptors
  and removes only the superseded organization-rounding query; all still-used
  descriptors are preserved. Final offline/all-target Clippy `31781` passed
  (1m20s), including the freshly invalidated CLI/integration targets. Source
  contents outside the scoped diff, dependencies, schema and real data are unchanged.
- Final format and format-CI `13638` passed: 532 files, zero changes; diff checks
  pass. GitHub `82730` confirmed #212 is open/draft on the existing branch against
  master. Publish this verified increment unsigned without merging or activation.

Next: publish unsigned on #212, then continue FR-019's full
coverage/submitted-editing/expense work after closing its material lifecycle
decisions. No canonical policy activation or full-feature completion is claimed.

## 2026-10-04 — Verified selected-person ordinary command integration

- Reused `feat/scoped-permissions` and the authorized specialist's existing
  worktree. The specialist's completed report-reader branch and draft #217 are
  preserved; its new command branch starts at `e1ddd9a`. No additional agent,
  runtime policy change, migration or external Harvest write was introduced.
- Integrated the local command DTO contract (`48a6533`, from `8360b20`) to build
  the consumer against the agreed API. Its fail-closed placeholder is **not a
  deliverable** and must not be published without the verified implementation.
- Route RED `47573` failed the two new selected-person cases; route GREEN
  `83397` passed all four cases. Day/Week/Calendar retain the `user` query and
  malformed IDs remain explicit failures. The outer requester survives keyed
  person/date/view remounts. The existing history guard now recognizes Timesheet;
  Node RED `95605` failed its two new messages and GREEN `58241` passed 10/10.
- Connected the person picker, owner-specific minimal tracking choices and seven
  captured-context command intents in local UI code. Existing SelectField and
  utility classes are reused; no shared CSS or shell timer owner changed.
  Multi-entry row removal is one atomic command, not independent deletions.
  Current-response checks suppress old picker searches and tracking contexts.
  Dirty drafts/dialogs/rows and pending writes participate in navigation guards.
- Review caught a legacy compatibility edge: editing a historical source must
  not depend on eligibility for creating new time there. The local UI preserves
  that explicit policy-0 distinction, while canonical edits require eligible
  source pairs. Restart eligibility is independent of the source entry's state;
  read access alone enables neither editing nor a delegated stop.
- The specialist reports seven passing PostgreSQL cases after an initial
  two-case RED against placeholders. Parent read the implementation and requested
  sanitized authenticated errors plus real-session coverage. A hypothesized
  rejected-approval row was disproved by schema and `reject_submission`: rejection
  deletes that row. Require a reopen/write regression, not a fabricated status.
- Intermediate consumer run `4551` passed all 41 Timesheet tests, including
  actual Day renderer read-only controls, preserved Week inputs and advisory
  owner/delegate recovery predicates. Subsequent small DTO/label/identity changes
  still need the final integrated run. Formatting `95384` passed; no browser
  acceptance is inferred from these VDOM tests.
- Current boundary: submitted editing, weekly coverage and combined submission
  remain unfinished FR-019 work. Legacy own submission still uses the existing
  separate endpoint; it is not delegated or complete captured-context approval
  integration. No activation or full Timesheet acceptance is claimed.
- Integrated the real backend as `02c4245` (specialist `a0ea691`), replacing the
  placeholder. Its 13 PostgreSQL tests and registered-session matrix pass, with
  real rejection/reopen, sanitized failures and both revocation orders. Complete
  SQLx adds 38 descriptors without removal; specialist server/all-target Clippy,
  format and diff checks pass. Independent UI review found no additional high
  identity/retention issue. Parent full server regressions are running in `34248`
  against the specialist's Unix-only disposable PostgreSQL, not real data.
- Integrated full-server `34248` passed **1,135 tests**, zero failures and 11
  pre-existing ignored cases in 141.14s. This includes the final 41 consumer
  tests, five route cases and the registered-session matrix. All-target lint
  `28922` found the separate AdminShell test router still lacked `user`; updated
  only that controlled fixture and started its five tests plus both lint targets
  in `75640`. No production fix or server-suite rerun is implied by that fixture
  correction.
- Final `75640` passed all five AdminShell regressions, offline server/all-target
  Clippy (1m15s) and WASM Clippy (14.33s), both with warnings denied. Formatting
  `9385` checked 531 files with zero changes. GitHub `62107` confirms #212 remains
  open/draft on `feat/scoped-permissions` against `master`. No temporary command
  implementation remains. Publish the verified consumer and evidence unsigned
  on that existing branch, without merging or activating the canonical policy.

Next: after publication, integrate FR-019's actual approval coverage, submitted
editing and frozen rounding, starting with the concrete transaction/storage
dependencies below. Do not replace these with an Open-only product rule or ask
the confirmed recovery question again. Browser acceptance remains unavailable,
not passed; the full goal remains active.

FR-019 dependency checkpoint: `submit_user_week` freezes `rounded_minutes` under
the exclusive owner barrier; `approve_periods` and `reopen_period` lock approval
rows before entries without the organization/owner fences needed for scoped
commands. Storage remains whole-week `(user_id, period_start)` rather than
project/date coverage. The current explicit conflict covers any approval interval
intersecting a source or destination (including empty Submitted cells), non-Open
entries and billed rows. Reconcile those writers, storage and rounding before
removing that temporary boundary. Retain the distinct legacy submission and
combined time/expense acceptance requirements.

## 2026-10-04 — Actual Timesheet consumer and terminal recovery decision

- Revalidated `5faed76` on the existing worktree/branch; the preceding delegation
  status answer was no implementation progress. The page-context commit was
  already pushed to draft #212; its SQLx/native/WASM gates were complete, not
  work to repeat. No new worktree or replacement feature was created.
- Connected the existing own-sheet Day/Week/Calendar to `load_timesheet_page`,
  consuming `VisibleTimeEntry` directly. Continuations retain requester, subject
  and policy and cannot publish partial totals. Historical labels no longer
  depend on the general client directory or currency data. Own tracking choices
  and mutation endpoints remain separate pending delegated integration.
- RED loader run `30030` exited 101: all five tests failed against a one-page
  stub. Initial consumer run `44419` passed 33 tests. Independent review found
  two P2 regressions: a Ready value could belong to the old week, and refreshing
  one cell could unmount another cell's uncommitted DOM input.
- Fixed date context by tagging both results and errors with their requested
  week. Added a real Dioxus Week-renderer input/remount test: the first attempt
  had a Rust pattern mismatch (`39352`), corrected before the meaningful RED
  `68384` failed on lost input. Parent-owned dated drafts fixed retention; the
  intermediate `2743` passed 35 tests. Follow-up static review caught restored
  drafts needing blur-based save and explicit row discard; both were corrected,
  with pending-write guards and captured-week draft removal.
- Final consumer tests in `56864` pass 36/36, including restored input/blur
  dispatch and cross-week/task discard preservation. The same process completed
  with exit 0: offline server/all-target Clippy (1m22s) and WASM Clippy (16.54s),
  both with warnings denied. No SQL or migration changed, so cache regeneration
  is unnecessary.
- Format and format-CI run `61224` completed with exit 0: 524 files checked,
  zero formatting changes; `git diff --check` passed. Publication stays on the
  existing unsigned-commit workflow for draft #212, without merge.
- The authorized parallel specialist published unsigned `dd141c5` in draft
  [#217](https://github.com/numtide/horae/pull/217), based on `5faed76` against
  `feat/scoped-permissions`. It reports 1,331 passing server tests, 11 existing
  ignored, 1,473 SQLx descriptors and passing offline server/WASM Clippy/format.
  Parent reviewed its bounded production/transaction/HTTP change; no merge.
- Investigated delegated tracking prerequisites using current official Harvest
  help. Existing FR-006 restrictions are not overridden by time-write grants.
  The remaining terminal-stop exception was asked once; the user selected **B**:
  only the owner may recover after losing tracking eligibility. Recorded FR-033
  and propagated it to the plan, integration contract and tasks. This is Horae's
  explicit rule, not an observed Harvest custom-permission result.
- Rust/testing/async and simplicity guidance kept the consumer within existing
  types, controls and dependencies. Impeccable hardening guidance focused on
  loading/error/stale-input states; no shared CSS or design-system change.
  Browser/MCP tools remain absent; VDOM evidence is not browser acceptance.
  No Spec Kit command execution is claimed in this environment.

Next: implement the **selected-person vertical flow**:
route/picker, target tracking contexts, captured actor/owner commands and all
Day/Week/Calendar/timer consumers together, with FR-033 denial/restoration tests.
Do not ship another disconnected reader as that flow. Dirty/pending navigation,
atomic row removal, submitted editing, real expenses/approval coverage, privileged
corrections, migration/cutover and complete acceptance remain open. The full goal
remains active. No real data, runtime policy or external Harvest state changed.

## 2026-10-04 — Bound selected-person page reads

- Revalidated existing `60f60f9` worktree and active goal after the delegation
  status turn. That status response itself was not implementation progress.

- Implemented `load_timesheet_page` and its explicit query/response in existing
  time modules. Subject resolution and entries share one transaction and both
  requester/target activity fences. Reused candidate/entry SQL, rather than
  composing independently authenticated reads or fabricating internal entries.
  Policy 0 is explicitly own-only; policy 1 never falls back after denial.

- The initial RED (`58531`, exit 101) failed all three planned positive-context
  tests on the temporary unimplemented reader. GREEN (`5996`, exit 0) passes six
  context tests (6.35s), eight candidate regressions (1.53s), eight scoped-reader
  regressions (1.67s) and the real-session HTTP matrix (9.42s). The tests cover
  empty selection, unrelated-project exclusion, identity/policy changes, 501-row
  paging, malformed query/state, selected-person archive ordering and cancellation.

- Real HTTP coverage proves session-derived identity, legacy own compatibility,
  canonical subject projection with exact safe fields, forged outer identity,
  foreign requester, unavailable subjects, sanitized failures and inactive denial.
  No browser result or page integration is claimed. SQLx and native/WASM lint are
  still pending for this increment; source formatting and diff checks pass.

- The parallel specialist reports a reproduced six-case revoked-reader leak and
  incoherent invoice header/lines. Its seven focused reader tests now pass; a
  route-name ambiguity in its HTTP test helper was fixed, and its full server
  suite is running. Parent has not yet reviewed its final verified diff.

- Final verification (`63000`, exit 0): complete SQLx preparation (1m07s),
  offline all-target server Clippy (1m13s) and WASM Clippy (13.70s), both with
  warnings denied. All 1,465 SQLx descriptors remain unchanged: the implementation
  reuses existing SQL. Cleaned only 1.7 GiB of regenerable Horae package artifacts
  after validating the worktree target; the private PostgreSQL was stopped.

- Bounded parent adversarial review found no high/critical issue in the new read
  boundary: no authority from expected identity/policy, no fallback on denial,
  stable selected-person activity, independent row scope, minimal HTTP projection
  and no changed legacy consumers. This is not completed UI/delegated-write review.

- Reviewed the specialist's production diff and complete transaction/HTTP tests;
  no high/critical finding in that bounded legacy repair. It reports all 1,325
  server tests passing (1,104 in-crate, 221 external; 11 existing ignored tests).
  Its SQLx/lint verification remains independent and in progress.

- Read the remaining Timesheet renderers/tests and the full handoff Design System
  page (identical in both worktrees). Component-kit/import reading and the
  immediate pre-edit craft-floor step are still pending before visual changes.

Next: complete SQLx/native/WASM checks and publish the context increment on draft
#212, then connect the actual Timesheet consumer and commands. Finish the remaining
handoff imports before visual edits. T014/T015/T018, policy activation/migration,
expense/approval coverage and lock semantics remain open; no merge or real-data
change was made. Keep the full goal active.

## 2026-10-04 — Connected Timesheet integration and parallel reader repair

- Published candidate discovery as unsigned `60f60f9` on existing draft #212;
  push completed successfully and the worktree was clean. Final format CI passed
  with zero changes. T179–T181 are complete, not the full permission feature.
- Read the full Timesheet handoff and traced current page loading, URL state,
  grid/dialog/Calendar dispatch, `ProjectTaskPicker` and `RunningTimer`. The
  handoff has no teammate selector. More importantly, all current actions and
  tracking choices assume the session person; edits omit project/task movement,
  and row removal uses independent requests. Recorded the connected integration
  boundary in `contracts/timesheet-integration.md`, including requester/subject
  binding, stale responses and preserving shell ownership. No UI change yet.
- Read `design-implement` and Impeccable's instructions. The context launcher
  failed because its engine is absent and the default cache is not writable;
  reported the prescribed fallback and read existing design context directly.
  `PRODUCT.md` is absent. Preserve the incumbent world; no tooling installation,
  design-system rewrite or browser validation is claimed. Shared handoff imports
  still need to be read before visual implementation, and craft-floor guidance
  must be read immediately before UI edits.
- The specialist found no new defect in completed export contracts, but located
  the already-open T014/T039 revocation gap in `report_detailed`, `list_invoices`
  and `get_invoice`. Parent inspection confirms these pass organization identity
  after an initial manager gate without retaining the actor. Delegated the
  bounded legacy repair and real race tests in a separate worktree/branch from
  `60f60f9`, with a draft dependent PR allowed after verification, no merge.
- Reflink is unavailable. The specialist will use its own target with debug
  symbols/incremental disabled and two build jobs; no shared target, full-copy
  fallback or unrelated cleanup. Parent compilation remains idle while the
  first reproduction is prepared. No browser/MCP tools are loaded in this turn.

Next: implement the connected Timesheet context/commands and screen path from the
recorded boundary, completing its required source/reference reading first. Review
the specialist's actual reproduction and verified diff when returned; do not
mistake its proposed test for executed evidence. Approval/expense coverage,
company locks, full policy migration and cross-surface acceptance remain open.
The goal stays active with parallel implementation work, not complete or blocked.

## 2026-10-04 — Timesheet candidate discovery integration

- The user confirmed A: active managed-project participants are selectable even
  without time entries; visible hours remain restricted to authorized projects.
  The decision was already published in `9b53182` and is not asked again.

- Implemented minimal ID/name discovery under the existing scoped-time read
  transaction, independent of the displayed dates. Direct person management,
  project participation and retained project history form a deduplicated union;
  no directory, finance, legacy-role or Administrator-identity bypass is added.

- Added session-authenticated `list_timesheet_people`, search/keyset pagination
  and selected-ID narrowing. This does not activate policy, connect the selector
  UI or grant delegated writes. OP03 and the Timesheet contract track the boundary.

- T179 initial RED failed with the expected unimplemented-reader denial. The
  first GREEN run (`11838`, exit 0) passes five tests covering profiles/custom
  scope, zero-entry membership, history, tenant parents, pagination and revocation.
  Three further tests add reader-first ordering, cancellation/inherited pool
  defaults and invalid policy/state. The real HTTP matrix checks exact minimal
  payloads, forged authority, sanitized failures and no unrelated-project hours.

- Resumed verification after status-only user exchanges. Session `74926` runs
  complete SQLx preparation, all eight candidate tests, existing scoped-time
  regressions, authenticated HTTP coverage and server/WASM Clippy against its own
  disposable PostgreSQL. Results are pending, not covered by the older Nix gate.

- Rust/testing/async and simplicity guidance keep the implementation in existing
  modules with one shared authority helper and no new dependency. Spec Kit command
  skills remain absent; do not equate manual artifact updates with running them.

- `74926` is terminal, exit 101. All eight candidate tests (2.00s), all eight
  existing scoped-time tests (1.95s) and the authenticated HTTP matrix (10.66s)
  pass. Offline all-targets Clippy caught 90 unchanged integration-test SQLx
  descriptors omitted by Cargo's cached compilation; this is not a reader test
  failure. Session `85936` validates the exact worktree target, cleans only Horae
  package artifacts, then fully regenerates SQLx and repeats native/WASM lint.

- Bounded adversarial self-review recorded in `quickstart.md`: no directory or
  financial bypass, no scope expansion through discovery/search/cursors, current
  authorization after waits, minimal HTTP projection and preserved history. This
  is not independent review or full-feature acceptance. The prerequisite helper
  passes; no new browser observation, policy activation, schema or CSS change.

- `85936` is terminal, exit 0. Clean SQLx regeneration passes (1m10s), all 1,460
  previous descriptors remain unchanged, and five new descriptions bring the
  total to 1,465. Offline all-targets server Clippy (1m28s) and WASM Clippy (47.95s)
  pass with warnings denied. Removed only 10.8 GiB of regenerable Horae package
  artifacts from the validated worktree target. The temporary PostgreSQL stopped.

- The user authorized a parallel specialist. `reports_permissions_review` is
  reviewing reports/export authorization read-only, without touching Timesheet
  files or competing for compilation resources. Findings are not yet received;
  this is separate from the completed candidate self-review.

Next: run the final formatting gate and publish the increment to existing draft
#212 without merging. Continue
the selected-person/delegated-operation contract and actual Timesheet consumers;
T014/T015/T018, lifecycle coverage, migration review and full acceptance remain
required. The goal is active, not complete or blocked.

## 2026-10-04 — Authenticated template-capacity acceptance

- The intervening benefits explanation was no implementation progress. Resumed
  existing worktree state, collected the terminal formatter and published the
  full-gate evidence as unsigned `8af562e` on existing draft #212. No merge.

- Rechecked the lock overview/Q&A through Harvest's public Help Center API;
  the web reader failed but public JSON succeeded. Calendar edge semantics
  remain unspecified, so T043 is not implementation-ready. No account access.

- Continued independent T016/T017 acceptance via T178: real session requests
  from two different canonical Administrators at count 49, observed concurrent
  database waiters, overflow with the current revision, exact replay and
  state/receipt preservation. No production code or permission rule changed.

- Read the Rust, testing, async and simplicity guidance. Spec Kit's checked-in
  prerequisite helper passes for feature 015; its command skills remain absent
  from the local catalog, so this is not a claim to have rerun the complete suite.

- Verification session `76322` runs the existing HTTP matrix after complete
  SQLx preparation in its own disposable C-locale PostgreSQL. Results pending;
  the earlier full Nix success does not verify this new test revision.

- Full SQLx preparation passed in 1m17s, adding one waiter-count descriptor and
  changing/deleting none of the 1,459 existing descriptors. Test compilation is
  still live in the same session. Formatting and diff checks pass. GitHub confirms
  draft #212 at `8af562e28b9bfa3d538558cd628eb8bfde40ae91`.

- Reminded the user of the single pending Timesheet candidate choice while
  independent verification continues; no answer has yet been recorded.

- Session `76322` is terminal, exit 101: compilation passed in 2m24s, but the
  new two-waiter observation timed out before capacity assertions (3.97s test).
  This is not yet evidence of a production capacity failure. Added temporary
  private-fixture lock diagnostics and started `83996` in a new disposable
  cluster. Inspect actual waiter dependencies before changing the barrier;
  remove diagnostic instrumentation and its descriptor afterward.

- The first diagnostic compile (`83996`) caught an optional PostgreSQL PID
  formatted as Display; corrected the temporary diagnostic. `40680` then
  reproduced two actual organization-gate requests: one waits on a transaction
  and the other on a tuple. Counting only direct blockers missed the queued
  request. The final barrier follows the blocking chain with a bounded recursive
  query, retaining the requirement that both requests are waiting. Removed
  temporary query logging. No production authorization defect is inferred.

- Incremental preparation omitted 90 unchanged cached descriptors after the
  failed compile. `72459` now regenerates with `CARGO_INCREMENTAL=0` before
  rerunning the matrix in another disposable cluster. Verify the whole cache
  against HEAD before publication; do not commit those omissions.

- `72459` completed, exit 0: non-incremental SQLx preparation (1m07s) restores
  every previous descriptor unchanged and adds only the final transitive-waiter
  query; temporary diagnostics are absent. Test compilation took 1m56s and the
  complete registered HTTP matrix passes in 10.54s (one harness test, 1,099
  filtered). The script stopped its private PostgreSQL.

- The user confirmed Timesheet candidate option A: active managed-project
  participants without recorded time are selectable, while entry visibility
  remains scoped. Updated spec, both read/integration contracts, research and
  tasks. This is no longer a pending user decision and must not be asked again;
  delegated tracking, date/history cases and full integration remain required.

- Offline all-targets server Clippy is running in `33507`. A separate private
  fixture reuses the exact just-built test binary for template regressions and
  a second HTTP matrix run; no recompilation or shared development database.

- Both are now terminal, exit 0: `33507` passes all-targets offline Clippy with
  warnings denied in 1m26s; `5225` passes 19 template regressions (3.99s) and the
  repeated HTTP matrix (10.47s), then stops its private PostgreSQL. T178 is
  complete. Local adversarial review and runnable verification are recorded in
  `quickstart.md`; no production changes or full-feature completion are claimed.

Next: format and publish the verified
acceptance increment. Then implement the Timesheet integration following the
confirmed candidate choice and remaining documented operation predicates.
T016/T017/T020 remain open; no partially enforced policy activation or merge.

## 2026-10-04 — Corrected full Nix gate passes

- Previous goal turn: verified wait. Revalidated the unchanged published code at
  `3308926` and resumed the same session, `18654`. It is now terminal, exit 0:
  `all checks passed!` for x86_64-linux. Other architectures were explicitly
  omitted by Nix and have not been verified. Do not poll or restart this session.
- The corrected release, Clippy, browser, SQLx, core/server/integration tests,
  formatting and both NixOS deployment checks completed successfully. Exact
  derivations, outputs and counts are recorded below. T174–T177 are complete.
- This closes the browser-fixture/input and Unicode-name regressions only.
  T020 and the full permission goal remain open: operation contracts, canonical
  cross-surface integration, combined approval/expense coverage and reviewed
  preserved-data activation are not supplied by a passing regression suite.
- No real database was migrated; fixtures and deployment VMs were disposable.
  PR #212 remains OPEN/DRAFT. GitHub has not reported checks for `3308926`; local
  Nix success must not be represented as a successful GitHub Actions run.
- Revalidated both deployment outputs with `nix path-info`:
  `/nix/store/i1haiysdw61vxygjz266ca0w04nidk79-vm-test-run-horae-e2e` and
  `/nix/store/xwvz4fijkby56xk20yks7g3da78gmndy-vm-test-run-horae-e2e-oidc`.
  The evidence-only formatting check passes with zero changes after formatting
  the progress record; no code changed after the successful full gate.

Next: format and publish this evidence-only update, reusing cached Nix outputs;
then resume the integrated permission work. Resolve the already-pending Timesheet
candidate predicate before its implementation; do not assume an answer, activate
partially enforced policy or merge the draft.

## 2026-10-04 — Unicode storage regression completion

- Previous goal turn: progress. Revalidated local `4294aa3`, the seven pending
  storage/documentation/cache paths and the exact live verification session.
  No browser suite or database verification was restarted on quiet output.
- Session `38465` completed, exit 0: all 11 storage tests pass, including both new
  migration preservation/rollback tests and accented/Cyrillic name equivalence.
  Compilation took 4m30s; the tests took 1.76s. Other integration binaries were
  filtered out by `storage_tests`; this is not a full-server regression result.
- Complete SQLx preparation had already passed with one added DDL descriptor and
  1,458 unchanged descriptors. The verification script stopped its own temporary
  C-locale PostgreSQL. No development or real database was migrated.
- Rechecked the full task list: T006/T007/T009, canonical consumers, approval
  coverage, real expense integration and preserved-data activation remain open.
  No browser/MCP tool is loaded, and the pending Timesheet candidate question has
  no new answer. Passing these regressions cannot complete the full goal.
- Offline server all-targets Clippy passes with warnings denied in 1m16s (session
  `12678`, exit 0). Formatting passes again (520 files, zero changes). The scoped
  review confirms transactional index replacement, unchanged prior migrations,
  name/grant preservation, strict tenant uniqueness and no second Rust name key;
  it is a local review, not independent full-feature acceptance.
- The filesystem has about 19 GiB free with 17 GiB in this worktree's regenerable
  Cargo target; avoid parallel Nix builders and monitor free space during the
  corrected full gate. No build cache was deleted in this iteration.
- Published unsigned commits `4294aa3` (browser isolation/assets) and `3308926`
  (Unicode uniqueness and preservation). GitHub confirms #212 OPEN/DRAFT at
  `330892610dd102c1ec559441ab2cddd37cb5c1df`; no merge or activation occurred.
- A fresh format check passes and `nix flake check --keep-going --max-jobs 1 --cores 4 --print-build-logs --log-format raw` is running against clean
  `3308926` in terminal session `18654`. Evaluation succeeded. Resume this exact
  process; the old full-check session is terminal and must not be polled.
- New immutable derivations: package `3dylpw8qxrhwd37c2qrb5kc232p47can`, tests
  `xdijnvgpnacsh4208r0l2i44xczxc0a3`, browser `x5vd822zcyj18rnmm3czqh3mxipw8s0f`,
  Clippy `bvfga48y1rl4bnl1ffq3xb0a5rks48yp`, SQLx
  `psyxix2bs95nik0nic6lpx0mkp16zrv3`, e2e `2r96lz5gl9ig5p01fji8w6g5z931vlk6`,
  OIDC `jbrzkhq03r1jckmq0mz0045wlyv2fwb3` and treefmt
  `cl5rh1fgk96y70cwd44c3dky4dn9hsbx`. No corrected full-gate result yet.
- Following continuation: verified wait. Resumed `18654` repeatedly without
  restarting it; the release derivation is still active. A host process check
  confirms `rustc` using a CPU after five minutes in that compile, with the Nix
  process at eight minutes. Free space is stable around 13 GiB. No additional
  code change, new failure or successful full-gate result is claimed.
- Next continuation: verified wait; the same session has advanced from the
  release package to Nix Clippy. The corrected release output is valid at
  `/nix/store/s1r5p6z1qprakrcgd9sy9vwk3hrfazka-horae-0.1.0`; about 15 GiB is free.
  The remaining corrected full-gate results are still pending. No build restart,
  policy activation or merge occurred.
- The same run has now advanced past Nix Clippy to the corrected browser
  derivation; its Clippy output is valid at
  `/nix/store/0czsm1f49yfz79fsv8hj125036kggzlg-horae-clippy-0.1.0` and about
  18 GiB is free. The browser check remains live after the latest bounded waits.
  GitHub's current #212 head/base are `3308926`/`master`, OPEN/DRAFT,
  but `gh pr checks` reports no checks on this revision. The three latest CI
  runs returned for the branch belong to older heads, not this change. Their
  success is not current evidence; do not cancel the live local verification or
  claim green CI. The cause of missing current checks is not established.
- Latest continuation: verified wait completed the corrected Nix browser gate.
  Output `/nix/store/5j5piwghj60djpvl93lrwsmvqkr6jm2x-horae-browser-checks` is
  valid, and its log ends with all permission-recovery scenarios passing in
  Chromium 148.0.7778.96. The full 19-suite runner now passes inside Nix with
  the corrected release, source layout, login fixture and viewport sequencing.
  T175 is complete. This is browser regression evidence, not full permissions
  integration. Session `18654` remains live and has started SQLx verification;
  about 15 GiB is free. No duplicate build was started.
- Further continuation: verified wait. Session `18654` advanced from SQLx to
  `xdijnvgpnacsh4208r0l2i44xczxc0a3-horae-tests-0.1.0`. The full test and
  deployment outcomes remain pending; do not substitute the earlier 11 storage
  tests or pre-correction runs for this gate. About 14 GiB is free.
- Corrected Nix SQLx output is valid at
  `/nix/store/pfp1skk71p4i817w2izmhsdl1f6bd0gi-horae-sqlx-prepare-0.1.0`;
  its log confirms successful preparation checking and shutdown of its own
  PostgreSQL. Compilation took 1m48s; the phase finished in 1m50s.
- During test compilation free space fell to 8.6 GiB. Revalidated both realpath
  and Cargo metadata as this worktree's exact `target` and ran
  `cargo clean -p horae --target-dir /home/aldo/Dev/aldoborrero/horae/.worktrees/scoped-permissions/target`.
  It removed 3,368 regenerable package artifacts (12.6 GiB), preserving dependency
  artifacts, sources, scratch evidence and every database. About 20 GiB is free;
  the immutable Nix test build continues independently. The user was informed.
- Corrected full Nix tests pass: output
  `/nix/store/5q9irw4j4arq8ilsp9s5slcrmv3ln74w-horae-tests-0.1.0` is valid.
  The log reports 180/180 core tests; 1,089 server-binary tests passed with the
  existing 11 ignored, zero failed (102.64s); all 221 tests across the twelve
  additional integration binaries passed. Both migration-preservation tests and
  the non-ASCII uniqueness regression pass. Compilation took 3m43s; the whole
  phase took 5m43s. Do not double-count the child-process rerun summaries.
- Session `18654` is now running the deployment VM checks. Free space returned
  to about 29 GiB after Nix cleaned its test build scratch. Full-command exit and
  the new deployment outputs remain to be confirmed.

Outcome: session `18654` completed successfully; see the final gate record above.
T177 is now complete and T020 remains open. Resume the integrated contracts;
no merge or runtime policy activation is authorized by this correction.

## 2026-10-04 — Full-gate failures and corrective verification

- Previous goal turn: no implementation progress (the permission-benefits
  explanation). Revalidated published `228e151` and resumed the exact recorded
  process instead of restarting it. Full Nix session `57201` is terminal, exit 1.
- The original release, Clippy and SQLx outputs are valid. Both NixOS deployment
  outputs are also valid: `kyay78h61w8krsiimgya880vmzvp0zh1-vm-test-run-horae-e2e`
  and `zb73z0pz991i6ayh49k869s77ac26lpc-vm-test-run-horae-e2e-oidc`. These prove
  regression coverage at `228e151`, not canonical activation or the full feature.
- Server-binary tests in Nix report 1,086 passed, one failed, 11 ignored. The
  failure is `template_names_are_case_insensitive_and_tenant_local`: the C-locale
  cluster admits both `Ágil` and `ágil`. A separate disposable PostgreSQL probe
  confirms default `lower('Ágil')` remains `Ágil`, while ICU root lower produces
  `ágil` without equating `Agil`. The probe stopped its own cluster afterward.
- Corrected browser derivation `mxn163sp54gf1a2k6ic6ighap7g9y745` is also terminal,
  exit 1. Preserving crate-relative asset paths allowed the first 18 suites to
  pass, then the permission editor could not open. The project-permission suite
  retained a second active legacy administrator; DEV_LOGIN can select that user,
  who has no canonical state in the subsequent fixture. A new explicit assertion
  reproduces the two-candidate failure. Keep the other draft owner as a Manager
  (still permitted to create drafts), and assert a single login candidate.
- The corrected paired run exposed an intermittent focus assertion after viewport
  restoration; the shared menu intentionally closes on resize. Isolated and
  paired instrumented runs both subsequently passed all recovery assertions.
  Synchronize the capture helper with the rendering frame after each viewport
  change, including artifact-disabled runs. No production UI/JS/CSS or permission
  assertion changed; the temporary event instrumentation was removed.
- Full browser runner session `13276` completed successfully (exit 0): all 19
  suites, including permission recovery, against the valid `228e151` release
  and corrected test scripts, using new disposable data. The fixture/source-path
  repair is committed unsigned as `4294aa3` (not yet pushed). This is not yet a
  passing corrected Nix derivation and does not exercise migration 0047.
- Migration 0047 pins the profile-name index to deterministic ICU root comparison;
  prior migrations are unchanged. New tests cover preservation and transactional
  collision failure, plus accented/Cyrillic case pairs, accent distinction and
  distinct Unicode normalization forms. The storage contract no longer permits
  cluster-locale drift; the deployment README records the ICU requirement.
- Verification session `38465` applied migrations only in its new C-locale
  disposable cluster. Complete SQLx preparation passed in 2m39s: 1,459 descriptors,
  one new test DDL descriptor and all 1,458 existing descriptors unchanged. It is
  now compiling storage regression tests. Formatting passes (520 files, zero
  changes). No real database, profile or policy mode was modified. Skills
  informed Nix source layout, minimal fixture changes, Rust tests and deployment
  documentation. Impeccable context loading was unavailable; DESIGN.md and the
  existing interaction code were read directly. No visual redesign was needed.

Next: collect session `38465`, resolve any failures, verify the full
corrected Nix gates, review and publish scoped unsigned commits to draft #212.
T175/T177/T020 remain open. The Timesheet choice, integrated canonical enforcement,
combined approvals/expenses and preserved-data activation are still outstanding;
these regression repairs do not shrink the full goal.

## 2026-10-04 — Full Nix verification in progress

- Previous goal turn: progress. Published `228e151` with new source evidence and
  the integrated Timesheet map. The zero-entry participant decision is still
  unanswered; no default was accepted or implemented.
- Revalidated a clean worktree and evaluated the actual Blueprint checks. The
  dry run requires 269 derivations and 162 fetched paths (305.9 MiB download,
  922.6 MiB unpacked), excluding additional compilation scratch space.
- The filesystem initially had about 19 GiB free. Confirmed Cargo's target path
  is this worktree's own `target`, then used `cargo clean --target-dir` on that
  exact path: 32,436 regenerable build files, 21.1 GiB reported removed. Free
  space rose to about 37 GiB. No source, scratch evidence, database, other
  worktree or global Nix store was deleted.
- Started `nix flake check --keep-going --max-jobs 1 --cores 4 --print-build-logs --log-format raw` against `228e151`. The live terminal
  session is `57201`; resume that process, do not start a duplicate. Evaluation
  succeeded and the package build started. No final gate result is claimed.
- The package derivation is `k8413hw3ni8nzqbyjb0r4qvxp4s6niqs-horae-0.1.0`;
  tests `8pkxh348mbh6swsw4nx2613kq2ariw2g-horae-tests-0.1.0`; SQLx
  `6sgvmkyqqv8xr02v73w1mmh4ksj0l7lf-horae-sqlx-prepare-0.1.0`;
  Clippy `ii8vn5nl49ykh6mypbqld82prv4mfvhr-horae-clippy-0.1.0`; browser
  `64wnbh15kpm2fnzjwndma093rn28v71p-horae-browser-checks`. These immutable
  derivations preserve the checked revision even while this log is updated.
- The checks create their own databases and deployment VMs. A passing legacy
  browser/deployment suite would be regression evidence, not proof of missing
  canonical integration, migration, combined approvals or full-feature parity.
- The isolated shell hides host processes and `/dev/kvm`; an elevated read-only
  check confirms KVM is present and the build has active Cargo/rustc processes.
  The initial isolated KVM result is not a deployment blocker. About 32 GiB
  remains free during package compilation; no terminal build result yet.
- Continuation classified the preceding turn as a verified wait and resumed
  session `57201`. The release package is now valid at
  `/nix/store/d9hrpqn3g2hmadvyysn43sfcd67nw5xw-horae-0.1.0`.
- The original browser derivation failed before its first suite: the source
  interpolation copied only `tests/browser`, while `editor-navigation.cjs` and
  `permission-recovery-storage.cjs` load application JavaScript two directories
  above. The stored log proves `ENOENT /nix/assets/js/project-edit-navigation.js`.
  Corrected `nix/checks/browser.nix` to retain the crate-relative layout; no
  browser assertion, runtime code or permission rule was weakened. Remaining
  immutable-revision checks continue under `--keep-going`.
- The corrected browser check is running in session `87425`, derivation
  `mxn163sp54gf1a2k6ic6ighap7g9y745-horae-browser-checks`; its build plan reuses
  the existing release package and builds only this check. Do not restart it.
- Verified valid Nix outputs for Clippy
  `/nix/store/jv5jf18h691xpg3sr3syh6sgw8hzp8zi-horae-clippy-0.1.0` and SQLx
  `/nix/store/95fr8jzdwwpdhm41352k6bq4lf3c1n3m-horae-sqlx-prepare-0.1.0`.
  The full-run session is now building the tests derivation. Browser and
  deployment results remain pending; free space is about 32 GiB.

Next: collect the live check's results, monitor free disk space, investigate any
actual failures and record exact acceptance limits. Continue the integrated
Timesheet package when its pending product predicate is resolved; keep T020 open.

## 2026-10-04 — Timesheet archive and Calendar contract findings

- Previous goal turn: progress. Revalidated clean `5ec183a`, published to draft
  #212 with the full recorded server-binary regression and offline build gates.
- Investigated the remaining Timesheet integration instead of adding another
  unrelated compatibility repair. Fresh sources establish archived-person
  exclusion from Timesheets while retaining report history, delegated timer-stop
  write authority, and a Calendar-specific Administrator locked correction
  exception. Updated the read and company-lock contracts accordingly.
- Added `contracts/timesheet-integration.md`: selected context must reach reads,
  labels, all three views, edits, timer actions and submission without changing
  the shell timer's owner. It distinguishes source-backed facts, outstanding
  candidate decisions and lifecycle/coverage dependencies.
- Asked once whether active participants without time should be discoverable
  under managed-project time access; the response remains pending. No browser
  tool is loaded, no unchanged connection retry was made, and no new live
  observation, runtime implementation, activation or migration is claimed.

Next: incorporate the candidate decision, close selected-person tracking/write
predicates and implement the integrated Timesheet transaction/UI package. Keep
the general historical reader unchanged. Complete approval/expense and policy
cutover gates remain required, not waived by these documentation findings.

## 2026-10-04 — Activity-fence regression completion

- The preceding benefits-only response was no implementation progress. Resumed
  the existing full-suite process, rather than restarting it, and revalidated
  the worktree at `c80233b` with the activity-fence changes still uncommitted.
- The complete server-binary suite passes: 1,087 passed, zero failed, 11 ignored
  in 277.79s after 2m14s compilation. All 43 time-entry tests pass, including
  the five new activity tests and both repaired foreign-user fixtures.
- Rechecked the shared prefix, its six production callers, submission lock
  compatibility and unchanged lower-level service-import barrier. The local
  review found no critical/high issue in this bounded change; this is not an
  independent review or approval/canonical-policy completion.
- Complete SQLx preparation passes in 56.48s: five new descriptors, all 1,453
  previous descriptors unchanged. The package-local clean removed 1.7 GiB of
  regenerable artifacts. The owned disposable PostgreSQL is stopped.
- Fresh offline all-targets server Clippy passes in 1m07s and WASM Clippy in
  13.32s, both with warnings denied. The format check corrected Markdown list
  spacing; rerunning the final format gate before the unsigned commit.
- T171–T173 close this activity prerequisite only. No UI/CSS, service authority,
  canonical grants, policy activation, migration or real data changed. Browser
  and full `nix flake check` acceptance remain open for the complete feature.

Next: publish the verified change unsigned to draft #212 and resume the
Timesheet/delegated-write integration. The full permission goal remains active.

## 2026-10-04 — Interactive writer activity prerequisite

- Previous goal turn: progress. Revalidated clean published `c80233b`; the shared
  invoice-identity repair is on draft #212 with its recorded checks.

- Reopened Harvest's permission, other-person timesheet and time-editing guides.
  They confirm separate read/write scope and teammate navigation. Unlocked-entry
  editing permits project/task changes but not reassigning the person; the
  current modal/update transport cannot express those edits. Existing company
  lock and approval contracts still own independent locks and submitted editing.
  No legacy Administrator prose is treated as a new custom-grant predicate.

- The existing Chrome MCP client initialized, but `browser_tabs` timed out while
  connecting. No page observation, mutation, screenshot or successful browser
  test is claimed. Requested client disconnection without closing browser tabs;
  continuing independent work instead of repeatedly reconnecting.

- Tracing the seven interactive time operations found that all six transaction
  helpers join the submission barrier without rechecking current account
  activity after session lookup. Added a production-helper regression before
  implementation; its isolated-database RED run is in progress.

- `contracts/time-writer-activity.md` and T171–T173 define the bounded activity
  prerequisite. Rust, async, testing and simplicity guidance require one shared
  transaction prefix and deterministic database waits, not per-button guards.
  This does not waive Timesheet, canonical policy or approval integration gates.

- RED confirmed a real post-deactivation edit: the production helper returned
  success and persisted 90 minutes instead of 60 (0.41s after 1m47s compilation).
  Implemented `begin_active_time_write` for all six production helpers, reusing
  bounded transaction settings, organization SHARE and the advisory barrier.
  The active actor is reloaded and SHARE-locked before entry/task locks. The old
  barrier-only begin helper is now test-only; import service use is unchanged.

- Added all seven operations across the three legacy roles and missing/inactive
  identities with whole-record preservation, direct and organization-gated
  deactivation in both orders, rollback and single-connection cancellation/default
  tests. The focused run passed 41/43 cases, including all new activity tests;
  two older foreign-actor checks instead supplied nonexistent users and received
  the new earlier forbidden result. Replaced those UUIDs with actual foreign
  users, retaining their original conflict/not-found assertions. Added inactive
  no-op coverage and started the complete server-binary regression suite.

- Review traced organization-before-actor-before-submission ordering, compatible
  FK locks, rollback on denial/cancellation and all six production call sites.
  No service import calls the new active-person prefix. No canonical grant,
  role promotion, new dependency, CSS or approval-state rule is introduced.

Next: finish the full regression, complete SQLx preparation and offline native/
WASM checks, then publish the verified fence unsigned to draft #212. Resume the
remaining Timesheet selection/delegated-write and approval predicates afterward;
no real-data migration, policy activation, Harvest write or merge.

## 2026-10-04 — Timesheet integration trace and shared payload repair

- The preceding goal turn was progress: published `4ac30fa` with the canonical
  time reader and passing database/HTTP/build checks. Revalidated its clean
  worktree and traced the existing Timesheet before changing any UI.

- The screen assumes own-user context throughout name lookup, complete-week
  totals, grid, dialog, calendar drag/reorder, timer and submission callbacks.
  Simply replacing its list resource would not deliver teammate access safely.
  Recorded those integration dependencies and current Harvest sources in
  `contracts/time-entry-reads.md`; no unverified teammate candidate rule is
  inferred from legacy-role documentation or the own-time design prototype.

- Found an independent FR-008 exposure during that trace: shared `TimeEntry`
  serialization includes `invoice_id`, unused by every page/component. SQLx and
  the separate invoice/compatibility projections must retain billing facts.
  Added T168–T170 and regression tests before changing the shared wire boundary.

- Rust/testing/async and simplicity guidance favor the existing model's single
  serialization boundary, not parallel wrappers for every time endpoint. The
  registered-session red test is running against the owned disposable database.
  Spec Kit prerequisites pass; unavailable skill procedures are not claimed run.

- Impeccable's context launcher could not use its unwritable engine cache; used
  the prescribed direct DESIGN.md fallback without installing anything. The
  design review does not authorize a UI change or claim visual verification.

- The registered HTTP red test fails on the actual populated `invoice_id` in a
  member's own-entry response (1.07s after 2m13s compilation). Added `serde(skip)`
  to the internal relation, leaving SQLx loading and every other field intact.
  Expanded the real-route fixture to all three legacy roles with a stored-link
  preservation assertion. Payload, session, time, invoice and Timesheet checks
  are running; no passing result is inferred from compilation alone.

- The intervening benefits-only response was no implementation progress.
  Revalidated the six-file diff at `4ac30fa` and resumed the existing test
  process instead of restarting it. Both model tests pass, the registered HTTP
  matrix passes (12.64s), all 38 time-entry regressions pass (18.46s), all 35
  invoice tests pass (8.43s), and all 25 Timesheet unit tests pass. The HTTP
  fixture proves omission and stored-link preservation for Member, Manager
  and Admin; it does not exercise invoice lifecycle, which has its own suite.

- Bounded adversarial self-review traced every shared-model consumer and the
  explicit plugin and Harvest projections. No production UI reads the omitted
  field; SQLx still loads it, input JSON cannot restore it, and no write guard,
  business-state transition or financial projection is changed. No critical or
  high finding remains in this repair; this is not an independent or full-feature
  review.

- Full SQLx preparation passes in 57.08s, retaining all 1,453 existing cache
  descriptors unchanged. Fresh offline all-targets server Clippy passes in
  1m07s and WASM Clippy in 13.75s, both with warnings denied. The package-local
  clean removed only 1.7 GiB of regenerable artifacts. Stopped the disposable
  PostgreSQL after its final required use. Formatting corrected one Markdown
  indentation and is checked again before publication. T168–T170 are complete
  for this bounded response repair; no browser or full flake run is claimed.

Next: publish the verified repair unsigned to draft #212, then resume the
Timesheet person-context/read-write integration
contract and remaining operation predicates. Full Timesheet integration and the
complete permission goal remain open; no real data, policy activation or
migration is authorized here.

## 2026-10-04 — Scoped time-entry read implementation

- The previous benefits-only response was no progress. Revalidated the clean
  worktree at published `3bb62ac` and resumed independent OP03 work; no unanswered
  creation, assignment or approval decision is inferred.

- Rechecked Harvest's current permission reference, confirmed own/managed/all
  reads, traced the existing Timesheet's complete-week totals and contextual
  labels, and recorded the bounded contract in `contracts/time-entry-reads.md`.
  Spec Kit prerequisites pass; its absent skills are not claimed executed.

- T165's initial compilation reproduced the missing reader/model imports.
  Implemented the explicit projection, transactional reader and registered
  session wrapper. Fixed SQLx's date/time feature ambiguity with explicit chrono
  overrides, following existing queries. First five PostgreSQL tests pass in
  3.90s after 1m56s compilation.

- Added crossed person/project scopes, ordinary-membership denial, read-first
  revocation and cancellation/default-isolation cases, plus the real HTTP exact
  payload fixture with populated rates and linked invoice. Their verification is
  in progress. One new fixture incorrectly assumed assignments had `org_id`;
  corrected it to the existing parent-scoped schema. No schema change is needed.

- Rust/testing/async and simplicity guidance reuse existing locks, stored grants,
  session harness and dependencies. No UI/CSS change, data migration, policy
  activation or Harvest mutation. All complete-feature acceptance gates remain.

- All eight expanded database tests pass in 5.17s; the registered HTTP matrix
  passes in 12.53s, including the exact populated-sensitive-data projection.
  T165/T166 are complete. WASM lint identified the deliberately not-yet-consumed
  query DTO; documented that boundary with the same feature-local `expect` used
  by existing staged readers, not a global suppression or fake consumer.

- Local adversarial review checks parent tenancy in each join, scope before
  pagination, no legacy-role/administrator fallback, grant and relationship
  revocation under the existing writer lock protocol, direct actor deactivation,
  contextual-only labels, and no monetary or invoice fields. No critical/high
  finding remains in this local read boundary; no independent review is claimed.

- WASM Clippy passes with warnings denied in 14.98s. Full SQLx preparation after
  cleaning 1.7 GiB of regenerable package artifacts completes in 1m01s: 16 new
  descriptors, all 1,437 prior descriptors unchanged, none deleted. The owned
  disposable database is stopped. Fresh offline all-targets server lint is
  running; no browser or full-flake result is claimed.

- Final offline all-targets server Clippy passes in 1m12s with warnings denied.
  T167 closes for this reader only. Formatting and diff checks precede the
  unsigned publication to existing open/draft #212; no merge is authorized.

Next: publish this verified reader, then continue canonical Timesheet/shell
integration and the remaining operation contracts without inferring answers to
the outstanding product questions. This does not complete time editing,
approval, person-management writes, migration or the full active goal.

## 2026-10-04 — Minimal session identity response

- Previous goal turn was progress: published `4c00660` with the project-form
  inventory and 16 verified editor baselines. Revalidated its clean worktree.
  The unanswered creation/designation question is not a new authorization.
- Reviewed independent work against current OP01. `get_me` still returns the
  complete database User, including cost/billable rates and OIDC subject. Traced
  every production consumer: account menu/display gates and recovery need only
  ID, organization ID, name, email and legacy role. None needs financial/provider
  data, activity or creation time. This permits a real payload repair without
  inventing pending financial-form or approval predicates.
- Added the closed response boundary to `contracts/own-permissions.md` and tasks
  T162–T164. Added exact-response and session tests to the existing registered
  HTTP harness; removed the old directory test's assertion of the excess own-user
  payload. Started the red check on the owned disposable database on 55416.
- Rust/testing/async and simplicity skills call for a concrete five-field DTO,
  existing session lookup and real-route tests; no policy framework or UI/CSS
  change. Spec Kit prerequisites pass; its absent skills are not claimed executed.
- The initial `--exact` short-name filter compiled but selected zero tests; the
  corrected command reproduces the private fields in the real JSON response.
  Implemented `CurrentUser` and converted the existing page helpers/fixture
  responses, retaining internal `User` authentication and unchanged legacy gates.
- The registered HTTP matrix now passes in 12.58s, including all three roles,
  forged foreign selectors, same-cookie demotion/deactivation, missing identity
  and both active/inactive logout. All 81 selected consumer tests pass. Their
  navigation fixture exposed unused full-User warnings after adopting the small
  DTO; exported its production-model module consistently with other fixtures,
  without a lint suppression. WASM Clippy passes with warnings denied.
- SQLx check passes with a potentially-unused-cache warning. Cleaned only the
  local package's 4.4 GiB of regenerable artifacts and ran full preparation:
  1,437 descriptors retained, none added, modified or deleted. The new fixture
  queries already existed; production SQL is unchanged. Offline all-targets
  server Clippy passed before that regeneration; a fresh final pass is running.
  The owned disposable PostgreSQL is stopped; no real service was touched.
- The final offline all-targets server Clippy passes in 1m06s with warnings denied.
  Formatting CI passes with 512 files and zero changes; diff checks pass. Local
  adversarial review verifies exact field selection, unchanged session/activity
  checks, no input-controlled identity and no financial/provider placeholders.
  T162–T164 close for this payload repair, not canonical policy or full acceptance.

Delivery: publish the verified increment to draft #212 unsigned, without merge.
Then continue the outstanding canonical form/shell integration and resolve the
already-recorded product questions without inferring their answers.
This does not deliver the still-open canonical form, shell, approval, migration
or full-feature gates. Keep the complete goal active.

## 2026-10-04 — Project form preservation integration review

- The preceding benefits-only answer was no progress. Revalidated `6b5dbae` and
  the retained three-document inventory rather than restarting it. Removed a
  duplicate task heading and preserved the existing branch/worktree and draft PR.
- Added source-backed budget and note distinctions to the field/effect inventory.
  Existing Administrator-only private notes remain protected independently of
  cost grants; Harvest's configurable shared-note preference is not implemented
  by that existing field. Initial creation/designation authority remains the
  already-asked, unanswered product question; no default was inferred.
- Mapped existing editor regressions to the integration contract so that private
  cost preservation, indirect removal, redaction and nullable budgets are reused.
  Started the full existing editor test group against the owned disposable
  PostgreSQL on 55416. No new production code or test is claimed in this iteration.
- Simplicity and Rust testing guidance favor reusing these real transaction tests
  over a new unconsumed protected-field abstraction. Spec Kit prerequisites pass;
  its skill suite is unavailable. No real data, UI/CSS, schema or policy is changed.
- The full existing editor group passes: 16 tests, zero failures, in 2.13s after
  2m07s compilation. Command: `nix develop --command env DATABASE_URL=postgres://horae@127.0.0.1:55416/horae_storage SQLX_OFFLINE=false CARGO_INCREMENTAL=0 cargo test -p horae --features server --bin horae server_fns::project_creation::editing::tests::`. These are legacy baselines,
  not the still-unimplemented canonical form acceptance. No SQL query changed,
  so no SQLx cache regeneration is required. Formatting normalized one new table;
  the following CI formatting check passes with 511 files and zero changes.
- GitHub confirms #212 open/draft on `feat/scoped-permissions` at `6b5dbae`
  before publication. The local contract review keeps note preferences, monetary
  budgets and initial creation grants distinct rather than claiming those open
  rules have been resolved. No new independent review or browser test is claimed.

Delivery: publish this inventory with an unsigned commit to draft #212, no merge.
Next: close the remaining non-rate field/effect rules for T159 and integrate the
actual projected form and explicit unchanged/reset/zero intent together (T160),
using the existing rate evaluator. Creation-only questions must not block safe
existing-project work. T159–T161 and full operation/approval/person-management,
preserved-data activation, browser and Nix acceptance remain open. The full goal
is neither complete nor blocked.

## 2026-10-04 — Context-bound project people reader

- The preceding explanation of permission benefits was no progress. Resumed the
  retained test process; it ended with missing modules and missing offline query
  descriptions. Repeated against the owned disposable database on 55416 and
  confirmed only the two missing implementation imports remained (exit 101).

- Implemented the identity-only model, transactional search/selected-ID reader
  and authenticated `project_people` server function. Creation and exact-project
  editing use different grants; no directory grant, financial projection, legacy
  role fallback or permission promotion is introduced. Existing UI/editor guards
  and real data remain unchanged.

- The first five database tests pass (3.52s after 2m04s compilation), including
  context isolation, minimal fields, pagination, input limits, invalid authority
  and designation revocation after a lock wait. Added further tests for both
  revocation orders, cancellation/pool defaults, exact pages/unconfigured targets
  and registered-session HTTP boundaries; those are not yet accepted as passing.

- Rust/testing/async and simplicity guidance led to reusing the existing cursor,
  transaction settings and HTTP harness without a new dependency or abstraction.

- The expanded run initially reports two invalid-fixture revocations (and the
  HTTP equivalent): empty stored grants violate the member floor. Replaced those
  fixtures with normalized Member grants; malformed-state coverage is retained.
  The complete server permission suite now passes all 141 tests in 82.89s,
  including all nine new reader tests. The registered HTTP matrix passes in
  14.21s. WASM Clippy passes with warnings denied after three localized non-server
  dead-code expectations for DTO roots awaiting the real UI consumer.

- Initial SQLx preparation completes but omits 91 unchanged integration-test
  descriptions. Confirmed a missing descriptor's query still exists in
  `tests/integration.rs`; cleaned 1.7 GiB of regenerable package build artifacts
  and restarted complete preparation. Source and database records are preserved.

- Complete preparation passes in 1m16s with 1,437 descriptors: ten additions,
  no deletions or changes to existing descriptions. Each addition maps to the
  project reader or its fixtures. Draft #212 remains open on the same branch.

- Offline all-targets server Clippy passes in 1m14s with warnings denied. The
  formatting CI check passes with 510 files and zero changes; whitespace checks
  pass. The owned disposable PostgreSQL is stopped. No real service is stopped.

- Review checks current grants, tenant-qualified project designation, minimal
  projection, lock order and sanitized errors; no new material issue is identified
  in this bounded self-review. T155–T157 close for the reader only.

Delivery: unsigned commit to the existing draft #212, without merge. Next:
resolve the remaining project-editor financial projections and report-filter
identity rules, then integrate canonical consumers/shell under the reviewed
full-policy gates. The complete operation matrix, actual expense/time approvals,
person-management writes, preserved-data activation, browser and full Nix gates
remain required. No full-feature completion or MVP readiness is claimed.

## 2026-10-04 — Project and report identity contracts

- Published approval labels as `981d0e3` on draft #212, then revalidated the clean
  worktree and reused it. The latest explanatory reply made no code progress.
- Traced Reports, ProjectDetail and both project-editor catalog paths, including
  selected-ID resolution. Read current Harvest documentation and the Reports/
  New Project design handoffs. Spec Kit prerequisites pass; its skills remain
  absent, so no skill execution is claimed.
- Fresh Chrome connection succeeds through the existing stdio client. Opened a
  dedicated Harvest tab, observed an empty week and an available teammate filter,
  then inspected an existing project editor without toggling or saving anything.
  Cancel returned to Projects. The single already-assigned owner cannot prove
  restricted candidate behavior. Raw evidence remains in ignored scratch; only
  redacted findings are committed. No account or business-data writes occurred.
- Closed the project identity-only search/resolution contract with explicit
  create/exact-project edit contexts, active same-organization candidates, minimal
  fields, bounded requests, revocation and separate finance/designation writes.
  Added T154–T157 and requirement-to-test cases. Report-filter scope remains
  explicitly unresolved; deriving choices from result rows is contradicted by
  the browser observation.

Next: T155 red tests for the context-bound project people reader, followed by
T156 implementation and T157 validation. Do not add another general directory
lookup, infer rate access from selection or activate policy. Full operation,
approval/expense, person-management, migration and cross-surface gates stay open.

## 2026-10-04 — Approval labels without directory access

- The preceding status reply was no progress. Resumed the retained UI test
  process, which completed with all three tests failing on the unwanted directory
  call. The earlier registered HTTP test reproduced the missing `user_name` field.

- Added the submitter name to the approval query with a same-organization join
  and removed the page's separate directory resource and UUID fallback. Archived
  submitters remain nameable; malformed foreign references are excluded. Existing
  authorization, minute aggregation, controls and CSS remain unchanged.

- The first post-fix UI run passed action and resource-state tests; its escaped
  name assertion expected named HTML entities, while Dioxus emits numeric entities.
  Corrected that test expectation, not the production escaping.

- The corrected UI suite passes all three tests. The registered HTTP matrix
  passes in 12.62s and the 14 existing approval regressions pass in 3.89s.
  Cached SQLx preparation omits unchanged integration-test queries (92 deletions,
  only one intended). Cleaned 18.3 GiB of regenerable package artifacts and rerun
  complete preparation; no source, database or real records were removed.

- Full preparation passes in 1m14s with 1,427 descriptors: five additions and
  only the replaced approval-list query removed. Offline all-targets server
  Clippy passes in 1m12s. The following status-only response made no additional
  progress; resumed that retained lint process rather than restarting it.

- Offline WASM Clippy passes in 15.07s with warnings denied. Formatting and
  whitespace checks pass. Focused adversarial self-review found no additional
  issue in this projection; this is not an independent whole-feature review.
  T151–T153 close, with general requirements still 12/16. Rust/testing and
  Impeccable hardening guidance kept names escaped and existing components/CSS
  unchanged. Spec Kit prerequisites pass; its absent skills were not executed.

Delivery: unsigned commit on existing draft #212, without merge. Next: close
the remaining report-filter and project-assignment identity contracts, then
integrate the canonical directory/shell under the reviewed full-policy gates.
Flexible approval lifecycle, actual expenses, person-management writes and
preserved-data activation remain required. No policy activation, real data,
fresh browser or full-feature completion is claimed.

## 2026-10-04 — Canonical scoped-directory reads

- The preceding MVP response was status-only. Revalidated the existing worktree
  at `7f14e4b` and continued its uncommitted directory contract/tests rather than
  restarting the feature. Spec Kit prerequisites pass; its skills remain absent.

- Implemented `users::list_people` with session-derived requester identity,
  strict policy/state loading, current people grants and direct person scope.
  The query selects only ID/name/email/activity, applies scope before pagination
  and never falls back to legacy roles. Every page reauthorizes under the bounded
  organization/actor read locks. Errors exclude internal storage diagnostics.

- Initial tests fail on missing reader/model imports. Corrected one test's
  borrowed profile comparison during implementation. All seven directory tests
  then pass in 4.35s after compilation: six default profiles, custom managed
  scope, foreign/inactive/missing/invalid authority, hidden rows, duplicate and
  mixed names, exact/continued pages, activity filters, deleted/foreign cursors,
  relationship/grant revocation, direct deactivation and cancellation/pool reuse.

- Registered-session HTTP matrix passes in 11.97s, including exact minimal
  payload with non-null sensitive database fields, forged authority ignored,
  archived/active filters, invalid cursor, same-session revocation and sanitized
  invalid-state errors. Existing legacy directory checks still pass.

- WASM lint initially identifies the four not-yet-consumed directory DTOs.
  Added two individually scoped, non-server-only `expect(dead_code)` annotations
  on the root types, explaining the pending UI cutover; these must be removed
  with that consumer. Removed redundant nested-type expectations after lint
  reported them as unfulfilled.
  Full SQLx preparation passes in 1m02s: 1,423 descriptors, 14 additions and no
  removals. Offline all-targets server Clippy passes in 1m13s; final WASM Clippy
  passes in 13.38s, both with warnings denied. The 141 selected permission
  regressions pass in 61.24s after an offline-cache compilation.

- Reopened current Harvest permission and Users API documentation. The new
  catalog supports separate people grants; the API's legacy-role descriptions
  do not prove new-model inactive/email enforcement. The contract labels that
  inference and retains reference acceptance before activation. No browser or
  real-account mutation occurred.

- Focused adversarial source review checks field projection, scope-before-limit,
  session-derived identity, gate ordering against existing writers, rollback,
  pagination and sanitized errors. No new material local finding; this is not
  independent review or whole-feature acceptance. No schema, dependency or CSS
  change. T148–T150 close for the reader only; general readiness remains 12/16.

- The owned disposable PostgreSQL is stopped after verification. Formatting
  corrected one Markdown list boundary; final formatting is checked before
  publication. No real records were changed.

Delivery stays on existing draft #212 without merge. Next: close the per-consumer
workflow identity rules (report/approval names and project assignment choices)
before directory/shell integration. Full OP19 and reviewed activation remain
open, as do approvals, actual expenses and person-management writes.

## 2026-10-04 — Minimal user-directory response

- The preceding MVP response was status-only (no implementation progress).
  Revalidated clean synchronized `b735b3a` and OPEN/DRAFT #212, reusing the branch.
  Spec Kit prerequisites pass; the suite's skills are absent locally and were
  not executed. No full requirement or migration gate is waived.
- Traced all four `list_users` consumers and reread current official Harvest
  permissions/team-overview documentation. Recorded OP19's closed payload repair
  and remaining cross-consumer scope/field predicates in
  `contracts/people-directory.md`; no authenticated Harvest/browser claim.
- A real registered-route test reproduced ten response fields instead of the
  five consumed fields. Replaced the full database `User` response and Member
  rate scrub with `UserListItem` and a minimal SQL projection. Existing session,
  legacy-role, tenant/activity filters, mutations and own/API responses remain
  unchanged. Updated the navigation test double to the real response type.
- Final offline-compiled HTTP matrix passes (12.81s), including role/filter
  combinations, inactive/anonymous denial, tenant isolation, duplicate names,
  exact field values, own-data preservation and same-cookie demotion. All 89
  selected navigation/permission tests pass. No CSS, schema or dependencies change.
- SQLx initially omitted cached integration queries. Cleaned only 4.2 GiB of
  rebuildable Horae artifacts; full preparation then passes in 53.97s with 1,409
  descriptors. Only the replaced list query is removed; two descriptors are
  added. The owned disposable PostgreSQL is stopped; real data is untouched.
- Focused adversarial source review checks every caller, SQL/DTO projection,
  tenant/filter provenance and unchanged authorization. No new material local
  finding; full directory scopes, email/role visibility and in-flight read
  revocation remain open, not silently accepted. No independent review or new
  browser/full-flake result is claimed. Offline server/WASM Clippy and formatting
  pass. T145–T147 close; the general readiness checklist remains 12/16.

Next: resolve T006/OP19's canonical per-consumer field and scope predicates:
ordinary directory versus administrative role fields, report/approval identity
reads, project choices and inactive visibility. Then integrate the reviewed
directory/shell boundary under the full cutover gates. Scoped approvals, real
expenses, person-management writes and preserved-data activation remain required.
Delivery stays draft #212 without merge; no MVP/full-feature readiness is claimed.

## 2026-10-04 — Permission-editor subject picker

- Previous user-facing MVP answer was status-only, not implementation progress.
  Revalidated clean synchronized `8d49421` and OPEN/DRAFT #212; reused the worktree.
  Spec Kit prerequisites pass; its unavailable skills were not executed.
- Refined the existing editor contract and T142–T144, then reproduced four
  missing-picker handler failures. Added paged Change person using shared Menu
  and utilities, independent authorized reads, requester-bound selection and
  the existing dirty/pending/recovery guards. No directory fallback or cutover.
- The regression suite caught names remaining after revoked form access; moved
  the picker inside the same availability boundary. Corrected component keys
  and test event identifiers. Final 66 selected tests pass (50 editor, 11 own
  permissions, five admin shell), including empty/error/retry coverage.
- Full server/WASM bundle passes (78.82s). The extended real-browser recovery
  suite passes on disposable PostgreSQL and Chromium 148 / Playwright 1.60;
  keyboard focus/Escape, cancelled/confirmed dirty switch, clean switch back and
  no selection mutations precede the existing real-command recovery scenarios.
  Inspected all six desktop-dark/mobile-light editor/menu/recovery captures.
- Offline all-targets server and WASM Clippy pass (1m06s / 13.17s). No SQL, schema,
  dependencies, CSS or shared components changed. Removed the now-consumed DTO's
  temporary web lint expectation. Owned browser services stop on runner exit.
- Impeccable context and detector are unavailable because the engine is absent;
  no installation or system changes. Source/capture review used existing design
  context. Fresh independent finish review returns `ship` for this increment,
  with no material findings. Its six-capture verdict does not cover rendered
  pagination/duplicate/inactive/error states or full permissions acceptance.
- Independent documentation review confirms no new durable system rule or
  feature-contract correction. Existing stale DESIGN.md paths/tooling format
  remain untouched. T142–T144 are complete; T018 and the full gates remain open.
  Final formatting/whitespace checks pass. Delivery stays draft #212, no merge.

Next: continue T006/OP19's general-directory projection and scope contract before
canonical shell integration, without using this Administrator-only picker as a
substitute for ordinary people access. Approval coverage, real
expense dependency, person-management writes and preserved-data cutover remain
open. General requirements remain 12/16; no full-feature/MVP readiness is claimed.

## 2026-10-04 — Permission-editor subject discovery

- Previous turn made progress: published approval-boundary contract `eb56af3`.
  Revalidated its clean synchronized worktree and continued independent editor
  integration. Spec Kit prerequisites pass; its skills remain absent locally.
- Refined the closed current-Administrator discovery contract as T139–T141.
  Reused the editor transaction and sanitized session/error boundary, adding
  only ID/name/activity pages and requester identity. No legacy-role fallback,
  general directory replacement, grants, schema, UI or activation change.
- Tests first failed for the missing reader. The initial implementation passed
  five cases; the deleted-cursor fixture incorrectly assumed a cascading user
  deletion. Removed that disposable subject's permission row first. The final
  134 selected permission tests pass (56.28s), including six new cases; the
  registered real-session HTTP matrix also passes (11.67s).
- Coverage includes inactive/missing-state people, duplicate names, exact/full
  pages, deleted/foreign/arbitrary cursors, minimal JSON fields, legacy Admin
  versus explicit identity, invalid policy/storage, gate-wait and next-page
  revocation, direct deactivation, cancellation and one-connection reuse under
  inherited READ ONLY/REPEATABLE READ defaults. No fixture used real data.
- Focused adversarial self-review checked authorization before empty-page
  success, tenant predicates, cursor non-authority, activity retention, absence
  of sensitive fields and use of the shared transaction/error implementation.
  No independent agent, visual or full-policy acceptance is claimed.
- Clean SQLx regeneration completes in 54.24s, preserving all previous queries
  and adding three descriptors (1,408 total). Only 1.6 GiB of rebuildable Horae
  artifacts were cleaned. The owned disposable PostgreSQL has been stopped.
- Offline all-targets server Clippy passes (1m04s). WASM initially reports the
  unconsumed discovery DTOs; a single web-only `expect(dead_code)` on the page
  response records the missing picker and will require removal when connected.
  An unnecessary second expectation was removed. Final WASM Clippy passes
  (12.69s); no global lint suppression or UI placeholder was introduced.

Next: connect subject discovery under the reviewed editor/shell integration
contract, including requester changes and pagination, without activating mixed
policy or bypassing ordinary-directory guards. General directory predicates,
scoped approvals, person-management writes and preserved-data cutover remain open.
Delivery remains existing draft #212 without merge; general requirements 12/16.

## 2026-10-04 — Approval production-boundary review

- The preceding MVP response was status-only, not implementation progress.
  Revalidated clean synchronized `3f45b7c` and reused the existing worktree.
  Spec Kit prerequisites pass; its skills remain absent. No task completion or
  runtime acceptance follows from that prerequisite check.
- Read current official flexible approval, submission, FAQ, unlocking and
  approval-history sources. Older weekly/admin-only descriptions remain mixed
  with the flexible flow; neither resolves post-withdrawal submission state.
  No browser/MCP tool is loaded and no reference-account mutation occurred.
- Added `contracts/approval-coverage.md`: concrete production replacement map,
  transaction obligations, AC01–AC06 acceptance fixtures and discriminating
  unknowns. Distinguished other-project record state from whole-timesheet
  empty-cell coverage so FR-009 cannot erase FR-019's documented promotion case.
- Verified `feat/expense-parity` at `ba1b8e9` has reference/spec artifacts but no
  expense model, migration or server module. Combined approval fixtures are an
  actual implementation dependency, not a helper already available to import.
- Focused adversarial self-review rejects weekly row deletion as withdrawal,
  existing-row-only coverage, visibility-filtered mutation selection and assumed
  cross-writer ordering. Material lifecycle/schema findings remain open. No
  Rust/schema/UI/data change, policy activation, fresh runtime test or merge.
- GitHub confirms #212 remains OPEN/DRAFT at the starting revision. Formatting
  normalized the new Markdown list; the repeat CI-format check passes with
  496 files and zero changes, as do staged/worktree whitespace checks.

Next: resolve the new contract's discriminating coverage/withdrawal cases before
selecting its schema; reconcile expense delivery and T042 writer ordering.
Person-management writes still await the existing archived-endpoint answer.
Other closed operation-matrix/integration work remains available; do not repeat
the same documentation searches as if they supplied restricted-actor evidence.

## 2026-10-04 — Combined-approval record guard

- Previous turn made progress: published project-delegation activity fences as
  `202ee96` on draft #212. Revalidated the clean synchronized worktree and reused
  it. The archived-person assignment question remains unanswered; no default
  or repeated question has been introduced.
- Spec Kit prerequisites pass. Its skills remain absent locally; followed the
  existing spec/plan/contracts/tasks directly without claiming skill execution.
  Refined the closed FR-006/024 record-level conjunction as T136–T138. Unresolved
  self-approval, withdrawal, empty-date coverage and transaction rules remain
  outside that pure guard, not removed from the goal.
- Added the guard and 11 tests using existing selections/scopes and borrowed
  time/expense slices. It returns one boolean, never a filtered authorized
  subset or private denial details. No new dependency or catalog prerequisite.
- Initial test compilation fails for the missing function. Implementation passes
  all 180 core tests. The deliberate OR-for-AND mutation fails the mixed hidden
  expense regression; restored the correct code and all 180 pass again.
- Core all-targets Clippy and WASM compilation pass, as do formatting and
  whitespace checks. Focused adversarial self-review checked provenance, scope
  union versus capability intersection, empty inputs and caller obligations.
  This is not independent agent review or combined backend/browser acceptance.
- No application consumer, SQL/schema, UI, live data or policy activation changed.
  Delivery stays draft #212 without merge; general requirements remain 12/16.

Next: finish the approval lifecycle/coverage contract and its production
transaction with feature 016 fixtures; implement person-management transactions
when the pending inactive-endpoint decision is answered. Continue independent
T006/T009 work meanwhile. The pure guard cannot prove complete-set loading,
concurrent revocation, atomic rollback or full permission enforcement.

## 2026-10-04 — Project-delegation activity fences

- Previous turn was status-only, not implementation progress. Revalidated the
  clean synchronized `c88ca6d` worktree and reused draft #212's branch. Spec Kit
  prerequisites pass; its skills remain absent locally, so followed the existing
  spec/contract/task workflow without claiming skill execution.
- Investigated the next person-management transaction. Official assignment/API
  and archive documentation does not settle new links involving archived people.
  Asked one product question and recorded it in `research.md`; no answer or
  default has been inferred, and no existing decision was reopened.
- Independent work found project delegation's plain actor/new-manager activity
  reads can race direct deactivation. Three red regressions reproduced those
  two cases and inherited READ ONLY failure before the fix.
- Reused `configure_administration` and tenant-scoped user SHARE reads, without
  new user/project writes or lifecycle changes. Added reverse-order deactivation
  and single-connection cancellation/retry checks. All 128 selected permission
  regressions pass, including six new tests, in 54.22s after compilation.
- Focused adversarial source review checked replay authority, activity retention,
  retained inactive targets, the project NOWAIT rollback and cancellation effects.
  No additional actionable finding in this repair; this is self-review, not an
  independent agent review or full-policy acceptance.
- Cached SQLx preparation omitted unchanged integration-test descriptors; a
  missing sample still exists in `tests/integration.rs`. Cleaned only rebuildable
  Horae package artifacts (4.2 GiB); complete regeneration produced 1,405
  descriptors, removing only the replaced plain activity query. The locking
  query already has a shared descriptor. Offline all-targets server Clippy passes
  with warnings denied (1m03s), as do formatting and whitespace checks. No new
  WASM/browser/full-flake result is claimed. Delivery stays existing draft #212.

Next: the person-management transaction once its inactive-endpoint rule is
answered, or independent T006/T009 contracts
while it remains open. Full scoped approvals, cross-surface enforcement and
reviewed migration remain required. No policy activation, real-data mutation or
merge is authorized by this increment.

## 2026-10-04 — Real-browser recovery verification

- Previous turn made progress: published durable recovery as `1ecfa21` on draft
  #212. Revalidated the clean synchronized worktree and reused it. Spec Kit
  prerequisites pass; the full scope and independent-increment authority remain
  unchanged. General requirements are still 12/16, not an activation gate pass.
- No interactive browser/MCP tool is loaded, but inspection found the pinned Nix
  Playwright package and Chromium already available. Built matching server/WASM
  bundles in 164.14s and used the existing design runner, fresh local socket-only
  PostgreSQL and test-only canonical policy. No user browser, Harvest account or
  real database was used.
- Added the actual browser suite to the default runner. Real person/template
  commands commit before deliberate response loss. Reload/replay preserves the
  exact requester/command and a single PostgreSQL receipt. Cases also cover
  account switching, same-user reauthentication, authority denial/restoration,
  deleted-template replay, storage refusal and cleanup without resubmission.
- The final Chromium 148.0.7778.96 / Playwright 1.60.0 run also verifies successful
  self-demotion followed by browser cleanup failure: retry clears only local
  storage and sends no command under the now non-administrative identity. The
  combined runner passes all 14 existing script unit tests and the browser cases.
- Initial test failures identified test assumptions, not a disappearing dialog:
  its profile section was still collapsed. The corrected flow opens native
  details. Replaced an over-specific focus-wrap assertion with actual Tab access,
  background inertness and opener-focus restoration. Dirty Escape refusal and
  confirmation, plus keyboard Enter recovery, pass in the real browser.
- Inspected desktop-dark and narrow-light editor/recovery captures in
  `.scratch/permission-browser-evidence/`; horizontal overflow and panel bounds
  checks pass. No production UI, CSS, SQL macro or schema change was needed.
  Browser contexts close and the runner stops its owned PostgreSQL/server.
- Adversarial test review checked actual committed outcomes before dropped
  responses, exact-body comparisons, receipt counts, target deletion, account
  isolation and fixture teardown under the organization gate. No application
  change was needed. Formatting and whitespace checks pass; no repeated Rust
  regression or full-flake result is claimed for this test-only increment.

Next: continue T006/T009's unfinished operation/approval/migration contracts and
canonical shell/directory dependencies. This browser pass closes the enumerated
recovery evidence gap, not full T018, full enforcement, scoped approvals, reviewed
migration or the complete goal. Delivery remains draft #212 without merge.

## 2026-10-04 — Durable permission request recovery

- Previous turn was status-only, not implementation progress. Revalidated the
  clean synchronized worktree and OPEN/DRAFT #212 at `6bba224`; reused them.
  Spec Kit prerequisites pass. The implement skill is absent from the current
  local catalog/tools; followed the existing contract/plan/tasks directly under
  the independent-increment approval. General requirements remain 12/16.
- Added requester-scoped tab storage for exact person/template commands before
  sending. The shipped script refuses conflicting records and bounds UTF-8 size;
  Rust rejects malformed/misbound/noncanonical data. No new endpoint, dependency,
  schema, authority rule or CSS was needed. Existing get_me selects only the local
  storage slot, never server authority.
- Integrated explicit recovery into the existing dialog, including remount with
  no selected person. No automatic submission or invented request IDs. All server
  rejections retain the record; checked discard explains that an earlier attempt
  may have saved. Cached acknowledgements bind the whole original request and
  permit cleanup without issuing another command after a successful response.
- Storage tests first failed for the missing implementation, then all six passed.
  Existing handler expectations were updated to require recovery after rejection,
  not silently resume editing. Corrected a test that incorrectly expected public
  validation messages to be hidden like authentication diagnostics. The final
  selected run passes 61 UI tests and the combined script run passes 14 tests.
- Adversarial source review checked storage acknowledgement ordering, identity
  isolation, denied/unknown retries, replacement records, self-demotion cleanup,
  request-bound acknowledgements, dismissal and canonical decoding. Corrected
  permanent aria-busy during idle recovery and added an error-state regression.
  No further actionable finding in this increment; this was source review, not
  independent rendered acceptance. Existing design primitives remain unchanged.
- Final offline all-targets server Clippy (1m02s), WASM Clippy (13.26s), formatting
  CI and whitespace checks pass. No SQL changed, so no cache preparation or
  disposable database startup was needed for this increment.

Delivery remains existing draft #212, without merge or policy activation. No
browser tool is loaded, so real reload/sessionStorage, focus, keyboard, theme and
viewport acceptance remain unverified. Next: obtain isolated browser evidence for
T018 when available; continue the unresolved operation/approval/migration contracts
and canonical shell/directory integration without weakening their existing gates.
Full permissions, scoped approvals, enforcement and reviewed migration remain the
goal, not merely the completed editor increments.

## 2026-10-04 — Bind permission saves to the original requester

- Previous goal turn made progress: navigation protection was published as
  `e7d8a36` on existing draft #212. Revalidated the clean synchronized branch and
  reused its worktree. Spec Kit prerequisites pass; the independent-increment
  authorization and unfinished full-feature gates remain unchanged.
- Investigation of invoice recovery identified a prerequisite before copying its
  browser storage flow: an unsent permission draft could execute under a changed
  login. Added the local contract and sequential T130–T132 without inventing new
  Harvest behavior or changing the full goal.
- A registered HTTP regression reproduced the issue: a different Administrator
  received 200 from the person save. The editor now returns its authenticated
  organization/user pair; both save transports require it to match the session
  before execution/replay. Existing canonical authorization still runs inside
  the command transaction. UI callers keep the loaded pair on retries.
- Initial registered HTTP matrix and 48 selected UI tests pass. The final HTTP
  run (11.63s) additionally keeps the non-admin forged-payload case bound to that
  actual caller, so denial verifies authority rather than only identity mismatch.
  All 34 profile/editor PostgreSQL regressions pass (15.41s).
- Offline all-targets server Clippy (1m02s), WASM Clippy (12.67s), formatting CI
  (491 files, zero changes) and whitespace checks pass. SQLx prepare --check
  completes successfully (52.24s), warning of potentially unused descriptors.
  No cache file was pruned or changed; fixtures reuse existing checked SQL.
- Adversarial source inspection checked changed-login first sends, historical
  replay, cross-organization identity, omitted transport, all-grant non-admins and
  existing transaction/revocation ordering. The separate pair cannot select
  server authority or alter historical canonical intent. Full browser/recovery
  acceptance is not inferred from this check.

Delivery uses draft #212 without merge. Next: implement durable tab storage and recovery UI,
storing the exact command/pair before sending and preserving unresolved records
across denied retries. Do not mistake a later 401/403 for proof that an earlier
attempt did not commit. No real data, schema or runtime policy has changed.

## 2026-10-04 — Permission editor navigation protection

- The preceding MVP reply was status-only (no implementation progress). Revalidated
  the retained worktree changes and OPEN/DRAFT #212 at `8db19ba`; continued the
  existing T018 work without restarting it. Spec Kit prerequisites pass; seven
  local checklists pass and general requirements remain 12/16 under the existing
  independent-increment authorization. No extension hooks exist.
- Connected dirty/pending state to the existing project/invoice history guard.
  Reverted grants, profile source/revision and independent identity participate
  in dirty tracking. Template operations cannot clear unsaved person changes.
  The shared script changes only its permission-specific messages.
- Source review found initial/reload reads were still marked clean. A failing
  controlled-response test reproduced it; those reads now protect navigation
  and dismissal. Close, Cancel, native cancel/Escape and backdrop share native
  discard confirmation; refusal or bridge failure preserves the draft. A delayed
  reply checks the current target, generation and pending request before closing.
- Eight shipped-script unit tests and 48 selected UI tests pass. Fixed test
  harness assumptions about Dioxus static IDs and pointer event construction;
  corrected the grant-reversal fixture to retain dependency-removal effects.
  Independent source re-review closes the read/dismissal findings. The design
  documentation review confirms no new global design rule/artifact is needed.
- Final offline all-targets server Clippy (1m03s) and WASM Clippy (12.76s) pass
  with warnings denied. No query changed, so SQLx regeneration is unnecessary.
  Formatting CI passes (491 files, zero changes), as does whitespace validation.
- Scoped consistency review maps FR-004/012/016/018 to T018, its navigation
  contract, handler tests and script regressions. No new local specification or
  constitution conflict was found. Full Spec Kit analysis/acceptance remains
  unfinished with the general requirements and policy-transition gates.

Delivery uses existing draft #212 without merging. Next implementation: durable
same-request recovery after
forced reload with original requester/workspace binding; use the invoice recovery
flow as prior art, not its legacy role assumptions. Browser acceptance remains
unverified (no browser tool loaded). Full enforcement, scoped approvals and
reviewed migration remain required; no real data or active policy changed.

## 2026-10-04 — Readable management-loss previews

- The preceding turn was status-only. Revalidated clean synchronized `0f97cb2`
  and OPEN/DRAFT #212; resumed T018 under the existing independent-work approval.
  Spec Kit prerequisites pass; general requirements remain 12/16.
- Added tenant-local subject names to the authorized loss preview, including
  inactive subjects, without changing exact-ID confirmation or audit storage.
  Reused the existing historical relationship type and wrapping utility; no CSS,
  dependency, schema, policy activation or real-data change.
- RED reproduced missing names. All 34 profile/editor database tests then passed.
  Two UI assertions expected named HTML entities instead of Dioxus's equivalent
  numeric escapes; corrected the expectations. All 42 selected UI tests and the
  real-session HTTP authorization/lifecycle matrix passed. Native/WASM lint passed,
  but cached SQLx preparation omitted 91 still-used integration-test descriptors.
  Cleaning only rebuildable Horae artifacts (4.2 GiB) restored complete generation:
  1,406 descriptors, five added and none removed or modified. Repeated offline
  all-targets server Clippy (1m02s) and WASM Clippy (12.74s) passed with warnings
  denied. Formatting CI passed (491 files, zero changes); disposable PostgreSQL
  was stopped after verification.
- Independent source review found no actionable issue. The documentation review
  confirmed existing design primitives suffice. Neither review establishes
  rendered, keyboard, viewport or theme acceptance; no browser tool is loaded.

Delivery remains the existing draft #212, with no merge. Next implementation:
draft/navigation/uncertain-save recovery using the existing editor navigation
guard as prior art. Full T018, enforcement, approvals and migration remain open.

## 2026-10-04 — Reusable profile controls

- Previous turn made progress by publishing the person editor. Revalidated clean
  synchronized `9e6d8bd`; reused the worktree, branch and draft #212. Executed
  Spec Kit prerequisites; existing independent-increment authorization remains
  in force, with full requirements still 12/16 and no policy activation.
- Added template creation/deletion within the existing dialog using shared
  Input, FormGroup, Checkbox, descriptions and CSS utilities. No CSS, SQL,
  migration, dependency or server policy changed. Removed obsolete WASM DTO
  unused-consumer annotations now that all three template DTOs have consumers.
- Initial RED reproduced missing create/delete controls. Corrected the preview
  call to supply both expected revisions; used the existing constant-ID pattern
  for production control events in the VirtualDom harness. Expanded editor suite
  passed all 24 tests (38.54s build, 0.02s execution).
- New cases cover final draft grants without saving the person, same-command
  retry, explicit deletion confirmation including empty profiles, stale/mismatched
  previews, cancellation preserving the person draft, duplicate/limit/validation
  errors, creation identity/count gates and denial hiding.
- Selected regressions initially passed 40 tests; offline server/WASM lint also
  passed before review fixes. Independent source review found no high/critical
  issue, but requested pending-deletion-preview locking and replacement of
  nonexistent `my-*` utilities. Corrected both; its source verdict confirms the
  two fixes, without claiming rendered acceptance.
- Added pending-preview, loaded-preview cancellation and explicit reload tests.
  The expanded run passed 25/26 and reproduced reload retaining the old form when
  returned revisions were unchanged. Fixed all dialog reload entry points with a
  local generation key, preserving server revisions and pending-save guards.
  The post-fix run passed all 42 selected UI tests (editor 26, Settings 11, admin
  shell five), offline all-targets server Clippy (1m02s) and WASM Clippy (12.53s),
  with warnings denied. Independent source review also found the reload fix sound.
- The read-only design documentation review found no new system rule or remaining
  source-to-token mismatch in its scope. Existing DESIGN.md remains unchanged;
  no rendered/focus/keyboard acceptance is inferred from this source verdict.

Delivery remains the existing draft #212, with unsigned commits and no merge.
Next implementation: readable person-editor relationship-loss labels from its
authorized preview, then draft/navigation recovery and isolated browser acceptance.
No loaded browser tool is available; keyboard, viewport, theme and rendered
acceptance are not claimed. Template mutation currently requires explicit reload/
discard of unsaved person changes; smoother draft recovery and all full-feature
enforcement/approval/migration gates remain required. No real data changed.

## 2026-10-04 — Person editor publication

- Published unsigned commit `98b1692` on `feat/scoped-permissions` and updated
  existing #212's description with separate backend/UI evidence and limitations.
  GitHub confirmed OPEN/DRAFT at that exact commit; local branch synchronized.
  No merge, browser observation, migration, activation or real-data change.
- Reused the completed test/lint process rather than rerunning it. Final
  formatting CI passed again after the evidence update: 490 files, zero changes.
  Full requirements remain 12/16 under the existing independent-work approval;
  T018 and the full implementation goal remain open.

Next: implement T018's template create/delete controls from the existing
`template-commands.md` and `permission-editor.md` contracts. Start with their
production-control regression tests: duplicate/limit errors, explicit final
grants, stale deletion preview, exact affected-person confirmation and identical
retry after an uncertain response. Preserve grants on deletion. Readable loss
labels, navigation recovery, browser acceptance, canonical shell/directory,
full enforcement/scoped approvals and reviewed data migration remain required.

## 2026-10-04 — Person permission editor controls

- Prior turn made progress: backend and publication record were committed and
  pushed. Revalidated clean synchronized `7f7fd1c`; continued in the same worktree
  and draft #212. Ran Spec Kit implementation prerequisites for feature 015.
  Full requirements remain 12/16 under the existing independent-work authorization.
- Implemented T018's first person editor consumer in `pages/admin/permission_editor.rs`
  with a tested local draft. The existing user page offers entry only for the
  supported own-permission projection's explicit Administrator identity. The
  canonical load/preview/save endpoints independently authorize every operation.
- Reused the core grant graph, shared descriptions, Modal, Checkbox and utility
  classes. No CSS, schema, SQL query, dependency, legacy guard or active policy
  changed. Removed only unused-consumer lint expectations that are now obsolete;
  template-create/delete transport expectations remain applicable.
- RED reproduced the missing draft implementation. First GREEN passed six tests;
  expanded native state/SSR tests passed 12. Interaction harness initially failed
  because Rust path-included modules needed explicit sibling paths; corrected
  those paths. The first interaction run passed 15 tests, including actual Dioxus
  double-click, identical-command retry and revocation/form-hiding scenarios.
- Added a further control-level test for explicit keep-project access, a fresh
  preview, independent person-loss confirmation and stale-save recovery. Selected
  UI regressions passed 32 tests (editor 16, Settings 11, admin shell five).
  Clippy requested one equivalent boolean simplification; corrected it. The
  post-fix editor run passed all 16 tests, followed by offline all-targets server
  Clippy (1m01s) and WASM Clippy (12.60s), with warnings denied.
- The intervening MVP reply collected the existing verification handle rather
  than restarting it. Its successful completion is evidence for publication,
  not completion of the full feature. No code changed after that verification.
- Final formatting CI passed (490 files, zero changes), and whitespace checks
  passed. GitHub confirmed #212 is still OPEN/DRAFT on the expected branch.
  This UI increment has no new independent visual review or browser evidence.

Next: publish this verified person-editor increment on draft #212 after final
formatting and whitespace checks. Complete template create/delete controls,
browser keyboard/viewport/theme acceptance, readable management-loss labels and
navigation/uncertain-save recovery. Preserve the separate canonical shell/directory,
full enforcement/approval and reviewed migration gates; T018 and the goal remain
open. No Chrome/Harvest observation or real-data change occurred in this iteration.

## 2026-10-04 — Editor backend publication and UI entry-point check

- The preceding MVP response was status-only. Resumed the actual retained final
  formatting process: it completed successfully, 486 files and zero changes.
  Both staged whitespace and unstaged-diff checks passed before committing.
- Published unsigned commit `03e90b1` on the existing branch and updated draft
  #212's body. GitHub confirms OPEN/DRAFT at that exact commit; no merge. The
  worktree was clean and synchronized after publication. No test rerun was needed
  for this unchanged, previously verified backend.
- Executed the Spec Kit implementation prerequisite check against feature 015.
  No extension hooks exist. This is not completion of the remaining implementation
  tasks or the full requirements checklist.
- Read the Workspace prototype and actual `pages/admin.rs`. The route still owns
  legacy user creation, role changes, activation and task management. Preserve
  these working flows; the new editor must not imply that policy 1 is active or
  that the legacy three-role list is canonical permission state.
- Workspace's prototype has a read-only three-role matrix and no per-person
  editor. Its visual system remains useful, but its role semantics conflict with
  the approved six-profile/custom-permission specification. Use the confirmed
  feature contracts for behavior, not the obsolete prototype role matrix. Do not
  copy its sample seats, last-active values, invitation counts or audit records.
- The design context launcher failed because its engine is not installed and
  its cache directory is unwritable. Read the existing DESIGN.md and handoff
  directly instead; PRODUCT.md is absent. No UI/CSS edits, browser validation,
  policy activation or real-data change occurred in this publication iteration.

Next: implement T018's person-permission controls on the existing AdminUsers
surface using the published load/preview/save endpoints, shared permission
descriptions and existing components. Complete the design import/component
inspection before editing. Cover stale previews, exact relationship-loss
confirmation, cancellation and uncertain-save retry without changing request
identity. Keep legacy mode non-editable for the new model; record the visual
extension beyond the old three-role prototype. Full enforcement, assignment and
approval integration, reviewed migration and browser acceptance remain open.

## 2026-10-04 — Permission editor integration

- Previous turn was an MVP status answer: no implementation progress. Revalidated
  clean synchronized `300d1e9`; reuse the existing worktree/branch and draft #212.
- Executed Spec Kit Plan/Tasks/Implement helpers, preserving all existing artifacts.
  Seven local checklists pass 7/7; full requirements remain 12/16 under the existing
  authorization for confirmed independent increments. No extension hooks exist.
- Independent Plan research identified concrete command activity/settings gaps.
  Added the editor contract and T126–T129 without activation, legacy guard changes,
  guessed product predicates or data migration. Full scope remains unchanged.

Next: reproduce the command races/settings failures, protect command authority,
then connect shared preview/save effects and authenticated editor delivery. The
remaining UI, enforcement, scoped approvals and reviewed migration are not complete.

- RED reproduced all three new profile failures: actor replay and remaining-admin
  deactivation did not wait for competing writes; inherited READ ONLY rejected
  organization locking with SQLSTATE 25006. Added SHARE activity protection and
  a shared bounded transaction configurator for profile/template administration.
  The focused GREEN run is in progress; no passing result is assumed.

- GREEN: all nine selected command tests passed, including the three reproduced
  failures and direct template replay deactivation. Added the shared profile
  calculation, editor projections and mode1-only load/preview. Editor RED failed
  for missing modules before implementation; the first broader run passed all
  104 permission tests (45.61s), including preview/save/no-op equivalence.

- Registered-route RED reproduced zero `load_permission_editor` endpoints.
  Implemented all five session-derived editor endpoints, preserving command wire
  shapes and sanitizing authentication/storage failures. Added template deletion
  preview and actual HTTP lifecycle coverage. Full server regression is running.

- Independent research review found no blocking issue in internal calculation,
  activity locks or replay shapes. Added its requested template-deletion
  equivalence/stale-preview and command-winning survivor-deactivation tests.
  Wrapper review and final validation/publication are still pending.

- Full server binary passed: 1,030 tests, zero failures, 11 pre-existing ignored
  manual measurements (250.14s), including all 15 new database cases and the
  registered editor lifecycle. T126/T127 are complete. Independent wrapper
  re-review found no high/critical issue. WASM lint passed after scoped transport
  DTO annotations for the not-yet-connected Workspace consumer. No UI is claimed.

- GitHub confirms #212 remains OPEN/DRAFT on `feat/scoped-permissions` at
  `300d1e9`. The expanded HTTP matrix, complete SQLx/offline gates and publication
  remain next; no merge or policy activation is authorized by these passing tests.

- Expanded real-session HTTP matrix passed (11.53s), including sanitized initial
  authentication failure/recovery, every endpoint's legacy/future-policy denial
  and input errors. T128 is complete. SQLx preparation finished in 52.55s with
  14 added/six obsolete removed descriptions (1,401 total); inspected all removals
  against the changed profile queries. Only 1.6 GiB of rebuildable package
  artifacts were cleaned, with no source or database removal. Offline native
  lint remains running. Retry formatting after SQLx's transient cache deletion;
  the overlapping format check was not a passing result.

- Fresh offline all-targets native Clippy passed (1m01s), and the subsequent
  formatting CI check passed with zero changes. T126–T129 are complete for this
  backend. No schema, dependency, UI/CSS, real data or policy activation changed.

Next: publish this verified editor backend as an unsigned commit on existing
draft #212, without merging. Then apply the design skills to the Workspace/person
permission editor, using these real load/preview/save functions and shared grant
descriptions. Inspect the existing `pages/admin.rs`/AdminUsers route rather than
assuming a Workspace page already exists. Keep legacy mode non-editable for the
new model. UI/browser acceptance, remaining operation predicates, person-management
commands, scoped approvals, full enforcement and reviewed migration remain
mandatory. The full goal stays active; this is not an MVP-ready declaration.

## 2026-10-04 — Authenticated audit delivery

- Previous MVP reply was status-only, not implementation progress. Revalidated
  clean `8c15bfe` and reused `feat/scoped-permissions`; no activation or real-data
  change. Spec Kit plan/task/prerequisite scripts reused feature 015. No extension
  hooks are configured. General requirements remain 12/16; seven existing local
  checklists remain 7/7 under the confirmed independent-increment authorization.
- Refined T123–T125 for FR-010/011/013/018's authenticated single-receipt lookup.
  Independent read-only design review identified direct deactivation and inherited
  READ ONLY gaps before exposing the existing reader. Contract now requires
  requester fencing, explicit transaction mode and local time limits.
- Added registered-route/session/disclosure tests and reader regression cases.
  Started only the retained disposable PostgreSQL on 55416. RED test is running;
  no pass, completed task, publication or browser verification is claimed yet.
- RED confirmed all three missing behaviors: inherited READ ONLY rejects SHARE
  (SQLSTATE 25006); direct deactivation does not block the reader; the registered
  route count is zero. The first GREEN compile exposed a missing qualified UUID
  and a SELECT macro needing `fetch_all` rather than `execute`; both are corrected.
- Implemented the typed session wrapper, shared historical DTOs and current
  requester fence. Added both race orders, stricter statement-limit preservation,
  cancellation/single-connection cleanup, real HTTP shape round-trips and separate
  operator attribution. The complete permission regression is compiling.
- Permission regression completed: 97 passed, zero failures/ignored (45.19s).
  Independent implementation review found one error-projection gap before the
  reader: the initial session-user query could leak a raw SQL error. A real HTTP
  cancellation fixture reproduced it; the wrapper now preserves 401 and sanitizes
  other authentication failures. Final post-fix HTTP and build gates remain.
- Scoped Analyze covers four requirement subsets (FR-010/011/013/018), three
  sequential tasks, 100% local mapping, zero unmapped tasks, ambiguity,
  duplication or critical constitutional conflicts. General full-feature gates
  remain unchanged. GitHub confirms #212 OPEN/DRAFT on the expected branch.
- Registered HTTP matrix now passes in the running full-server suite, including
  the reproduced initial-query error and post-error recovery. Independent
  re-review closes that finding with no further blocker. T123/T124 are complete;
  T125 still awaits the full run, cache, offline builds/lint and final formatting.
- Full server-binary suite completed after the authentication fix: 1,015 passed,
  zero failures, 11 existing ignored manual measurements (240.64s). All 97
  permission cases and the registered HTTP matrix are included. No new exclusion
  was added. Complete SQLx preparation and offline server/WASM Clippy follow.
- Clean SQLx preparation passed (51.45s): 14 new descriptors, none removed or
  changed, 1,393 total. Offline all-targets server Clippy passed (1m00s). WASM
  denied-warning lint detected the deliberately unconnected historical DTOs;
  added a web-only `expect(dead_code)` with the missing UI consumer as its reason.
  A module-level expectation was unfulfilled; moved it to the root `AuditEntry`,
  matching the project's earlier own-permission DTO pattern. Denied-warning
  offline WASM then passed (11.21s). No runtime code or tests are bypassed;
  the final offline server lint is finishing.
- Final offline all-targets server Clippy also passed (1m00s). Review of all 14
  new cache descriptors finds only this increment's checked production/test SQL;
  no unrelated metadata was lost. Only rebuildable package artifacts were
  cleaned, and the owned disposable PostgreSQL is stopped with its data retained.
- Final formatting succeeds and `git diff --check` is clean. T123–T125 are
  complete for authenticated single-record delivery. Publication uses an unsigned
  commit and normal push to the existing draft; no merge or full-flake/browser
  acceptance is claimed. The PR description preserves the remaining full scope.

Next after publication: continue the canonical operation/transition contracts and authenticated permission
editing, incorporating pending product answers without guessing them. Single-record
audit delivery is not a history browser, policy activation or complete MVP.

## 2026-10-04 — Own-permission Settings integration

- The preceding MVP response was status-only (no progress). Revalidated the clean
  worktree at `8aac739` and reused draft #212. No policy activation or real-data
  change. Spec Kit prerequisites and task setup reused feature 015; the general
  checklist remains 12/16 under the independent-increment authorization.
- Refined the existing own-reader contract and T120–T122 for the Settings
  consumer, preserving General/Plugins and using only existing CSS utilities.
  No new dependency, SQL, schema, profile inference or privilege editor.
- RED: nine SSR tests fail against the empty view. GREEN: all nine pass after
  implementation. The interaction harness initially failed compilation on a
  source-module path and missing ElementId import; both were corrected.
- Adversarial self-review against the captured reference found that saved-report
  `inactive` describes the owner, not product availability. Corrected both labels
  and assertions; draft-invoice copy now includes creation. Broad withdrawal copy
  does not invent unresolved lifecycle predicates. No blocking local finding
  remains; this is self-review, not a new independent reviewer approval.
- Final interaction/SSR suite passes all 11 tests (38.16s build, zero failures or
  exclusions). It exercises actual resource loading, refresh, pending-click
  suppression, stale-content removal and error recovery with controlled reads.
  Session 80701 completed: all-targets server Clippy passes with warnings denied
  (1m00s), as does denied-warning WASM (9.88s). Formatting normalized four changed
  source/test files. No SQL changed, so cache regeneration is unnecessary.
- Scoped Analyze maps FR-012/016/018 to all three new tasks, with no unmapped
  task, local constitutional conflict, ambiguity or duplication. Browser layout,
  keyboard and themes remain unverified; no Playwright tool is exposed this turn.
  Full Settings/Workspace acceptance and activation gates remain open.
- Final format check passes: 478 files, zero changes (2.604s); `git diff --check`
  is clean. Verified #212 is OPEN/DRAFT on the expected branch. Publication uses
  an unsigned commit and normal push; its refreshed description distinguishes
  implemented storage/UI from still-incomplete canonical enforcement.

Next after publication: continue
the remaining canonical operation/transition contracts; incorporate pending
product answers when received, without repeating questions or inferring consent.

## 2026-10-04 — Invoice scope research and migration preflight

- The intervening MVP response was status-only. Revalidated clean branch at
  `0793ce7`, one ahead; normal authorized push succeeded. No merge occurred.
- Spec Kit Clarify/Plan/Tasks reused feature 015 and existing artifacts. Fresh
  primary-source research still cannot establish mixed-project invoice scope.
  Asked one new product question about whole-invoice coverage; no answer is
  inferred and earlier pending questions are not repeated.
- Started the existing Chrome MCP client for read-only evidence; connection is
  pending, not a successful browser observation. No account data was changed.
- Refined the independent M01/M07/M08 count-only source preflight, T117–T119.
  This advances reviewed-transition preparation without selecting mappings,
  repairing historical data or treating a clean report as activation readiness.
  Independent design review closed the local boundary, including inbound approval
  references and independent counts for attribution and unexpected states.
- Scoped Spec Kit Analyze maps five requirement subsets to T117–T119 with no
  unmapped task, local constitutional violation, duplication or unresolved
  critical/high issue. The general checklist remains 12/16; seven local
  checklists remain 7/7 under the prior independent-increment authorization.
- Browser tab listing timed out. Closed only the owned MCP client; no tabs or
  account data were modified. Source documentation and existing snapshots do not
  substitute for a current restricted-user observation.
- RED fails on the missing cross-tenant count (0 versus 1; 0.52s after 1m43s
  compilation). The implemented reader passes six focused tests (3.56s after
  1m36s compilation). Subsequent test-only additions cover the independent
  review's historical inactive approver case and lock-timeout cleanup; the final
  permission regression now recompiles that snapshot. No final pass is inferred
  from the earlier six-test result.
- Final permission regression passes all 92 tests, including seven preflight
  tests, zero failures/exclusions (40.77s after 1m36s compilation). Independent
  re-review finds no blocking implementation issue. An optional approver-only
  inbound fixture was suggested; current SQL covers it, while the existing
  inbound fixture exercises both subject and approver together.
- Session 47930 continues with clean complete SQLx preparation and offline gates.
  Only rebuildable package artifacts were removed (173 files, 1.6 GiB); no data
  or source files were deleted. Full server/flake acceptance is not claimed.
- Session 47930 completed successfully: clean SQLx preparation 51.94s, offline
  all-targets Clippy 1m00s and denied-warning WASM 9.71s. Cache review finds 17
  new descriptors, zero removed/modified and 1,379 total. No unrelated cache
  entry was lost. Only the owned disposable database is being stopped, retaining
  its data. Final formatting and unsigned publication remain before handoff.
- Disposable database shutdown succeeded. Formatting passes with zero changed
  files. T117–T119 are complete for this internal diagnostic only; reviewed
  activation, complete source preflight and invoice scope decisions stay open.
  Publication targets the same verified OPEN/DRAFT #212, with no merge.

Next after publication: integrate the user's mixed-project invoice answer when
received; separately resolve unlinked invoice and source/projection permissions
before implementing canonical OP21–OP24. Do not silently infer those answers,
activate policy or substitute another unrelated legacy repair for integration.
If no answer is available, refine the remaining reviewed-transition inventory
without mapping roles, repairing records or treating the diagnostic as readiness.

## 2026-10-04 — Budget email authority preparation

- The preceding MVP answer was status-only, with no implementation progress.
  Revalidated clean `feat/scoped-permissions` at local `dcf4ac8`, one ahead of
  origin. Normal push succeeded; local and origin now match. Existing draft
  #212 is reused; no merge or real-data mutation occurred.

- Spec Kit Plan/Tasks setup and prerequisite scripts reused existing artifacts.
  Seven local checklists pass 7/7 each; general requirements remain 12/16 under
  the prior authorization for independently clarified increments. No extension
  hooks or agent-context generator exist. Refined T114–T116 and the bounded
  budget-email contract; full policy and activation gates remain open.

- Source review confirms migration 0039's child-to-parent trigger ordering:
  recipient/project gates need fresh eligibility reads, not later child locks.
  Independent design review is in progress. Only disposable PostgreSQL and
  local executable sender stubs will be used.

- Independent review closed the local design: late outbox UPDATE and terminal
  rejection inside the transaction avoid stale-payload terminalization and lock
  upgrades. Notification retarget/appearance is drift, not ineligibility.

- Scoped Analyze maps FR-006/007/010/017/018 and SC-006 subsets to T114–T116;
  no unmapped task, ambiguity, duplication or local constitutional/critical/high
  finding. Full operation/migration/UI/activation gates remain open.

- RED reproduced the missing organization gate (5.62s after 1m47s compilation).
  Implemented private preparation and preserved post-release transport/acks.
  Initial notification run passes 20/21; the assignment fixture wrongly updated
  a nonexistent membership. Corrected to INSERT the lead assignment before
  revoking it; no production predicate was weakened. Expanded session 72877
  rejected a misplaced test-output flag before compiling. The corrected command
  formats and tests the final fixture snapshot; no pass is inferred from 72877.

- Independent code review found no critical/high defect and requested explicit
  inherited READ ONLY coverage; added a preparation-only check so unrelated
  post-send queue writes are not conflated with the authorization transaction.

- Expanded review strengthened cancellation to wait after acquiring domain locks
  and verify their release from another connection. The blocked sender fixture
  now opens its FIFO read/write before launch and bounds joining the task.
  Independent re-review confirms both fixes and no remaining blocking finding.

- Added winning activation/alert enable, project retarget/disappearance and exact
  terminal-reason coverage. SQLx caught test-only mistakes in legacy assignment
  columns and a cleanup SELECT's alias/fetch method; corrected them rather than
  weakening checks. Sessions 64383/3852/91469 are terminal compile failures.
  Session 93679 passes all 30 notification tests: 13 new authority tests plus
  existing mail tests, zero failures (2.86s after 1m34s compilation).

- T114/T115 are complete. The final source snapshot now runs the full server
  binary regression, then clean complete SQLx preparation and offline lint/WASM.
  No full-suite pass is inferred from the focused notification result.

- Full server-binary regression passes: 993 passed, zero failures and 11 existing
  exclusions, 1,004 discovered (238.69s). Session 46574 continues with clean SQLx
  preparation and offline gates after removing only rebuildable package artifacts.
  Formatting passed (two documentation files changed; no source behavior change).

- Next-action review identifies T006's invoice operation contract as the next
  canonical integration target. Verified the three invoice mutation roots and
  fee parent-trigger ordering, and revisited current Harvest invoice permissions,
  project linking and fixed-fee context documentation. Research records evidence
  and unresolved mixed/manual scope; no predicate or existing-data mapping was
  selected. Previously asked product questions remain open without repetition.

- Session 46574 finishes successfully. Clean SQLx preparation takes 52.89s;
  review confirms 44 new descriptors and only the two replaced notification
  queries removed, 1,362 total. Offline all-targets Clippy passes (1m00s) and
  denied-warning WASM passes (9.48s). T116 is complete for this boundary.
  Final scoped Analyze retains six requirement subsets/three mapped tasks,
  no ambiguity/duplication/local constitutional violation or unresolved critical/
  high review finding. Full feature gates and `nix flake check` remain open;
  no merge readiness or canonical runtime completion is claimed.

- Stopped only the owned disposable PostgreSQL, preserving its data. The final
  format check normalized list whitespace in this log and is rerun before the
  authorized unsigned publication to the existing branch/draft PR.

Next after publication: refine OP21–OP24's canonical invoice permission and
transaction contract from reference evidence before implementation; do not
substitute another unrelated legacy repair or activate a partially enforced policy.

## 2026-10-04 — CSV authority integration

- The intervening MVP answer was status-only, with no implementation progress.
  Revalidated clean `feat/scoped-permissions` at published `108594e`, matching
  origin. This completes the preceding increment's publication next-action;
  no merge or real-data change occurred.
- Spec Kit Plan/Tasks reused existing artifacts and refined T110–T113 under
  `contracts/csv-exports.md`, using the previously verified native-cursor probe
  and independent design review. No new product predicate is introduced.
- Keep source snapshots separate from current release authority. The review
  requires an independent output-row cap and a post-capture invoice fixture;
  a pre-DECLARE table wait would weaken the original snapshot assertion.
- Scoped analysis maps FR-006/007/010/017/018 and SC-006 subsets to all four
  tasks: no unmapped task, ambiguity, duplication or local constitution issue.
  General requirements remain 12/16; seven local checklists remain complete.
  Prior independent-increment authorization applies. No extension hooks exist.
- RED reproduced inactive project actors receiving HTTP 200 (1.03s after
  4m30s compilation). Integrated the invoker cursor helper, per-block fresh
  savepoint authority and all three producers, retaining actor identity.
  Migration 0046 was applied only to the owned disposable database.
- Initial GREEN passes all nine existing CSV regressions (13.07s after 5m14s
  compilation), including exact types/amounts, 10,001 rows and post-capture
  invoice coherence. Those results precede the new race/cleanup test module.
- Independent code review prompted dropping the first invoice row after its
  metadata/totals are captured and adding production-path tests rather than
  relying on the old test-only streaming helper. Session 48976 compiles the
  expanded report suite and real-cookie HTTP matrix. A new invoice fixture
  incorrectly wrote a generated subtotal; corrected it to update only total.
  The in-flight compilation predates that fixture correction.
- Session 48976 terminated at compile time: SQLx rejected both generated-column
  fixture updates. Corrected session 56478 compiles in 1m50s and runs the report
  family before the HTTP matrix. No test pass is inferred from the failed build.
- Review additionally strengthened the mixed-project block test (deny the whole
  block when one current ID loses access; allow it when only a previously sent
  ID is revoked) and asserted HTTP 200/CSV headers before parsing empty results.
  The report binary already running predates these last test-only changes;
  the final full regression must cover the corrected snapshot.
- The expanded report family passes 84 tests, zero failures and two existing
  manual exclusions (57.30s). Cancellation during FETCH and authority waits,
  native byte/row bounds, snapshot preservation and current release checks pass.
  Session 56478 is recompiling for the HTTP matrix after test refinements.
  Independent re-review confirms its three findings resolved; no remaining
  material issue found. Full final-snapshot regression remains required.
- The real-cookie HTTP matrix passes (11.46s after 1m55s recompilation), now
  including all three CSV routes alongside existing XLSX/PDF. T110/T111 are
  complete. Session 56478 is terminal; session 36093 runs the final-snapshot
  full server suite, then complete SQLx preparation and offline Clippy/WASM.
  The previous report run does not substitute for this final regression.
- Full final-snapshot server regression passes: 980 passed, zero failures,
  11 existing exclusions, 991 discovered (257.91s after 1m50s compilation).
  T112 is complete. Session 36093 continues with complete cache regeneration
  and offline Clippy/WASM. GitHub confirms existing #212 is still open/draft
  on this branch at 108594e; no PR mutation or merge occurred.
- Session 36093 finishes successfully: SQLx preparation 55.52s, offline
  all-target Clippy 1m06s, denied-warning WASM 27.57s. Catalog inspection confirms
  the installed helper is SECURITY INVOKER and PUBLIC has no EXECUTE privilege.
  Cache review, however, finds 93 removed descriptors and 41 new ones (1,229
  total); sampled removals still exist in `tests/integration.rs`. Cached target
  compilation omitted their regeneration, so these cache/offline results are
  not accepted as complete-cache evidence. Session 15361 cleans only Horae's
  rebuildable package artifacts, regenerates all targets and repeats offline
  gates. No source or business data is removed by that cleanup.
- Clean preparation passes in 56.54s: 41 new, two obsolete and zero modified
  descriptors, 1,320 total. Both removals are the old unqualified invoice
  snapshot fixture updates, replaced by the reviewed post-capture fixture.
  All unrelated integration descriptors are restored. Offline all-target Clippy
  passes (1m05s) and denied-warning WASM passes (10.41s); session 15361 is terminal.
- T110–T113 are complete for all three CSV families. Final scoped analysis maps
  six requirement subsets to four tasks with no unmapped task, ambiguity,
  duplication or local constitutional/critical/high finding. Independent review
  findings are resolved and covered by the full final-snapshot passing suite.
  This does not close full feature policy, approvals, UI or activation gates.
- Final formatting check passes with zero changes (3.001s). Stopped only the
  owned disposable PostgreSQL after all checks, preserving its data. Publication
  uses the existing isolated branch and draft PR; no merge is authorized.

Next after unsigned publication to existing draft #212: refine the bounded
`notifications::deliver` preparation contract from the new research evidence,
including current recipient eligibility and claim lease after waits, before
implementing its tests/transaction. Do not select unresolved historical import
requester/retry policy, activate canonical grants, send real mail or merge.
Full policy, approvals, UI and reviewed activation remain open.

## 2026-10-04 — Materialized project exports

- Previous goal turn made progress: T104–T106 verified and published unsigned
  as `dc822a0` to draft #212. Revalidated clean worktree and matching origin;
  all previous command handles are terminal and disposable PostgreSQL stopped.
- Spec Kit Plan/Tasks reused existing artifacts and added T107–T109 and
  `contracts/project-exports.md`. Independent design review confirms the
  READ COMMITTED/single-statement approach and separately refreshed final
  relationship check. No new product predicate, writer or schema change.
- Local constitution check passes. Seven local checklists remain complete;
  general requirements remain 12/16 with prior independent-increment authority.
  No extension hooks or context generator apply. Rust/async/testing conventions
  require real transaction tests before implementation and no new dependency.
- Confirmed stopped disposable PostgreSQL (pg_ctl status 3), then restarted
  only that cluster. Session 42461 compiles the initial actor-rejection RED test.
  Follow-up research identified the existing registered HTTP harness as the
  real finalization/editor race path, avoiding new test-only production hooks.
- RED reproduces invalid-actor acceptance (0.73s after 3m49s compilation).
  Added the READ COMMITTED actor boundary, bounded single-statement loader,
  private captured IDs and fresh release check after sorted parent locks.
- The first GREEN build stopped on fixture role names (`freelancer`, not
  `member`), an unqualified response-body type and an inferred SQL integer width.
  Corrected all three and added scope/size/snapshot/cancellation cases plus the
  real finalized-project and editor visibility HTTP races. No passing execution
  is claimed from that failed compilation.
- Adversarial review found no authority-order defect, but identified an
  unnecessary pre-limit sort, duplicate visibility fixtures, an obsolete
  inactive-actor expectation and missing reader-first parent-lock coverage.
  Removed the inner sort (the final sort retains output order), distinguished
  UPDATE from INSERT, changed only the inactive privacy assertion to 403, and
  added parent-retention and loading-cancellation tests with a private test view.
  Session 22988 was already compiling its earlier snapshot; its results will
  not certify these later review changes. No parallel build was started.
- Initial GREEN passes all ten then-present project tests (10.47s after 3m43s
  compilation). The same command detected the review edits and is recompiling
  them before the real HTTP matrix and complete report suite. Twelve focused
  tests are now present. Follow-up adversarial review confirms all four findings
  resolved and no remaining material gap; it did not execute tests.
- Corrected real HTTP matrix passes (12.71s after 3m27s recompilation), including
  newly finalized assignment and editor visibility expansion while the export
  waits on their actual writer. Report regression passes: 68 passed, zero failed,
  two existing manual exclusions (50.80s), including all twelve project tests.
  Session 22988 is terminal. Full server regression and complete cache/offline
  gates follow before publication; no partial result certifies the full feature.
- Scoped Analyze maps five FR subsets and SC-006's regression subset to all
  three tasks, with no unmapped task, ambiguity, duplication or constitutional
  conflict. Full-feature analysis and activation remain separate open gates.
- Full server-binary regression passes: 964 passed, zero failed, 11 existing
  exclusions, 975 discovered (327.07s). T107/T108 are complete. Formatting changed
  only the newly added test layout while the compiled suite ran; no functional
  change followed the reviewed passing tests. Session 34644 continues with
  complete SQLx regeneration and offline lint/WASM after cleaning only the
  app's rebuildable artifacts (1.6 GiB); the baseline cache has 1,250 descriptors.
- The intervening MVP-status answer made no implementation progress. Resumed
  the existing command handle instead of restarting it: session 34644 terminated
  at Clippy with fourteen redundant test-only dereferences; WASM did not run.
  Removed only those dereferences without suppressing the lint. Session 38923
  reruns offline Clippy/WASM and the twelve affected project tests.
- Complete SQLx preparation had passed in 2m03s: 33 new, two obsolete and zero
  modified descriptors, 1,281 total. Inspected both removed descriptors: the
  former separate project size query and the project stream projection before
  its private ID field. No migration, dependency or runtime policy change.
- Corrected gates pass: offline all-target server Clippy with denied warnings
  (2m12s), denied-warning WASM check (21.70s), and all twelve focused project
  tests (12.64s after 3m44s compilation). Session 38923 is terminal. The earlier
  full 964-test run remains the full regression evidence; only test borrowing
  syntax changed afterward. Formatting check passes with zero changes (3.913s).
- GitHub confirms #212 remains open/draft at `dc822a0`. A temporary cursor
  transport example is being used only for the next CSV research dependency;
  it must be removed from crate discovery before final publication checks.
- The isolated CSV transport probe confirms direct `query_as!(FETCH ...)`
  yields an untyped `PgRow` at compile time. Checked DECLARE plus a native-record
  helper passes online (2.18s compilation, 0.05s test) and offline (2.11s,
  0.06s), preserving exact types, source snapshot and fresh subsequent reads.
  The first helper build needed the repository's explicit chrono override and
  an unambiguous transaction trait call. The initial scratch-cache lookup failed:
  pinned SQLx reads its offline-directory override from dotenv metadata, unlike
  its live cache-output environment setting. A temporary crate-local cache
  proved the standard offline lookup without modifying workspace descriptors.
- Removed the prototype from crate discovery and its nine temporary local-cache
  descriptors; sources and generated metadata remain recoverable under
  `.scratch/cursor-probe/`. Only the disposable `permission_cursor_probe` schema
  was created. No production migration was added or real data changed. Research
  records the remaining batching, authorization, cancellation and HTTP gates.
- T109 is complete for this materialized project-export increment. The scoped
  requirement mapping remains five FR subsets plus SC-006 across three tasks;
  no local critical/high finding remains. Full-feature gates remain open.
- Final formatting passes with zero changes (4.45s); workspace SQLx cache remains
  1,281 descriptors and no probe files remain in application discovery. Stopped
  only the owned disposable PostgreSQL, preserving its data and scratch evidence.

Next: publish this verified increment to draft #212 without merging. Then
refine the CSV streaming contract/tasks from the validated
transport candidate and implement its current-authority integration.
Full policy/approval/UI/CSV/transition remains open.

## 2026-10-04 — Materialized export integration

- Previous goal turn made progress: T101–T103 published unsigned as `15c82ef`
  to draft #212. Revalidated clean worktree and matching origin; all previous
  test/check/publication commands are terminal. No merge occurred.
- Spec Kit Plan/Tasks retain existing artifacts and add T104–T106. Independent
  research confirms three manager-only materialized consumers and a second
  authorization boundary after rendering; no locks may follow rendering/body
  lifetime. Extended the existing contract without a new product predicate.
- Local constitution check passes. Requirements remain 12/16 with the existing
  independent-increment authorization; no hooks or context generator exist.
  No deployment database, reference-account mutation or schema change is needed.
- Confirmed the disposable server was stopped, then restarted its existing
  cluster on port 55416 with the recorded socket. RED reproduced revoked-manager
  disclosure (0.58s after 2m58s compilation), not a compilation/setup failure.
- Integrated only the three manager materializers and post-render release;
  retained trusted actor IDs, existing limits/deadlines and all CSV/Member
  behavior. Eight local tests and the real-cookie export matrix are implemented.
  Focused and HTTP checks are running in session 46239; adversarial review is
  inspecting the same working tree. No verification completion is claimed yet.
- Initial focused GREEN passes all eight export tests (13.52s after 4m25s
  compilation). The HTTP harness then failed on the fixture source constraint,
  independently identified in review: an invoice line lacked a time/fee source.
  Corrected it to reference its own time entry, without relaxing the constraint.
- Added the review's suggested final-check interruption test: a rendered body
  waiting for authorization must release both admission permits and its single
  pool connection after cancellation or timeout. No production change followed
  focused GREEN. The corrected HTTP and nine-test verification remain pending.
- Follow-up adversarial review confirms both findings are addressed, with no
  remaining blocker or material test gap. Scoped Spec Kit analysis maps five FR
  subsets and SC-006's regression subset to all three sequential tasks, with
  zero unmapped tasks, ambiguity, duplication or constitutional conflict. This
  does not close full-feature analysis or runtime cutover readiness.
- Session 44742 runs the corrected nine-test subset, real HTTP matrix and full
  server-binary regression sequentially; no new build was started while its
  predecessor remained active. PostgreSQL is still the disposable instance.
- The intervening MVP-status turn was a verified wait on session 44742, not an
  implementation increment. Resumed that live handle without restarting it.
  All nine focused tests pass (14.96s), as does the registered HTTP matrix
  (12.66s); the 963-test server-binary regression is still running.
- Revalidated Implement/Analyze prerequisites and constitution 1.1.0. The seven
  local checklists remain complete; general requirements remain 12/16 under the
  existing independent-increment authorization. No extension hooks apply.
- Session 44742 is terminal: full server-binary regression passes, 952 passed,
  zero failed and 11 existing exclusions, 963 discovered (333.10s). T104/T105
  are complete. No production/test change followed the passing run.
- Session 78435 runs complete SQLx regeneration after cleaning only the app's
  local build artifacts, followed by offline all-targets Clippy and web/WASM
  checks with warnings denied. The baseline cache contains 1,235 descriptors.
- Session 78435 is terminal: SQLx regeneration passes (1m52s), adding 15
  descriptors with none modified/deleted, 1,250 total. Offline all-targets
  Clippy (2m13s) and web/WASM check (22.21s) pass with warnings denied.
  GitHub confirms #212 remains open/draft on the same branch at `15c82ef`.
- Next-consumer research, independently checked against migrations 0035/0039,
  `projects::{insert_assignment,remove_assignment}`, project editor save/finalize,
  `db::lock_organization` and the project export readers: the existing manager
  prelude cannot certify Member scope. Relationship changes advance the parent
  project revision, not the organization revision. Locking only snapshot-visible
  projects misses winning scope expansion; locking all existing projects still
  misses a newly finalized assigned project. The next contract must cover both
  gains and losses, plus current access to every captured ID after rendering.
  `ProjectExportRow` currently carries no IDs. CSV remains separate because
  its transaction spans browser-paced sends. No new implementation or runtime
  policy decision is inferred from this research.
- Formatting passes with zero changes (4.274s); whitespace checks pass. The
  disposable PostgreSQL server stopped cleanly, preserving its data. T104–T106
  are complete; no full flake/browser, canonical activation or merge readiness
  is claimed. Only the app's rebuildable artifacts were removed during checks.

Next: publish this verified increment unsigned to draft #212 without merging,
then refine Member project-export freshness and response-release checks using
the recorded scope-expansion evidence before implementation.
CSV, Member scope and full canonical policy/approval/UI/transition remain open.

## 2026-10-04 — Invoice editor verification

- The intervening status turn was a verified wait: session 83018 confirmed all
  14 snapshot tests passed (12.63s) and the full server-binary regression was
  still running. Resumed that same live command without restarting tests.
- Revalidated published `22ffdab` and the existing scoped dirty worktree.
  Spec Kit Implement/Analyze prerequisites resolve the same feature; no hooks
  exist. Requirements remain 12/16; all seven local checklists remain 7/7.
  Prior authorization permits this closed increment, not full policy activation.
- RED reproduced the inherited READ ONLY editor failure. Both reader preludes
  now reuse the shared manager helper; mutation locking, DTOs, revision checks
  and financial calculations are untouched. Five new real-reader tests and the
  existing HTTP harness cover the editor boundary without production test hooks.
- Independent adversarial review found no blocker: both race orders, coherent
  metadata/revision/lines, stale reviews, cancellation/pool reuse, business-row
  preservation and real-cookie authorization are covered.
- Full server-binary regression passes: 943 passed, zero failed, 11 existing
  exclusions, 954 discovered (271.23s). It includes the extended HTTP matrix and
  existing invoice arithmetic/state fixtures. T101/T102 are complete.
- Scoped Spec Kit analysis maps five FR subsets (006/007/010/017/018) and the
  SC-006 regression subset to T101–T103: no unmapped task, ambiguity, duplication
  or constitutional conflict. Full feature analysis/readiness remains open.
- Complete SQLx regeneration passes (1m41s): seven new descriptors, none
  modified/deleted, 1,235 total. Offline all-targets Clippy (1m52s) and web/WASM
  check (21s) pass with warnings denied. Session 83691 is terminal. No production
  or test change followed the passing regression.
- Formatting passes with zero changes (3.668s); whitespace checks pass. The
  disposable PostgreSQL server stopped cleanly without deleting data. Nix
  commands used sandbox escalation, not a deployment database or Harvest data.
  T101–T103 are complete; full flake/browser and feature acceptance remain open.

Next: publish this verified increment unsigned to draft #212, then refine the
materialized-export integration. Source inspection confirms `reports::limits`
still discards current actor authority for manager-only entries/invoice/PDF
materialization. Preserve existing size/query deadlines, separate CSV streaming
and Member project-scope fencing, and retain session identity through the actual
Axum delivery paths. Full policy/UI/approval/transition gates remain open; no
merge or real-data mutation is authorized.

## 2026-10-03 — Invoice editor snapshot integration

- Previous goal turn made progress: T098–T100 published unsigned as `22ffdab`
  to open draft #212. Revalidated clean worktree and matching origin; all prior
  commands are terminal and the disposable PostgreSQL server is stopped.
- Spec Kit Plan/Tasks retain the existing artifacts and add sequential
  T101–T103. Independent read-only research confirms the same manager prelude
  fits editor load/review without changing their business checks or mutation
  callers. Extended the existing contract, not the policy or product scope.
- Local constitution check passes; the full checklist remains 12/16, with
  existing authorization to implement closed independent increments. Extension
  hooks and an agent-context generator are absent. No real-data change is needed.

Next: reproduce the editor's inherited READ ONLY failure, integrate the shared
prelude, prove both reader race orders/preservation/HTTP behavior and complete
cache/offline/lint/format/review gates before publishing to the same draft.
Full scoped policy, approvals, UI, migration and cross-surface acceptance remain
open; no merge or canonical activation is authorized by this increment.

## 2026-10-03 — Financial snapshot verification

- The intervening MVP response was status-only (no implementation progress).
  Revalidated the same dirty worktree at `5d51b0e` and the live disposable
  PostgreSQL instance on 55416; did not restart any build or database.
- Resumed T098–T100 with Spec Kit Implement and the existing authorization for
  independent closed increments. General readiness remains 12/16; the other
  seven local checklists pass. No extension hooks are present.
- The prior RED run reproduced invoice preparation accepting an absent actor.
  Fixed the subsequent compilation error: SQLx SELECT maps use `fetch_one`, not
  `execute`. Connected the pending settings/retry tests.
- Focused GREEN passes all eight financial snapshot tests (5.45s after 2m34s
  compilation). Both actual readers cover current same-tenant actors, both
  revocation orders, legacy updates without revision changes, fresh-revision
  retries, consistent fee values, cancellation/pool reuse, inherited settings,
  exactly three serialization attempts and no retry for unrelated errors.
- Added registered-route HTTP coverage for session-derived identity, forged
  actor/organization fields, missing/member/inactive sessions and foreign
  resources. This new HTTP coverage has not yet been run.
- Independent adversarial review found no blocker and suggested timeout
  restoration/enforcement coverage. Added all three inherited isolation levels
  crossed with 0/250ms/10s timeouts, plus held-gate timeout and pool reuse through
  both readers. The follow-up review finds both suggestions addressed.
- The first full-regression compile caught duplicate SQL output names in the new
  settings fixture. Added explicit distinct aliases and reran only after the
  failed command terminated. Final regression is in progress.
- Scoped Spec Kit analysis covers five FR subsets (006/007/010/017/018), three
  sequential tasks and their executable scenarios: 100% local task mapping,
  zero unmapped tasks, ambiguities, duplications or constitutional conflicts.
  This does not close full SC-001–009 or the general readiness checklist.
- First full run: 937 passed, one failed, 11 existing exclusions (252.73s).
  All nine new snapshot tests and the HTTP matrix passed. The failing existing
  invoice-transition fixture attempted its preservation read with the actor it
  had just revoked. It now asserts that denial and uses a separate active
  same-tenant administrator to prove the invoice balance remains reserved,
  without restoring the revoked actor. Focused and full reruns are in progress.
- Offline web/WASM check with warnings denied passed (43.95s). Nix formatting
  adjusted only new/changed Rust tests; final zero-change gate remains required
  after the fixture correction. T098/T099 are complete; T100 remains open.
- The corrected invoice-transition test passes (1.15s). Final full server-binary
  regression passes: 938 passed, zero failed, 11 pre-existing exclusions, 949
  discovered (258.34s). This includes all nine snapshot cases, registered HTTP
  delivery and existing financial/import/export regressions. Complete SQLx
  regeneration and fresh offline all-targets Clippy are now running.
- Complete SQLx regeneration passed (1m18s): 22 new descriptors, zero existing
  modifications/deletions, 1,228 total. Offline Clippy requested the equivalent
  `is_none_or` predicate for non-serialization errors; applied its simplification
  without warning suppression. Rerunning offline Clippy and the nine focused
  snapshot tests after this final boolean refactor. SQL text/cache is unchanged.
- Final offline all-targets Clippy passes with warnings denied (1m41s); all nine
  snapshot tests pass after the boolean simplification (8.19s after 2m53s
  compilation). Full regression/HTTP passed before that equivalent refactor;
  no further production or SQL change followed. T098–T100 are complete. Full
  flake/browser acceptance and complete canonical policy remain open.
- Disposable PostgreSQL stopped cleanly; no data was removed. Nix formatting
  passes with zero changes (3.373s), and staged/unstaged whitespace checks pass.
  This increment is ready for unsigned publication to existing draft #212;
  no critical/high local review finding remains.

Next after publication: refine and integrate the same current-authority boundary
in invoice editor load/review.
Materialized exports follow; CSV needs a separate non-client-blocking contract,
and Member exports need a legacy relationship fence. No merge, canonical
activation, real-data mutation or full-feature acceptance is authorized by this
increment. The full implementation goal remains active.

## 2026-10-03 — Financial snapshot reauthorization

- Previous goal turn made concrete progress: T095–T097 published unsigned as
  `5d51b0e` to draft #212. Revalidated clean worktree and matching origin; all
  preceding commands are terminal. No merge or policy activation occurred.
- Spec Kit Plan/Tasks refine T098–T100 for production fee balances and invoice
  preparation. Independent research found that organization locking alone does
  not refresh a snapshot after legacy actor changes; actor SHARE and complete
  prelude retry are also required. Official PostgreSQL sources and rejected
  alternatives are recorded in research and the new manager-snapshot contract.
- Shared CSV configuration is deliberately unchanged: streaming waits on client
  backpressure and cannot retain these revocation locks. Member scope changes
  and editor load/review are distinct integration obligations, not waived.
- Local contract review found no blocker; retain prior/stricter lock timeouts,
  map missing organizations to non-disclosing denial and stop on rollback
  failure. General readiness remains 12/16; authorized independent increments
  proceed without declaring full implementation readiness. No hooks or
  agent-context generator exist.
- The disposable database's first restart terminated because it omitted its
  custom socket/port options. Confirmed the log and terminal process, then
  restarted the same cluster with its recorded port 55416/socket; it is running.
  No data reset or other server interruption occurred.

Next: reproduce denial failure, implement the shared snapshot prelude, exercise
both production consumers and races, then verify SQLx/offline/format/review before
publishing. Full scoped policy, approvals, UI and migration remain open.

## 2026-10-03 — Authenticated own-permission explanation

- The intervening MVP-status turn added no implementation; its collection of
  terminal test output confirmed seven reader tests and the registered HTTP
  matrix passed. Revalidated the existing dirty worktree at published `c0cfb8f`;
  resumed the same T095–T097 increment, without restarting completed tests.
- Spec Kit Plan/Tasks reuse the strict loader and existing relations under
  `contracts/own-permissions.md`. Requirement readiness remains 12/16; the seven
  independent checklists pass. Prior authorization permits closed increments,
  not full cutover. No extension hooks are present.
- Added a no-argument session-owned endpoint and separate shared display DTO.
  READ COMMITTED plus organization SHARE then active actor SHARE prevents stale
  reads across winning revocation. Legacy mode does not inspect staged grants;
  unsupported/malformed state gets a fixed HTTP error. No schema, dependency,
  UI/CSS, legacy guard or real-data change is included.
- RED failed on the legacy None behavior against the unavailable stub. Initial
  GREEN: seven reader tests passed in 3.71s; registered HTTP matrix passed in
  9.11s. Independent adversarial review found no blocker. Added its suggested
  two-project ordering and unsupported-catalog HTTP error cases before full
  regression, cache and offline checks.

Next: finish regression, SQLx regeneration, offline server/web and lint/format
checks, then publish unsigned to draft #212 without merging. Full runtime
enforcement, UI, approvals, migration review and acceptance remain open.

- Full server-binary regression after review additions: 929 passed, zero failed,
  11 pre-existing exclusions, 940 discovered, 202.37s. The registered HTTP matrix
  includes the unsupported catalog and valid-restored-state checks. T095/T096
  are complete; T097 verification remains in progress.

- Initial WASM build passed in 48.66s with one dead-code warning for the pending
  UI consumer. Added a non-server-only `expect(dead_code)` on that DTO, linked
  to T018, rather than suppressing warnings globally or inventing a UI consumer.
  The final web build treats warnings as errors. First Nix format adjusted four
  Rust files; no unrelated file changed.

- Scoped Spec Kit analysis maps all six local FRs to T095–T097 and their test
  scenarios with no ambiguity, duplication, unmapped task or constitutional
  conflict. Full SC-001–009 acceptance remains open. GitHub confirms #212 remains
  open/draft at the expected published branch head.

- Complete SQLx regeneration passes in 51.20s: 26 added descriptors, zero
  existing descriptors modified/deleted, 1,206 total. Fresh offline all-targets
  server Clippy passes with warnings denied in 59.74s. Offline WASM build with
  warnings denied passes in 41.00s. PostgreSQL has stopped cleanly; no data was
  removed. Full flake/browser acceptance is not claimed for this read-only slice.

- Nix formatting passes with zero changes (3.224s). T095–T097 are complete and
  ready for unsigned publication to the existing draft; no critical/high local
  review finding remains. The full implementation goal remains active.

Next after publication: resume T006/T042's operation integration gates, including
snapshot-based report/fee/invoice readers in `contracts/permission-state.md`.
Their read-only REPEATABLE READ transactions cannot accept a row-lock gate
unchanged; preserve snapshot consistency and verify winning revocation/retry
against production helpers before guard replacement. Do not infer pending
historical-job, inactive-manager or lifecycle answers from this increment.

## 2026-10-03 — Approved rate-field policy

- The preceding turn was a status report, not implementation progress. Rechecked
  clean worktree at published `e949e4c` and resumed existing feature/branch.
- Spec Kit Plan/Tasks preserve existing artifacts and add sequential T092–T094
  for approved FR-021/022, with no new product decision. Requirements checklist
  remains 12/16; existing authorization permits this closed pure increment, not
  full activation. No extension hooks are present.
- New pure financial gates distinguish person, project and global task fields;
  costs use separate explicit grants. Reuse the catalog and record-scope checks,
  add no dependency, schema, runtime guard, UI/CSS or real-data change.
- RED produced five expected behavioral failures. Initial GREEN passed all 168
  core tests and warnings-denied Clippy. Independent adversarial design and code
  review found no blocker; added its suggested read-prerequisite revocation test.
- Scoped cross-artifact analysis maps all three tasks to FR-006/008/010/017/021/022
  and profile defaults to FR-001, with no local ambiguity, duplication, unmapped
  task or constitutional conflict. Full T006/T014/T015/T019/SC acceptance remains
  open; test coverage of a financial gate is not complete operation authorization.
- Fresh Harvest research identifies the archived-client prerequisite for project
  restoration, recorded in research; custom lifecycle grants remain unresolved.
  No account mutation or browser test was performed.

Next: final core tests/lint/format, then publish unsigned to existing draft #212
without merging. Continue the unresolved operation/consumer integration gates;
do not repeat pending historical-job or inactive-manager questions or infer
answers from this independent financial policy. The goal remains active.

- Final core regression after the review addition: 169 passed, zero failed or
  ignored, 0.02 seconds. All-targets core Clippy passed with warnings denied in
  0.62 seconds. T092/T093 are complete. Nix formatting adjusted only the two new
  Rust files; final zero-change check remains for T094. GitHub confirms #212 is
  still open/draft at the prior published commit on the expected branch.

- Final Nix formatting passes with zero changes in 2.515 seconds; staged and
  unstaged whitespace checks pass. T092–T094 are complete and ready for unsigned
  publication on the existing draft. No critical/high local review finding
  remains. Full server/browser/flake acceptance is not claimed for this pure
  increment; no SQL changes require cache regeneration and no database ran.

Next after publication: resolve the remaining operation predicates and connect
the approved field gates through the reviewed consumer/cutover contract. Keep
the person/project distinction when selecting actual payloads; do not classify
raw person history as an inherited project projection. Full six-profile runtime,
scoped approvals, UI, reviewed migration and cross-surface acceptance remain open.

## 2026-10-03 — Interrupted import session disposal

- Previous goal turn made concrete progress: T086–T088 published unsigned as
  `482b7c5` to draft #212. HEAD and origin match and the worktree is clean; all
  prior process handles are terminal and the owned test database is stopped.
- Spec Kit Plan/Tasks reuse the current feature and existing authorization for
  closed increments. Independent read-only research traced the recorded missing
  savepoint to pinned SQLx's server-response/bookkeeping cancellation window.
  API and CSV both use the same disposal path; no separate policy is selected.
- T089–T091 specify a deterministic divergent-savepoint fixture, narrow pending
  error recovery, explicit full rollback before unlock, and preserved failures.
  No schema or data model changes are needed. No extension hooks or agent-context
  generator are present; full readiness remains incomplete, not waived.

Next: run the cleanup fixture RED, implement shared disposal, verify real adapter
cancellation/regression and offline gates, then publish without merging. Worker
execution/retry/unknown-requester policy and full scope remain open.

- RED reproduces the exact missing `_sqlx_savepoint_1` error at shared cleanup
  (one failed test, 0.34 seconds). The controlled fixture constructs lost
  acknowledgement, not scheduler timing. Shared cleanup now drains only stale
  savepoint rollback errors, explicitly rolls back and then unlocks/closes.

- Added untracked-BEGIN and backend-failure fixtures. Focused verification and
  independent adversarial review are in progress. Analysis maps all three tasks
  to the stated FR-007/010 prerequisites, FR-017/018 preservation and SC-006
  import subset, with no local ambiguity, duplication or constitutional conflict.

- Focused cleanup tests pass: three passed, zero failed, 1.07 seconds.
  Adversarial review finds no critical/high production issue; it identifies a
  fixture race because backend termination without a timeout confirms only
  signaling. The fixture now waits up to five seconds for actual termination
  and bounds cleanup separately. Corrected documentation distinguishes its
  precommitted organization data from actual batches in adapter regressions.
  Full server verification includes these corrections.

- Full server-binary verification passes after the fixture correction: 922
  passed, zero failed, 11 pre-existing exclusions, 933 discovered, 189.83 seconds.
  Actual API/CSV cancellation, committed-page/batch preservation, preview and
  producer-join regressions pass. T089/T090 are complete; SQLx/offline Clippy and
  final formatting remain for T091. The pending policy questions are unchanged.

- Complete SQLx regeneration passes in 51.47 seconds: seven added descriptors,
  no existing cache changes or deletions. Fresh offline all-targets Clippy passes
  with warnings denied in 59.11 seconds. Initial Nix formatting changes Rust
  layout and Markdown spacing only; final zero-change verification follows.
  The owned disposable PostgreSQL cluster is stopped. PR #212 remains OPEN/DRAFT
  on the authorized branch; no merge or live-data operation was performed.

- Final Nix formatting passes with zero changes in 2.092 seconds. T089–T091 are
  complete, with the review's fixture race corrected and no critical/high
  finding outstanding in this increment. Publish unsigned to the existing draft.
  No full-flake, browser or complete permission-feature acceptance is claimed.

Next after publication: resume T042's worker authority/execution contract. The
recorded missing-savepoint cleanup issue is now reproduced and repaired, not
an open blocker. Historical unknown-author handling, retry delegation and
permission-restoration behavior still require the explicit execution contract.
Do not repeat pending questions or infer answers; continue independent full-scope
work where safe. Full six-profile integration, scoped approvals, permission UI,
migration review and cross-surface acceptance remain required.

## 2026-10-03 — Original import requester provenance

- Previous goal turn made concrete progress: `e5fcc5a` published T083–T085 to
  existing draft #212; revalidated the clean worktree. No live test handles remain
  from that increment and its disposable PostgreSQL cluster is stopped.

- Spec Kit Plan/Tasks reused the existing artifacts. Read-only independent
  research confirms only the two authorized session commands enqueue production
  imports; the pool helpers are test-only. No production user deletion exists.
  Tenant-bound NO ACTION matches existing audit references without cascading jobs
  or erasing provenance. No historical actor can be reconstructed safely.

- T086–T088 define a closed, non-authorizing storage increment: record the original
  actor on insertion only, retain attribution across duplicates and retries,
  preserve unknown NULL and external DTOs. Historical authorization/retry policy
  remains pending, not silently resolved by storing a UUID. No real-data migration
  or new grant activation is authorized by this work.

- RED reproduced missing attribution through the real API command before schema
  or insertion changes. Migration 0045 now adds only the nullable composite FK;
  production enqueue functions require the current actor, with optional private
  helpers reserved for historical test fixtures. Applied only to the owned
  disposable database. Retry, claim, lease, worker and DTOs remain unchanged.

- Initial focused verification passes: 22 tests, zero failed, one existing stress
  exclusion, 17.46 seconds. Independent review found no critical/high code issue,
  but the CSV HTTP fixture had discarded forged actor fields. It now sends them
  as query parameters; API sends them in JSON. Added a real blocked concurrent
  duplicate check and retained complete-row rollback/conflict assertions.

Next: finish full server verification, scoped analysis, SQLx/offline Clippy and
formatting; publish without merging. Historical execution/retry policy is pending.

- Continuation audit: the intervening status turn yielded new terminal evidence
  from the original test handle (not a restarted run): 918 passed, one failed,
  11 existing exclusions in 188.89 seconds. The old report-upgrade snapshot
  included the newly added NULL field only after migration. Its comparison now
  retains every old metadata assertion and checks unknown requester provenance
  separately. No production behavior or migration is relaxed. Re-running the
  full server suite is the next verification step.

- After the preservation assertion correction, the full server-binary run passes:
  919 passed, zero failed, 11 pre-existing exclusions, 930 discovered, 195.85
  seconds. T086/T087 are verified; T088 still needs regenerated SQLx, offline
  all-targets Clippy and formatting. PR #212 is confirmed OPEN/DRAFT at the
  existing published head; no merge.

- Scoped Spec Kit Analyze maps FR-007/010 provenance prerequisites, FR-017/018
  preservation/verification and the import subset of SC-006 to all three tasks.
  No local ambiguity, duplication or constitutional conflict; no unmapped task.
  Full-feature requirements remain 12/16 and existing approval permits only
  closed independent increments. No extension hooks are configured.

- Complete SQLx regeneration passes in 57.02 seconds; four changed-query
  descriptors replace their predecessors and nine test queries are added. No
  unrelated descriptor is removed. Fresh offline all-targets Clippy passes with
  warnings denied in 66 seconds. The initial formatting pass changed only
  Markdown spacing in this log; the zero-change verification follows.

- Final formatting passes with zero changes in 2.908 seconds, and the owned
  disposable PostgreSQL cluster is stopped. T086–T088 are complete. Publish the
  verified increment unsigned to the existing draft PR; no browser/full-flake
  or full-feature acceptance is claimed.

Next after publication: resume T042's worker authority contract and integration,
including the recorded cancellation/savepoint observation. Unknown historical
requesters, retry delegation and permission restoration still need their explicit
execution contract; provenance alone grants nothing. Preserve the six-profile,
scoped approval, permission UI and cross-surface scope. Do not mark the feature
or the MVP complete from this increment, merge #212 or migrate real data.

## 2026-10-03 — Worker authority and bounded CSV preparation

- Previous goal turn made concrete progress: `b4672a4` published T080–T082 to
  draft #212. Revalidated the clean existing worktree; no merge.

- Ran Spec Kit Plan/Clarify/Tasks setup against feature 015 without replacing
  existing artifacts. No hooks or agent-context generator are configured.
  Requirements remain 12/16, seven local checklists remain 7/7. Existing approval
  permits independent closed increments, not full runtime activation.

- Independent read-only research confirms requester identity is discarded by
  enqueue/retry; lease ownership and connection generation cannot replace it.
  Late authorization at checkpoint would invert organization/project order.
  All production import execution is durable; API batches are already prepared
  before SQL, while CSV still waits on its parser inside the batch transaction.

- Asked one unresolved migration question: retain/hold unknown-requester jobs for
  explicit new Administrator authorization (recommended), or require the old
  pending queue drained/cancelled before activation. No answer is assumed and
  no historical identity, state or data is changed. Different-person retry and
  permission-restoration behavior also need an explicit execution contract.

- T083–T085 are the independent CSV transaction prerequisite. Keep the same
  checkpoint boundaries and test-only whole-run adapter. No worker authority,
  requester schema, legacy transition or six-profile cutover is implemented here.

- The intervening MVP status answer made no implementation progress. Revalidated
  the existing worktree and polled the original test handle to completion rather
  than restarting it. The first run failed during cancellation with a missing
  SQLx savepoint; retain that observation for worker cancellation review. Moving
  the unchanged cancellation assertion after the transaction assertion exposes
  the intended RED: Commit after zero checkpointed rows spans a parser wait.

- T084 now prepares the first and subsequent durable batches before opening SQL;
  it stops immediately at absolute record 500 multiples or Complete. No partial
  prepared batch is applied after parser failure. The unleased test adapter still
  reads incrementally inside its whole-run transaction.

- Independent read-only review found no critical/high/medium correctness issue
  in this local diff. Verification is still running; no full-feature claim.

- First post-change CSV run: 14 passed, three failed, four existing exclusions.
  Fixed a globally unique email collision in the new fixture. Two existing
  tests depended on one-row SQL before EOF; job-row publication barriers now
  preserve applied-write cancellation/expiry checks with prepared batches.
  Added incomplete-input recovery in both modes before/after a checkpoint.

- Expanded CSV verification passes: 18 passed, zero failed, four existing
  exclusions, 9.30 seconds. Follow-up review identified a fixture race between
  old rollback and SKIP LOCKED reclaim; wait for old execution cleanup before
  claiming replacement. Separate token-reclaim tests retain concurrent coverage.

- Scoped Spec Kit Analyze: four FRs and the import portion of SC-006 map to all
  three tasks; no unmapped task, local ambiguity, duplication or constitutional
  conflict. No hooks configured. Full feature gates remain open.

- Full server-binary regression after the fixture-order correction passes:
  914 passed, zero failed, 11 pre-existing exclusions, 925 discovered, 174.14
  seconds. This includes both new tests and the adapted cancellation/expiry
  cases. Initial Nix formatting changed the test's layout and Markdown only.

- Complete SQLx regeneration passes in 44.08 seconds: one added test query,
  no existing cache changes/deletions. Fresh offline all-targets Clippy passes
  with warnings denied in 51.62 seconds. Package cleaning removed only 165
  regenerable build files (1.5 GiB), not source or business data. GitHub confirms
  #212 remains open/draft on the existing branch at the published baseline.

- Nix formatting passes with zero changes in 2.752 seconds, and the owned
  PostgreSQL cluster is stopped. T083–T085 are complete; publish unsigned to
  existing draft #212 without merging. The reviewed fixture race is corrected;
  no critical/high finding remains in this bounded increment. No full Nix,
  browser or full-feature acceptance is claimed.

Next after publication: resume worker provenance and current-authority contracts
under T042. The historical-requester decision remains pending; do not infer it or
repeat the question. Preserve the full six-profile, scoped-approval, cross-surface,
permission UI and migration scope. The earlier cancellation/savepoint observation
also remains evidence to investigate during worker cleanup review.

## 2026-10-03 — Bounded import result downloads

- Previous goal turn made concrete progress: `4fac6af` published T077–T079 to
  draft #212. Revalidated a clean worktree synchronized with origin; no merge.

- Spec Kit Tasks/Implement reuse feature 015 and its existing read protocol.
  Requirements remain 12/16 and seven local checklists pass 7/7. The existing
  authorization permits this closed independent increment, not full cutover.

- Traced the production route and all fragment/body callers. HTTP admission
  checks the Administrator once; subsequent pages and captured tail have no
  actor identity. Preserve the 16-fragment boundary and snapshot/retention tests
  while sharing the current importer transaction guard.

- T080–T082 define the closed local contract and ordered tests/implementation
  work. No schema, real-data, worker identity or new-profile activation change.

- RED reproduced two actual failures: revoked preparation returned 200 and a
  revoked body released its captured tail. Shared short authorization transactions
  now protect preparation, every archive page and the inline/empty tail.

- The intervening TLDR/MVP answers made no implementation progress. Revalidated
  the existing worktree at `4fac6af` and polled the original test handle to its
  terminal result: eight passed, one failed. No duplicate run was started while
  that handle was live.

- The remaining failure was a fixture error: job status prefers the checkpoint
  report, so changing only the final report did not exercise malformed-report
  rollback. Clear the disposable checkpoint in that case; do not weaken parsing.

- Added registered HTTP revocation after successful response preparation and
  before body consumption. Require interrupted transfer rather than successful
  truncated EOF, subsequent 403 for that session, and unchanged bytes for a
  different authorized reader. Preserve the existing CLI exercise and headers.

Next: finish the full server-binary run, review requirement coverage, regenerate
SQLx, run offline all-targets Clippy/formatting, then publish without merging.

- Scoped Spec Kit Analyze completed read-only for T080–T082: four functional
  requirements (FR-007/010/017/018) and the local download portions of
  SC-002/003/006 have task coverage; three tasks, none unmapped, no duplication,
  ambiguity or constitutional conflict found in this closed increment. This
  does not satisfy those outcomes across the whole feature. No hooks exist.

- GitHub confirms #212 remains open/draft on `feat/scoped-permissions` at the
  published baseline `4fac6af`; do not merge it.

- Full server-binary verification passes: 912 passed, zero failed, 11 pre-existing
  exclusions, 923 discovered, 176.57 seconds. T080/T081 are complete; T082 still
  needs cache, offline lint and final formatting. Initial formatting changed
  four Rust files and Markdown spacing only; rerun after final evidence edits.

- Complete SQLx regeneration passes in 43.26 seconds: three added test-query
  descriptions, no existing cache changes/deletions. Fresh offline all-targets
  Clippy passes with warnings denied in 52.05 seconds. Package cleaning removed
  165 regenerable build files (1.5 GiB), not source or data. Final formatting and
  unsigned publication remain next; full feature acceptance remains open.

- Nix formatting passes with zero changes in 2.139 seconds after applying the
  formatter's Rust/Markdown changes. The owned disposable PostgreSQL cluster is
  stopped. T080–T082 are complete; publish this verified increment unsigned to
  existing draft #212 without merging.

Next after publication: resume T042's bounded worker-execution/service-authority
inventory and bind remaining operations to the final matrix before policy
activation. Download protection does not close worker execution, full T006–T009,
six-profile integration, scoped approvals, permission UI or migration acceptance.

## 2026-10-03 — Import job command authority

- The intervening agency-MVP answer made no implementation progress. Revalidated
  the existing uncommitted importer changes at `d7a5a21`; the prior formatter
  handle is terminal/missing, and the owned PostgreSQL instance is running.
  Continued verification without restarting or duplicating live work.
- Previous goal turn made concrete progress: `d7a5a21` is published and the
  current worktree is clean/synchronized. Connection-management authority tests
  pass; full permissions and service execution remain incomplete.
- Spec Kit Tasks/Implement reuse feature 015. Requirements remain 12/16; seven
  local checklists pass 7/7. Prior authorization permits independent closed
  increments without claiming full policy activation or full-feature Analyze.
- Traced four production mutation callers, all in authenticated importer server
  functions. Other queue mutation calls are fixtures, not worker impersonation.
  Extract transaction-accepting queue operations and keep test-only pool adapters;
  production server helpers must authorize, mutate and load the returned status
  on the same connection. Include standalone status/history in this boundary.
- CSV body buffering remains outside locks; duplicate-request lookup and header
  validation follow fresh authorization. Preserve cancellation semantics,
  idempotency, generation fencing, retention and queue acceptance during imports.

Next: T077 reproduces revoked mutation/status access against extracted production
helpers; T078 applies organization/actor locking and transactional queue helpers;
T079 verifies races, HTTP/CLI regressions, SQLx, offline Clippy and formatting.
No schema, real-data, worker identity or new-profile activation change is included.

- RED: both initial production-helper tests failed with successful API/duplicate
  CSV acceptance after demotion. The extracted helpers were wired to the actual
  registered server functions before the guard was implemented.

- GREEN initial importer suite: 11 passed, zero failed, one pre-existing stress
  exclusion in 10.78 seconds. Includes the HTTP fixture that demotes the actor
  when the CSV body is polled after admission; the final request is forbidden and
  queues nothing. Existing HTTP/remote CLI flows still pass.

- Refactored the same queue SQL into caller-owned transactions; legacy pool
  enqueue/retry adapters are now test-only. Status/history use executor reads
  within the shared organization/current actor authorization transaction. New
  concurrency/rollback/foreign/single-connection cases are running next.

- Expanded importer verification passed 17 tests, zero failures, one existing
  stress exclusion in 14.28 seconds. Added malformed-CSV denial and a ninth
  command test for queue acceptance while the import reservation is held. Full
  server-binary regressions are running against disposable databases. The
  requirement/test map and scoped adversarial self-review are recorded in
  `quickstart.md` and `research.md`; no complete-feature review is claimed.

Next: finish server regressions, regenerate the complete SQLx cache, run offline
all-targets Clippy and formatting, then publish the verified increment without
merging. Execution-time authority and report-download authorization remain open.

- Full server-binary regressions pass: 906 passed, zero failed, 11 pre-existing
  exclusions, 917 discovered, 172.52 seconds. This includes all nine command
  tests and the registered HTTP/CLI regression. T077/T078 are complete.

- Scoped Spec Kit Analyze maps FR-007/010/017/018 to T077–T079: four requirements
  with tasks, no unmapped local tasks, ambiguity, duplication or constitutional
  conflict found. This is not a complete-feature analysis; T006–T009/T042 remain
  open. No extension hooks are configured.

- Regenerating the complete SQLx cache after clearing only package build outputs;
  offline all-targets Clippy and final formatting remain to be verified.

- Complete SQLx regeneration passes in 43.97 seconds, with four new test-query
  descriptions and no existing cache changes/deletions. Fresh offline all-targets
  Clippy passes with warnings denied in 51.89 seconds. Package cleaning removed
  165 regenerable build files (1.5 GiB), not source or data.

- The first formatting run overlapped SQLx cache regeneration and warned about
  temporarily absent descriptions; it also inserted two Markdown blank lines.
  The cache is now complete. Repeat formatting only after regeneration and the
  final evidence edits, rather than interpreting this run as a final gate.

- GitHub confirms #212 is open/draft on this branch at `d7a5a21`. No unresolved
  critical/high finding remains in the scoped self-review. Full policy activation,
  independent full-feature review, browser and full-flake acceptance remain open.

Next: finish formatting, stop the owned test cluster and publish this verified
increment. The next traced boundary is `jobs::report::{download,download_body}`:
admission checks the Administrator once, but subsequent archive pages and the
captured inline tail carry no actor identity. Refine its bounded-read contract
and tests without retaining a transaction across client-paced streaming.

- Nix formatting passes with zero changes in 2.076 seconds after applying
  Markdown spacing. The owned test cluster is stopped; T077–T079 are complete.
  Publish the unsigned increment to existing draft #212 without merging.

## 2026-10-03 — Harvest connection transaction authority

- The preceding MVP-status turn made no implementation progress. Revalidated
  clean `feat/scoped-permissions` synchronized at published `907bc88`; no merge.
- Spec Kit Tasks/Implement reuse feature 015 and the existing T042 credential
  inventory. Requirements remain 12/16; seven local checklists pass 7/7 each.
  Prior authorization permits closed independent increments, not policy cutover.
- Traced all three writers: OAuth completion checks the actor after HTTP but
  outside storage; disconnect/change receive no actor. All reserve an import
  connection nonblockingly before generation/credential writes. Add current
  tenant-bound Administrator checks inside those transactions, before generation.

Next: T074 reproduces revoked authority against production credential commands
in disposable PostgreSQL; T075 wires the shared transaction check and safe error
mapping, then T076 verifies regression/cache/lint/format results. No real accounts,
schema, grant mapping or runtime policy activation changes are authorized here.

- RED: both initial production-writer tests failed behaviorally: a Member could
  replace credentials and a disconnected Member could advance the connection
  revision. Actor parameters were wired before adding the transaction guard.
- The guard now reuses the organization SHARE helper and locks the current
  active Administrator before generation access. Callback/server-function error
  mappings return only the safe forbidden message, including contextual errors.
- First focused run: 20/21 passed, including all revocation/writer-first cases.
  The rollback fixture failed when adding its temporary constraint on a later
  iteration because an earlier successful revision already exceeded the bound.
  Added NOT VALID so the fixture constrains new writes without rejecting prior
  rows. The expanded Harvest suite is running; no final GREEN claim yet.

Next: confirm the corrected rollback, first-connect and error-projection cases,
run full affected regressions and complete SQLx/offline lint/format verification.

- Corrected Harvest-filtered suite passes: 204 passed, zero failed, 8 existing
  scale-test exclusions in 34.52 seconds. All seven new database authority cases
  pass alongside the existing switch/version/history/import/refresh regressions.
  Full server-binary tests are running after Rust formatting. No merge or real
  account mutation occurred; full feature activation remains pending.

- Full server binary regressions pass after formatting: 897 passed, zero failed,
  11 pre-existing exclusions, 908 discovered, 164.51 seconds. Complete SQLx
  regeneration/fresh offline Clippy are running after cleaning only this package's
  regenerable build artifacts. The removed post-HTTP EXISTS query has one obsolete
  description (`3e9078ea…`); verify that no other cache entries disappear.

- Scoped Spec Kit consistency analysis maps all four relevant FRs to T074–T076;
  no unmapped local task, conflicting policy or constitutional exception found.
  This is not complete-feature Analyze; T006–T009/T042 remain open. Focused
  self-review records lock/FK compatibility and separate service authority in
  `research.md`; no independent full-feature review is claimed.

Next: inspect regenerated metadata, finish offline Clippy and Nix formatting,
stop the owned test cluster and publish the unsigned commit to #212 without merge.

- Complete nonincremental SQLx regeneration passes: eight new test-query
  descriptions; only the expected obsolete `3e9078ea…` description is removed,
  with no other existing cache changes. Fresh offline all-targets Clippy passes
  with warnings denied in 50.24 seconds. Cleaning removed 699 regenerable build
  files (6.1 GiB), not source or data. Initial Nix formatting inserted one blank
  line in this log; repeat the final check after this update.
- GitHub confirms #212 open/draft on `feat/scoped-permissions` at `907bc88`
  before publication. All seven database authority cases, two safe error-mapping
  tests and existing affected regressions pass. No unresolved critical/high
  finding remains in the focused self-review. This does not establish independent
  full-feature review, policy activation, browser acceptance or complete T042.

Next: finish final formatting, commit unsigned and publish this verified family.
Then continue T006/T042 for actor-aware import submission/cancellation/retry and
bounded execution: current `jobs::{enqueue_api,enqueue_csv,cancel,retry}` still
receive organization but no authenticated actor. Reconcile their service callers
and generation/job/FK ordering before changing the shared helpers. The pending
person-management lifecycle clarification remains separate; do not ask it again
or treat this implementation as approval to infer the answer.

- Final Nix formatting passes with zero changes in 2.214 seconds; staged and
  unstaged whitespace checks pass. The owned PostgreSQL cluster is confirmed
  stopped. T074–T076 are complete for this family only. Publish after one final
  format check of these completion markers; keep #212 draft and do not merge.

## 2026-10-03 — Branding transaction authority resumed

- The preceding MVP-status turn made no implementation progress. Revalidated
  clean `feat/scoped-permissions` synchronized at published `3ae8e08`; no merge.
- Spec Kit Clarify found a concrete contract overrestriction: dedicated Harvest
  time/expense guides permit named Administrator corrections and deletion without
  general unlocking. Corrected `company-locks.md`, preserving unverified custom
  grants, company-lock combinations and financial/deletion effects as open.
  No new user decision is inferred; the prior person-management question remains
  unanswered. Requirements remain 12/16, with no changed checklist markers.
- Spec Kit Tasks/Implement reused feature 015. T071–T073 define a closed branding
  reauthorization repair: current code takes only an organization ID after the
  wrapper's admission check. No-op responses can also disclose stale-authority
  branding. Rust/testing/async/simplicity guidance keeps the fix in its existing
  transaction, without new dependencies, UI or policy activation.

Next: reproduce the branding revocation failure against disposable PostgreSQL,
then reauthorize after the organization lock and verify both change/no-op paths.

- RED: both new tests failed against the original helper: a Member and a Manager
  demoted while waiting each received a successful unchanged-branding response.
- GREEN: all ten organization tests pass, including five new authority/race/
  rollback cases. READ COMMITTED is explicit; the waiting-client fixture defaults
  to REPEATABLE READ. Actor-only waits and a writer-first table-lock barrier verify
  rechecking and retention through commit without sleeps. Focused review replaced
  a NOWAIT rollback probe with a bounded wait because SQLx drop queues rollback.

Next: run full server regressions after that test hardening, regenerate complete
SQLx metadata, check offline Clippy/formatting, then publish without merging.

- The full server binary suite passes after rollback-test hardening: 888 passed,
  zero failed, 11 pre-existing ignored in 180.24 seconds. All 161 core tests pass.
  Formatting CI passes with zero changes. Complete SQLx regeneration and fresh
  offline all-targets Clippy are running; no separate integration binary, browser
  or full-flake result is claimed.

- Complete SQLx regeneration adds three test-query descriptions with no existing
  changes/deletions. Fresh offline all-targets Clippy passes with warnings denied
  in 50.43 seconds. Cleaning removed 1.5 GiB of regenerable package artifacts only.
  GitHub confirms #212 open/draft at `3ae8e08` before this publication. No merge,
  schema, real-data, UI or policy activation change is included.

- T071–T073 close the branding writer's current-authority boundary, not complete
  OP27 or T042. The sole wrapper, role equivalence, error/no-op disclosure, lock
  lifetime, rollback and event ordering were self-reviewed; no critical/high
  local finding remains. No independent full-feature review is claimed. The owned
  test cluster is stopped. Final formatting inserted one missing Markdown blank
  line in this log; the corrected document is checked again before committing.

Next: publish the verified repair to #212 without merging, then continue the
remaining T006 operation contracts and T042 credential/
import/entry integration. The person-management lifecycle answer remains pending;
do not repeat or infer it from automatic continuation. Full implementation remains
active and incomplete.

## 2026-10-03 — Project-family integration resumed

- The preceding user-facing MVP assessment made no implementation progress.
  Revalidated the clean branch at published `c3d17cb`; the converter repair is
  committed and synchronized. No implementation scope was removed by that answer.
- Spec Kit Implement/Tasks prerequisites resolve feature 015 and preserve the
  existing artifacts. Requirements remain 12/16; seven local checklists pass
  7/7. Existing user authorization permits independent closed increments.
- Read-only lock research closed the project-family prefix and identified the
  required NO KEY UPDATE modes for organization and new parent prelocks. Keep
  the editor's existing UPDATE/history exclusion and snapshot actor checks.
  T068–T070 record production tests before implementation; none pass yet.
- No activation, real-data migration, UI/CSS change or merge is authorized by
  this increment. Person-management lifecycle clarification remains separate.

Next: reproduce the project-family gate failures using disposable PostgreSQL,
implement all listed callers coherently, then verify FK/cascade compatibility.

- RED: the production inline-client request held actor SHARE while waiting for
  the organization FK. The explicit NOWAIT actor assertion failed with 55P03,
  demonstrating the inverse ordering before implementation.

- Added the shared final-mode organization helper, updated all ten creation/
  editor callers, both task callers and legacy assignment mutations. Removed
  the late organization locks; retained editor isolation and its project UPDATE.
  Additional production-race tests are compiling. One fixture compilation error
  (nullable `pg_stat_activity.pid`) was corrected with an explicit non-null alias.

- GREEN: the focused gate filter passed 12/12. Full server binary regressions
  passed 883 tests, zero failures and 11 pre-existing ignored cases in 168.58
  seconds, including all five new tests and the stronger SHARE-reader assertion.
  T068/T069 are checked; T070 awaits cache/offline/final formatting verification.
  GitHub confirms existing #212 open/draft at `c3d17cb`; no merge is requested.

- Final verification: 161 core tests pass; clean-package SQLx preparation adds
  26 descriptions and removes only five replaced queries. Fresh offline
  all-targets Clippy passes with warnings denied. Formatting CI inserted one
  missing Markdown blank line, corrected before the final rerun. The owned
  disposable PostgreSQL is stopped. Package cleaning removed 1.5 GiB of
  regenerable artifacts only. No real data, schema, UI/CSS or policy activation
  changed; no browser/full-flake/separate integration-binary result is claimed.

- T068–T070 close the named family, with the caller/trigger review and production
  test mapping in `quickstart.md`. Existing role predicates, historical values,
  tenant behavior, editor isolation and post-commit events are preserved. The
  independent review was source/contract research; implementation review was
  adversarial self-review, not independent full-feature acceptance.

Next: publish this verified increment to existing draft #212 without merging,
then continue T006/T042 across the remaining access-affecting writers and
approval contracts. Keep the unanswered person-management lifecycle choice
separate. The full implementation goal remains active and incomplete.

## 2026-10-03 — Legacy report lock integration resumed

- The preceding MVP status turn made no implementation progress. Revalidated
  the clean worktree at published `fc85231`; the previous audit publication is
  complete. The root's untracked browser artifacts are unrelated and untouched.
- Spec Kit prerequisites resolve feature 015. Requirements remain 12/16, all
  six prior local checklists 7/7; existing authorization permits closed increments.
  Added T065–T067 for the independently reviewed converter lock repair, not full
  T042 or policy activation. No extension hooks are present.
- Project-editor research found that an organization UPDATE prefix would add
  cycles with ungated invoice/budget FK writers. A NO KEY UPDATE alternative
  needs separate caller/trigger coverage; it is not implemented or accepted here.
- The converter repair is independent of unanswered person-management lifecycle
  predicates. Only the owned disposable PostgreSQL cluster on 55416 is started.

Next: reproduce the actual converter/checkpoint race, implement the reviewed
organization-first recheck, then verify concurrency, regression and SQLx gates.

- RED reproduced the intended production race: the actual converter failed with
  PostgreSQL `deadlock detected` while an organization-gated worker archived and
  saved its checkpoint. No mocked lock sequence was substituted.

- Implemented the organization-first recheck and added four concurrency tests:
  worker lease preservation, simultaneous converters, deleted candidate with
  cross-organization rediscovery, and changed payload with a REPEATABLE READ
  connection default. The focused suite is compiling; success is not yet claimed.

- The corrected focused suite passes 11/11, including a fifth new test for an
  ungated job-owning worker's organization FK compatibility. All 161 core tests
  pass. Full server regressions are running; T065/T066 are checked, T067 remains
  open until cache/offline verification. Formatting made zero changes.

- GitHub confirms #212 remains open/draft at `fc85231`; no merge or policy
  activation is requested. The bounded implementation uses no new dependencies.

- Full server binary regressions passed: 878 passed, zero failed, 11 pre-existing
  ignored (185.13 seconds). Complete SQLx regeneration and fresh offline Clippy
  follow the package-local artifact clean; no source or data is removed.

## 2026-10-02 — Resume PR #212 with the current Harvest account

- Objective: extract all available permission evidence from the existing account;
  retain full web parity, without substituting the legacy three-role model.
- Starting branch: `feat/scoped-permissions`, commit `6ce9071`, clean worktree.
  Reuse this worktree and PR; no runtime cutover, migration, purchase, invitation,
  permission change or merge is part of this research iteration.
- Read repository guidance, the proposed 1.1.0 constitution and feature artifacts.
  Executed Spec Kit clarification prerequisite discovery and plan setup with
  `SPECIFY_FEATURE=015-scoped-permissions`; the existing plan was preserved.
  No extension hooks are configured. Clarification questions are reference
  questions first, not requests to repeat the confirmed product scope.
- Continued Phase 0 research. The full-task artifact explicitly remains incomplete,
  so a complete `speckit-analyze` pass is not yet eligible. Do not describe a
  focused evidence review as full-feature analysis or acceptance.
- Reopened current official permissions, flexible approval, user API, person
  profiles, teammate-assignment and retainer documentation. New-model and legacy
  descriptions coexist; an administrator-only legacy endpoint is not proof of
  the new web capability boundary.
- The historically observed permission-editor asset now returns HTTP 404.
  Preserve the historical observation, but reacquire the current asset through
  an approved live browser before treating its source as fresh evidence.
- Started the existing Horae Playwright MCP client. Browser access is awaiting
  extension approval; no fresh account snapshot has been obtained yet. Existing
  snapshots remain dated evidence, not a revalidation of the current account.
- Next action: finish the read-only account probe register, inspect the current
  editor/catalog and owner-visible assignment/approval/billing controls after
  connection approval, then reconcile findings and remaining evidence limits.

The full-feature checklist remains 12/16. T006–T009 and runtime acceptance remain
open. This entry records progress, not completion of the investigation or PR.

### Evidence checkpoint and review

- Added `contracts/current-account-investigation.md`: 15 safe read-only probes,
  six documentation findings, source links and explicit limits. Linked it from
  the existing research and evidence register, preserving historical observations.
- Independently reviewed the approval reference, then directly checked the new
  company-cutoff guides. Access through links in the general approval article
  succeeded where direct opening failed. Company locking, submission, scoped
  approval and invoicing must not be conflated.
- The old public editor asset returned 404. A current asset cannot be selected
  from that historical name; capture the asset actually loaded by today's page.
- Adversarial review of this documentation increment found no actionable issues.
  It did not validate browser connectivity, pending probes or full-feature parity.
- `nix fmt --` on the four changed Markdown files and `git diff --check` passed.
  No Rust, CSS, SQLx, migration or runtime change; no new application test or
  full-flake result is claimed for this documentation checkpoint.
- Product clarification questions asked/answered: 0. The confirmed scope is
  unchanged. Evidence questions remain; no new spec requirements were invented.
  Checklist re-evaluation remains 12/16, with no newly passing items or regressions.
- Connection checkpoint: both browser-list calls timed out after 240 seconds
  each on the same MCP client. No account page was read in this session. The
  client remains available, but no browser operation is reported as a successful
  or still-running probe. Await connection approval/evidence before another retry.

| Clarification coverage | Status and next evidence |
| --- | --- |
| Functional scope and personas | Scope clear; detailed operation mapping still partial |
| Domain model and lifecycle | Partial: profile persistence, approval transitions and independent locks |
| Interaction and UX | Partial: 15 read-only probes pending browser access |
| Quality attributes | Security boundaries clear; full-feature validation not performed |
| External dependencies | Current documentation reviewed; live account access not yet established |
| Edge cases | Partial: rate conflict, assignment promotion and overlapping approval coverage |
| Constraints and tradeoffs | Clear: existing account, no purchases/invites/writes or weakened parity |
| Terminology | Distinguish profile, descriptive role, assignment, approval and company cutoff |
| Completion signals | Clear: research evidence is not implementation or merge acceptance |
| Outstanding reference questions | Retained explicitly; not converted to product defaults |

Next action remains the same live-account investigation, not a new feature:
approve Playwright's connection in Chrome, then execute R01–R15 and attach dated,
redacted outcomes. Do not repeatedly retry the browser without new evidence.

## 2026-10-02 — Approved browser connection and fresh account inspection

The earlier connection limitation is superseded. The same Playwright MCP client
connected after user approval; a dedicated Harvest tab was used and the unrelated
tab preserved. Continued Spec Kit plan Phase 0; setup preserved the existing plan,
and no extension hooks are configured. This is not a completed full plan/analyze.

- Revalidated one immutable owner, no archived people and a paid second-seat gate.
  Owner profile selectors and save remain disabled. No bypass or invitation.
- Retained a fresh 50-grant/11-category catalog and six profile defaults, plus the
  current public editor asset with SHA-256 provenance. Unknown IDs stay unknown.
- Inspected assigned projects/people, rates, project editing, approval status views,
  preferences/modules, expenses/categories, detailed expense report, invoice draft
  actions, retainer form, reports and saved reports. Account activity is plan-gated;
  estimates are disabled. Captures stay in ignored scratch, not Git.
- The project editor raised a leave-page dialog despite no field changes. Accepted
  leaving without saving; the next Reports page loaded successfully. This is not
  evidence of a business mutation. Expense editing was explicitly cancelled.
- Added C01–C07 conflicts/discrimination cases. Most importantly, template deletion
  help promises grant preservation while source warns of possible Member downgrade.
  Marked the expectation provisional consistently in spec, model, tasks, quickstart
  and evidence; neither alternative is approved for implementation from this alone.
- Report-specific data access, managed rates, legacy cost help, deadline gating,
  custom approval dependencies and project-manager loss preview remain distinct
  questions. No owner success is promoted into non-owner enforcement evidence.
- No saved profiles, assignment changes, lock/submission/approval actions, invoices,
  payments, retainer funding, settings changes or migrations. No runtime/CSS edits.

Full-feature checklist remains 12/16; T006–T009 and runtime acceptance remain open.
Next: finish focused evidence review/formatting, publish this research increment
to #212, then use the recorded discriminating cases when separately authorized
editable non-owner reference access and disposable mutation fixtures are available.
Do not repeat owner-only probes as substitutes or request a parity simplification.

### Research increment validation

- R01–R15 now have inspected outcomes or explicit current-account access limits.
  Settled current-week/all-time approved views are empty; an actual detailed
  expense report was run read-only. No empty view is treated as transition proof.
- Independent adversarial review found one low-severity provenance omission:
  owner Archive/Delete controls required the expanded-menu capture. Added the
  existing capture path; no critical/high findings in the reviewed increment.
  The final R09/R11 additions and progress entry were subsequently checked by the
  primary reviewer against their raw captures, not claimed as part of that review.
- The owner DOM contains hidden generic reset copy, not an observed deletion;
  it was read without changing visibility. This strengthens C01's conflict record,
  not either proposed persistence outcome.
- Targeted `nix fmt --` and `git diff --check` passed for the Markdown increment.
  No application tests/full flake run claimed for documentation-only changes.
- Current-account read-only research is recorded; full-feature checklist remains
  12/16. The next implementation gate is still T006's verified operation matrix,
  followed by migration/dependency review and detailed tasks. PR remains draft;
  no merge or runtime activation is authorized by these observations.

## 2026-10-02 — Permission implementation resumed with local fixtures

- Active objective: implement permissions, all five stories/SC-001–009. This is a
  new implementation goal, not a continuation of the completed read-only research.
  No paid Harvest seat or company-account changes are required for confirmed work.
- Revalidated clean `feat/scoped-permissions` at `6443d56`; reused its isolated
  worktree and #212. Ran Spec Kit implementation prerequisite discovery; checklist
  remains 12/16. The user's explicit implementation continuation permits confirmed
  work while unresolved reference cases remain isolated, not marked resolved.
- Used Rust best-practices/testing and Ponytail guidance: existing pure core,
  typed closed enum, standard-library set, no policy-engine or dependency added.
- Added `contracts/grant-catalog.md`, refined the plan/model and T027–T029, then
  implemented 50 known grants, six profile defaults and normalized selections.
  Addition includes prerequisites; removal removes dependants; floor removal is
  rejected atomically. Unknown wire names fail and grants do not infer profile
  identity, management assignments or resource access.
- Initial missing-type tests failed before implementation. Full core now passes
  143 tests, including 22 new tests; 2,500 pair combinations exercise normalization.
  A deliberate missing invoice prerequisite triggered three failures, then the
  restored code passed the full suite. Clippy and full formatting passed.
- Focused adversarial review: dependency fixture is separate from implementation;
  all floor members have only floor prerequisites; insertion/removal terminate;
  no public mutable set or direct selection deserialization bypasses normalization.
  `BuiltInProfile` has no rank ordering/legacy conversion. No runtime guard consumes
  this model yet, so this does not prove saved enforcement or migration safety.
- No Harvest calls, account writes, migrations, user-data changes or UI/CSS edits
  in this iteration. Existing policy remains active until reviewed integration.

Next action: complete the existing-surface operation contract and persisted
profile/individual-grant/assignment/revision model for T006–T009, separating
confirmed predicates from C01–C07. Add local persistence and revocation tests before
server integration; do not silently choose a legacy Manager mapping or destructive
template lifecycle. Continue toward full implementation, not another owner-only
research loop. Goal remains active; T027–T029 do not complete any full user story.

## 2026-10-02 — Reauthorize user-administration transactions

- Reused the same worktree/branch and draft #212 at `e8dcb77`. The previous
  implementation iteration made progress; this iteration closes a demonstrated
  prerequisite race, without narrowing the active full-permission objective.
- Spec Kit implementation prerequisites resolve feature 015; requirement checks
  remain 12/16 with previously authorized continuation. Applied Rust testing,
  best-practices, async and Ponytail guidance: test the actual transaction helpers,
  observe lock dependencies, reuse PostgreSQL locks and add no dependencies.
- Four new negative tests failed before the fix. Creation, role and activation
  now take the authenticated actor, acquire the organization lock, then reload
  and lock active same-organization administrator authority until commit. Events
  remain post-commit; existing last-administrator protection remains in force.
- Added seven tests, including allowed lifecycle, denied revoked/foreign/unknown
  actors, revocation while waiting at both locks, actor-lock lifetime and rollback
  after duplicate creation. Full server binary suite: 795 passed, 11 manual
  measurements ignored, zero failures. Core: 143 passed. SQLx regenerated in full
  with two new cache entries and no removals; offline Clippy and formatting passed.
- Focused adversarial review covered wrapper-derived identity, all helper callers,
  lock ordering, rollback, unchanged records after denial and post-commit events.
  No unresolved critical/high finding for this increment. No browser or full-flake
  result claimed. T030–T032 are complete, not T010/T011 or full SC-003.
- PostgreSQL uses the isolated worktree development stack on port 55415; tests
  use throwaway databases. No Harvest access, real-account changes, schema
  migrations, UI/CSS changes or merge in this increment.

Next: finish the persisted authorization/revision/audit contract and its file-level
tasks for T008/T009, then add failing local persistence tests. Keep the operation
matrix and legacy mapping review explicit before runtime cutover; unresolved
C01–C07 remain reference limitations, not invented defaults or a reason to repeat
owner-only browser probes. Full five-story implementation remains active.

## 2026-10-02 — Concrete persistence proposal and dependent tasks

- Previous goal turn: progress, committed/pushed user-transaction reauthorization
  as `b80f8ab`. Revalidated the clean existing worktree at that commit. No new
  branch, feature restart or merge.
- Executed the checked-in Spec Kit plan/task setup scripts with feature 015;
  both preserved existing artifacts. No extension hooks or agent-context update
  script exists. Full Phase 0 research is still incomplete, so the new mechanics
  are explicitly a partial design proposal, not completed Phase 1/analyze or an
  exemption from the existing schema/runtime gate.
- Expanded `data-model.md` and added `contracts/permission-state.md`: canonical
  grants versus provenance, explicit administrative identity, composite tenant
  constraints, revisions, atomic state/audit/receipt changes, replay and service/
  operator boundaries. No template lifecycle or legacy-role mapping was invented.
- Spec Kit planning's independent read-only review found concrete lock-order
  integration risks, organization-row upgrade risk, operator-audit ambiguity and
  replay ordering. Corrected the proposal and made the remaining full resource/
  trigger hierarchy an explicit pre-implementation task (T042).
- Refined US1 storage/command tests (T035–T038), US3 lock integration (T039/T040)
  and US4 audit disclosure (T041), retaining the full story acceptance packages.
  These are planned tests, not runnable passing fixtures; T006–T009 remain open.
- Asked one optional product clarification for C01: whether to authorize
  preservation when deleting a template despite the contradictory Harvest editor
  warning. No response is assumed and neither lifecycle is implemented.
- No application code, schema, database, Harvest account or UI/CSS changes in
  this planning iteration. The last executed runtime tests belong to `b80f8ab`;
  they do not prove this proposed persistence layer.
- Follow-up independent review found no remaining high/critical contradiction
  within the corrected proposal and no task dependency cycle. T033/T034 are
  complete; T042 and the full-feature gates remain open. No full analysis or
  runtime acceptance is inferred from that scoped review.

Next: incorporate the review outcome and any C01 response, then complete the
operation-level matrix and cross-command lock inventory (T006/T042) needed to
finalize the storage schema and activate T035–T038. Do not keep expanding unrelated
legacy fixes in place of the requested six-profile implementation. Full goal stays
active; this design increment alone does not deliver a user story.

## 2026-10-02 — Operation matrix and concrete transaction constraints

- Previous immediate turn was a clarification, not implementation progress.
  Revalidated worktree `feat/scoped-permissions` at `b7e730c`; the unfinished
  operation matrix was the only initial worktree change and was retained.
- Ran Spec Kit plan setup, preserving the existing plan. Read the current
  constitution and relevant Rust/async/Ponytail guidance. Full research gates
  remain open; no complete planning/analyze or runtime acceptance is claimed.
- Added `contracts/operation-matrix.md`: 45 concern rows covering 80 public async
  server-function symbols, other delivery paths and future web-domain contracts.
  A source check found no missing public symbol under the inspected modules.
  Explicit C/U cells retain unresolved scope, lifecycle and financial predicates.
- Independent matrix review found no high/critical draft contradiction. Corrected
  an omitted authentication-route boundary and wording that could imply an
  unverified prohibition on invoice-draft creation. Public health/static resources
  and protected SSR data are distinguished too.
- Integrated concrete writer order, parent-writing triggers/cascades and a
  candidate common hierarchy into `contracts/permission-state.md`. T042 remains
  open for final predicates and remaining maintenance/identity/job edges.
- A second independent review found no high/critical contradiction in this
  partial lock inventory. Clarified that unchanged child UPDATE rows do not
  trigger parent revision writes. This review did not rerun the local diagnostic
  or establish concurrency safety.
- A fresh local PostgreSQL diagnostic confirmed that the proposed row gate fails
  in READ ONLY transactions used by three current read/preview paths. Recorded
  the required snapshot/revision fence and fresh retry. Also identified inline
  import's network wait inside a transaction; a gate cannot safely be added
  without reconciling existing atomicity. These findings change integration work,
  not the requested final permission scope.
- `git diff --check` and `nix fmt -- --ci` passed after formatting one Markdown
  separator. Revalidated PR #212 as open/draft on `feat/scoped-permissions`
  before publication. No full-flake or Rust regression run is claimed for these
  documentation-only edits.
- No Harvest account access, schema, application code, real data, UI/CSS changes
  or merge. No answer to the earlier C01 clarification is assumed. Full five-story
  implementation and SC-001–009 remain required; T006/T042 are not marked complete.

Next: finish the remaining T042 writer edges and resolve the specific operation
predicates in T006, then finalize migration mappings/persistence and execute the
failing storage tests T035. Do not repeat owner-only browser probes or treat this
inventory as permission to activate partially enforced profiles. The goal remains
active; the new evidence is progress toward integration, not delivered permissions.

## 2026-10-02 — Finish named maintenance inventory and resolve draft creation

- Previous goal turn: progress (`412035d`, published to draft #212). Revalidated
  clean worktree at that commit and reused Spec Kit plan setup without replacing
  artifacts. This iteration remains contract research; no runtime gate is waived.
- Current official Harvest permissions documentation explicitly allows creating
  managed-project invoice drafts. Corrected OP22; this is documented behavior,
  not observed non-owner enforcement or resolution of mixed-invoice scope.
- Completed the named T042 source-tracing gaps: identity/session and credential
  writes, job admission/cancel/retry, claim/lease/recovery/retention, report
  conversion/publication/download and outbox delivery. Documented maintenance
  exceptions rather than adding organization gates across external waits.
- Independent review identified a concrete future deadlock edge in legacy report
  conversion (job lock → organization FK) against org-first import completion.
  Corrected the proposed hierarchy, including job root before chunks, and added
  the necessary candidate/recheck and production concurrency cases. No runtime
  fix or successful concurrency experiment is claimed.
- The source inventory is now more complete, but T006/T007–T009/T042 remain open:
  unresolved reference predicates, migration choices and executable acceptance
  cannot be replaced by static lock inspection. No new schema, account access,
  business-data change, application code or merge occurred.
- `git diff --check` and `nix fmt -- --ci` pass after normalizing Markdown list
  spacing. No runtime suite was rerun for documentation-only changes. Spec Kit
  planning remains incomplete because its reference/product gates are unresolved;
  the next action is a decision, not another claim of implementation readiness.

Next: resolve the first pending product/reference decision C01 (template deletion
preserves existing grants despite contradictory editor copy). The earlier question
has no recorded answer; do not treat an automatic goal continuation as approval.
Then address remaining operation predicates and migration choices one at a time.
Do not generate more standalone foundation tasks merely to avoid those decisions
or present another contract increment as permission implementation. Full goal and
all five stories remain active and unfinished.

## 2026-10-02 — User resolves template-deletion behavior

- User accepted the recommendation: deleting a reusable template retains current
  permissions as person-specific configurations; removing access is a separate
  explicit action showing affected people. This resolves C01 for Horae and is not
  evidence of a Harvest save/delete experiment. Do not ask this decision again.
- Revalidated clean `feat/scoped-permissions` at `324084c`. Used `speckit-clarify`
  and its paths-only prerequisite check; no extension hooks are registered.
  Integrated one accepted answer in the spec's dated Clarifications section,
  FR-015 and US4, and reconciled the model, matrix, evidence, plan, tasks and
  validation guide. Historical observations remain labelled as such.
- Deletion tests must cover individually adjusted assignees, retained management
  scope, detached provenance, unavailable deleted templates, clear confirmation,
  cancellation and separate revocation. They are acceptance requirements, not
  tests already run. No application code, schema or account data changed.
- Re-evaluated all 16 requirement checklist items: 12/16 remains 12/16, with no
  marker changes or regressions. Ambiguous remaining requirements, incomplete
  acceptance scenarios, incomplete criterion coverage and unachieved success
  criteria remain unchecked. This approval does not complete T006–T009 or US4.
- `git diff --check` and `nix fmt -- --ci` passed with zero formatting changes.
  The consistency scan found no remaining current C01-provisional/deletion-gate
  statements. No runtime suite is claimed for this specification-only update.

Next: clarify C02's report-specific financial visibility versus ordinary rate
access using the existing evidence. Ask one decision at a time; do not reopen
C01 or require paid/company-account access for the already approved behavior.

## 2026-10-02 — User resolves report financial visibility

- User explicitly selected C02 option A. A financial report grant authorizes its
  defined financial projection and matching exports within report scope without
  ordinary rate/cost grants. It does not grant source/rate/history access,
  editing or unrelated report access; rate-only access does not open reports.
- Used Spec Kit clarify and its paths-only prerequisite check in the existing
  clean worktree at `5fb78a7`; no extension hooks are configured. Integrated the
  accepted answer into dated Clarifications, FR-008, US3, the operation matrix,
  evidence register, plan, model, T014 and validation guide.
- Removed US3's contradictory blanket financial-redaction condition. Acceptance
  now distinguishes report-only, rate-only, both and neither, including export,
  tenant/scope, forged-selector and revocation cases. These are specified tests,
  not executed runtime checks or verified restricted-user Harvest behavior.
- Spec quality remains 12/16, with no marker changes: remaining requirements,
  acceptance coverage, complete requirement-to-criterion coverage and achieved
  success criteria remain incomplete. C03–C07 and migration/persistence gates
  are not waived. No runtime, schema, real-account data or browser config changed.
- Validation: `git diff --check` and `nix fmt -- --ci` passed (zero formatting
  changes); reviewed the cross-file diff for blanket-redaction contradictions
  and accidental source-access grants. No runtime suite was rerun for this
  specification-only update.

Next: investigate C03's managed-person versus managed-project rate scope from
the existing evidence before requesting any necessary product decision. Do not
reopen C01/C02 or activate runtime policy before the existing integration gates.

## 2026-10-02 — Narrow managed-rate clarification to the owning resource

- Reused clean worktree at `f17fafa`; Spec Kit clarify paths check passed and no
  extension hooks are configured. C01/C02 remain settled for Horae.
- Reviewed official rate-setting, rate-editing, permissions and Users API pages.
  Person defaults and project overrides have distinct effects; legacy Manager
  prose does not establish the new custom grant's enforcement. Retained C03's
  conflict rather than marking it resolved from documentation.
- Recorded one pending proposal in `contracts/rate-scope-evidence.md`: managed
  person authority for global person rates; managed project authority for local
  project/person/task rates, with corresponding financial and resource grants.
  Explicitly distinguish inherited-rate display from personal history, and
  person-default propagation from modification of unrelated project overrides.
- Next: ask whether to adopt this resource-specific rule as a Horae decision or
  retain C03 pending authoritative/reference verification. No answer is assumed;
  no application, schema, account or permission-spec requirement changed.

## 2026-10-02 — User accepts resource-specific managed billable rates

- User confirmed option A, resolving C03 for Horae. General person rates follow
  person management; project-owned rates/person/task overrides follow project
  management. Both require the matching financial permission. No reference
  enforcement claim or automatic privilege grant follows from this decision.
- Used Spec Kit clarify in the existing clean worktree at `8455740`; paths-only
  prerequisite check passed and no extension hooks are configured. Integrated
  one accepted answer into Clarifications, FR-021 and US2, then reconciled the
  rate contract, matrix, reference register, model, plan, T014/T015 and quickstart.
- Acceptance covers the independent person/project cross-product, read/write
  distinction, inherited rates versus personal history, default-rate propagation
  without override mutation, foreign organizations and loss of either required
  grant or relationship. Tests are specified, not executed implementation tests.
- Checklist remains 12/16 with no marker changes. Requirements outside this
  decision, full acceptance coverage, complete criterion coverage and achieved
  outcomes remain incomplete. C04–C07, migration and runtime activation gates
  remain open. No application/schema/data change or merge.
- Validation: `git diff --check` and `nix fmt -- --ci` passed with zero formatter
  changes. Cross-file review removed a stale requirement for Harvest-only
  verification of the now user-resolved C03 rule, and qualified managed-only
  revocation expectations so independent organization-wide grants remain valid.
  No runtime test suite or full-flake result is claimed.

Next: resolve C04's cost-visibility contradiction using current role documentation
and captured grants; do not reopen C01–C03. Continue Spec Kit clarification before
finalizing the integrated policy plan; do not call this delivered permissions.

## 2026-10-02 — User accepts explicit cost permissions

- User accepted the new-model C04 recommendation. Accounting/Executive Manager
  read organization cost rates without writing, Administrator has both, and the
  other built-ins have neither by default. Custom/per-person effective grants,
  not Administrator identity, control cost access; write includes read.
- Used Spec Kit clarify from clean `168d536`, with paths-only prerequisite
  check and no extension hooks. Integrated one accepted answer into dated
  Clarifications, FR-022 and US3; reconciled the rate contract, evidence register,
  operation matrix, profile notes, model, plan, T014/T015 and quickstart.
- Preserved FR-008 report-only projection and independent person/project/action
  constraints. Billable/management grants do not imply costs. Acceptance covers
  built-in/custom read/write, history, supported overrides, denied mutations,
  tenant isolation and revocation. These are planned tests, not executed checks.
- Re-evaluated the quality checklist: 12/16 remains 12/16, no regressions or marker
  changes. Remaining ambiguous requirements, acceptance coverage, complete
  requirement criteria and achieved outcomes still prevent full completion.
  C01–C04 are settled for Horae, not verified restricted-user Harvest enforcement.
  C05–C07 and migration/integration gates remain; no runtime/schema/account change.
- Validation: `git diff --check` and `nix fmt -- --ci` pass. The first formatter
  check exposed a loose-list spacing change around FR-022; restored the compact
  requirement list and reran successfully with zero changes. Reviewed the diff
  for lingering C04 product gates and accidental report/resource privilege
  expansion. No runtime tests or full-flake run are claimed for this docs-only edit.

Next: investigate C05's submission-deadline versus independent scheduled-lock
behavior using current documentation and existing captured settings. Continue
clarification only where a material product choice remains, without reopening
C01–C04 or treating these contracts as implemented permissions.

## 2026-10-02 — Resolve C05 scheduling dependency and narrow C06/C07

- Reused clean `5bf841d`. Ran Spec Kit plan setup (existing artifacts preserved)
  and tasks setup. No extension hooks are configured. Continued bounded Phase 0
  research and task refinement; unresolved full-feature gates prevent claiming
  completed planning or running full Spec Kit Analyze as if tasks were final.
- Current dedicated lock guides and the refreshed general approval guide agree:
  independent schedules do not need a submission deadline. Recorded C05 as
  documented target behavior under the existing parity mandate, not a new user
  choice or successful live settings experiment. Added FR-023, company-lock
  contract and dependent T043–T046 with disposable validation and worker ordering.
- An independent research agent confirmed C06's explicit approval/withdrawal
  grants do not establish custom expense visibility. Asked one product question:
  require visibility of the complete selected time/expense set or leave that
  reference-dependent contract open. No answer has been assumed.
- The same agent narrowed C07: no-project-access removal is documented, but its
  precise read/write threshold is not; keep-access explicitly adds managed read
  AND write. Recorded the distinction and cancellation/preview limitations
  without inventing assignment-driven promotion or verified POST persistence.
- Independent adversarial review found no critical/high C05 issue and two medium
  issues. Corrected both: manual lock submission remains available independently
  of recurring scheduling, and immutability excludes permitted timer finalization
  and explicit corrections rather than contradicting those required actions.
- No code, schema, real records, browser state, queue activation or merge changed.
  The requirements checklist remains 12/16. The other ambiguous requirements,
  incomplete acceptance/criterion coverage and unachieved outcomes remain open.
- Read-only MVP status check: GitHub reports #216 open/non-draft, #208 and #212
  open/draft. Root master still has the existing application baseline; local
  Clients increment records implementation/verification, not a merge. Suggested
  MVP sequencing is not authorization to remove full-parity scope.
- Validation: `git diff --check` and `nix fmt -- --ci` pass, with zero formatter
  changes. This documentation-only iteration does not claim runtime tests or a
  completed full-feature Analyze pass.

Next: integrate the user's C06 answer when provided; then settle C07's exact
assignment-loss/promotion predicates and reviewed migration choices. Do not
restart owner-only probes or create more foundation increments to evade these
decisions. Full-feature analysis awaits executable coverage of the remaining gates.

## 2026-10-02 — User accepts combined approval visibility

- Integrated one accepted clarification using Spec Kit clarify from clean
  `ece40f5`; the paths-only prerequisite check selected feature 015. No extension
  hooks exist before or after clarification.
- Added the dated answer, US2 scenarios and FR-024 to `spec.md`, with
  `contracts/approval-visibility.md` and matching OP06, expense evidence, design
  gate, plan, T012/T013 and quickstart updates. C06's visibility decision is
  resolved for Horae, not proven restricted-user Harvest enforcement.
- Approval requires authority and visibility of every affected time/expense
  record. Denial is atomic and non-disclosing; explicit scoped selection remains
  supported, but silently omitting hidden expenses does not. No implicit grant,
  ordinary expense-edit permission or new catalog prerequisite is introduced.
- Reviewed the accepted wording against the earlier broader proposal: the final
  question concerns approval, not withdrawal. Kept withdrawal, self-approval,
  force submission and coverage/migration gates open rather than enlarging the
  user's answer. Review also distinguishes expense-free selections from hidden
  expenses and report projections from source-record visibility.
- Re-evaluated all checklist items: 12/16 → 12/16, no newly passing items or
  regressions. Remaining unchecked areas are complete unambiguous requirements,
  full acceptance scenarios, criteria for every requirement and achieved
  measurable outcomes. Tests described here remain planned, not executed.
- No runtime code, schema, real records or active permissions changed. T006 and
  full-feature Analyze remain open; no merge is authorized by this clarification.
- Validation: `git diff --check` and `nix fmt -- --ci` pass; the formatter changed
  zero files. No runtime tests or independent adversarial review were run for
  this documentation-only clarification.

Clarify coverage after this answer:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for approval visibility; remaining lifecycle/assignment decisions deferred |
| Domain and data model | Deferred: coverage transitions and reviewed migration |
| Interaction and UX | Clear for atomic denial; remaining profile/assignment flows deferred |
| Non-functional quality | Clear for isolation, current authorization and exactness; implementation validation pending |
| Integrations and dependencies | Clear for this rule: feature 016 expense fixtures and existing transaction protocol |
| Edge cases and failure handling | Resolved for unreadable records, revocation and concurrent additions |
| Constraints and tradeoffs | Clear: no implicit grants, data changes or reduced parity scope |
| Terminology and consistency | Clear: approval is not withdrawal or ordinary expense editing |
| Completion signals | Deferred: full matrix, acceptance and runtime verification |
| Miscellaneous placeholders | Clear: no new unresolved placeholder introduced |

Next: continue Spec Kit clarify/research on C07's project-manager assignment-loss
threshold and explicit keep-access behavior, then remaining lifecycle/migration
predicates before completing plan/tasks and Analyze. Do not reopen the accepted
approval-visibility decision or activate partial policy.

## 2026-10-02 — Narrow C07 to the read-only assignment threshold

- Continued Spec Kit clarify from clean `7bcb3e2`, using the paths-only check,
  current spec and constitution; no extension hooks exist. No accepted answer
  was added or existing product decision reopened.
- Rechecked the Users API, newer permissions article, person/project creation
  guides, assignment API and bulk-assignment guide. Legacy promotion/default
  behavior cannot establish the newer custom-grant threshold. Recorded source
  links and limits in `contracts/current-account-investigation.md`.
- Re-read the retained editor's loss-preview and keep-access handlers. The
  server owns the unknown predicate; adding read AND write in an optional
  preservation action does not prove both are required to retain a designation.
  No POST, account change or redundant owner-only browser probe was performed.
- Prepared one clarification: recommend retaining existing designations with
  project read access, keeping editing independent; contrast requiring editing
  too. Preview/confirmation is required before removing designations, with no
  implicit grants or loss of tracking membership/history. This is a proposal,
  not an accepted requirement or claim of observed Harvest persistence.
- No runtime, migration, data or permission change. Full-feature readiness and
  the 12/16 checklist status are unchanged; no full Analyze pass is claimed.
- Validation: `git diff --check` and `nix fmt -- --ci` pass with zero formatting
  changes. No runtime tests were run for this research-only update.

Next: obtain the user's C07 retention-threshold decision, integrate the answer
and matching acceptance cases, then resolve explicit assignment/promotion
authority. Do not infer acceptance from a generic request to continue.

## 2026-10-02 — User accepts read-only designation retention

- User selected A for C07's existing-designation retention threshold. Recorded
  one accepted answer using Spec Kit clarify from clean `0fe8f56`; ran the
  paths-only prerequisite check and reviewed the constitution. No before/after
  extension hooks exist.
- Updated `spec.md` Clarifications, reference status, US2 acceptance and FR-025.
  Reconciled the current-account evidence/acceptance table, OP16/OP20,
  data-model gates, plan, T012/T013 and quickstart. Effective project read retains
  an existing designation; editing stays independent. Read loss requires preview
  and confirmed atomic removal, preserving membership and historical work.
- Scope review: retention is not new-assignment authority, implicit promotion,
  restoration of a previously removed designation or permission to copy the
  reference keep-access action's read/write expansion. Independent person/all
  scopes remain available only under their own grants; no other capabilities
  are added or removed merely because editing was withdrawn.
- The accepted rule resolves a Horae choice, not Harvest's restricted-user
  enforcement. Remaining assignment/promotion, lifecycle and migration gates
  are still open. No runtime, schema, real data or active permission changed.
- Re-evaluated the quality checklist: 12/16 → 12/16, no new checks or regressions.
  Full unambiguous requirements, all acceptance scenarios, criteria for every
  requirement and achieved outcomes remain unchecked. No complete Analyze pass
  or runtime acceptance is claimed.
- Validation: `git diff --check` and `nix fmt -- --ci` pass; zero formatter
  changes. Reviewed the diff for stale C07 proposals and accidental privilege
  expansion. No runtime tests or independent review were run for this docs-only
  clarification.

Clarify coverage after this answer:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for retention; new-assignment/promotion and lifecycle rules deferred |
| Domain and data model | Resolved for retention versus membership; coverage/migration details deferred |
| Interaction and UX | Resolved for loss preview, confirmation and cancellation; other flows deferred |
| Non-functional quality | Clear for authorization, exactness, revisions and audit; validation pending |
| Integrations and dependencies | Clear for current cross-surface reauthorization; full transition pending |
| Edge cases and failure handling | Resolved for read-only, absent read, independent scope and stale confirmation |
| Constraints and tradeoffs | Clear: no silent grant restoration, data reset or reduced parity scope |
| Terminology and consistency | Clear: designation, membership and edit permission are separate |
| Completion signals | Deferred: remaining contracts and complete implementation/acceptance |
| Miscellaneous placeholders | Clear: no new unresolved placeholder |

Next: continue Spec Kit clarify/research on authority to create manager
designations and explicit privilege expansion; do not reopen the accepted
read-only retention rule. Complete remaining contracts before final plan/tasks
and full-feature Analyze.

## 2026-10-02 — Continue assignment authority and independent reference checks

- Previous turn made progress: accepted FR-025 was committed and published as
  `a1cc799`. Revalidated the clean worktree and active continuation goal before
  proceeding. The full permissions outcome is still incomplete.
- Continued Spec Kit clarify research with the paths-only check; no extension
  hooks. The Member FAQ explicitly describes Manager project-manager assignment,
  but newer custom-grant mapping is still absent. Asked one focused question:
  allow project editors to delegate to already-compatible people, while only an
  Administrator may change global grants, or reserve all designations to admins.
  No answer has been assumed.
- Independent fresh evidence: the current permissions page's Project Manager
  exclusion says withdrawal, while older search snippets say approval. Updated
  the evidence register and profile notes to stop carrying that particular
  textual contradiction forward. The explicit approval catalog, custom
  enforcement and lifecycle boundaries remain separately unverified.
- Checked documented template adjustment/save-as-new flows against retained
  editor evidence; no proof of global assignee propagation or in-place rename.
  Recorded the evidence boundary rather than invent those operations.
- Followed independent approval FAQ/report/activity links: no evidence settles
  withdrawal state or overlap splitting. The activity-log guide does establish
  scoped operational history distinct from administrator-only permission audit;
  added OP46 and its T012/T014/T015 verification obligations. No arbitrary
  custom-grant mapping or new runtime report was inferred.
- No new browser connection or owner-only retry would discriminate the missing
  non-owner cases. No account writes, code/schema changes, migrations or merges.
  Accepted decisions remain intact; the checklist stays 12/16 and full Analyze
  is not claimed complete.
- Validation: `git diff --check` and `nix fmt -- --ci` pass on the complete
  research/matrix diff with zero formatter changes. No runtime tests were run
  for these research-only changes.

Next: integrate the assignment-authority answer when available and reconcile
project creation/editing paths. While it is pending, continue independent
withdrawal/coverage research; do not substitute another foundation increment
for unresolved policy.

## 2026-10-02 — Accept project-editor delegation and reconcile dependent specs

- Continued from published `9c07d60`, preserving the existing branch/worktree
  and draft PR #212. Initially advanced independent T008 reconciliation while
  the assignment question was unanswered; the user's subsequent explicit `a`
  selected option A. No automatic continuation was treated as acceptance.
- Used Spec Kit Plan's existing-plan setup for bounded Phase 0 research, then
  Spec Kit Clarify's paths-only prerequisite check to incorporate that answer.
  Read the constitution; no extension hooks exist. Full Plan and Analyze remain
  gated, not completed by these document changes.
- Added one clarification, US2 acceptance cases and FR-026: project editors may
  add/remove manager designations within authorized projects; adding requires
  compatible existing target grants. Only Administrators change global grants.
  Distinguish retention, assignment-derived scope and privilege expansion.
  Reconciled OP10/11/16, evidence status, plan, data model, tasks and quickstart.
- Added `contracts/dependent-spec-reconciliation.md` with inspected revisions
  for Project Detail, Clients, Workspace and My Settings. Their spec worktrees
  were clean and inspected read-only. Updated only this branch's shared-editor
  spec: conditional cost/private-note separation, delegation and legacy approval
  authority. Clients MVP remains explicitly legacy until reviewed cutover.
- Independent adversarial review found three medium integration omissions:
  Workspace settings versus Company grants, budget-alert payload authorization,
  and the legacy approval assumption. Recorded unresolved settings policy and
  delivery-time payload obligations; corrected the assumption. Follow-up review
  found no high issue and one medium wording conflict: replaced a blanket
  'no partial approval' phrase with no partial mutation of a denied selected set,
  preserving legitimate project/date-scoped approvals.
- Revalidated the quality checklist: 12/16 → 12/16, no changed markers. Complete
  unambiguous requirements, all acceptance scenarios, criteria for every
  requirement and achieved feature outcomes remain unchecked. One answer was
  integrated; no additional product question was asked in this iteration.
- No runtime code, schema, data, browser/account state, other worktree or merge
  changed. Acceptance cases are test obligations, not executed runtime evidence.
- Validation: `git diff --check` and final `nix fmt -- --ci` pass, with zero
  formatter changes. No runtime tests were run for this documentation-only delta.

Clarify coverage after this answer:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for project-editor delegation; remaining operation predicates deferred |
| Domain and data model | Resolved for designation versus grants; creation and migration deferred |
| Interaction and UX | Resolved for current authority, denial and atomic save; remaining flows deferred |
| Non-functional quality | Clear for existing authorization/privacy requirements; validation pending |
| Integrations and dependencies | Clear ownership recorded; T008 reconciliation still partial |
| Edge cases and failure handling | Resolved for compatible targets, revocation, scope and atomicity |
| Constraints and tradeoffs | Clear: no grant escalation or partial runtime activation |
| Terminology and consistency | Clear: retention, delegation and privilege administration differ |
| Completion signals | Deferred: full matrix, migration and runtime acceptance remain incomplete |
| Miscellaneous placeholders | Clear: no new placeholder |

Next: continue Spec Kit research/clarify on remaining operation predicates and
creation/person-management boundaries before final plan/tasks and full Analyze.
Do not reopen the accepted delegation choice or claim full permission readiness.

## 2026-10-02 — Separate person delegation and missing project lifecycle surfaces

- Previous goal turn made progress: published `ab9b1a4` with accepted FR-026 and
  dependent-spec reconciliation. Verified that commit and a clean worktree.
- Continued Spec Kit Clarify: read the skill, repository guidance, constitution
  and complete current spec; ran its paths-only prerequisite check. No extension
  hooks exist. No unanswered choice was treated as acceptance.
- Rechecked the dedicated people-assignment guide and API against the current
  six-profile guide. Relationship writes are explicitly Administrator-only in
  the former sources; ordinary PeopleWriteAll remains insufficient evidence of
  the new mapping. Recorded the API's role-add/remove and whole-set replacement
  semantics so they cannot become silent canonical grant changes in Horae.
- Asked one focused question: Administrator-only person-management assignment
  writes versus organization-wide people writers. Recommended the explicit
  documented relationship boundary. The question remains unanswered; no new
  FR or user decision was recorded. Project delegation A remains settled.
- Added OP47 for person-management relationships and OP48 for project duplication
  and permanent deletion. Fresh lifecycle/creation documentation distinguishes
  these from archive/restore and their dependent records. Read the parity
  worktree's delivered project specs without modifying it: previous increment
  exclusions do not exclude those operations from full web parity.
- Bound the new matrix obligations to T006/T009/T012–T015. This is inventory and
  evidence progress, not final operation predicates, completed domain tasks or
  permission to implement destructive behavior. No browser/account writes,
  runtime/schema edits, migrations or merges were performed.
- Self-review checked source versus inference, legacy API versus web/custom
  rules, existing accepted decisions, and archive versus deletion. No independent
  adversarial review or runtime tests ran for this research-only delta. The
  specification and its 12/16 checklist are unchanged; full analysis stays open.
- Validation: `git diff --check` and `nix fmt -- --ci` pass; zero formatter
  changes. The new tests listed in the matrix are pending obligations.

Next: integrate the person-assignment answer when supplied. Continue independent
contract work without assuming that answer, purchasing account access or treating
legacy role labels as the final capability matrix.

## 2026-10-02 — Migration dependency and preservation review

- Previous goal turn made progress in `80e2e42`; confirmed that revision and a
  clean worktree. Person-management delegation remains unanswered. No automatic
  continuation or recommended option was recorded as a user decision.
- Used Spec Kit Plan for bounded Phase 0 research: read skill/template, current
  spec, constitution and existing artifacts; ran setup-plan, preserving the
  existing plan. No extension hooks exist. Remaining policy gates prevent full
  Phase 1/Analyze completion or runtime activation.
- Inspected SQL migration dependencies, with an independent research review of
  assignment relationships. Found that deleting/reinserting memberships would
  cascade-delete costs, budgets and restricted-task allowances; matching parent
  counts would not prove preservation. Assignment rates also distinguish NULL
  inheritance from zero, and SQL views retain old role-based financial access.
- Added concrete cases M01–M08 to `contracts/migration.md`, linked from the data
  model and T007/T019: tenant anomalies, business-child identities, rate resolution,
  role namespaces, SQL consumers, stale/exhausted revisions, approval provenance
  and unknown job requesters versus already tenant-bound artifacts. No live data
  audit was performed or claimed; these facts describe checked-in schema only.
- Independent adversarial review of the resulting delta found no high/medium
  findings. `git diff --check` and `nix fmt -- --ci` pass, with zero formatter
  changes. No runtime or database tests ran; the listed fixtures remain pending.
- No migration, runtime/schema edit, account write, new product choice or merge.
  T007/T019 and full implementation acceptance remain open; no checklist items
  were marked complete from static inspection.

Next: integrate the person-management authority answer when available; continue
remaining reference/operation contracts without reopening accepted project
delegation. Migration role mappings still require a complete access comparison
and review before implementation.

## 2026-10-02 — Continuation paused on unresolved policy

- Revalidated clean `eb85bc1`. The preceding continuation produced no artifact
  progress: Spec Kit Analyze stopped because tasks explicitly remain a partial
  breakdown; a successful file-existence check did not satisfy its prerequisite.
- The person-management authority question remains unanswered across successive
  continuations since `80e2e42`. Independent inventory and migration-preservation
  reviews have been recorded; repeating them adds no evidence. No automatic
  continuation selects A or B.
- Reviewed all remaining open tasks. Runtime/storage/tests depend on T006–T009;
  T042 depends on finalized operation predicates. Full Analyze cannot substitute
  for those decisions. No further implementation is justified from the current
  evidence, and no reference/account mutation or expanded access is authorized.
- Pause the active goal as blocked, not completed, pending the user's next policy
  answer. A reserves person-management assignment writes to Administrators;
  B also permits organization-wide people writers. This answer is the next
  clarification, not a claim that all other permission/migration gates are solved.
- No runtime changes or tests. This status record is not implementation progress.

## 2026-10-02 — Accept Administrator-only person-management assignments

- The user explicitly answered A to the pending person-management writer
  question. This resolves the recorded policy-answer blocker, not all feature
  gates. No automatic continuation or project-delegation answer was reused.
- Followed Spec Kit Clarify: read the skill, constitution and current spec; ran
  the paths-only prerequisite check. No before/after extension hooks exist.
  Integrated one answer into Clarifications, US2 acceptance, FR-005 and FR-027.
- Only current active same-organization Administrators may add/remove/replace
  person-management relationships. People Admin, Executive Manager and ordinary
  custom people grants cannot confer this authority, including for one's own
  managed-person set. Ordinary person operations and own-access explanations
  remain independently authorized; project delegation FR-026 is unchanged.
- Reconciled OP16/OP47, reference status, dependent-screen obligations, plan,
  data model, tasks, research and quickstart. Tests cover atomicity, revisions,
  revoked authority, scope, audit and no implicit profile/global grant changes.
  Target eligibility, self-assignment and retention remain separate predicates.
- Revalidated the checklist: 12/16 → 12/16, no newly checked items or regressions.
  Complete unambiguous requirements, all acceptance scenarios, criteria for every
  requirement and achieved outcomes remain unchecked. Full Plan/Analyze and
  implementation acceptance are not claimed complete.
- Self-reviewed actor/subject scope, explicit Administrator identity, project
  versus person relationships and remaining open gates. No independent review or
  runtime tests ran for this documentation-only clarification. No schema/data,
  browser/account, runtime policy or other worktree changed; no merge.
- Validation: `git diff --check` and `nix fmt -- --ci` pass with zero formatter
  changes; the acceptance scenarios above remain pending runtime verification.

Clarify coverage:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for person-management writer authority; other predicates deferred |
| Domain and data model | Clear distinction between relationships and grants; eligibility/retention deferred |
| Interaction and UX | Resolved for assignment controls versus permitted explanations/ordinary edits |
| Non-functional quality | Clear existing privacy/authorization requirements; runtime validation pending |
| Integrations and dependencies | Clear cross-screen/task ownership; full reconciliation deferred |
| Edge cases and failure handling | Resolved for own-set requests, revocation, stale revisions and atomic denial |
| Constraints and tradeoffs | Clear: no grant expansion or policy activation |
| Terminology and consistency | Clear: project delegation differs from person-management administration |
| Completion signals | Deferred: matrix, migration and full acceptance remain incomplete |
| Miscellaneous placeholders | Clear: no new placeholder |

Next: continue Spec Kit clarify/research on remaining target eligibility and
operation predicates, then finish plan/tasks before full Analyze. Do not ask the
settled person-management writer question again or interpret it as migration
approval.

## 2026-10-02 — Implement strict restoration of stored grants

- Resumed the existing worktree/PR at `3cc9afe` following the explicit implementation
  request and continuation despite the 12/16 checklist. Read Spec Kit Implement,
  Rust best practices/testing and Ponytail; reran the implementation prerequisite
  helper. No extension hooks exist. Full feature gates remain incomplete.
- The initial intent was additive persistence, but the checked-in plan explicitly
  gates schema on unresolved saved-profile classification and operation contracts.
  Kept that boundary and added T047–T049 for the confirmed pure loading validation;
  did not silently reclassify T035/T036 as completed or waive their dependencies.
- Implemented `PermissionSelection::from_stored` with a catalog-version check and
  typed errors for duplicate or incomplete grants. It never calls editor
  normalization, infers Administrator identity or reapplies profile defaults.
  Unknown identifiers retain the existing closed-enum decoding rejection.
- TDD: nine new tests first failed on the missing API, then the full 152-test core
  suite passed. A deliberate prerequisite-check bypass was caught, restored and
  the suite passed again. Core Clippy with denied warnings passed; formatting and
  whitespace checks passed. No database/browser/full-flake result is claimed.
- Self-reviewed version rejection, prerequisite closure, floor, custom/revoked
  selections, absence of runtime callers and no new dependencies. No external
  account, database, schema, other worktree or legacy authorization changed.
  PR #212 remains open/draft; no merge. Full permission implementation is pending.

Next: resolve the remaining saved-state/relationship predicates under T006–T009
before implementing T035/T036 database storage. Reuse the strict loading boundary
there; do not add another normalizing loader or present this increment as a
delivered permission UI or completed goal.

## 2026-10-02 — Person-management eligibility clarification pending

- Previous goal turn: progress, with tested code published in `524c29e`. Rechecked
  that commit and a clean worktree before continuing. Full runtime work is still
  gated; the pure restoration API is not permission-feature completion.
- Followed Spec Kit Clarify and its paths-only prerequisite helper. Reopened the
  official assignment, permissions and teammates API guides. They repeat the
  existing evidence; no new custom-profile eligibility rule was established.
  This lookup is not additional implementation or parity-validation progress.
- The actor decision FR-027 is settled and must not be asked again. The next
  distinct question is whether a person may receive new managed-person
  relationships when none of their current grants can use that person scope.
  Recommended A: require at least one applicable existing managed-person grant
  (time, expense, people, billable-rate or approval withdrawal), without adding
  any permission. Alternative B: permit dormant relationships even without such
  grants; they still confer no authority on their own. Neither option is accepted
  yet. This question does not decide self-assignment or retention after revocation.
- No new runtime code, schema, migration or account mutation. No fresh test suite
  or full Analyze pass is claimed. No extension hooks exist. The current
  implementation gate remains open pending explicit product clarification;
  the goal is not complete or yet eligible for blocked status on this recurrence.

Next: integrate the user's eligibility answer into FR-005/027 and the operation,
data-model and acceptance contracts. Keep explicit Administrator identity and
the already accepted writer rule unchanged. Do not repeat the same documentation
lookup or fabricate a response on automatic continuation.

## 2026-10-02 — Accept compatible grants before new person assignments

- User answered A to the pending eligibility question. This resolves that blocker,
  not all implementation gates. No assumption is made about self-assignment or
  keeping previously saved relationships after later permission loss.
- Followed Spec Kit Clarify, ran the paths-only prerequisite helper and read the
  current spec/constitution. No before/after extension hooks exist. Integrated
  one accepted answer in Clarifications, three US2 acceptance scenarios, FR-027
  and new FR-028. No additional question was asked in this iteration.
- New assignments require an existing grant applicable to managed people. This
  is the receiving manager's eligibility, not the Administrator actor's authority
  or the managed person's profile. Cover time, expenses, people, person billable
  rates and withdrawal; the corresponding all-scope grants remain compatible.
  Own/unrelated-only grants do not qualify. Do not add grants or require general
  people-directory/edit access, and allow the first otherwise valid assignment.
- Propagated to OP47, the current evidence register, data-model gates, plan,
  T012/T013, dependent-screen obligations and quickstart acceptance. Newly added
  edges in replacements share the same current-eligibility/atomicity check;
  existing relationships and later retention are not silently redefined.
- Checklist remains 12/16, with no new passes or regressions: full unambiguous
  requirements, complete acceptance scenarios, criteria for every requirement
  and achieved outcomes remain pending. T006–T009 and runtime/storage tasks are
  not marked complete. This clarification does not yet unblock full integration.
- Self-review checked actor/recipient distinctions, no circular first-assignment
  prerequisite, read-only compatibility, revocation before commit and no automatic
  promotion. No independent review, runtime test, schema/data/account change or
  merge is claimed. Validation uses formatting and diff whitespace checks.

Clarification coverage:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for new-assignment grant eligibility; other operations deferred |
| Domain and data model | Eligibility distinguished from actor authority; self-assignment/retention deferred |
| Interaction and UX | Clear denial rather than dormant assignment or automatic promotion |
| Non-functional quality | Clear atomicity, current authorization and privacy requirements |
| Integrations and dependencies | Clear T012/T013 and cross-screen ownership; full integration deferred |
| Edge cases | Resolved first assignment, unrelated grants and eligibility loss before commit |
| Constraints and tradeoffs | Clear: no privilege expansion or active-policy change |
| Terminology | Clear Administrator actor, receiving manager and managed person distinctions |
| Completion signals | Deferred full matrix, migration and runtime acceptance |
| Miscellaneous placeholders | No new placeholder or inferred user choice |

Next: finish the remaining relationship and saved-profile contracts through
Spec Kit Clarify before final Plan/Tasks/Analyze and database integration.
FR-028 must not be asked again or treated as an answer about later retention.

## 2026-10-03 — Narrow person-management retention from reference evidence

- Previous user turn made progress: `d965718` integrates the accepted eligibility
  rule. Revalidated that HEAD and a clean worktree; do not reopen FR-027/028.
- Targeted research found an explicit Users API statement missing from the local
  retention register: a People Manager downgraded to Member loses their assigned
  teammates. Recorded its scope and source in the current-account investigation.
  It does not establish arbitrary custom-grant thresholds or a web confirmation.
- Inspected the retained assignment snapshot: it shows the owner explanatory
  state, not a picker that could prove self-assignment. No new interactive browser
  session, permission save or paid/company-account access was attempted.
- Followed Spec Kit Clarify and ran its paths-only prerequisite helper; no
  extension hooks exist. One new question is pending: remove relationships with
  confirmation when the last compatible grant is revoked (recommended A), or
  preserve dormant relationships (B). The prior A settled creation eligibility,
  not this lifecycle transition. No answer is inferred from goal continuation.
- No runtime changes or test pass claimed. Spec/checklist and tasks remain open;
  this evidence narrows the next decision, not feature completion. Formatting
  and diff checks are the applicable validation for the documentation increment.

Next: integrate the retention answer once received, preserving audit, atomicity,
current authorization, unchanged business history and the separate project rules.
Do not repeat these reference lookups as new progress while awaiting the answer.

## 2026-10-03 — Confirm person-management retention on permission loss

- The user answered A to the pending retention question. Spec Kit Clarify
  integrates one accepted answer; no new question is asked. Ran the paths-only
  prerequisite helper and checked extension hooks (none configured).
- Added FR-029, a dated clarification and US2 acceptance scenarios. Existing
  assignments remain while any FR-028-compatible grant remains; loss of the last
  grant requires a current preview and explicit confirmation of atomic removal.
  Cancellation, missing/stale confirmation, revoked Administrator authority or
  write/audit failure must leave permissions and assignments unchanged.
- Updated OP47, plan, data model, T012/T013, quickstart, dependent-screen contracts
  and the evidence register. Remove outgoing person-management relationships only;
  preserve people, incoming relationships, membership and history. Test combined
  FR-025/029 losses with both affected sets confirmed and one atomic commit.
  Returning grants do not recreate removed assignments.
- This is an approved Horae rule, not newly observed Harvest enforcement. No new
  browser session, application code, schema, migration or real-data change occurred.
  Runtime tests were not rerun for this documentation-only clarification.
- Validation: the first formatting check normalized the specification's list
  spacing; rerun the formatter and whitespace check before publishing.
- Checklist remains 12/16 → 12/16, with no newly passing items or regressions.
  Unambiguous/testable requirements, complete acceptance scenarios, acceptance
  criteria for every requirement and measurable outcomes still need attention.
  T006–T009 and runtime/storage tasks remain open.

Clarification coverage:

| Category | Status |
| --- | --- |
| Functional scope and behavior | Resolved for retention; other operation contracts deferred |
| Domain and data model | Resolved outgoing relationship lifecycle; self-assignment deferred |
| Interaction and UX | Resolved preview, confirmation and cancellation |
| Non-functional quality | Clear current authorization, atomicity and durable audit |
| Integrations and dependencies | Clear independent FR-025/029 effects; full integration deferred |
| Edge cases | Resolved stale state, failure and no automatic restoration |
| Constraints and tradeoffs | Clear no dormant retention or implicit privilege restoration |
| Terminology | Clear outgoing/incoming relationships and separate project membership |
| Completion signals | Deferred complete matrix, migration and runtime acceptance |
| Miscellaneous placeholders | No new placeholder or inferred choice |

Next: continue Spec Kit Clarify for remaining relationship and saved-profile
contracts before final Plan/Tasks/Analyze and database integration. Do not reopen
FR-027/028/029 or treat this clarification as completed runtime implementation.

## 2026-10-03 — Specify explicit keep-project-access from editor evidence

- Previous turn made progress: commit `6474382` recorded the user's retention
  decision. Revalidated that HEAD and a clean worktree before continuing.
- Used Spec Kit Clarify and its paths-only prerequisite helper; no extension
  hooks exist. No user answer was inferred. Rechecked the official
  permissions/API guides and inspected the retained editor confirmation handler,
  verifying its SHA-256 against the evidence register.
- Closed the explicit keep-access contract as FR-030 under the existing parity
  mandate: initially unchecked, explicit managed-project read/write additions,
  cancellation without persistence and no implicit editing on read-only retention.
  Client behavior is observed; Harvest backend persistence remains unverified.
- Added `contracts/keep-project-access.md` with seven acceptance cases and task
  ownership; reconciled spec, plan, data model, matrix, dependent screens,
  evidence and quickstart. No runtime task was marked complete.
- Self-review checked administrator versus project-editor authority, project-only
  grants versus FR-028 eligibility, combined FR-025/029 effects, stale previews,
  cancellation, audit rollback and no restoration of historical designations.
  This is not an independent review or an executed acceptance test.
- The design handoff's Workspace section still describes three fixed roles;
  the approved six-profile/custom-permission scope supersedes that description.
  No design asset or application code was changed. No account was mutated.
- Checklist remains 12/16 with no changed markers: full unambiguous requirements,
  all acceptance scenarios, criteria for every requirement and achieved outcomes
  remain open. Formatting and whitespace checks passed for this docs-only change.

| Clarification coverage | Status |
| --- | --- |
| Functional behavior; interaction; edge cases | Resolved for explicit keep-project-access |
| Data model; integrations | Clear reuse of existing grants and atomic protocol; full feature deferred |
| Non-functional quality; constraints; terminology | Clear current authority, no implicit expansion and distinct retention/opt-in |
| Completion signals; remaining placeholders | Deferred full operation matrix, saved-profile lifecycle, migration and runtime tests |

Next question: may an Administrator assign a person as their own managed person?
The targeted official documentation check does not specify that identity case;
the retained owner view cannot demonstrate an editable picker. Recommend A:
reject self-relationships, preserving independent own/all access. B would allow
an explicit Administrator-created self-relationship under existing grants;
neither option settles self-approval. One question pending, none answered here.
Saved-profile contracts remain independent work before final Plan/Tasks/Analyze.
Do not re-ask FR-025/029 or treat observed client behavior as verified persistence.

## 2026-10-03 — Separate profile selection from unchanged saves

- Previous turn made progress in `3a19a26`; confirmed clean worktree and that
  revision. The self-assignment question is still unanswered. This continuation
  does not select an option or add another product question.
- Followed Spec Kit Clarify, checked its feature prerequisite and constitution;
  no extension hooks exist. Revalidated the requirements checklist: 12/16 remains
  unchanged, with no new passes or regressions. Complete requirements/acceptance
  and achieved outcomes remain pending.
- New source evidence: the selected custom-profile radio uses a click handler
  calling `gi`, replacing the draft baseline even when the template ID is the
  same. The retained extra set is Forecast-only. Reset and built-in selection
  are separate draft actions. This distinguishes them from the documented API
  repeat-profile preservation, rather than declaring an unsupported contradiction.
- Refined FR-004 and US4, added `contracts/profile-application.md` with eight
  acceptance rows and traced T010/T011/T016–T018. Updated plan/data-model gates
  and the evidence register. No new user clarification was fabricated.
- Self-review covered no-op versus explicit intent, adjustments after selection,
  reset restoring previously removed grants, deleted/same-name template races,
  Administrator identity, last-admin protection, rollback and relationship loss.
  Saved classification, creation-name equivalence and in-place template lifecycle
  are not settled by this evidence. No runtime test or independent review claimed.
- No app/schema/data changes or Harvest mutations. Formatting and whitespace
  checks are required before publishing this documentation increment.

| Clarification coverage | Status |
| --- | --- |
| Functional behavior; interaction; edge cases | Resolved selection/reset/unchanged-save distinction |
| Data model; integrations | Clear explicit intent and revisions; saved classification deferred |
| Non-functional quality; constraints; terminology | Clear canonical grants, atomicity and source/projection distinction |
| Completion signals; other placeholders | Deferred full matrix, migration, lifecycle and runtime acceptance |

Next: retain the one pending self-assignment question; investigate saved
classification and template-name/lifecycle contracts independently where evidence
can settle them. Do not reopen accepted retention decisions or mark full
Plan/Tasks/Analyze ready from this increment.

## 2026-10-03 — Guard display classification against stored-state changes

- Previous turn made progress in `91f7cc3`; revalidated HEAD and clean worktree.
  The same self-assignment question remains unanswered; no option is inferred.
- Checked Spec Kit Implement prerequisites: requirements checklist is still
  12/16, so no new implementation task begins without the required confirmation.
  No extension hooks exist. Continued independent source/contract work only.
- Inspected the complete initialization around `dt(L)`: non-admin display choice
  is derived from available templates, not just explicit prior selection.
  This yields a new adversarial fixture: add a matching template or change an
  equal-grant tie order without changing the person's grants.
- Extended the existing profile-application contract, data-model distinction and
  task acceptance to prohibit read-time provenance/grant/revision/audit changes.
  Ten profile-application cases are now specified, not executed. Do not assume
  template ordering or saved backend classification from client initialization.
- No code, schema, account data, checklist marker or completed-task changes.
  Formatting and whitespace checks are required before publication.

Next: retain the pending self-assignment decision. Saved classification and
template lifecycle still require discriminating evidence or explicit decisions;
the client fallback must not be used to invent them or bypass the full-policy gate.

## 2026-10-03 — Blocked audit after independent contract work

- Revalidated clean HEAD `602b04f`, open tasks and their dependency rules. The
  previous turn made progress; it did not answer the pending self-assignment
  question. That question has persisted through three consecutive goal turns
  (`3a19a26`, `91f7cc3`, `602b04f`) while independent evidence work continued.
- All designated independent implementation increments are complete. Remaining
  runtime tasks require T006–T009; storage explicitly forbids guessing remaining
  profile, assignment, approval and migration contracts. Spec Kit Implement also
  requires confirmation before proceeding with the incomplete checklist.
- The retained editor evidence has been incorporated without claiming saved
  enforcement. Re-reading it or the owner-only snapshots cannot settle the
  remaining server predicates. No new editable reference fixture, user answer
  or live verification handle is available in this continuation.
- Stop automatic continuation as blocked, not complete. Resume with the pending
  self-assignment answer (A rejects the self-link; B permits explicit Administrator
  assignment), preserving other unresolved gates. That answer alone will not
  make the full feature ready. No further product choice is inferred or queued
  to the user ahead of it; no runtime, schema or account data changed.

## 2026-10-03 — Reject person-management self-relationships

- The user explicitly answered A to the pending self-assignment question.
  Revalidated clean HEAD `1105b75`; the preceding blocked audit did not select an
  answer. This decision resolves that blocker, not the whole permission feature.
- Followed Spec Kit Clarify, ran its paths-only prerequisite helper and read the
  constitution/checklist. No before/after extension hooks exist. Integrated one
  answer; no further question was asked.
- Added FR-031, one dated clarification and three US2 acceptance scenarios.
  Reject equal responsible/managed person identities even for Administrators
  with compatible grants. An invalid add/replacement batch fails atomically.
  Preserve independent own/all access and distinguish the actor from the two
  relationship endpoints: an Administrator's own set may contain other people.
- Reconciled FR-027/028, OP47, plan, data-model gate, T012/T013, quickstart,
  dependent screens and the evidence register. Self-approval, distinct-person
  relationship rules and historical cleanup are not inferred from this choice.
  This is user-approved Horae behavior, not newly verified Harvest enforcement.
- Self-review covered direct requests, mixed batches, administrator bypass,
  actor/endpoint confusion and unintended loss of independently granted access.
  No code, schema, migration or account data was changed; no runtime test or
  independent review is claimed.
- Checklist remains 12/16 → 12/16 with no newly passing items or regressions.
  Full unambiguous requirements, complete scenarios, acceptance criteria for
  every requirement and achieved measurable outcomes remain open.
  Documentation formatting and whitespace checks are required before publishing.

| Clarification coverage | Status |
| --- | --- |
| Functional scope; domain identities | Resolved self-relationship validity |
| Interaction; edge cases | Resolved direct/batch denial and actor distinction |
| Non-functional quality; constraints; terminology | Clear atomicity, independent access and no implicit cleanup |
| Integrations and dependencies | Clear OP47/task ownership; broader integration deferred |
| Completion signals; remaining placeholders | Deferred saved-profile, other operation and migration contracts |

Next: continue remaining saved-profile and operation contracts before final
Plan/Tasks/Analyze and runtime integration. Do not ask FR-031 again or interpret
this answer as approval to bypass the remaining full-policy gates.

## 2026-10-03 — Authorized repair of planning dependencies

- The user explicitly authorized correcting unnecessary planning dependencies.
  Started at clean `cf5d635` in the existing `feat/scoped-permissions` worktree;
  reused PR #212. No implementation, migration, activation or merge is claimed.

- Used Spec Kit Plan and Tasks and ran both setup scripts, preserving completed
  artifacts and task IDs. No extension hooks or agent-context update script are
  present. This is scoped remediation, not completed full-feature Plan/Analyze.

- Reconciled FR-002 with the constitution's pre-cutover gate. Removed hypothetical
  in-place template rename/update as a mandatory FR-015 prerequisite after
  checking current Harvest help; unknown behavior remains an evidence watch,
  not an assertion of absence or an authorized implementation.

- Corrected the spec checklist's outcome interpretation without checking off
  missing acceptance coverage: still 12/16. Actual runtime outcomes remain T020.

- Added a closed contract, local readiness checklist and T050–T052 for pure
  person-management compatibility/self-link prerequisites. Exact existing grant
  variants, negative cases, requirement traceability, paths and test commands
  are defined. The approved person-management decisions are not reopened.

- Focused adversarial self-review found and corrected the stale self-assignment
  statement in the data model, the blanket FR-002/task gates, and the risk of
  checking compatibility on removals rather than additions. The new contract
  separates addition eligibility from whole-proposal self-link validation and
  explicitly rejects using either as complete authorization. Full transaction,
  migration and cross-surface findings remain open; no independent review or
  passing runtime test is claimed.

- Validation: both Spec Kit setup scripts succeeded; Nix formatting and whitespace
  checks passed. Task inventory has 52 unique IDs, 22 complete and 30 pending:
  US1 6, US2 9, US3 4, US4 4, US5 1 and 28 foundation/cross-cutting tasks. Existing
  IDs/completion are preserved. Story tasks carry labels and all tasks name paths;
  no parallel code work is designated. The three new tasks are US2's next
  increment, not a reduced replacement for any story's acceptance above.

- Verified PR #212 is open/draft with the expected branch and prior HEAD; no
  merge is requested. No Rust, PostgreSQL, browser or full-flake suite was run
  for this documentation-only change.

Next: execute T050 (RED) → T051 (GREEN) → T052 (regressions/review) using the local
readiness checklist. Do not wait for saved-profile classification or unrelated
approval predicates, and do not activate server consumers. Storage remains
pending its own design gates; full-policy activation retains T006–T009/T042 and
all migration/security acceptance. No further product approval is needed for
this already-authorized pure increment.

## 2026-10-03 — Implement person-management prerequisites

- Previous goal turn made progress: `804b1d8` repaired authoritative dependencies.
  Revalidated clean worktree and executed Spec Kit Implement's prerequisites.
  Local checklist 7/7 passes; full checklist 12/16 remains incomplete. Proceeded
  under the explicit continuation authorization, without asking again or claiming
  full-policy readiness. No extension hooks apply; existing ignored build/scratch
  locations suffice for this increment.
- Used Rust best-practice/testing and simplicity skills: reused the typed catalog,
  UUID and existing error dependency, borrowed inputs and standard iteration;
  no service abstraction, assignment planner or new dependency.
- Completed T050–T052: seven new tests and pure grant-compatibility/self-link
  functions in `permissions/person_management.rs`. RED failed for missing API;
  GREEN passes all 159 core tests and all-targets Clippy. Mutation checks detect
  lost withdrawal compatibility and a first-element-only self-link check; both
  deliberate faults were removed. Detailed evidence is in `quickstart.md`.
- Focused adversarial self-review found an overly broad helper name and replaced
  it with `validate_no_self_management`. Verified no mutation, role promotion,
  authority inference or new runtime consumer. Input grants do not identify an
  Administrator, and the receiving manager is not confused with the acting user.
  No unresolved high/critical finding in this pure increment; no independent
  review or full-feature security acceptance is claimed.
- No schema, SQLx cache, real data, UI or active authorization changed. Do not
  call the goal complete: full US2, storage/command integration, other operation
  predicates, migration and cross-surface acceptance remain pending. PR #212
  remains draft and no merge is authorized by this work.

Next: resolve the storage-specific saved classification/name-equivalence gates
in T008 using retained/current Harvest evidence, then refine/execute T035/T036
only with a closed non-activating storage contract. Preserve all confirmed
decisions; independent company-lock contract work remains available if reference
verification of template creation cannot progress safely.

## 2026-10-03 — Confirm template names and implement isolated storage

- Previous turn made progress with T050–T052; reused `feat/scoped-permissions`
  and verified PR #212 remains open/draft at the expected prior head. No merge.
- Executed Spec Kit Plan prerequisites and incorporated the user's one naming
  answer with Clarify: `Equipo` and trimmed `equipo` conflict within one
  organization (FR-032). Propagated acceptance and task ownership; do not ask again.
  The help/API and retained editor evidence do not establish Harvest's backend
  name-creation comparison or original template identity. Current matching labels
  are presentation, not permission authority or provenance.
- Closed the bounded storage contract and local checklist, with adversarial
  design review. Fixed its high finding: administrative identity is independent
  of source shape and grant equality, and survives C01 template detachment.
  Follow-up review found no remaining high/critical issue in this contract.
- Executed Spec Kit Implement prerequisites. Full requirements remain 12/16;
  storage readiness is 7/7. Used the already-authorized independent increment,
  not an additional permission request or a claim of full-feature Analyze.
  No extension hooks apply. The broader task breakdown and integration gates
  remain incomplete; no full Spec Kit Analyze completion is reported.
- Added migration 0042, server-only typed state and strict loaders, pure name
  validation and nine PostgreSQL tests plus two core tests. Used Rust/testing/
  async/simplicity skills: existing catalog, serde, SQLx macros, constraints and
  direct borrowed reads; no dependency, service layer or new mutation endpoint.
- Only a newly created disposable database was migrated. Existing roles and
  permissions remain in force. Tests do not authenticate or expose this policy.
  RED, focused GREEN, 161 core tests, core/server offline Clippy and complete
  SQLx regeneration passed. Full server binary regression passed 804 tests with
  zero failures and 11 pre-existing ignored checks; formatting/diff checks passed.
  T035/T036 are complete for this non-activating scope. No browser, full flake or
  separate integration-binary execution is claimed. Details are in quickstart.md.

Next: publish this increment to #212, then resolve T042's cross-command hierarchy and command-specific contracts
before T037/T038 (authorized changes, replay and audit). Preserve the closed
naming/provenance decisions; full T006–T009, migration review and cross-surface
acceptance still gate activation. The goal is active, not complete or blocked.

## 2026-10-03 — Reusable-template commands in progress

- Confirmed the stored naming answer; FR-032 already records case-insensitive
  organization uniqueness after trimming. No repeated product question.
- Refined T053–T055 and the local command contract following lock-order research
  and adversarial contract review. Full requirements remain 12/16; local command
  readiness is 7/7. Re-ran Spec Kit Implement prerequisites and continued under
  the existing authorization for independent increments.
- Added internal create/delete operations with fresh canonical authority,
  explicit READ COMMITTED isolation, organization-first locking, revision checks,
  canonical replay and transactional receipt/audit storage. Additive migration
  0043 has been applied only to the owned disposable compilation database.
  Runtime routes, legacy roles, real data and policy activation are untouched.
- The first test build failed for the missing command module and an incorrect
  test enum variant (corrected). The initial five command tests then passed.
  Added further preservation, limit, revocation, rollback and malformed-state
  cases; the expanded run passed all 12 tests with zero failures or ignored
  cases. Do not infer full T053–T055 completion from this focused result.
- User requested an agency-MVP assessment during implementation. Existing
  tracking/import/project work is a starting point, not an end-to-end agency
  acceptance certificate. Separate a daily-use release from full Harvest parity;
  no scope removal or new MVP definition was authorized by this status question.

Next: cover remaining principal constraints
and affected-person validation; review code, regenerate SQLx cache, run offline
Clippy/regressions/formatting and publish only after verification. Changes remain
uncommitted on the existing feature worktree. Full authorization integration,
permission editing UI, reviewed migration and cross-surface acceptance still
remain; the implementation goal is active.

## 2026-10-03 — Verify audited create/delete commands

- Previous goal turn was progress: it added actual command/schema code and 12
  passing tests. Revalidated the existing worktree and the owned running test
  PostgreSQL rather than restarting or using application data. PR #212 remains
  open/draft on `feat/scoped-permissions`; no merge.
- Re-executed Spec Kit Implement prerequisites. Local readiness remains 7/7;
  full requirements remain 12/16, with existing authorization to continue closed
  increments. No extension hooks apply. The agency-MVP discussion did not replace
  or reduce the full permissions objective.
- Finished T053–T055 with 17 command tests, including affected-person validation,
  exact concurrent retries, full creation audit, principal constraints/isolation
  and receipt versions. Mutation testing demonstrated that removing assignee
  validation fails the dedicated test; restored the strict implementation.
- Used the Rust/testing/async and simplicity skills to reuse the catalog,
  serialization, SQLx macros and database constraints. No dependency, policy
  framework or public mutation surface was added. Focused adversarial self-review
  and coverage corrections are recorded in `quickstart.md`; no unresolved
  high/critical finding in this increment, not full security acceptance.
- Recovered the incomplete incremental SQLx output by preparing without
  incremental compilation: 36 new cache files, no deletions. Offline all-targets
  server Clippy passed with warnings denied, as did 161 core tests and 821 server
  tests (11 pre-existing ignored). Formatting/diff checks passed. Separate
  integration-binary, browser and full flake execution are not claimed.
- Only disposable databases applied migration 0043 or enabled policy version 1.
  Current application permissions and data remain unchanged. Reusable templates
  now have real internal create/delete/replay/audit behavior, but are not yet a
  delivered UI feature. Full T016/T017/T037/T038/T042 remain open.

Next: publish the verified increment to #212, then refine and implement person
profile application with last-administrator protection and confirmed relationship
effects, reconciling the affected T042 lock order before those writers. Preserve
the full enforcement, UI, migration and end-to-end acceptance gates. The goal is
active, not complete or blocked.

## 2026-10-03 — Person-profile commands in progress

- The previous user-facing turn answered the agency-MVP question; it made no
  implementation progress and did not redefine the active permissions goal.
  Revalidated clean `d50c979` and continued in the existing worktree/branch.
- Executed Spec Kit Plan, Tasks and Implement helpers. Preserved existing task
  history and added T056–T058. Full requirements remain 12/16; local readiness is
  7/7 under the existing authorization for independent increments. No extension
  hooks or agent-context updater are present.
- Bounded read-only adversarial research and retained Harvest editor evidence
  closed local identity transitions; inactive-target edits preserve existing
  Horae behavior without claiming verified Harvest persistence. Recorded the
  contract and no-activation lock limitations in `person-profile-commands.md`.
- Added migration 0044 only to the owned disposable database, distinct management
  relations, and internal profile commands with exact final grants, explicit
  identity/provenance, current authorization, last-admin protection, confirmed
  relationship losses, revision fencing and atomic audit/replay. Template requests
  now reject cross-command request-key conflicts consistently.
- The initial offline test compile lacked new query caches; reran with the live
  disposable compilation DB and observed the missing module failure alone.
  Implemented the command; five then thirteen production-command tests passed.
  Five further tenancy/malformed/stale/overflow/input cases are under verification.

Next: finish focused/regression checks and independent contract-to-code review,
regenerate complete SQLx metadata, run offline Clippy/formatting and publish only
verified changes. T056–T058 are not yet marked complete. Full runtime enforcement,
UI, relationship-addition commands, T042 and migration remain open; no real-data
operation, policy activation, merge or completed feature is claimed.

## 2026-10-03 — Verify atomic person-profile changes

- All 20 focused production-command tests pass. Full server binary regressions
  passed (852 discovered, 11 pre-existing ignored), as did all 161 core tests.
  Independent read-only contract-to-code review found no high/security defect;
  its two coverage findings became historical-link-preservation and real gated
  remove/recreate regressions, both passing. No full-policy security claim.

- Recovered SQLx's omission of 91 warm integration-target entries by cleaning
  only this worktree's Horae package build artifacts and repeating preparation.
  Final cache adds 30 entries and deletes/changes none. Fresh non-incremental
  offline all-targets Clippy passed with warnings denied. Full flake/browser and
  separate integration-binary execution are not claimed.

- PR #212 was verified open/draft at `d50c979`, on the expected branch. The
  separate optional description read stalled and was cancelled; no PR mutation
  followed from that read. No real data or runtime guards were changed.

- Formatting CI and diff checks passed; T056–T058 are complete for the internal
  boundary only. Kept Rust changes in existing modules/dependencies and used
  actual PostgreSQL commands rather than mocked authorization outcomes.

Next: publish this verified increment to #212, then continue
the reviewed relationship-write and shared authorization integration work. Person
profile application now has actual internal transactions; authenticated surfaces,
management add/remove commands, full T042, UI and migration remain required.
The full goal stays active, not complete or blocked.

## 2026-10-03 — Project delegation implementation

- Previous user-facing turn was an MVP status answer: no implementation progress
  and no scope reduction. Revalidated clean, published `f5e0dde` in the existing
  worktree/branch; the preceding person-profile increment is already on #212.
- Executed Spec Kit Plan/Tasks/Implement helpers, preserving existing artifacts.
  Full requirements remain 12/16; existing authorization permits confirmed local
  increments, not activation. No extension hooks or context updater are present.
- Independent read-only research exposed the project INSERT FK/editor lock cycle.
  The reviewed project-only contract and T059–T061 close that local design with
  nonblocking parent acquisition and whole-transaction rollback. Person-management
  inactive-target rules and all full-feature gates remain open.

Next: run failing production-command tests, implement project replacement, verify
atomicity/replay/concurrency and publish only verified code. No real-data mutation,
runtime activation, merge or complete-story acceptance is claimed.

### Focused verification checkpoint

- Initial RED compilation failed on the missing command module. After adding the
  implementation, the first run found a fixture missing the required explicit
  `is_administrator = false`; fixed it without weakening schema constraints.

- All 17 project-management production-command tests now pass, including active
  authority, exact audit/preservation, historical replay, atomic denial, real
  organization/project lock contention, retry after Busy and compatible user locks.

- Independent read-only implementation review found no high/security defect in
  the command; its fixture finding and four coverage requests were addressed.

- Verified #212 remains open/draft at published `f5e0dde`. Current changes are
  not yet published. Full server/core regressions are running next, followed by
  SQLx metadata regeneration and offline Clippy/formatting. Full T042, UI and
  policy activation remain open.

- Full server binary regressions passed: 858 passed, zero failed, 11 pre-existing
  ignored; all 161 core tests passed. Formatting CI passed with zero changes.
  T059/T060 are complete locally; T061 awaits the running cache/offline checks.

- One new Spec Kit Clarify question is pending about active receiving managers for
  person-management additions. Official documentation does not settle that write
  predicate; no answer or lifecycle rule has been invented. Project delegation is
  independent. Full requirements remain 12/16; its local readiness checklist is 7/7.

### Verified project-delegation increment

- Complete SQLx preparation adds 28 entries, changes/deletes none. Fresh offline
  all-targets Clippy passed with warnings denied. Cleaning removed only 5.7 GiB
  of regenerable package artifacts in this worktree; no source or data deletion.
- T059–T061 are complete: 36 of 61 listed tasks are checked, not a percentage of
  feature completion. Full requirements remain 12/16 and runtime policy remains
  inactive. Spec Kit and Rust/testing/async guidance kept the change in existing
  storage and transaction mechanisms with no new abstraction, dependency or UI.
- Only the owned disposable PostgreSQL cluster was used. No Harvest mutation,
  real-data migration, endpoint activation, merge or full-policy claim.

Next: publish this verified increment on #212, then implement person-management
commands after the pending lifecycle clarification; continue independent
operation/lock integration work while that answer is pending. Editor integration,
authenticated surfaces, full T042, approvals, migration and end-to-end gates remain
required. The goal remains active, neither complete nor blocked.

## 2026-10-03 — Internal audit reads

- Previous goal turn was progress: project delegation, 17 new tests and verified
  regressions were published as unsigned `9a7e05d` on #212. Revalidated the clean,
  synchronized existing worktree. The prior publication next-action is complete.
- Person-management active-target clarification remains unanswered; do not repeat
  it or invent a default. Continued the independent FR-013/T041 read boundary.
- Executed Spec Kit Plan/Tasks/Implement helpers, preserved artifacts and added
  T062–T064. Full requirements remain 12/16 under existing authorization for closed
  increments. No extension hooks or context updater are present.
- Independent design research calls for strict historical DTOs, required nullable
  fields and organization SHARE before plain reads. Raw intent/results and trusted
  authorization models must not leak into the historical projection.

Next: write failing reader/decoder tests, implement and verify this contract,
then publish only verified changes. Full audit UI, enforcement, person-management
commands, T042, migration and complete-story acceptance remain open.

### Audit implementation resumed

- The intervening MVP status answer made no implementation progress. Revalidated
  the uncommitted audit work, reused this branch and the running owned disposable
  PostgreSQL cluster. The prior test handle was missing, so started a fresh focused
  test run rather than assuming its outcome.
- Re-ran Spec Kit prerequisites. Requirements remain 12/16; all six local
  readiness checklists are 7/7 under the existing closed-increment authorization.
  No extension hooks exist. The pending person-management question is unchanged.
- Added strict historical decoding and the current-Administrator lookup. The
  compiler caught the SQLx timestamp default (`OffsetDateTime`); the query now
  explicitly decodes the existing application `DateTime<Utc>` type. Additional
  tests initially used incorrect relationship table names and an optional backend
  PID; corrected those fixtures against migration 0044 and SQLx's inferred type.
- Added production-reader coverage for both revocation lock orders, inactive
  historical authors, missing current authority, detached template assignees and
  removed relationship snapshots after later grant changes. No production timing
  hooks, new dependencies, schema changes or active endpoints were needed.

Next: obtain the focused test result, review the actual writer/reader contract,
then run regressions and complete SQLx/offline/format validation before publishing.
Full-feature acceptance and runtime cutover remain open.

- Focused audit tests passed 15/15. Full server binary regressions then passed
  873 tests with zero failures and 11 pre-existing ignored cases in 172.11 seconds;
  all 161 core tests passed. T062/T063 are checked; T064 awaits cache/offline/final
  formatting completion. The added malformed stored-document assertion passed in
  the full suite. Local adversarial review and requirement/test mapping are in
  `quickstart.md`, explicitly separate from full-feature acceptance.
- GitHub confirms #212 is open/draft at `9a7e05d`; publication is not yet done.
  The complete cache regeneration is running after a worktree-local package clean.

### Verified audit lookup increment

- Complete non-incremental SQLx preparation adds 14 query descriptions and
  changes/deletes none. Fresh offline all-targets Clippy passed with warnings
  denied. Formatting CI passed with zero changes; diff checks passed.
- T062–T064 are complete. The three actual writer shapes round-trip through
  historical projections with current tenant/Administrator authorization; no
  trusted permission model gained unchecked deserialization. Both real lock
  orders, inactive authors and preserved removed/detached snapshots are covered.
- Spec Kit implementation, Rust/testing/async guidance and the simplicity skill
  kept the change within existing receipts, SQLx transactions and dependencies.
  Only regenerable worktree package artifacts (5.8 GiB) were cleaned. No real
  data, Harvest account, schema, active endpoint, UI or CSS was changed.

Next: publish the verified audit increment to existing draft #212 with an unsigned
commit, without merging. The next implementation work must address the remaining
T006 operation predicates and T042 cross-command lock integration prerequisites,
not infer that the internal commands activate a complete permission policy.
Person-management additions still await the existing active-responsible-person
clarification; continue independent integration analysis without guessing it or
asking it again. Authenticated surfaces, scoped approvals, UI, reviewed migration
and full acceptance remain required. The goal stays active.

## 2026-10-03 — Verified legacy report conversion repair

- T065–T067 are complete. The original production race failed with a real
  PostgreSQL deadlock before the organization-first repair. All 11 report tests,
  including five new concurrency cases, now pass. Full server regressions passed
  878 tests, zero failures and 11 pre-existing ignored; all 161 core tests passed.
- Complete SQLx preparation adds 11 descriptions and removes only the old locking
  discovery query. Fresh offline all-targets Clippy passed with warnings denied;
  formatting passed with zero changes and diff checks passed. The local package
  clean removed 1.4 GiB of regenerable artifacts, no source or data.
- Earlier independent design research and this turn's adversarial self-review
  cover this local edge, not full T042. Spec Kit and Rust/testing guidance kept
  production changes inside the existing converter and used real PostgreSQL
  blockers, lease/archive functions and bounded rollback fixtures. No dependency,
  schema, UI, CSS, authenticated endpoint or new policy activation was introduced.
- Requirements remain 12/16; seven local readiness checklists pass 7/7. GitHub
  confirmed existing #212 open/draft before publication. Publish unsigned on the
  existing branch without merging; no live-data migration or Harvest writes.

Next: continue the remaining T006/T042 integration prerequisites, particularly
project/editor and task-assignment writers against invoice/budget/import paths.
Review the proposed organization NO KEY UPDATE staging mode before implementing
it; preserve existing editor isolation and test both task-linking callers and
revision triggers. Person-management additions still await the existing
active-responsible-person clarification, which is not answered by this repair.
Full authenticated enforcement, approvals, UI, migration and acceptance remain
required. The goal remains active, not complete or blocked.
