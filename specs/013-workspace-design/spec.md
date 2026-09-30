# Feature Specification: Workspace Administration

**Feature Branch**: `feat/workspace-design`

**Created**: 2026-09-30

**Status**: Draft — invitation, backup and deletion policies need confirmation

**Input**: Implement the Workspace portion of Project Detail + Clients list/detail + Settings/Workspace using the checked-in design, preserving existing authentication, permissions, financial history and Harvest imports. Personal Settings remains a separate required surface, not absorbed into or completed by this feature.

## Clarifications

### Session 2026-09-30

- Q: Keep the handoff's three fixed roles or implement Harvest permission parity? → A: The user requested Harvest parity: six built-in profiles, reusable custom profiles, per-person adjustments, distinct managed-people/project assignments and scoped approvals. Feature 015 owns the shared authorization and migration contract. Workspace must administer and explain that model rather than preserve the handoff's three-role limitation.

The dependency is `specs/015-scoped-permissions/` on `feat/scoped-permissions` ([PR #212](https://github.com/numtide/horae/pull/212)) until merged. Current runtime authorization remains in force until its reviewed cutover. This clarification does not approve automatic privilege migration or claim that the new policy is implemented.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand and manage workspace membership (Priority: P1)

As an administrator, I can see who belongs to the workspace, invite teammates and manage access without losing their recorded work.

**Why this priority**: People already has working administration. The redesign must preserve it while making invitation and membership status truthful.

**Independent Test**: List active/inactive people and pending invitations, invite a new teammate, accept through the existing sign-in flow, change a role and deactivate/reactivate a person. Compare their historical work before and after.

**Acceptance Scenarios**:

1. **Given** people in different states, **When** I open People, **Then** names, emails, roles, status and counts come from actual records; missing activity dates are explicitly unknown.
1. **Given** an administrator and valid invitation recipients, **When** invitations are submitted, **Then** each recipient has a truthful pending/delivery result; a failed send never appears as delivered or accepted.
1. **Given** a valid invitation under FR-006, **When** the recipient authenticates and accepts, **Then** the intended identity joins with the authorized role, without creating duplicate identities or bypassing sign-in.
1. **Given** an invalid, expired, revoked or already used invitation, **When** someone tries to accept it, **Then** it cannot grant fresh access.
1. **Given** an active person with time and invoices, **When** I deactivate them, **Then** subsequent access is denied while their identities and history remain intact.
1. **Given** concurrent attempts to remove the final administrator, **When** changes are applied, **Then** at least one active administrator remains.
1. **Given** the Roles & permissions page, **When** I compare it with actual actions, **Then** it describes feature 015's implemented six profiles, custom templates, per-person differences and effective person/project scopes, using the same explanations as My Settings.
1. **Given** an authorized people manager without workspace-administration authority, **When** they open People directly, **Then** only their permitted people operations and records are reachable; General, permission administration, imports, exports/backups and audit remain separately protected.
1. **Given** an administrator changing a profile, custom adjustment or management assignment, **When** the change is saved or cancelled, **Then** feature 015's current-authority, stale-edit, audit and last-administrator rules apply; a read-only matrix alone does not satisfy permission management.

### User Story 2 - Change workspace preferences safely (Priority: P1)

As an administrator, I can maintain the workspace identity and reporting preferences, understanding what a change affects before saving it.

**Why this priority**: Timezone, week start, currency and approval settings influence existing workflows and cannot be decorative controls.

**Independent Test**: Save/reload valid settings, reject invalid and stale edits, then exercise timesheet, reporting and invoice preparation across changed date boundaries and permissions.

**Acceptance Scenarios**:

1. **Given** saved settings, **When** I open General, **Then** it shows the real workspace name, deployment address, timezone, week start, default currency and approval policy.
1. **Given** edited fields, **When** I explicitly save valid values, **Then** the saved values survive reload and the workspace header reflects the new name. Cancel leaves saved state unchanged.
1. **Given** an invalid name, timezone or currency, or a conflicting concurrent edit, **When** I save, **Then** input is retained, the problem is identified and no partial settings are applied.
1. **Given** a changed timezone or week start, **When** I use date-sensitive screens, **Then** their current-day/week boundaries agree; historical work dates, recorded durations and finalized invoices are not rewritten.
1. **Given** existing currency-dependent rates or costs, **When** a currency change would reinterpret them, **Then** saving is blocked with a reason rather than silently relabeling money.
1. **Given** approval is required, **When** new hourly time is prepared for invoicing, **Then** only approved eligible time can be selected. A preview made before a policy change cannot bypass the current policy when generating the invoice.
1. **Given** approval is not required, **When** invoices are prepared, **Then** existing eligibility rules remain: open/approved time may qualify, but submitted, running or already invoiced time does not become newly billable through the toggle.

### User Story 3 - Export data and inspect actual backup results (Priority: P2)

As an administrator, I can obtain usable copies of workspace data and know whether an automatic backup really completed.

**Why this priority**: Data portability and recovery must provide downloadable artifacts, not simulated success messages.

**Independent Test**: Export a populated multi-currency workspace, verify entity identities and totals, simulate interrupted/failed generation and restart, and exercise the approved backup contract from FR-014.

**Acceptance Scenarios**:

1. **Given** a workspace with people, clients, projects, tasks, time and invoices, **When** I export it, **Then** the archive contains documented business-data files with exact amounts, currencies, dates and relationships from a consistent snapshot.
1. **Given** only a time export is requested, **When** I download it, **Then** its scope is explicit and its detailed-time columns match the supported interchange format.
1. **Given** an export is pending or fails, **When** I inspect its status or retry, **Then** there is no fake download, partial-success archive or duplicate completed artifact for the same accepted request.
1. **Given** the approved automatic-backup mode is configured, **When** its scheduled time occurs or the application restarts, **Then** the UI reports the actual outcome and the latest successful artifact remains distinguishable from a newer failure.
1. **Given** missing storage or delivery prerequisites, **When** I attempt to enable automatic backups, **Then** the UI explains the missing prerequisite and does not claim protection is enabled.
1. **Given** access is revoked, **When** an old download link is reused, **Then** it cannot bypass current authorization; credentials, live sessions and secret connection material never appear in a portable business export.

### User Story 4 - Review administrative history (Priority: P2)

As an administrator, I can understand which administrative actions actually occurred, who initiated them and when.

**Why this priority**: Membership, settings, imports and data-management changes need traceability without exposing secrets.

**Independent Test**: Perform each supported administrative operation, cancel and retry others, then reconcile the audit page with committed outcomes and load older events across page boundaries.

**Acceptance Scenarios**:

1. **Given** recorded administrative events, **When** I open Audit log, **Then** I see newest-first timestamps, readable action summaries and real actors, including an explicit system actor when appropriate.
1. **Given** a successful administrative mutation, **When** I inspect history, **Then** exactly one committed event describes that change; cancellation and validation failure do not appear as successful mutations.
1. **Given** older events, **When** I load more, **Then** pages have deterministic ordering without duplicates or omissions, including equal timestamps and newly arriving events.
1. **Given** missing historical coverage or an inactive actor, **When** I inspect the log, **Then** coverage and identity are labeled honestly; past events are not fabricated or lost when access changes.

### User Story 5 - Handle workspace deletion explicitly (Priority: P3)

As the workspace administrator, I can understand the real deletion policy and, if enabled under FR-019, confirm the exact consequences before any destructive operation.

**Why this priority**: The design includes an irreversible operation but does not define its self-hosted recovery and retained-export behavior.

**Independent Test**: In a disposable workspace only, exercise the approved deletion path, cancellation, revoked authority, concurrent jobs and interrupted execution; verify the specified retention and sign-in result.

**Acceptance Scenarios**:

1. **Given** the approved deletion policy, **When** I open General, **Then** the UI states the actual supported procedure rather than presenting a no-op destructive button.
1. **Given** browser deletion is approved, **When** I request it, **Then** confirmation uses the real workspace identity and current counts and explains irreversible and retained data precisely.
1. **Given** a cancelled or unauthorized request, **When** confirmation ends, **Then** no workspace, membership, time, invoice, import mapping or artifact changes.
1. **Given** an approved confirmed deletion, **When** it completes, **Then** all in-scope records, sessions and jobs follow the approved policy without affecting another organization or arbitrary host files.

### Edge Cases

- Duplicate/case-varied invitation emails, existing or imported inactive identities, wrong signed-in recipients and attempts to invite someone already belonging elsewhere.
- Last-administrator removal, stale role changes, revoked permissions while a form/export is open and retries after a lost acknowledgement.
- Empty people lists, no audit events, no completed backups, loading and retryable errors are different states.
- Week boundaries near year changes, daylight-saving transitions, unsupported stored weekdays and historical approval periods must not be silently coerced.
- Existing money that depends on the organization's currency must not be redenominated by changing a default.
- Large exports, multiline/non-Latin text, spreadsheet formula injection, failed artifact writes and expired downloads.
- Deletion racing with an import, an invoice generation, a timer or an export; no false success while work can recreate removed data.
- At 320 CSS pixels, short viewports and 200% text size, navigation, dialogs and actions remain reachable; wide data tables scroll within their labeled container.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Workspace MUST provide the designed People, Roles & permissions, General, Export & backups and Audit log surfaces, retain the working Harvest importer destination and preserve the existing application shell.
- **FR-002**: Workspace settings, permission administration, imports, exports/backups and administrative audit MUST remain administrator-only. People-directory, people-management and assignment operations MUST instead enforce the verified capability/scope rules of feature 015, including reachable authorized destinations for non-administrator profiles; the current admin shell MUST NOT silently deny required Harvest access. Every operation MUST recheck current authorization and organization scope; hiding navigation alone is insufficient.
- **FR-003**: Roles & permissions MUST expose feature 015's implemented Member, Project Manager, People Admin, Accounting, Executive Manager and Administrator profiles, reusable custom profiles and per-person adjustments. Management assignments, prerequisite/dependent changes, template application/deletion and effective-access explanations MUST follow that shared contract. My Settings and Workspace MUST describe the same saved access. The handoff's three fixed roles are superseded, not an accepted functional simplification; runtime migration requires feature 015's reviewed cutover.
- **FR-004**: People MUST show actual names, emails, roles and active/inactive/invitation states with reconciled counts. Last-active information MUST use a defined observed activity event, not creation or edit timestamps; unknown historical values MUST be labeled unknown. Prototype seat quotas MUST NOT be invented for a self-hosted installation.
- **FR-005**: Role/activation changes MUST preserve historical work and existing final-administrator protection, including concurrent changes. Creation of an account MUST NOT be represented as a sent or accepted invitation.
- **FR-006**: Invitation admission requires confirmation: [NEEDS CLARIFICATION: Restrict invitations to named recipients with individual expiring links, or also support the design's reusable workspace-wide join link? The latter needs an explicit admission/role policy.]
- **FR-007**: Invitations MUST show per-recipient validation, pending delivery, failure, expiry, revocation and acceptance outcomes. Acceptance MUST use the existing authenticated identity and prevent replay, foreign-organization access, unintended reactivation and role escalation. Mail failure MUST retain a retryable invitation without claiming successful delivery.
- **FR-008**: General MUST load actual settings and provide explicit Save/Cancel with field validation, retained failed input, pending guards and stale-edit handling. Missing or invalid stored values MUST not silently become prototype defaults.
- **FR-009**: Workspace timezone/week start MUST consistently govern workspace-relative reporting/calendar boundaries without rewriting historical dates, durations, finalized invoices or existing approval decisions. Changes that cannot preserve coherent pending approval periods MUST be rejected with a reason. Existing valid weekdays outside the prototype's Monday/Sunday choices MUST remain representable.
- **FR-010**: Default currency changes MUST affect only eligible future defaults. Changes that would reinterpret stored rates/costs MUST be rejected unless a separately approved migration handles them. No automatic exchange conversion or retroactive repricing is permitted.
- **FR-011**: Require approval MUST enforce current policy at invoice review and generation, preserving invoice locks and fixed-fee rules. Submission, scoped approval and withdrawal behavior MUST agree with feature 015; this screen MUST NOT reintroduce an unconditional submitted-work edit lock. Existing workspaces retain current invoice eligibility until an explicitly reviewed policy change; the prototype's enabled switch is not a migration default.
- **FR-012**: The Workspace URL MUST identify the actual deployment, not an invented tenant slug. Proposed self-hosted behavior is a read-only address with operator-managed changes; this deviation needs acceptance before the General surface is accepted.
- **FR-013**: Full export MUST document its business-entity inventory, preserve identities/relationships and exact per-currency totals, include a format/version manifest and use spreadsheet-safe text encoding. Exclusions, coverage and snapshot time MUST be explicit. A business-data archive MUST NOT be labeled a proven disaster-recovery backup.
- **FR-014**: Automatic-backup scope requires confirmation: [NEEDS CLARIFICATION: Should weekly backups be portable business-data archives matching Export zip, operator-managed recoverable infrastructure backups, or both? Storage ownership and the meaning of Download latest depend on this choice.]
- **FR-015**: Automatic backups MUST default to off for existing installations and enable only with a real configured destination. The proposed schedule is Monday 03:00 in workspace time with 90-day artifact retention. Status, size, completion time and download availability MUST come from actual jobs/artifacts, including restart, failure and expiry. Existing operator-managed files MUST not be deleted by this feature.
- **FR-016**: Artifact generation MUST be bounded, recoverable after restart and idempotent for accepted requests. Download authorization MUST be independent of guessable filenames; a missing, incomplete, corrupt or expired artifact MUST yield a truthful error.
- **FR-017**: Audit MUST cover committed workspace settings, membership/invitation, import lifecycle, export/backup and approved deletion operations. Events MUST have stable identity, action, time, actor/system identity and organization scope; no raw secrets, tokens, invite links or private connection payloads may be rendered or stored in event details.
- **FR-018**: Audit paging MUST be deterministic and show real coverage/counts. Retain at least 12 months of newly recorded events; do not purge existing history in this feature. The prototype's conflicting 90-day empty-state message MUST NOT be copied. No prior administrative events may be invented during migration.
- **FR-019**: Deletion policy requires confirmation: [NEEDS CLARIFICATION: Implement browser-confirmed permanent deletion, including the exact treatment of retained exports/backups and later sign-in/bootstrap, or keep destructive reset operator-only and explain that procedure in General? The prototype's 30-day download promise has no existing service contract.]
- **FR-020**: If browser deletion is approved, it MUST require current administrator authority, explicit typed identity confirmation, actual affected counts and protection against concurrent writes/jobs. Replayed/interrupted requests MUST not delete another scope or report success prematurely. Real/imported data MUST not be used for destructive acceptance tests.
- **FR-021**: Every visible action MUST have a verified outcome, accessible name, keyboard interaction and pending/error state. Dialog cancellation MUST preserve saved data and restore focus. Both themes and narrow/enlarged layouts MUST preserve shared control behavior across the rest of Horae.
- **FR-022**: Personal Settings, Clients and unfinished Project Detail remain required by the wider delivery. Completing this specification or one Workspace section MUST NOT mark those surfaces complete.

### Key Entities *(include if feature involves data)*

- **Workspace preferences**: Display identity, actual deployment address, timezone, first weekday, default currency and approval requirement with conflict-safe edits.
- **Membership**: Existing person identity, feature 015's profile/custom adjustments and person/project management assignments, activation state and explicitly defined last activity; historical attribution survives deactivation.
- **Invitation**: Workspace, recipient/admission scope, intended role, sender, expiry, delivery state, revocation and acceptance. Not interchangeable with an active account.
- **Portable export**: Snapshot scope, versioned business-data manifest, exact entity files, requester, processing state and authorized artifact.
- **Backup policy/run**: Approved backup kind, destination ownership, schedule, retention and truthful artifact/result metadata.
- **Audit event**: Immutable committed administrative fact with actor/system identity, scope, event time and safe human-readable details.
- **Deletion request**: Explicitly approved scope, confirmation, processing outcome and post-deletion artifact/access policy; pending FR-019.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every Workspace section is directly reachable by an administrator, and non-administrator profiles can reach their feature 015-authorized people operations without gaining workspace administration. Every unauthorized direct entry, read, mutation and download is denied without exposing restricted data. Tests cover all six built-in profiles, custom profiles, per-person adjustments and revoked assignments.
- **SC-002**: Membership and invitation counts match actual records, every invitation has a testable delivery/admission outcome, and concurrent access changes cannot remove the last active administrator.
- **SC-003**: Valid preference changes survive reload and affect all documented consumers consistently; invalid, stale and cancelled changes produce zero partial writes and zero monetary/history reinterpretation.
- **SC-004**: Exported fixture identities, relationships, minutes and per-currency amounts reconcile exactly with the declared snapshot; corrupted or failed artifacts are never presented as successful downloads.
- **SC-005**: Each supported administrative mutation has exactly one corresponding committed audit event; paging retrieves all retained fixture events without duplicates, omissions or secrets.
- **SC-006**: Invitation, settings, export/backup and approved deletion flows pass keyboard-only, 320-pixel and 200%-text checks with no unreachable actions or page-level horizontal overflow.
- **SC-007**: All designed sections and policy-sensitive actions have verified implementations or explicitly accepted deviations. Pending FR-006/014/019, the proposed URL deviation and feature 015's shared permission/migration acceptance prevent whole-feature acceptance.
- **SC-008**: Existing timesheet, project, invoice, report, importer and user-access regression checks continue to pass with the new administration surfaces.

## Assumptions

- This is single-workspace administration for the existing self-hosted installation, not tenant provisioning, paid seats or Harvest subscription billing.
- Existing sign-in remains authoritative; invitations do not add a password, magic-link login or a second authentication mechanism.
- Individually addressed invitations, if chosen, expire after seven days and are single-use; resending invalidates the earlier acceptance link. Role defaults to member and elevated roles require explicit administrator selection.
- Workspace name changes do not silently overwrite invoice issuer branding. Deployment address changes remain operator-owned unless explicitly approved otherwise.
- Last active means the latest successful authenticated application request; pre-feature history is unknown until a new qualifying activity occurs. Background jobs do not make a user appear active.
- Newly introduced workspace timezone preserves the installation's documented pre-feature reporting boundary; planning must identify that boundary rather than assume browser-local time. Existing week start/currency and invoice eligibility are preserved.
- Backup restore claims depend on FR-014; no automatic restoration or new external storage account is implied. Retention only applies to feature-owned artifacts under the approved policy.
- Source of truth: `design/project/app/09_Workspace.dc.html`, its linked invitation surface, current domain/authentication rules and the user-confirmed permission contract in feature 015. Harvest is behavioral reference, not authority to import unrelated commercial features; the three-role handoff does not override the confirmed six-profile/custom-permission scope.
- This draft proposes new persisted preferences, invitation/job/activity metadata and audit coverage. Planning/implementation may not treat unresolved policy choices as approval of those contracts.
