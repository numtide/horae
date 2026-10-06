# Feature Specification: Clients List and Detail

**Feature Branch**: `feat/clients-design`

**Created**: 2026-09-30

**Status**: Draft — awaiting scope and archive-policy clarification

**Input**: Implement Clients list/detail from the design handoff, compare the flows with Harvest, and preserve existing project, invoice, import and access-control behavior. This is the Clients portion of Project Detail + Clients + Settings/Workspace, not a replacement for that wider objective.

## Clarifications

### Session 2026-10-02 — Authorized implementation increment

- Q: May the existing client journeys ship before the complete permission/contact/lifecycle package? → A: The user explicitly authorized the [Clients MVP increment](increments/mvp/spec.md): name search, lifecycle/currency filters, real detail, existing-field create/edit and contextual project/invoice navigation, preserving current runtime authorization and existing lifecycle behavior. Contacts, new archive policy, deletion, bulk actions and activation of feature 015 remain outside this increment. This does not complete or remove any full-feature requirement below.

### Session 2026-10-01

- Q: Does client access retain the three-role boundary or use the confirmed Harvest permission model? → A: Reuse the user's 2026-09-30 decision recorded in feature 015: six built-in profiles, custom profiles, individual adjustments and applicable scopes. This is propagation of an existing answer, not a new scope approval. Contact cardinality and archive policy remain unresolved.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Find and review clients (Priority: P1)

As a person managing client work, I can find a client and understand its project activity without opening every project separately.

**Why this priority**: The current list does not link to useful details. Navigation and truthful summaries make the main workflow usable.

**Independent Test**: With active/archived clients, several currencies and different access levels, search/filter the list, open a client and reconcile visible totals against authorized records.

**Acceptance Scenarios**:

1. **Given** active and archived clients, **When** I choose scope, currency and search text, **Then** only matching clients appear, with accurate counts and distinct no-matches/no-clients states.
1. **Given** clients with projects and time, **When** I open the list, **Then** I see client, currency, active/total projects, hours and unbilled time, subject to my permissions.
1. **Given** financial access, **When** I open a client, **Then** I see unbilled time, billed this year, lifetime hours, open invoices, projects, billing details and recent invoices from real records.
1. **Given** a member without financial access, **When** I open either screen directly, **Then** no total, contact field or destination discloses restricted information.
1. **Given** several currencies, **When** I inspect money totals, **Then** each currency is separately labeled, without fabricated conversion.

### User Story 2 - Maintain client information (Priority: P1)

As a person with client-management authority, I can create/edit clients through the designed dialogs while preserving their existing work.

**Why this priority**: The redesign must retain current client management and make the billing panel useful.

**Independent Test**: Create a client, reload, edit from both screens and cancel another edit. Compare saved information and existing project/invoice amounts before and after.

**Acceptance Scenarios**:

1. **Given** valid information, **When** I save, **Then** name, preferred currency, optional default hourly rate, billing address and tax identifier persist consistently across list, detail and project creation.
1. **Given** invalid information, **When** I save, **Then** the relevant field explains the problem, input is retained and no partial changes are saved.
1. **Given** a default rate, **When** I change currency, **Then** I explicitly replace or clear that rate instead of silently reinterpreting its amount.
1. **Given** existing projects/invoices, **When** I edit defaults, **Then** existing amounts, currencies and stored project/invoice terms remain unchanged.
1. **Given** a dialog without a pending save, **When** I cancel or press Escape, **Then** nothing is saved and focus returns to the triggering control.
1. **Given** the contact/payment-term scope approved in FR-015, **When** I save or clear optional values, **Then** the panels and search reflect those values, not prototype examples.

### User Story 3 - Manage selected clients (Priority: P2)

As a person with the required client-operation authority, I can review and apply a change to explicitly selected clients only.

**Why this priority**: Multi-selection and Actions are explicit design requirements and avoid repetitive editing.

**Independent Test**: Select rows, change filters, preview/cancel a change, then apply another and reconcile each selected client's outcome.

**Acceptance Scenarios**:

1. **Given** no selection, **When** I view the list, **Then** Actions is disabled. Selection enables it and shows its count; partial selection gives the header checkbox a mixed state.
1. **Given** a selection, **When** search, scope, currency or page changes, **Then** selection clears so hidden records cannot change accidentally.
1. **Given** selected clients, **When** I choose currency, rate, archive or reactivate, **Then** I review affected clients/consequences before applying; cancel changes nothing.
1. **Given** mixed currencies, **When** I set a default rate, **Then** I must choose one currency group and other clients are unchanged.
1. **Given** a failure/concurrent edit, **When** I apply, **Then** each selected client has a truthful applied, unchanged, blocked or failed result. Retry does not repeat an already completed transition as a new change.
1. **Given** a selection, **When** I export, **Then** the file contains exactly those authorized clients, explicit currencies and safely escaped text.

### User Story 4 - Continue work and manage lifecycle (Priority: P2)

