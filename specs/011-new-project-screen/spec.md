# Feature Specification: New Project

**Feature Branch**: `feat/new-project-screen`

**Created**: 2026-09-21

**Status**: Draft

**Input**: User description: "Vale revisa el disenio y vamos a implementar la pantalla new project"; "Implementa el disenio conforme a lo que hacemos con speckit y demas".

The reference is `design/project/app/13_New Project.dc.html` from the September 21 handoff. This feature delivers its project-creation workflow, including the behavior behind the displayed controls. Prototype sample people, clients, amounts and saved timestamps are not product defaults. Existing projects and existing screens must retain their behavior.

**Scope extension (2026-09-22)**: The user requested that Edit open the same designed editor with the existing project's values, that the old editor be removed, and that this ship in PR #207. US7 supersedes the original decision to retain the separate Projects edit form; it does not authorize rewriting historical financial records.

## User Scenarios & Testing

### User Story 1 - Create a usable project (Priority: P1)

An administrator or manager opens New Project from Projects, chooses a client, enters project details, and saves a project ready for tracking.

**Why this priority**: Creating a usable project is the primary purpose of the screen.

**Independent Test**: Create one project of each supported creation type, reopen it and verify its details and tracking behavior.

**Acceptance Scenarios**:

1. **Given** an authorized user and an active client, **When** the user enters a name and saves, **Then** exactly one project is created and its detail screen opens.
1. **Given** missing required details or invalid dates, **When** saving is attempted, **Then** field-specific errors appear without losing input or creating a partial project.
1. **Given** a new client is needed, **When** the user creates it in the client dialog, **Then** its name, currency and optional default rate are saved and the client is selected without resetting the project form.
1. **Given** valid code, optional dates, tags and currency, **When** the project is saved, **Then** those values survive reload; dates describe the planned period without forbidding tracking outside it.
1. **Given** a member, another organization, an archived client or a client archived while the form was open, **When** project creation is attempted, **Then** creation is refused without exposing another organization's data.

### User Story 2 - Resume a truthful draft (Priority: P1)

An authorized creator can leave and resume an unfinished project without confusing a draft with a real project.

**Why this priority**: The design promises autosaving across a long form; losing settings would undermine the workflow.

**Independent Test**: Edit a draft, wait for confirmation, reload, simulate a failed save, then retry final creation.

**Acceptance Scenarios**:

1. **Given** an incomplete form, **When** changes are acknowledged as saved, **Then** reload restores those values and the displayed save time corresponds to an actual successful save.
1. **Given** a disconnected session, **When** autosave fails, **Then** the form reports unsaved changes and provides a retry without discarding input.
1. **Given** the same draft open in two tabs, **When** both tabs save divergent changes, **Then** an older version cannot silently overwrite a newer one.
1. **Given** a saved draft, **When** the creator cancels or goes back, **Then** it remains resumable but never appears in project lists, timers, reports or invoices.
1. **Given** repeated final submissions or a lost success response, **When** creation is retried, **Then** the same completed project is returned and no duplicate tasks, memberships or events are produced.

### User Story 3 - Configure billing and budgets (Priority: P1)

A creator chooses time and materials, fixed fee or non-billable work and configures rates, fees and budgets that match actual tracking and invoicing.

**Why this priority**: Financial settings must do what their labels promise, not merely be stored.

**Independent Test**: Track known durations against projects with each rate and budget mode and compare spend, remaining budget and invoice amounts.

**Acceptance Scenarios**:

1. **Given** a time-and-materials project, **When** person, task or project rates are selected, **Then** tracking totals and invoices use that selected mode consistently, including a deliberate zero rate.
1. **Given** a fixed-fee project, **When** a single fee, dated milestones or monthly fee is saved, **Then** the selected schedule and exact amounts can be used when preparing invoices without also charging the same work by the hour.
1. **Given** non-billable work, **When** time is recorded, **Then** it remains non-billable in reports and cannot be selected for invoicing.
1. **Given** a total, per-task or per-person budget, **When** qualifying time is recorded, **Then** consumption is charged to the appropriate budget; monthly reset and inclusion of non-billable time follow the selected options.
1. **Given** an enabled budget alert and configured delivery, **When** consumption crosses its threshold, **Then** one logical notification per recipient is scheduled for that period; acknowledged deliveries are not resent and retries retain the notification identity.
1. **Given** a project predating this feature, **When** its time or invoice is recalculated, **Then** its existing rate precedence and amounts are unchanged.

### User Story 4 - Configure tasks, team and privacy (Priority: P1)

A creator assigns real teammates and tasks and controls who can track work and view project information.

**Why this priority**: The design promises access on saving; permission errors can expose confidential rates or block work.

**Independent Test**: Create a project with restricted tasks, a project manager and ordinary teammates; verify each actor's allowed and denied operations.

