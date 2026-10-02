# Feature Specification: Scoped Roles and Permissions

**Feature Branch**: `feat/scoped-permissions`

**Created**: 2026-09-30

**Status**: Draft — Harvest parity confirmed; detailed parity matrix and verification pending

**Input**: Focus on permissions as a separate SpecKit feature before continuing the Settings/Workspace redesign, using Harvest's current permission model as a reference.

## Clarifications

### Session 2026-09-30

- Q: Six fixed profiles or custom profiles and per-person adjustments? → A: The user requested parity with Harvest: all six built-in profiles, reusable custom profiles and per-person permission adjustments are required.
- Q: Whole-person approval only or separately actionable project portions? → A: The user requested parity with Harvest: approvals must be scoped to managed projects/people and the selected filters, including partial project approval within a person's week. Retaining only the existing whole-person approval is not an accepted simplification.

Parity is the acceptance target for this permissions and approvals feature, not merely visual resemblance. Existing Horae authorization or approval behavior is not a reason to omit a required Harvest behavior. Differences must be identified and resolved explicitly; unknown reference behavior requires investigation rather than another request to approve a smaller scope.

### Session 2026-10-01

- Q: Which product domains must the permission contract cover? → A: The user confirmed complete Harvest web parity, including expenses, estimates, retainers, invoicing and payment recording; native applications, Forecast and new integrations are excluded. Domain implementation remains separate from this permission feature.

### Reference verification — 2026-10-02

The [current-account investigation](contracts/current-account-investigation.md)
records conflicting evidence: public help promises preserved permissions after
template deletion while the editor warns of a possible Member downgrade. The
2026-10-02 decision below resolves C01 for Horae; actual Harvest deletion remains
unverified, not established parity. The same session resolves C02 for Horae:
report permission authorizes its displayed financial data within the report's
scope, without granting ordinary rate access. Restricted-user Harvest enforcement
remains unverified. C03 is resolved for Horae by the resource-specific rule below,
not by proving Harvest enforcement. C04 cost visibility remains unresolved.
C05–C07 retain the scheduling, approval and assignment-loss gates. T006 remains
open; no acceptance scenario is marked passed from owner-visible controls.

### Implementation continuation — 2026-10-02

The user requested implementing permissions after deciding not to purchase a
Harvest seat or depend on the company account. Implement confirmed behavior with
disposable local tests, isolate unresolved reference cases, and retain the full
scope. The [grant catalog](contracts/grant-catalog.md) is an independently testable
increment; it does not activate runtime grants, approve migration mappings or
claim exact parity for C01–C07. Account access is not a prerequisite for work on
confirmed contracts; local tests alone cannot settle contradictory reference facts.

### Session 2026-10-02

- Q: Should deleting a reusable permission template preserve the permissions of its existing assignees despite the conflicting Harvest editor warning? → A: Yes. Remove the template from future assignment choices and retain every assignee's current permissions as a person-specific configuration. Explain this in the confirmation dialog; revoking permissions is a separate explicit action that previews the affected people. This is an approved Horae behavior, not a claim of verified Harvest persistence.
- Q: Does access to a financial report authorize its displayed amounts without separate ordinary rate permissions? → A: Yes (option A). Authorize the report's defined financial fields and corresponding exports within its scope, without granting general rate access or editing. Ordinary source records, rate history and unrelated report families retain their own permissions. This resolves C02 for Horae; restricted-user enforcement in Harvest remains unverified.
- Q: Should managed billable-rate access follow the person for general rates and the project for project-specific rates? → A: Yes (option A). With the corresponding financial permission, managing a person covers their general billable rates; managing a project covers its project-specific rates, including person/task overrides, without authorizing changes to participants' general rates. Management alone grants no financial access. This resolves C03 for Horae, not verified enforcement of Harvest's new model.

## User Scenarios & Testing

### User Story 1 - Assign a role that matches a person's responsibilities (Priority: P1)

An administrator assigns Member, Project Manager, People Admin, Accounting, Executive Manager or Administrator and can see the resulting access before saving.

**Why this priority**: The current broad Manager permission combines operational and financial authority. The new profiles must represent actual boundaries, not different labels for the same access.