As an authorized person, I can open a client's project/invoice workflows and archive inactive business without losing history, subject to each destination's separate permissions.

**Why this priority**: Detail must connect to working flows; lifecycle changes affect tracking beyond this page.

**Independent Test**: Open contextual destinations and cancel creation; exercise the approved archive/reactivate policy with active and previously archived projects.

**Acceptance Scenarios**:

1. **Given** an active client, **When** I choose New project/invoice, **Then** the existing creation flow preselects this client; navigation creates nothing and does not overwrite an unrelated saved draft.
1. **Given** a project/recent invoice, **When** I open it, **Then** I reach that record. View in Projects opens the client-filtered list.
1. **Given** an archive request, **When** I review it, **Then** confirmation states the FR-016 policy and exact affected project count; cancel preserves all state.
1. **Given** successful archive, **When** I review existing reports/invoices, **Then** history remains available while new tracking is prohibited.
1. **Given** linked projects, invoices or import history, **When** I request deletion, **Then** it is blocked with a reason and archive alternative; history is never cascaded away.
1. **Given** an eligible client without linked history, **When** I explicitly confirm deletion, **Then** only that client and its owned metadata are removed.
1. **Given** a client, **When** I choose Duplicate, **Then** an unsaved new-client dialog copies editable metadata, not projects, invoices, time, lifecycle state or external identity.

### Edge Cases

- Long/non-Latin names, multiline addresses and equal display names do not corrupt identity, selection or navigation.
- Missing rates and unknown external billing status are partial/unknown, not confident zeros. Unbilled time is not remaining fixed fee.
- Zero prior-year billing yields an explanatory comparison label, not an infinite percentage.
- Running timers, concurrent project creation and changed permissions are rechecked before archive; operations that cannot preserve tracking integrity are rejected with a reason.
- Missing/deleted/cross-organization identifiers disclose no foreign records.
- Loading, retryable error, empty organization, no matches and archived-client states are distinct.
- At 320 CSS pixels and 200% text size, dialogs/controls remain usable. Wide tables may scroll within their own labeled region, not force the whole page sideways.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: List MUST match the Clients handoff: title, New client, selection-based Actions, Import, client/contact search, lifecycle/currency filters, four cards, selectable table and empty state.
- **FR-002**: Detail MUST match its handoff: back link, identity/currency, contextual creation/edit/actions, four cards, project table, contact/billing panels and recent invoices. Prototype shell/example data MUST NOT appear as real content.
- **FR-003**: Client list/detail, metadata, contact, financial and lifecycle operations MUST use feature 015's verified effective capabilities and scopes, including its six built-in profiles, custom profiles and individual adjustments. Client management MUST NOT imply project, invoice, rate-editing or import authority. Minimal client identities needed by an authorized tracking picker MUST remain distinct from full catalog/contact access. Sensitive fields, counts, search matches and exports MUST exclude unauthorized data. Existing broad role checks are a migration input, not the final acceptance boundary; do not invent a contact permission or permanently restrict client management to a role named Manager.
- **FR-004**: Search MUST ignore case across client names and authorized contact names/email. Currency filtering MUST match preferred currency or visible project currencies. Scope counts MUST honor search/currency filters across all lifecycle alternatives.
- **FR-005**: Active/archived card MUST count matching clients across both states. Other list cards MUST use clients matching all current filters. Hours card MUST be year-to-date; row/detail hours MUST be explicitly lifetime. Project counts MUST exclude inaccessible projects.
- **FR-006**: Billed this year MUST sum non-draft, non-void invoices issued in the current year through today, preserving invoice currency/amount. Year comparison MUST use the equivalent prior-year elapsed period, labeled year-to-date, with no percentage for a zero denominator. Count/last issue date MUST use the same population.
- **FR-007**: Open invoices MUST exclude draft/paid/void records and show outstanding amount/earliest known due date. Recent invoices MUST show five latest issue dates, deterministically ordered for ties, with real status/link and truthful draft labels.
- **FR-008**: Unbilled time MUST distinguish billable time without a local invoice from verified eligibility for new billing. Imported entries with unknown external billed status MUST show an explicit coverage warning, never a ready-to-invoice claim. Creation MUST retain existing invoice review/eligibility checks.
- **FR-009**: Money MUST be separated by ISO currency; hours MUST reconcile to recorded minutes. Missing/partial values MUST differ from zero. Project spent/budget states MUST match project reporting rules, including fixed-fee/non-billable behavior.
- **FR-010**: Create/edit MUST share validation and preserve name, currency, rate, address and tax identifier. Reject blank/overlong names, unsupported currency, negative/unrepresentable rates and malformed supplied contact fields. An absent rate MUST differ from zero.
- **FR-011**: Defaults MUST NOT silently redenominate money, reprice history or rewrite project/invoice settings. New-project prefill MUST preserve explicit user choices and unrelated drafts.
- **FR-012**: Selection MUST be keyboard operable, client-labeled, visible-row scoped and cleared on filter/page changes. Actions MUST be disabled without selection and during pending mutations.
- **FR-013**: Bulk currency/rate/archive/reactivate MUST preview scope/consequences, require explicit apply, recheck current permissions/state and report individual outcomes. Currency changes MUST explicitly clear/replace an existing rate, with no exchange conversion.
- **FR-014**: Selected-client CSV export MUST include identity, preferred currency, address, tax identifier and lifecycle state, escaped safely for spreadsheets. Import MUST open the existing importer without starting a job. Export MUST NOT require enabling Actions without selection.
- **FR-015**: New client metadata needs scope confirmation: [NEEDS CLARIFICATION: Add one editable primary contact (name, title, email, phone) and optional payment terms to match the handoff, or multiple contacts with a primary selection? Both require new persisted client information.]
- **FR-016**: Archive/reactivate needs policy confirmation: [NEEDS CLARIFICATION: Archive the client and active projects together as designed, then restore only projects archived by that operation, or follow Harvest by requiring all projects archived first and restoring only the client? Current Horae deactivates only the client and preserves project flags.]
- **FR-017**: Approved archive/reactivate semantics MUST apply to individual/bulk actions and preserve time, invoices, import mappings and reporting history, including concurrent-edit safety.
- **FR-018**: Delete MUST require confirmation and reject clients with projects, invoices, tracked activity or import history. Duplicate MUST copy editable configuration into an unsaved form only, resetting lifecycle/external identity.
- **FR-019**: Contextual creation, project filtering/edit/archive/reactivate and invoice links MUST retain client context and destination permissions. Project Pin MUST follow the shared per-user Project Detail policy; its unresolved policy remains a dependency, not authorization for a different pin model.
- **FR-020**: Controls MUST have real outcomes, labels, keyboard focus and pending/error states. Save failures MUST retain input; dialogs MUST manage focus, cancellation and narrow/enlarged-text layouts.

