# Feature Specification: Personal Settings

**Feature Branch**: `feat/personal-settings-design`

**Created**: 2026-09-30

**Status**: Draft — Harvest permission parity confirmed; profile ownership and notification delivery need confirmation

**Input**: Implement the personal Settings portion of Project Detail + Clients list/detail + Settings/Workspace from the handoff. Preserve working theme/plugin controls, current sign-in, organization boundaries and financial privacy. Workspace administration is specified separately in feature 013.

## Clarifications

### Session 2026-09-30

- Q: Preserve three organization roles or implement Harvest permission parity? → A: The user requested Harvest parity: six built-in profiles, reusable custom profiles, per-person adjustments, distinct managed-people/project assignments and scoped approvals. Feature 015 owns that shared contract; Settings must reflect its implemented effective permissions, not retain a three-role substitute.

The dependency is `specs/015-scoped-permissions/` on `feat/scoped-permissions` ([PR #212](https://github.com/numtide/horae/pull/212)) until merged. Its detailed matrix, migration and runtime acceptance remain pending. This clarification confirms the required outcome; it does not claim that the current application already supports it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Maintain my basic information and appearance (Priority: P1)

As a signed-in person, I can understand my account identity and maintain approved personal profile fields without changing my sign-in identity or someone else's account.

**Why this priority**: Basic information is the main designed Settings surface; existing theme preferences must survive the redesign.

**Independent Test**: Open Settings as each role, save/reload approved profile changes, cancel another edit, test invalid input/photo failure and sign in again. Verify only the intended person's editable profile changed.

**Acceptance Scenarios**:

1. **Given** an existing account, **When** I open Basic info, **Then** I see its actual name, work email, timezone and photo/fallback state, not the prototype identity.
1. **Given** approved locally editable fields under FR-003, **When** I save valid changes, **Then** they survive reload and subsequent sign-in. Cancel and validation failure preserve saved data.
1. **Given** a legacy full name, **When** separate name fields are introduced, **Then** existing display names are retained until explicitly edited; the application does not guess which words are surnames.
1. **Given** a valid photo replacement or removal, **When** I save, **Then** the actual image/fallback appears consistently. An invalid or failed upload does not remove the previous saved image.
1. **Given** a personal timezone change, **When** I view personal timestamps/reminders, **Then** they use the selected zone without changing stored work dates or the workspace's approval/reporting week.
1. **Given** a chosen theme on this device, **When** I navigate/reload/sign in again, **Then** its existing behavior remains and no account-wide theme synchronization is silently introduced.

### User Story 2 - Understand my rates, assignments and effective permissions (Priority: P1)

As a person using Horae, I can understand which work and information I can access, without a settings screen granting privileges I do not have.

**Why this priority**: The handoff's role profiles and people-management claims differ from current authorization, so truthful presentation is essential.

**Independent Test**: Compare the Settings display with real tracking, reporting, approval, invoice and administration access for all six built-in profiles, reusable custom profiles, per-person adjustments and project/person assignments, including revoked access.

**Acceptance Scenarios**:

1. **Given** projects I can track against, **When** I open Assigned projects, **Then** authorized destinations and the reason for access are accurate; inactive or restricted work cannot become trackable from this page.
1. **Given** effective people-management assignments under FR-007, **When** I open Assigned people, **Then** the page reflects actual scope rather than inferring management from sharing a project.
1. **Given** my current permission assignment, **When** I open Permissions, **Then** the page explains the same effective capabilities, person/project scopes and template differences as Workspace. Changing a visual selection cannot elevate my privileges.
1. **Given** rate visibility or editing is unavailable, **When** I open Rates, **Then** the page explains the restriction without leaking amounts or rendering an enabled no-op edit action.
1. **Given** accessible rates, **When** I inspect them, **Then** zero and unset values differ, their currencies are explicit and defaults are not misrepresented as effective rates for every project.
1. **Given** authority changes while Settings is open, **When** data is refreshed or a change is attempted, **Then** current permissions are rechecked and restricted data/actions are removed.

### User Story 3 - Receive only the notifications I have chosen (Priority: P2)

As a person, I can choose meaningful reminders and event notifications, understand their schedule and stop receiving them when I opt out.

**Why this priority**: A saved checkbox is not a working notification feature; the delivery and authorization contracts must be real.

**Independent Test**: Save preferences, trigger each approved reminder/event against a captured test mailbox, exercise restart/retry and revoke permissions or opt out before delivery. Verify recipients, content, timing and suppression.

**Acceptance Scenarios**:

1. **Given** configured delivery and the approved notification scope in FR-010, **When** I save preferences, **Then** they persist per person and each enabled option names its actual event or schedule.
1. **Given** mail is unavailable or a prerequisite is missing, **When** I inspect or change an option, **Then** the UI explains the limitation and does not claim that reminders are being delivered.
1. **Given** an opted-in daily reminder or weekly summary, **When** its scheduled time arrives, **Then** it uses the approved timezone/period and only the recipient's authorized data; restart/retry does not intentionally create a second notification for the same occurrence.
1. **Given** a team reminder, **When** the relevant submission is already complete, **Then** it is suppressed. Any displayed deadline must come from actual workspace configuration, not the prototype's Sunday 23:00 example.
1. **Given** an approval or project event, **When** notification delivery occurs, **Then** recipients still have the corresponding authority and an applicable opt-in preference.
1. **Given** I opt out or my account is deactivated before delivery, **When** queued work is processed, **Then** it does not send the notification. Failures remain retryable and are not reported as delivered.

### User Story 4 - Reach real integrations and account-security controls (Priority: P2)

As a signed-in person, I can find the integrations and account-security actions Horae actually supports without encountering placeholder configuration or weakening authentication.

**Why this priority**: The design links to these sections, but their contents are explicitly placeholders and do not authorize a new authentication system.

**Independent Test**: Open direct Settings links and use available integration/security actions as each role; verify plugin visibility, admin-only Harvest management and existing logout/sign-in behavior.

**Acceptance Scenarios**:

1. **Given** installed plugins, **When** I open Integrations, **Then** their actual status remains available with the same visibility as before the redesign.
1. **Given** an administrator, **When** I open Harvest management from Settings, **Then** I reach the existing importer without starting a job; other roles do not gain its authority.
1. **Given** an externally authenticated account, **When** I open Security, **Then** it states where sign-in is managed and offers supported session actions, without pretending Horae can edit provider passwords or configure nonexistent two-factor authentication.
1. **Given** an account-setting destination cannot be derived safely from configured identity management, **When** I inspect the email/security help, **Then** it gives accurate guidance instead of a fabricated external link.

### Edge Cases

- Single-word/non-Latin/long names, absent photos and legacy identity fields must retain their meaning.
- Invalid, oversized or misleadingly named image files; interrupted upload, replacement races and deleted photo references.
- Timezone changes near midnight, daylight-saving transitions and a timezone different from the workspace's reporting zone.
- Hidden, zero, missing and currency-incompatible rates; shared project membership is not a people-management relationship.
- Inactive accounts, revoked assignments, foreign identifiers and direct navigation to a Settings subsection.
- Duplicate events, mail transport failure, restart, unsubscribe while queued and changes to authority before delivery.
- Missing workspace reminder configuration or unimplemented project deletion must not produce fake enabled notification options.
- Narrow/short viewports, 200% text, keyboard-only operation and page transitions with unsaved or pending changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: My settings MUST provide the handoff's Basic info, Rates, Assigned projects, Assigned people, Permissions, Integrations, Notifications and Security journeys with real outcomes or explicitly accepted deviations, preserving the shared application shell.
- **FR-002**: Reads and writes MUST remain scoped to the signed-in person and current organization. Viewing one's settings MUST NOT grant editing authority over roles, others' profiles, workspace preferences or financial configuration.
- **FR-003**: Profile ownership needs confirmation: [NEEDS CLARIFICATION: Allow local editable names/photo while keeping work email and sign-in identity managed by the existing identity flow, or keep names/photo provider-managed too? The handoff expects editable names and photo, but current accounts only persist a full name and email.]
- **FR-004**: Approved profile edits MUST validate input, retain failed edits, survive reload and avoid stale overwrites. Imported/legacy full names MUST not be automatically split. Stored authentication bindings and existing invoice identities MUST not change through a profile edit.
- **FR-005**: Photo handling MUST preserve the prior image on failure, reject unsafe/invalid uploads and support removal with an initials fallback. No arbitrary remote image fetch, secret URL exposure or executable upload is permitted. Profile-image persistence is new functionality, not simulated file selection.
- **FR-006**: Personal timezone MUST use a real named timezone and govern only documented personal displays/reminders. It MUST NOT silently change organization week start, stored work dates or approval periods. Theme MUST retain its existing device-local semantics and available options.
- **FR-007**: Permissions and Assigned people MUST consume feature 015's effective-access contract: Member, Project Manager, People Admin, Accounting, Executive Manager and Administrator, reusable custom profiles, per-person adjustments and distinct person/project management assignments. A profile label or project membership MUST NOT substitute for effective capabilities and scope. This read-only explanation MUST agree with Workspace; privilege administration remains governed by feature 015.
- **FR-008**: Assigned projects, people and permission descriptions MUST agree with current authorized operations and explain conditional/project-specific scope. No prototype “profile no longer available” warning or empty assignment claim may appear without supporting data. Non-admins MUST NOT be sent to an admin-only help page as their sole permissions explanation.
- **FR-009**: Rates MUST enforce feature 015's implemented financial visibility/edit authority, exact amounts and explicit currency. Being the subject of a rate MUST NOT itself grant visibility or editing. Any proposed rate editor needs a confirmed repricing contract; the prototype's placeholder “Set your rates” text does not authorize self-service financial changes. A restricted editor MUST have a truthful explanation or authorized working destination. Until the shared policy cutover, existing runtime boundaries remain unchanged rather than being replaced piecemeal by this screen.
- **FR-010**: Notification delivery needs confirmation: [NEEDS CLARIFICATION: Implement all operational email categories (daily/team reminders, weekly summary, authorized approval/project events) with configurable schedules and real delivery, or approve a smaller initial set explicitly? No notification option may count as implemented merely because a preference is saved.]
- **FR-011**: Notifications MUST default to opt-out for existing accounts, be independently configurable where meaningful and expose their real prerequisites. Promotional emails are not an existing service; no marketing opt-in or delivery promise may be fabricated.
- **FR-012**: Scheduled/event delivery MUST use the current preference, active identity, verified recipient and authorization at delivery time, preserve a stable occurrence identity across retries and record actual send outcomes. Weekly summaries MUST reconcile exactly with their labeled period, without exposing other people's rates or entries.
- **FR-013**: Team reminders MUST depend on actual workspace deadline/reminder configuration and the applicable submission status. People/project approval notifications MUST use the approved FR-007 scope. Project-deletion notices MUST depend on an implemented, approved deletion event rather than an archive event with a false label.
- **FR-014**: Integrations MUST preserve installed-plugin information and existing admin-only Harvest management. Security MUST preserve existing authentication and logout, with no invented password, multi-factor, email-change or cross-session management controls. Placeholder sections are not an authorization to add new identity providers.
- **FR-015**: The prototype's welcome-home checkbox MUST not persist a no-op value. There is currently no welcome-home destination; either a real destination must be specified and implemented or omission must be explicitly accepted. Existing root navigation remains unchanged until then.
- **FR-016**: Save/cancel, navigation guards, loading/empty/error/retry, focus restoration and pending state MUST be consistent with existing controls. Direct subsection navigation and browser history MUST resolve to the real section without losing uncommitted input silently.
- **FR-017**: Both themes, 320/390/768/1440px, short viewports and 200% text MUST keep controls reachable and labels readable. Reuse existing style tokens and controls; no replacement of the CSS framework or global defaults is implied.
- **FR-018**: Workspace administration, Clients and unfinished Project Detail remain separate required outcomes. This specification is not implementation or completion of the wider delivery.

### Key Entities *(include if feature involves data)*

- **Personal profile**: Existing identity plus approved editable name fields, photo and timezone; authentication identity remains a separate concern.
- **Effective access summary**: Feature 015's current profile, template differences, effective capabilities, trackable projects and explicit person/project management scopes; shared with Workspace.
- **Visible rates**: Authorized amounts, currencies and provenance; displayed defaults need not equal effective project-specific rates.
- **Notification preferences**: Per-person enabled categories, actual schedule/zone and prerequisite state.
- **Notification occurrence**: Authorized recipient, event or reporting period, delivery state, stable retry identity and safe failure information.
- **Integration/security information**: Existing plugin state, authorized integration destinations and supported sign-in/session actions.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every Settings journey is reachable through direct navigation and the designed navigation, with no sample data, inert save controls or fictitious permission profiles.
- **SC-002**: Approved profile edits persist through reload and sign-in; invalid, cancelled, stale or unauthorized edits cause zero unintended identity, financial or cross-user changes.
- **SC-003**: Every displayed access/rate claim matches tested operations for all six built-in profiles, custom profiles, per-person adjustments and person/project scopes; no restricted amount or person is disclosed after revocation. Settings and Workspace explanations reconcile for the same saved assignment.
- **SC-004**: Every enabled notification category produces a verified test-mailbox outcome with correct recipient, labeled period/event and exact totals; opt-out, revocation and duplicate processing are covered.
- **SC-005**: All supported flows pass keyboard-only, both-theme, narrow and enlarged-text checks without inaccessible actions or silent input loss.
- **SC-006**: Existing theme/plugin, authentication, timesheet, approval, project, report, invoice and importer regression behavior remains intact.
- **SC-007**: FR-003/010 and the rate/home/security/marketing deviations are explicitly resolved, and feature 015's shared permission behavior is implemented and verified, before whole-feature acceptance; a read-only profile or saved notification switches alone cannot satisfy the feature.

## Assumptions

- Existing sign-in, verified identity binding and the single-organization model remain authoritative. New account-security products are outside this visual integration unless separately approved.
- Locally editable profile data, if approved, does not automatically synchronize back to the identity provider. Provider sign-in must not overwrite approved local preferences.
- The default personal timezone inherits the effective workspace reporting timezone until explicitly changed; the exact legacy fallback must be identified during planning, not inferred from the mockup's Madrid example.
- Notification verification uses captured mail and disposable data, not unsolicited mail to real imported users. Missing transport configuration cannot be hidden behind a successful preference save.
- The full operational notification set is required unless a smaller scope is explicitly approved. Unsupported promotional/home/security placeholders require accepted deviations, not silent deletion from the delivery.
- New persisted profile, photo, timezone, preference and notification state requires reviewed contracts after clarification; this draft does not authorize unresolved schema or access-control choices.
- Source: `design/project/app/08_Settings.dc.html`, existing Settings behavior, current domain/authentication rules and the user-confirmed permission contract in feature 015. The prototype's static profile warnings are not evidence of a person's actual assignment.
