# Permissions discovery

Status: specification input, not a completed implementation plan. Baseline `9301112`; inspected 2026-09-30.

## Current Horae boundaries

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

There is a documentation conflict to resolve: the general approval article restricts withdrawal to administrators, whereas the new permissions article includes withdrawal capabilities in other profiles. Use the current permissions experience as the target and verify the effective capability rather than copying the old three-role rule. Assignment editing/promotion and custom-template update behavior also need evidence. None of these investigations authorizes replacing parity with simpler existing Horae behavior.

`design/project/app/08_Settings.dc.html` displays the six profiles. `09_Workspace.dc.html` explicitly describes three fixed roles with no per-person permissions. The new approved matrix must become authoritative for both screens; neither inconsistent mockup can silently settle the policy.

The constitution also names three organization roles. Record the approved extension through its amendment procedure before implementation. This draft does not amend it or grant new access.

## Workflow

- Followed the checked-in `speckit-specify` skill, local template and constitution. No extension hooks or template preset overrides were found.
- Feature 015 follows the independent 012/013/014 design drafts. The branch/worktree starts from fetched `origin/master`, not an unmerged application branch.
- The two scope questions are answered by the user's explicit parity instruction. The clarification is recorded in the spec and its scenarios, requirements and success criteria.
- Next: complete reference verification and the operation-level matrix, reconcile governance and migration, then plan, generate tasks, analyze consistency, implement and verify. No claim is made that those later stages have run.
- Checklist notes describe the initial draft; after clarification, 12/16 markers pass. The remaining gaps are detailed requirements/acceptance coverage and outcome readiness, not the two answered scope questions.