### Key Entities *(include if feature involves data)*

- **Client**: Organization-owned identity, currency, hourly rate, address, tax identifier, creation date and lifecycle. Creation date means added to Horae, not the start of an imported business relationship.
- **Client contact**: Name, title, email and phone; cardinality/primary selection pending FR-015. Not an application user or invitation recipient.
- **Client payment terms**: Optional days, including upon receipt, prefilling new project terms only when not explicitly chosen. Existing project/invoice terms retain precedence.
- **Client summary**: Authorized project/time/invoice totals with explicit periods, currencies and completeness.
- **Selected-client operation**: Explicit identities, proposed change, preview and per-client result; selection is not a persistent business relationship.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every authorized list row opens its real client; contextual actions retain the expected client without creating records merely by navigation.
- **SC-002**: Every count, minute and per-currency amount reconciles exactly against fixtures covering lifecycle states, mixed currencies, missing rates, imports, all project types and draft/open/paid/void invoices.
- **SC-003**: Create/reload/edit preserves every supported field from both entry points; all invalid-input and cancel cases leave saved state unchanged.
- **SC-004**: For each bulk operation, changed identities equal authorized/applicable selected clients; every selected client has a truthful result and hidden/unselected clients remain unchanged.
- **SC-005**: All six built-in profiles, custom grants, individual adjustments, revoked scopes and cross-organization cases pass feature 015's client-operation matrix with zero unauthorized records or financial/contact values. Verify direct requests, search/counts, exports and contextual destination access; permitted client management never grants an unrelated financial or import action.
- **SC-006**: List/detail/dialogs are keyboard operable at desktop and 320 CSS pixels, including 200% text size, without overlapping checkboxes or inaccessible actions.
- **SC-007**: Every designed section/action has a verified implementation or explicitly approved deviation. Unresolved policy questions and placeholder/no-op controls prevent completion.

## Assumptions

- Checked-in handoffs define Horae's appearance; Harvest is behavioral comparison, not authority to silently replace a conflicting design.
- Contact information is for reference and sends no mail. Contact import, invoice delivery and Harvest-specific client tax/discount defaults are outside this feature unless requested separately.
- Optional client terms prefill newly created projects. Existing project defaults/explicit invoice overrides retain precedence; unset client terms preserve existing defaults.
- Deletion is limited to clients without history; import identity is never discarded to make deletion succeed. Duplicate display names do not merge identities.
- Calendar periods use the organization's reporting date boundary. No currency conversion service is introduced.
- Shared Project Detail actions, invoice accounting/eligibility, project drafts and importer are dependencies, not independently redefined here.
- Feature 015 owns the effective-access matrix and migration. Its [pending PR #212](https://github.com/numtide/horae/pull/212) includes the constitution amendment; it is not merged into this branch. Runtime permission changes require that governance/migration gate, and the unresolved operation matrix prevents full acceptance here. Existing OIDC and organization isolation remain unchanged.
- Settings/Workspace and unfinished Project Detail work remain in the wider goal; this specification does not mark them complete.