**Independent Test**: Assign every profile to disposable users, sign in as each and exercise both permitted and forbidden operations through screens and direct requests.

**Acceptance Scenarios**:

1. **Given** an administrator, **When** a profile is selected, **Then** its permissions and scopes are explained before saving; cancel makes no change.
1. **Given** a saved assignment, **When** that person makes another request, **Then** the newly effective permissions apply without requiring a new sign-in.
1. **Given** a person without permission-administration authority, **When** they attempt to change their own or another person's privileges, **Then** the attempt fails without changing access or disclosing confidential settings.
1. **Given** concurrent administrator demotions or deactivations, **When** they are submitted, **Then** at least one active administrator remains and conflicting changes do not partially succeed.

### User Story 2 - Limit management to assigned work and people (Priority: P1)

A responsible person manages the projects and people explicitly assigned to them without gaining access to the whole organization.

**Why this priority**: A role alone cannot distinguish responsibility for one project from responsibility for every person and financial record.

**Independent Test**: Use two projects, two managed people, an unrelated person and a second organization. Compare overlapping, disjoint and revoked assignments, including a week and invoice containing multiple projects.

**Acceptance Scenarios**:

1. **Given** project-management authority, **When** a project is assigned, **Then** its permitted operations become available but unrelated projects remain outside that scope.
1. **Given** a person-management assignment, **When** the responsible person opens that person's work, **Then** the applicable person-scoped permissions are enforced independently of project membership.
1. **Given** a Member assigned to a project, **When** they track time or view permitted progress, **Then** they do not automatically gain management, rates, cost or invoice access.
1. **Given** managed billable-rate permission and responsibility for person B but not project Q, **When** accessing B's general rate or B's override in Q, **Then** the general rate follows the granted read/write action while Q's override remains inaccessible without independent project authority.
1. **Given** managed billable-rate permission and responsibility for project P but not person C, **When** accessing P's project/person/task rates, **Then** the permitted action is confined to P; C's general rate cannot be changed and unrelated personal rate history is not disclosed. An effective inherited rate may be shown within P's authorized financial view.
1. **Given** management responsibility without billable-rate permission, or read-only rate permission, **When** a rate mutation is attempted, **Then** it fails without changing rates. For managed-only authority, removing either the necessary financial grant or management relationship prevents subsequent access.
1. **Given** overlapping project/person scopes, **When** a list or total is displayed, **Then** each authorized record appears once and no excluded record contributes to its count or total.
1. **Given** mixed-scope work, **When** a bulk action, approval or invoice operation is attempted, **Then** its entire affected set is authorized under FR-009; no hidden partial mutation is allowed.
1. **Given** a person has submitted time in projects A and B during the same week and the approver manages only A, **When** A is approved, **Then** A's selected time becomes approved while B's approval state is unchanged. The person must not be labelled wholly approved while B remains pending.
1. **Given** grouping by person and filtering by client A, **When** an authorized approver approves a displayed submission, **Then** only the authorized work belonging to that client is approved; grouping must not widen the selection.
1. **Given** an approver also manages a person, **When** they select that person's work across projects, **Then** approval follows that person scope and the displayed filters without including another person's unrelated work.
1. **Given** a partially approved week, **When** another authorized approver completes the remaining portion or withdraws an authorized approval, **Then** the aggregate state and history match the actual portions without resetting unrelated approvals or invoice locks.

### User Story 3 - Enforce the same access everywhere (Priority: P1)

A person sees consistent access in the application, reports, downloads and connected tools. Hiding a button is not the authorization boundary.

**Why this priority**: Existing access checks span several delivery paths; a new profile is unsafe if one of those paths retains broader old permissions.

**Independent Test**: Replay forbidden requests and compare equivalent screen, export and integration reads; revoke permissions between preview, submission, background execution and download.

**Acceptance Scenarios**:

