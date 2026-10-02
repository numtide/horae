# Permissions discovery

Status: incremental planning; independent scope foundation specified, full policy research incomplete. Baseline `9301112`; inspected 2026-09-30.

## Current Horae boundaries

The expanded [entry-point inventory](contracts/current-access.md) now records current checks, redaction differences, jobs/plugin trust boundaries and migration deltas. The [reference evidence register](contracts/harvest-evidence.md) separates confirmed browser observations from documentation and unverified custom behavior.

- `crates/core/src/types.rs`: organization roles are Admin/Manager/Member. `OrgRole::is_manager_or_above` describes organization-wide billing and approvals. ProjectRole is a separate legacy concept.
- `crates/core/src/state.rs`: approval and invoice transitions currently accept organization roles. The new model must preserve valid state transitions while changing authorization deliberately.
- `crates/horae/src/server_fns.rs`: `require_user` reloads active identity; `require_admin` and `require_manager` enforce broad role gates.
- `crates/horae/src/server_fns/users.rs`: active-user listing redacts rates below manager, while user creation and role/activation changes require administrator authority. The organization lock protects concurrent last-administrator changes.
- `crates/horae/src/server_fns/approvals.rs`: submissions cover a person's configured week. A project-scoped permission cannot safely stand in for authority over that entire submission.
- `crates/horae/src/harvest/auth.rs`: the compatibility surface resolves active identity and organization role separately from fullstack server functions. Updating only the UI or shared helpers would not establish complete parity.
- Search also identifies role-dependent consumers in reports/downloads, imports, jobs, notifications, navigation and plugins. This is an entry-point discovery list, not a completed security audit.
- Feature 011 explicitly separates project-manager designation from organization promotion and protects cost/private-note data. Any changed boundary requires an explicit replacement contract and regression tests.

## Reference and design conflicts

The official [Harvest permissions reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions), checked 2026-09-30, documents a progressively deployed six-profile model and optional custom profiles/per-person adjustments. Only administrators administer role assignments. Its permission categories distinguish self, managed work and account-wide access; profile names alone are insufficient to copy those semantics.

This is documentation evidence, not a fresh interactive test of the user's account. Accounts may still have the outgoing model. The user's 2026-09-30 parity instruction resolves FR-004 and FR-009 in favor of custom permissions and project-scoped approval. They are no longer unanswered product choices.

