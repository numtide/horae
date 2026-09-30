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

### Independent foundation decisions

- Decision: model record coverage as an explicit union of own, managed-person, managed-project and organization scopes. Rationale: FR-006 and the reference distinguish these dimensions. Rejected: inferring authority from role ordering or ordinary project membership.
- Decision: evaluate trusted facts in the existing pure core with borrowed assignment slices. Rationale: correctness is independently testable without storage or new dependencies. Rejected: introducing a policy-engine dependency or a new crate.
- Decision: keep the foundation disconnected from runtime authorization until the full matrix and migration are reviewed. Rationale: a scope predicate does not define capability grants, locks or transactional revocation. Rejected: swapping existing guards piecemeal and claiming parity.

`design/project/app/08_Settings.dc.html` displays the six profiles. `09_Workspace.dc.html` explicitly describes three fixed roles with no per-person permissions. The new approved matrix must become authoritative for both screens; neither inconsistent mockup can silently settle the policy.

The constitution also names three organization roles. Record the approved extension through its amendment procedure before implementation. This draft does not amend it or grant new access.

## Workflow

- Followed the checked-in `speckit-specify` skill, local template and constitution. No extension hooks or template preset overrides were found.
- Feature 015 follows the independent 012/013/014 design drafts. The branch/worktree starts from fetched `origin/master`, not an unmerged application branch.
- The two scope questions are answered by the user's explicit parity instruction. The clarification is recorded in the spec and its scenarios, requirements and success criteria.
- Incremental plan and foundation contract now exist. Full research/design remains open; complete reference verification, operation matrix, governance and migration before runtime policy implementation.
- After clarification, 12/16 checklist markers pass. The remaining gaps are detailed requirements/acceptance coverage and outcome readiness, not the two answered scope questions.