1. **Given** neither ordinary rate/cost permission nor applicable report-specific financial permission, **When** a person reads a project, picker, report, export or integration response, **Then** restricted financial values are absent, not merely hidden in the screen.
1. **Given** permission for a financial report but no ordinary rate/cost permission, **When** the person reads that report or its corresponding export, **Then** its defined financial fields and amounts are present within the authorized report scope, while direct rate reads, rate history, edits and unrelated report families remain denied without their own permissions.
1. **Given** report permission has been revoked, **When** the person next requests the report, queues its generation or downloads a retained result, **Then** the earlier report permission cannot authorize disclosure; ordinary rate access alone does not restore access to the report.
1. **Given** a direct link or identifier outside the caller's scope or organization, **When** accessed, **Then** it reveals no private identity, amounts, notes, counts or file contents.
1. **Given** revoked authority, **When** queued work executes or a generated result is downloaded, **Then** it cannot use the earlier permission snapshot to bypass revocation.
1. **Given** approved/invoiced time or restricted tasks, **When** an otherwise authorized edit is attempted, **Then** existing state and integrity rules still apply; general edit permission does not unlock records.
1. **Given** an expense receipt or report outside the caller's current scope, **When** its download is requested directly or after revocation, **Then** neither its contents nor identifying metadata are disclosed. General time-management authority does not imply expense-category administration or privileged locked-expense correction.

### User Story 4 - Understand and maintain permission assignments (Priority: P2)

People understand their effective access in My Settings; administrators manage profiles and assignments in Workspace without inconsistent role descriptions.

**Why this priority**: The two handoff screens currently describe different role models. Both must explain the same implemented behavior.

**Independent Test**: Navigate directly to both surfaces as each profile, use keyboard-only controls, cancel/save changes and inspect an assignment's effective scope and change history.

**Acceptance Scenarios**:

1. **Given** a signed-in person, **When** they open their permissions, **Then** they see their own effective capabilities and scope, with no editable privilege controls unless authorized.
1. **Given** an administrator, **When** access changes are saved, **Then** a durable record identifies actor, subject, old/new grants and scopes, time and outcome without recording secrets.
1. **Given** an administrator, **When** they create or apply a custom profile, **Then** dependencies, per-person differences and effects on existing assignees are explicit before save.
1. **Given** a permission depends on another permission, **When** the administrator adds it, **Then** required permissions are included visibly; removing a prerequisite removes its dependants before the complete change is saved.
1. **Given** a person assigned a custom profile, **When** an administrator adjusts that person's permissions, **Then** the differences are visible and can remain person-specific or be saved as a reusable profile.
1. **Given** a reusable custom profile is deleted, **When** existing assignees next use Horae, **Then** their effective permissions, including person-specific adjustments and management scope, are unchanged and retained as person-specific configurations; the deleted template is unavailable for new applications. No automatic Member downgrade or reassignment occurs.
1. **Given** an administrator opens template deletion, **When** reviewing confirmation, **Then** it explains that current users retain their permissions; cancellation changes nothing. Revoking permissions requires a separate explicit change with affected people shown before confirmation.
1. **Given** a narrow viewport, enlarged text or keyboard navigation, **When** inspecting or editing permissions, **Then** labels, scope descriptions, focus, errors and save/cancel controls remain usable.

### User Story 5 - Transition existing accounts without hidden privilege changes (Priority: P1)

An administrator can review how existing roles and project assignments translate before the new model takes effect.

**Why this priority**: Horae is not in production, but it contains imported and development data that must not be reset or silently granted different access.

**Independent Test**: Upgrade a populated fixture covering all old roles, active/inactive accounts and project-specific permissions; compare effective access before and after the confirmed mapping and verify data preservation.

**Acceptance Scenarios**:

1. **Given** an old Manager whose privileges do not match a new profile, **When** migration is previewed, **Then** every grant and revocation is visible and no profile name is treated as evidence of equivalence.
1. **Given** approved migration mappings, **When** migration completes or is safely retried, **Then** identity, organization, work, billing records and assignments are preserved and the last-administrator rule still holds.
1. **Given** future Harvest imports or account linking, **When** external role information is encountered, **Then** it follows an explicit mapping and cannot overwrite local privileges or turn identity matching into administrator admission.

### Edge Cases

