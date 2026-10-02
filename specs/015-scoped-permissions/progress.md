# Scoped permissions investigation progress

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
