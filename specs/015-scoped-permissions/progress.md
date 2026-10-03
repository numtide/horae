# Scoped permissions investigation progress

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
