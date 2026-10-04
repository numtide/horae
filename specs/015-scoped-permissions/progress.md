# Scoped permissions investigation progress

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