**Acceptance Scenarios**:

1. **Given** the organization's task catalog, **When** tasks are selected or a new task is entered, **Then** the saved project has those tasks without duplicate associations and respects individual or all/none billable toggles.
1. **Given** active organization members, **When** Add everyone is used repeatedly, **Then** each person appears once and individual members can be removed or designated as project managers.
1. **Given** a restricted task, **When** an excluded teammate tries to start or edit time against it, **Then** the operation is refused even outside the project screen.
1. **Given** managers-only report visibility, **When** an ordinary teammate requests project-wide hours or budget information, **Then** it is withheld while their own permitted timesheet remains usable.
1. **Given** visibility for everyone on the project, **When** a teammate views progress, **Then** they see allowed hours and budget progress but no hourly rates, costs or private notes.
1. **Given** administrator notes or confidential costs, **When** unauthorized users request project data, **Then** confidential fields are absent, not merely visually hidden.

### User Story 5 - Reuse invoice defaults (Priority: P2)

A creator sets payment terms, purchase order, tax and discount defaults so project invoices start with the correct values.

**Why this priority**: These controls are part of the requested screen and only deliver value when invoicing consumes them.

**Independent Test**: Create a project with defaults, prepare its invoice, override the defaults and confirm that the project remains unchanged.

**Acceptance Scenarios**:

1. **Given** receipt, standard or custom payment terms and a PO number, **When** a project invoice is prepared, **Then** terms determine its due date and the PO is prefilled.
1. **Given** a discount and one or two named taxes, **When** the invoice is calculated, **Then** totals match exact rounding rules and their components are visible.
1. **Given** prefilled invoice values, **When** the user edits them, **Then** only that invoice changes; previous invoices retain their saved totals.
1. **Given** projects with incompatible defaults, **When** they are selected together for an invoice, **Then** the user must explicitly resolve the conflict rather than silently inherit one project's settings.

### User Story 6 - Use the designed screen across devices (Priority: P2)

The creator uses a screen faithful to the handoff inside the existing Horae shell, with accessible controls and no regressions elsewhere.

**Why this priority**: Visual alignment is the user's starting objective; functionality does not replace this requirement.

**Independent Test**: Exercise all conditional panels at desktop and mobile widths with keyboard navigation, then check Projects, Clients, Timesheet and Invoices.

**Acceptance Scenarios**:

1. **Given** a desktop viewport, **When** New Project opens, **Then** it follows the reference's heading, labeled rows, type cards, task/team sections and persistent action footer.
1. **Given** a narrow viewport or zoomed text, **When** the form is used, **Then** controls reflow without document-level horizontal scrolling or a footer obscuring focused fields.
1. **Given** keyboard or assistive-technology use, **When** selecting, removing, opening dialogs and submitting, **Then** controls have names, states, visible focus and understandable errors; dialogs restore focus on close.
1. **Given** a failed initial load, empty catalog or pending submission, **When** the screen displays the state, **Then** it offers a useful retry/empty action and prevents duplicate submission.

### User Story 7 - Edit in the same project editor (Priority: P1)

An authorized manager or administrator edits an existing project using the same layout and controls as creation, populated from its actual saved settings.

**Independent Test**: Open Edit for a configured project and an imported/legacy project, inspect all prefilled values, cancel without changes, save changes, reload and verify the same project identity and unchanged historical time/invoices.

**Acceptance Scenarios**:

1. Every project Edit action opens `/projects/:id/edit` in the shared editor; the old inline Projects form is absent. Direct navigation and reload work.
1. Basic details, selected client/currency, type, billing/budgets, fee schedule, tags, task/team settings, visibility, authorized private fields and invoice defaults are loaded from operational records, not from a creation draft or prototype defaults.
1. Editing does not create, consume or overwrite the user's New Project draft. Save changes updates the existing project atomically; Cancel leaves operational data unchanged. Unsaved navigation requires an explicit choice.
1. Validation/network failures preserve entered values and offer recovery. A concurrent change cannot be silently overwritten; retrying an uncertain successful update neither duplicates associations/events nor reapplies an older edit after a later update.
1. Existing projects without creation settings, including imported projects and retainers, preserve their actual billing semantics and currencies. Unsupported changes are explained, not silently converted or replaced with new-project defaults. Unchanged unavailable selections remain identifiable and do not prevent unrelated safe edits.
1. Time entries, invoice lines, materialized fee identities and invoice snapshots remain intact. Removing referenced tasks/people or changing charged fee schedules is validated transactionally; rejection makes no partial changes. Existing association identities and project roles survive unrelated edits.
1. Current organization/role authority is rechecked on load and save. Managers cannot receive or erase administrator-only notes/costs by saving a redacted form; cross-organization and inactive actors are denied.
1. The shared editor remains accessible at 390/768/1440 and desktop 200% text. Creation, Projects selection/bulk actions and unrelated shared-screen styles retain their regression coverage.