The current [approval reference](https://support.getharvest.com/hc/en-us/articles/360048181832-Submitting-and-approving-timesheets), checked 2026-09-30, explicitly describes approval limited by project/client filters, even when grouping by person. It also describes weekly locks and editing submitted work before approval. The exact interaction of filtered approval with empty days and new work needs direct verification; do not infer the entire lock scope from the selected approval rows.

The older general approval article restricts withdrawal to administrators, whereas the new permissions article includes scoped withdrawal for People Admin and Executive Manager. Project Managers cannot withdraw approvals; that is not a prohibition on approving. Use the current permissions experience as the target rather than copying the old three-role rule. Assignment editing/promotion and custom-template update behavior still need evidence. None of these investigations authorizes replacing parity with simpler existing Horae behavior.

### Flexible approval evidence

The current [flexible timesheet approval reference](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval), read 2026-09-30, refines the older weekly article:

- Submissions and approvals can cover dates shorter than a week; submitted work remains editable before approval.
- Managed-project or managed-person authority can cover work; one eligible approver suffices. Manager self-approval is disabled by default and configurable.
- Approval locks selected dates and project coverage, including cells without entries. Whole-submission coverage is possible with sufficient authority and qualifying filters.
- Withdrawal from Approval uses its date/filter scope. Withdrawal from Day/Week unlocks the whole week. There is no rejection transition; changes can be requested and work edited/resubmitted.

Independent locks must still apply after withdrawal; see [unlocking time and expenses](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses). These are documentation findings, not fresh browser observations. The precise arbitrary-custom-profile approval predicate, newly created projects after whole-submission approval, overlapping interval splitting and post-withdrawal submission state remain unverified.

### Custom-profile evidence

The new permissions reference documents administrator-managed templates, an immutable Member floor, automatically included prerequisites/dependent removal, unique names of at most 100 characters, at most 50 templates, per-person difference indicators and deletion that preserves existing permissions. Applying a profile requires saving the person. Read-only inspection of the live editor has now established its profile defaults, dependency traversal and classification rules; see [reference profiles](contracts/reference-profiles.md). Saved enforcement, name case sensitivity and template update/propagation still require direct verification. The live catalog also identifies approval-specific grants and report/rate-scope differences from the article; those findings supersede guesses based on its role summaries.

### Import and profile-write boundaries

The [API application evidence](contracts/reference-profiles.md#api-application-evidence)
adds documented reapplication and assignment-removal cases to T006/T007. It does
not establish template-update propagation or resolve the UI persistence gate.

Current code inspection on `757f43d` confirms separate integration boundaries:

- `importers::harvest::api_source::ApiUser` consumes only identity fields.
- `importers::harvest::resolve::resolve_user` matches existing same-organization
  users; it does not provision users or write their permissions. Missing and
  ambiguous identities fail instead of selecting an arbitrary account.
- `harvest::router` exposes only GET routes; `HarvestUser` currently serializes
  `is_admin`, not a six-profile/custom-grant representation.

FR-014/017 require preserving that import boundary through migration. External
metadata must not become a local grant merely because the importer can parse it.
The compatibility serializer needs an explicit reviewed projection when the new
model is implemented; adding write endpoints is not required to enforce reads.
T019 must exercise repeat imports and identity linking against existing custom
grants without changing them. This is a required future regression, not a test
already run or permission-policy activation.

### Independent foundation decisions

- Decision: model record coverage as an explicit union of own, managed-person, managed-project and organization scopes. Rationale: FR-006 and the reference distinguish these dimensions. Rejected: inferring authority from role ordering or ordinary project membership.
- Decision: evaluate trusted facts in the existing pure core with borrowed assignment slices. Rationale: correctness is independently testable without storage or new dependencies. Rejected: introducing a policy-engine dependency or a new crate.
- Decision: keep the foundation disconnected from runtime authorization until the full matrix and migration are reviewed. Rationale: a scope predicate does not define capability grants, locks or transactional revocation. Rejected: swapping existing guards piecemeal and claiming parity.

`design/project/app/08_Settings.dc.html` displays the six profiles. `09_Workspace.dc.html` explicitly describes three fixed roles with no per-person permissions. The new approved matrix must become authoritative for both screens; neither inconsistent mockup can silently settle the policy.

The baseline constitution named three organization roles. The 2026-10-01 amendment
to 1.1.0 in this branch records the approved extension and its migration gates;
it grants no runtime access. The exact operation matrix and dependent feature
acceptance remain to be reconciled before cutover.

## Workflow

### Persistence and concurrency design — 2026-10-02

- Decision: persist canonical person grants separately from template provenance
  and explicit administrative identity. Rationale: upgrades/display classification
  must not silently grant authority; custom all-grants selection must not count as
  Administrator. Alternatives rejected: live template-name lookup, rank ordering
  and inferring administrative identity from a particular grant set. This storage
  proposal does not decide pending template deletion/update/reapplication rules.
- Decision: propose the organization row as one shared/exclusive authorization
  gate, with current-state reload and revision checks. Rationale: it extends the
  proven user-access serialization boundary without an extra locking service.
  Alternatives rejected: admission-only checks and adding a new editor gate while
  leaving existing writers outside it. Every organization-row update starts
  exclusive, even when its permission is not administrative, to avoid upgrades.
- Existing `(id, org_id)` keys in migration `0030_project_creation.sql` support
  tenant-constrained references. The old `audit_log` in `0001_init.sql` lacks the
  required FK/revision/receipt contract and has no inspected insertion path; do
  not mistake its existence or plugin events for durable access-change auditing.
- Decision: one transaction records state, revisions, attributed audit and request
  outcome. User/operator actors are distinct; credentials/provider subjects are
  excluded. Exact replay checks current disclosure authority and recorded scope
  before returning a historical result, without requiring deleted references to
  exist again. Alternatives rejected: replaying old mutations, fabricated admin
  actors and logging denials in the transaction that will be rolled back.
- Independent read-only review identified actor-before-organization ordering in
  project finalization/editing and assignment paths, hidden project-revision
  trigger locks, operator-attribution ambiguity and live-reference-before-replay
  ordering. The proposed contract now addresses these; the exact cross-command
  resource hierarchy remains an explicit T042 design gate, not implemented code.
- The [state protocol](contracts/permission-state.md) and expanded data model are
  partial planning under T008. C01–C07, approved migration mapping, dependent-spec
  reconciliation and the operation matrix remain open. Neither complete Phase 1
  nor the full `speckit-analyze` gate is claimed: full tasks are still incomplete.

### Current-account investigation — 2026-10-02

- Decision: continue reference discovery using the existing account, as requested,
  before asking for additional seats or accepting a product deviation. The
  [probe register](contracts/current-account-investigation.md) tracks safe account
  surfaces, fresh documentation findings and evidence limits; [progress](progress.md)
  records the next action.
- Rationale: current owner-visible configuration can narrow the unknown catalog,
  dependency and operation boundaries even when non-owner enforcement cannot be
  exercised. Browser approval subsequently succeeded: one immutable owner and
  no archived people were confirmed, along with 50 grants, six profiles and the
  current delivered editor. Dated outcomes and limits are in the probe register.
- Alternatives rejected: bypassing owner-disabled controls, treating legacy API
  restrictions as six-profile web policy, or saving an unchanged form as a
  supposedly read-only probe. Assignment controls may autosave, and eligible
  invoice saves can move retainer funds.
- The new evidence identifies separate assignment read/write, own-permission
  visibility and retainer lifecycle questions, plus a documented company-cutoff
  lock distinct from scoped approvals. The independent approval review's lock
  findings were checked directly in the linked official guides. T006 remains
  open; these are reference questions, not invitations to simplify confirmed parity.
- Fresh browser/source evidence adds C01–C07: template-deletion preservation is
  contradicted by a downgrade warning; report access and ordinary rate access
  need distinct checks; managed-rate/cost help conflicts remain; deadline gating,
  approval prerequisites and project-manager assignment-loss preview need
  discrimination. No account writes or non-owner enforcement tests were performed.
  Decision: retain the full-feature gates and label provisional contracts instead
  of choosing whichever source grants more access. No runtime policy changes.

### Constitution reconciliation — 2026-10-01

- Executed the checked-in `speckit-constitution` workflow against the user's
  already recorded six-profile/custom-permission decision. Version 1.1.0 expands
  authorization constraints and verification guidance without replacing the five
  core principles, authentication, financial invariants or datastore rules.
- Checked all three feature templates and runtime guidance. No command-template
  directory or extension hooks exists. Kept generic templates and accurate
  current-runtime documentation unchanged; added the required Sync Impact Report.
- Updated this specification and plan to distinguish the included amendment from
  completed policy design or deployment. T008 remains unchecked: dependent-spec
  reconciliation and persisted authorization/revocation design are not finished.
- The change is proposed in PR #212, not merged or a substitute for reviewing
  migration differences. No schema, application behavior or data changed.

### Migration research — 2026-10-01

- Decision: compare effective operation/scope/field access, not profile labels,
  before activating a reviewed mapping. Rationale: the current Manager has
  financial/project writes but the timesheet still operates on the session
  person's entries; none of the new profile labels proves equivalent access.
  Rejected: automatic Manager-to-Project-Manager or Executive-Manager mapping.
- Decision: retain explicit unknowns for historical job requesters and approval
  coverage. Rationale: current storage does not contain those new facts.
  Rejected: attributing jobs to the current owner or inventing project approval
  history during migration.
- The new [migration contract](contracts/migration.md) supplies the previously
  missing T007 artifact, preview/activation acceptance and concrete fixture
  categories. Actual mappings, compatibility strategy and historical job/approval
  transition remain review gates; T007 and T019 are not marked complete.
- Reran the checked-in plan setup helper; it preserved the existing plan and
  resolved feature 015. Continued Phase 0 research only. Full Phase 1 design,
  post-design constitution approval and full-feature analysis remain incomplete.

Additional Phase 0 investigation of managed rates could not resolve the conflict
from current official documentation. [Rate-scope evidence](contracts/rate-scope-evidence.md)
records the sources, limits and independent person/project probes required to
settle it. No rate scope or prerequisite has been invented to close the gate.

### Previous workflow record

- Followed the checked-in `speckit-specify` skill, local template and constitution. No extension hooks or template preset overrides were found.
- Feature 015 follows the independent 012/013/014 design drafts. The branch/worktree starts from fetched `origin/master`, not an unmerged application branch.
- The two scope questions are answered by the user's explicit parity instruction. The clarification is recorded in the spec and its scenarios, requirements and success criteria.
- Incremental plan and foundation contract now exist. Full research/design remains open; complete reference verification, operation matrix, governance and migration before runtime policy implementation.
- After clarification, 12/16 checklist markers pass. The remaining gaps are detailed requirements/acceptance coverage and outcome readiness, not the two answered scope questions.