- Multiple administrators changing access concurrently, self-demotion and deactivation of an assigned manager.
- A person or project shared by several managers; revoked, inactive and cross-organization assignments.
- Mixed-project timesheets/invoices, partial approval and withdrawal, historical entries and financial totals containing out-of-scope records.
- Filtered approval across week boundaries, empty days, new entries after partial approval and concurrent approve/withdraw/edit operations.
- Custom permission dependencies, unknown grants, renamed/deleted templates and stale edit forms.
- Existing sessions, background work, retained exports, compatibility reads and plugin capabilities after revocation.
- Project-only financial overrides, private administrator notes, absent/zero rates and missing cost currencies.
- Imported roles with no equivalent local profile and retry after an interrupted migration.

## Requirements

### Functional Requirements

- **FR-001**: Provide six distinct built-in profiles: Member, Project Manager, People Admin, Accounting, Executive Manager and Administrator. Profile names MUST correspond to implemented capabilities and scope.
- **FR-002**: Define a complete allowed/denied matrix for Horae's existing time, project, task, client, people, rate, cost, invoice, approval, report, workspace, import/export and integration operations before implementation. Extend the target matrix to approved web-parity domains: expenses/categories/receipts, estimates, retainers and payment recording. Distinguish read, create, edit and lifecycle actions, including privileged corrections and downloads; financial visibility MUST NOT be implied by ordinary project or people management. Record dependencies on owning domain specifications without exposing unimplemented operations as working grants.
- **FR-003**: The proposed built-in boundaries are listed below. Their exact operation-level matrix MUST resolve differences from existing Horae rules and the Harvest reference, including FR-009. Unsupported Harvest products MUST NOT appear as working grants.
- **FR-004**: Provide all six built-in profiles, reusable custom profiles derived from them and per-person permission adjustments. Administrators MUST be able to create, apply and delete custom profiles and see differences between an applied template and a person's effective permissions. Fixed profiles alone do not satisfy this requirement.
- **FR-005**: Project membership, project management and person-management assignments MUST be distinct, organization-scoped relationships. Who may change each assignment and any associated profile transition MUST follow the verified Harvest parity matrix; do not impose an unverified administrator-only restriction on every assignment. Ordinary tracking membership or identity matching MUST NOT silently promote a person.
- **FR-006**: Within the same organization, a capability's authorized self/project/person scopes combine without duplicates. No capability or matching scope means denial. Inactive identity, organization boundaries, task restrictions and business-state locks remain mandatory constraints, not overridable grants.
- **FR-007**: All delivery paths MUST enforce current effective permissions, including direct operations, lists and aggregates, downloads, compatibility interfaces, remote administration and user-initiated background work. Trusted system jobs and plugins MUST retain explicitly documented service capabilities rather than acquiring unrestricted end-user authority.
- **FR-008**: Separate permission to view/edit billable rates, view/edit costs, view invoices, manage invoices, view financial report families and view private notes. A financial report permission MUST authorize that report's defined financial fields and derived amounts, including corresponding exports, within its authorized scope without requiring ordinary rate/cost permissions. It MUST NOT grant general rate access, rate history, editing, unrelated report access or access to underlying source records outside the report projection. Ordinary rate permissions alone MUST NOT grant financial report access. Restricted fields and derived amounts MUST be absent from otherwise unauthorized responses and exports. Aggregate project progress MUST NOT grant detailed personal-note access.
- **FR-009**: Support separately actionable project portions of a person's submission. The effective approval selection MUST be limited by current project/person authority and the displayed person/project/client/date filters; grouping MUST NOT widen it. Approving one project MUST leave other projects' approval state unchanged. Show partial versus complete approval truthfully and retain the actual actor, affected scope and history. Whole-person approval remains possible when its full selected scope is authorized, but MUST NOT be the only supported operation.
- **FR-010**: Permission changes MUST apply on the next authorization check, including still-live sessions, queued execution and result download. Mutations racing a completed revocation MUST not commit using stale authority. Already downloaded data cannot be recalled; no such promise may be displayed.
- **FR-011**: Only administrators may assign profiles or customize privileges. Assignment management MUST follow FR-005's verified capability/scope rules and MUST NOT become an indirect privilege-escalation path. Last-active-administrator protection MUST hold under concurrent changes. Changes MUST be atomic, reject stale edits and leave existing privileges intact on failure.
- **FR-012**: My Settings and Workspace MUST share the same effective permission descriptions. Read-only viewers MUST have a reachable explanation; administrator-only destinations cannot be their sole help path. Loading, empty, forbidden, error and pending states MUST be distinct.
- **FR-013**: Access changes MUST produce durable, organization-scoped audit records of actor, subject, time and actual change. Audit visibility is administrator-only; rejected unauthorized attempts MUST NOT disclose another person's permission configuration.
- **FR-014**: Existing access MUST be inventoried and migration differences approved before cutover. Preserve imported/development records; no account reset is authorized. Do not silently map old Manager to a narrower Project Manager or broader Executive Manager. The migration procedure MUST state any temporarily retained legacy profile and how it is retired.
- **FR-015**: Customization MUST include permission prerequisites and visible dependent-permission changes. Deleting a reusable profile MUST preserve existing assignees' effective permissions and management scope as person-specific configurations while preventing new applications; it MUST NOT downgrade or reassign them. The deletion confirmation MUST explain preservation, and cancellation MUST change nothing. Permission revocation MUST remain a separate explicit change showing the affected people before confirmation. Applying a profile and saving person-specific adjustments MUST be explicit operations. Template update/reapplication effects MUST be verified against Harvest before the contract is finalized; no silent propagation rule may be invented. Administrator status and last-administrator safeguards MUST not be bypassed through an equivalent-looking custom profile. Unknown permissions MUST be rejected.
- **FR-016**: Adapt the permission portions of Settings and Workspace to the selected model using existing controls and styling. Verify both themes, 320/390/768/1440px, short viewports, enlarged text and keyboard operation. Do not redesign unrelated pages as part of this feature.
- **FR-017**: Role and assignment changes MUST NOT recalculate historical money, alter invoice/time state, send unrelated notifications or rebind sign-in identity. Existing exactness, organization isolation and state-transition safeguards remain intact.
- **FR-018**: Permission verification MUST cover every identified entry point and each allow/deny/scope boundary, including negative payload and download checks. A passing role-selector screen alone is not acceptance.
- **FR-019**: Approval, withdrawal, submission editing and date/project lock coverage MUST match the verified Harvest flexible-approval contract, including mixed-project weeks, shorter date ranges, empty days and concurrent operations. Approval state and lock scope are distinct facts. Approved coverage MUST include applicable empty cells; submitted but unapproved work remains editable. Scoped withdrawal from Approval and whole-week withdrawal from Day/Week MUST follow their distinct verified contracts. Independent invoice and administrative locks MUST remain effective after withdrawal. Do not retain whole-week-only storage or Horae's current submitted-entry lock merely because they already exist.
- **FR-020**: Maintain a reference-to-requirement parity matrix for all in-scope permissions and approval behavior, including role defaults, custom-profile limits/lifecycle, assignment authority, approval and withdrawal, and scoped reads/writes. Mark each behavior documented, observed, conflicting or unverified; resolve conflicts using the current Harvest experience before acceptance. No deliberate functional deviation may be accepted without the user's explicit decision.
- **FR-021**: Managed billable-rate read/write permission MUST be scoped to the rate's owning resource: general person rates/history require managing that person; project-owned rates/history, including person/task overrides, require managing that project. Management alone MUST NOT grant financial access, and read permission MUST NOT permit writing. The corresponding organization-wide rate permission may cover these resources without a management assignment, subject to organization and independent operation constraints. Managing a project MUST NOT permit changing participants' general defaults or expose unrelated personal rate history; its authorized financial view may include the effective inherited rate. Managing a person MUST NOT permit accessing their overrides in unrelated projects. Global task defaults require organization-wide rate authority. Authorized person-default edits may affect projects inheriting that rate; explain this consequence without disclosing unauthorized project identities or changing project-specific overrides. Cost permissions remain independent.