### Edge Cases

- Whitespace-only names; unknown currencies; negative, overflowing or over-precision amounts; percentage outside 0–100; custom terms outside 0–365 days; end before start.
- Zero versus absent rates; a changed client changing inherited currency but not explicit currency; no implicit conversion of amounts across currencies.
- Duplicate tags differing only by case; tags removed from one draft but retained for other projects; no numeric previous code from which a suggestion can be derived.
- Task/person deletion, deactivation or cross-organization identifiers submitted while a draft is open; loss of the creator's permissions between load and submit.
- Removing a task/person with a scoped budget; switching billing types with hidden stale settings; milestones with missing dates or amounts; leap years and last-day monthly schedules.
- Autosave arriving after final creation, an old tab saving after draft discard, lost final responses and retry after an interrupted transaction.
- No mail delivery configured, permanent delivery failure and retries after a threshold notification has already been acknowledged.
- Mixed-project invoices, unassigned teammates, direct compatibility/export requests and imported legacy projects must respect their existing contracts and the new privacy boundaries.

## Requirements

### Functional Requirements

- **FR-001**: Authorized administrators and managers MUST reach a dedicated New Project screen from Projects; members MUST NOT gain creation authority.
- **FR-002**: Creation MUST require an active same-organization client and trimmed nonempty project name; code, dates and tags are optional. Name/code/tag lengths MUST be limited to 200/100/50 characters; at most 50 distinct tags are allowed.
- **FR-003**: The client selector MUST search real clients, identify unavailable archived clients and support explicitly creating and selecting a new client with currency and optional default billable rate.
- **FR-004**: Project currency MUST inherit from the selected client unless explicitly overridden. Cost figures MUST identify the organization's currency. Incompatible currency inheritance MUST require an explicit rate instead of treating an amount as converted.
- **FR-005**: Tags MUST be reusable organization labels, addable with Enter/comma and removable individually. A project's tags MUST be usable to filter project lists and reports without deleting the shared label.
- **FR-006**: Notes MUST be available only to administrators. Report visibility MUST distinguish managers-only from project-member progress; ordinary members MUST never receive rates or cost data through this setting.
- **FR-007**: Drafts MUST be private to creator and organization, retain incomplete values across reloads, show acknowledged save status, detect concurrent changes and support explicit discard. Cancel/back preserves a draft. Drafts MUST remain outside operational project data.
- **FR-008**: Final submission MUST atomically persist project configuration, tags, task associations, task access and memberships. Repeated submission of one draft MUST produce one project and one project-created event.
- **FR-009**: New projects MUST support time-and-materials, fixed fee and non-billable creation; existing retainer and imported projects MUST remain usable without conversion.
- **FR-010**: Time-and-materials projects MUST apply the selected person/task/project rate mode consistently to spend, reports, invoice preparation and compatible exports. Existing projects MUST retain their existing precedence unless explicitly changed.
- **FR-011**: Fixed-fee projects MUST preserve a single fee, dated milestone amounts, or monthly fee with first/fifteenth/last-day selection; these fees MUST be available when preparing project invoices without inventing time entries or double charging hours.
- **FR-012**: Budgets MUST support none, total hours, hours per task and hours per person; time-and-materials additionally supports total fees and fees per task. Scoped modes MUST expose per-task/person amounts rather than an ambiguous single total.
- **FR-013**: Supported budget options MUST control monthly consumption periods, non-billable inclusion and threshold alerts. Alerts MUST target the creator and current project managers with one logical notification per recipient/period/threshold. Acknowledged deliveries MUST NOT be retried; interrupted delivery attempts MUST retain the notification identity. A deployment without configured delivery MUST clearly disable email promises rather than report messages sent.
- **FR-014**: Task selection MUST reuse the real catalog, allow named task creation, prevent duplicates, configure billability and task-rate overrides, and enforce optional per-task member restrictions on every time-entry mutation path.
- **FR-015**: Team selection MUST use active organization members, deduplicate Add everyone, support removal and project-manager designation, and persist project-specific bill/cost overrides without silently changing anyone's profile or another project.
- **FR-016**: Invoice defaults MUST include payment terms, PO number, discount and up to two taxes; they MUST prefill project invoices and remain independently editable per invoice. Percentages MUST use at most two decimal places and be within 0–100.
- **FR-017**: Financial/time calculations MUST be exact and reject overflow. Discount is applied to the subtotal before two non-compounding taxes; each component is rounded half-up to the currency's supported minor unit. Invoice totals MUST equal displayed components.
- **FR-018**: Every selector, conditional panel, error, loading state, modal, draft state and action MUST work with keyboard navigation, accessible labels and visible focus. Errors MUST preserve input and identify the affected field.
- **FR-019**: The screen MUST follow the handoff's content hierarchy, spacing, typography and responsive intent within the current shared shell; changes MUST preserve the appearance and behavior of existing screens.
- **FR-020**: Suggestions and status text MUST derive from actual data. The form MUST NOT display prototype identities, rates, codes, saved timestamps or success messages as real workspace state.
- **FR-021**: Creation and editing MUST share the designed form sections; all project Edit entry points MUST load the existing project's complete authorized configuration, and the obsolete inline editor and unused handlers MUST be removed.
- **FR-022**: Editing MUST use explicit atomic updates to the same project, isolated from creation drafts, with conflict detection, safe uncertain-request retries and unsaved-navigation protection. Cancel MUST NOT persist edits.
- **FR-023**: Editing MUST preserve existing legacy semantics, referenced entity identities, private fields outside the actor's authority and historical time/invoice sources. Guarded changes MUST have explicit user-facing explanations and server-side enforcement.

### Key Entities

- **Project draft**: Creator-owned unfinished values, acknowledged version/save time and eventual completed-project reference.
- **Project**: Existing identity and client link with basic details, tags, confidential notes, visibility and billing/budget configuration.
- **Project task**: Catalog task association with billability, optional rate/budget and allowed members.
- **Project membership**: A person's project access, manager designation and explicit project-only financial overrides.
- **Fee schedule**: Single, milestone or monthly fee amounts and applicable dates/day rules.
- **Invoice defaults**: Project payment terms, PO, discount and tax defaults, copied into an invoice without linking subsequent edits.
- **Budget notification**: Threshold, consumption period, intended recipients and delivery state for deduplication/retry.

## Success Criteria

### Measurable Outcomes

- **SC-001**: An authorized user can create a basic project from Projects in under two minutes with an existing client; every configured value survives reopening.
- **SC-002**: Every displayed editable control has a persistence and downstream-behavior acceptance check; none is a no-op or fabricated placeholder.
- **SC-003**: Repeated submissions, interrupted responses and concurrent-draft tests produce zero duplicate projects and zero silent overwrites.
- **SC-004**: The permission matrix produces zero disclosure of another organization's data, administrator notes or unauthorized rates/costs.
- **SC-005**: For every billing/budget type, independently calculated fixtures exactly match reported spend, budget consumption and invoice components, with no changes to legacy fixtures.
- **SC-006**: All creation flows can be completed at 390, 768 and 1440 pixel viewport widths and with keyboard alone; no focused control is obscured by the footer.
- **SC-007**: Projects selection/actions, Clients, Timesheet, Invoices and shared navigation retain their existing regression checks after the new screen is introduced.
- **SC-008**: Configured and legacy projects round-trip through the shared editor without unintended field changes, duplicate projects/events, silent concurrent overwrites, private-data loss or altered historical invoice/time records; the old edit form is no longer reachable or present in source.

## Assumptions

- The complete creation workflow and unified editing workflow are in scope, not cosmetic forms with inert advanced settings. A general project-dashboard redesign, organization/auth redesign and automatic migration of existing projects to new billing semantics are out of scope.
- Scheduled fees describe billing availability; invoice preparation and issuing remain explicit user actions. Creating a project does not automatically issue invoices or email clients.
- Budget email uses an optionally configured self-hosted delivery service, not an assumed third-party account. Tests must use an isolated delivery stub and must not send messages to real users. A lost acknowledgement after acceptance may cause a duplicate delivery on retry; exactly-once email receipt is not promised.
- One resumable current creation draft per creator is sufficient; explicit discard starts a new one. A client created explicitly in the client dialog remains a real client even if the project draft is discarded.
- Project-specific cost overrides are administrator-only and never change user profile costs. Managers can create projects without permission to edit confidential costs or notes.
- Project-manager designation does not promote an organization member: project leads gain project progress access but not organization-wide financial reports or billing/cost rates. Existing self-profile financial fields are outside the new project visibility setting; project, task, progress and time-entry payloads must not leak new confidential settings through it.
- Revoked task access prevents new tracking and ordinary edits; stopping an already-running own timer remains allowed to avoid trapping a running timer. Privileged approval/import workflows retain their existing authority.
- One project-created event means one transactional event record; downstream plugin delivery retains its documented retry/availability semantics, not an exactly-once external side effect.
- Dates and monthly budget periods follow the organization's existing date convention; introducing organization timezone management is outside this feature.
- No external payment collection, tax-law determination or currency conversion is included. Rates and taxes are user-entered settings, not jurisdictional recommendations.
- The existing production/import database is not a test fixture; validation uses isolated development/test data.