### Proposed Built-in Boundaries

These are target responsibilities, not a substitute for the operation-level matrix required by FR-002. Approval authority remains subject to FR-009.

| Profile | Intended responsibility | Important exclusions |
| --- | --- | --- |
| Member | Own time on assigned, available work; permitted project progress | Other people's detailed work, rates/costs, invoices and administration |
| Project Manager | Assigned projects and managed people's/project work; create projects; manage clients/tasks | Financial rates/costs, invoices, people administration and workspace settings |
| People Admin | Manage people and organization-wide time; read projects; reference grants include the Contractor report | Project editing, rates/costs, invoices, profitability/invoicing reports and workspace settings |
| Accounting | Read time/projects/rates/costs; manage clients and invoices; authorized financial reporting | Editing time, projects, people or rates; workspace settings |
| Executive Manager | Manage organization-wide time, projects, people, clients/tasks and invoices; read rates/costs and reports | Editing rates/costs, workspace settings and permission administration |
| Administrator | All implemented organization capabilities, including permissions and configuration | Cannot bypass organization, integrity, identity or last-administrator safeguards |

### Key Entities

- **Capability**: A specific action on an existing product area, with prerequisites and an explicitly defined scope.
- **Built-in/custom profile**: A named permission set, with reusable templates and explicit per-person adjustments.
- **Effective permissions**: The person's resolved capabilities and authorized scopes, not merely their profile label.
- **Management assignment**: Explicit responsibility for a project or person within the same organization.
- **Access change record**: Attributed history of actual privilege/scope changes.
- **Migration mapping**: Reviewed translation from existing access to the new model, including explicit differences.
- **Scoped approval**: An attributed decision over selected work, with aggregate partial/completed state independent of other projects' decisions and the applicable locks.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All six profiles pass the final operation-level matrix: every documented allowed action succeeds with valid input and every documented denial remains denied through all applicable paths.
- **SC-002**: Scope tests disclose zero unauthorized identities, notes, rates, costs, totals or downloadable records, including mixed-project and cross-organization fixtures.
- **SC-003**: Revocation, stale edits and concurrent administrator changes pass without stale-authority commits or loss of the last active administrator.
- **SC-004**: Every approved migration fixture preserves business records and matches its reviewed access changes; retry introduces no duplicate assignments or privilege drift.
- **SC-005**: Settings and Workspace show the same effective access and pass keyboard, theme and viewport acceptance with no fake controls or unsupported grants.
- **SC-006**: Regression checks cover time entry, approvals, project creation/editing, reports/exports, invoices, importing, authentication and background/integration access. Unaffected business results remain exact and unchanged.
- **SC-007**: The complete permission/parity matrix, migration mapping and governance updates are resolved before implementation acceptance. Every in-scope reference behavior has a passing acceptance check or an explicitly approved deviation; neither fixed-only profiles nor whole-person-only approvals can satisfy this feature. The broader design delivery is not completed by this feature.
- **SC-008**: Two-project/two-approver scenarios pass approval, partial-state display, filtered selection, scoped/whole-week withdrawal, concurrent mutation and date/project lock checks including empty cells; approving A never changes B's approval state or exposes B's private records.
- **SC-009**: Custom-profile creation/application, prerequisite changes, person-specific adjustments and template deletion pass persistence and effective-access checks without unintended changes to other assignees.

## Assumptions

- Existing authentication and the single-organization deployment remain; new identity providers, login mechanisms and multi-organization administration are outside scope.
- The user explicitly selected Harvest parity for both custom permissions and scoped approvals. These scope decisions are settled; remaining uncertainty concerns reference evidence and the detailed contract, not permission to simplify the feature.
- The confirmed web scope requires expense, estimate, retainer and payment permission contracts even before those domains are implemented. Feature 016 owns expense behavior; its operation-level grants remain unresolved and must not be inferred from legacy role names. This permission feature does not implement those domains or authorize SaaS subscription billing, native applications, Forecast or new integrations. Existing imports and plugin/service access retain explicit regression coverage.
- Current data is preserved even though the application is not in production. Verification uses disposable fixtures, not destructive changes to the user's account or Harvest.
- Constitution 1.1.0 records the approved six-profile/custom-permission target and transition safeguards in this branch. Older feature permission statements still require reconciliation with the verified operation matrix; the amendment does not activate runtime access or approve migration mappings.
- This feature resolves permission dependencies of Project Detail, Clients, Settings and Workspace; it does not authorize their unrelated profile, notification, invitation, backup or deletion decisions.
