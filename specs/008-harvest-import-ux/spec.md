# Feature Specification: Consistent Harvest Import Experience

**Feature Branch**: `feat/harvest-import-ux`

**Created**: 2026-09-15

**Status**: Specified; implementation pending

**Input**: Improve the Harvest import UI's consistency with Horae's design and make connection, preview, import, progress, history and recovery understandable. Preserve the durable-job and account-switch guarantees delivered by features 005–007.

## Clarifications

### Session 2026-09-15

No critical ambiguity requiring a new user choice was found. Zero questions were asked or answered. Scope and conservative defaults are recorded under Assumptions, not represented as separately confirmed user answers. Read-model details and reusable components are planning decisions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand and manage the connection (Priority: P1)

As a workspace administrator, I can identify the bound Harvest account and find the appropriate connection action in one coherent area without confusing disconnecting with changing accounts.

**Why this priority**: Connection ambiguity can cause an administrator to authorize the wrong account or misunderstand what happens to existing data.

**Independent Test**: Exercise configured, unconfigured, loading, unavailable, connected, expired and disconnected-but-bound states without starting an import.

**Acceptance Scenarios**:

1. **Given** a connected account, **When** I open connection management, **Then** its identity, connection state, Disconnect and Change account are presented together with their distinct effects.
1. **Given** a disconnected but bound account, **When** I view the connection, **Then** I can reconnect that original account or see whether an explicit change is allowed; disconnection is not presented as removal of imported data.
1. **Given** imported data or active work blocks a change, **When** I inspect Change account, **Then** the reason is visible before confirmation and the action cannot be submitted.
1. **Given** a connection check fails or is loading, **When** the importer list appears, **Then** it does not imply that connection availability has been confirmed.
1. **Given** expired credentials, **When** I inspect the connection, **Then** I see an accurate recovery action and no unsupported promise that waiting alone will refresh the connection within a fixed time.

### User Story 2 - Preview and import with truthful feedback (Priority: P1)

As an administrator, I can follow one clear next action from source selection through preview and confirmation, and understand what the result actually changed.

**Why this priority**: A reassuring but inaccurate empty or success message undermines trust in imported records.

**Independent Test**: Use both Harvest connection and CSV fixtures with created, updated, skipped, errored and zero-record outcomes; no real account is required.

**Acceptance Scenarios**:

1. **Given** a source ready to preview, **When** I view the flow, **Then** preview is the primary action and its description says it does not modify business records.
1. **Given** a successful current preview, **When** I review its result, **Then** confirmation is the primary next action; a competing new-preview action is secondary.
1. **Given** a historical preview, a replaced source or an account change, **When** I view an old report, **Then** it cannot be mistaken for an immediately confirmable current preview.
1. **Given** no report is selected or retained history is empty, **When** I view the source, **Then** the UI does not infer that nothing was ever imported.
1. **Given** a completed import containing skipped records, **When** the result appears, **Then** created, updated, skipped and errored totals remain distinct and no message claims that every record was written.
1. **Given** a failed or cancelled operation with confirmed batches, **When** I view its report, **Then** it is explicitly partial and does not imply that previously committed batches were rolled back.

### User Story 3 - Follow and revisit background imports (Priority: P2)

As an administrator, I can distinguish waiting, running and completed work, leave the page safely, and return to the right report or recovery action.

**Why this priority**: Durable work already survives navigation; the interface must make that behavior discoverable and distinguish monitoring failure from job failure.

**Independent Test**: Restore queued/running/terminal fixtures after navigation or reload; test status-request failure independently of worker failure.

**Acceptance Scenarios**:

1. **Given** queued or running work, **When** I follow it, **Then** I see a human-readable state, phase and observed count; unknown totals never produce an invented percentage or ETA.
1. **Given** status retrieval fails, **When** monitoring stops, **Then** the UI explains that work may still continue and offers to resume monitoring without submitting another job.
1. **Given** retained history, **When** I browse it, **Then** source, preview/import mode when known, readable date/time, state and selected item are distinguishable without decoding internal job names.
1. **Given** a retryable old-account operation, **When** I view its history after an account change, **Then** its report remains accessible but retry is unavailable with an explanation.
1. **Given** cancellation was requested, **When** a later status arrives, **Then** the UI distinguishes cancellation requested from cancellation completed and prevents duplicate pending actions.

### User Story 4 - Use the flow consistently across devices (Priority: P2)

As an administrator using a keyboard or narrow screen, I can complete the same flow with Horae's established visual hierarchy and interaction conventions.

**Why this priority**: Visual alignment is incomplete if connection dialogs, progress and history are only usable on a large screen.

**Independent Test**: Traverse the connection, preview, report and history fixtures at 360, 768 and 1440 CSS-pixel viewport widths, including a 640-pixel-high viewport and 200% zoom.

**Acceptance Scenarios**:

1. **Given** a narrow viewport or long account identifier, **When** I navigate the flow, **Then** controls wrap without clipping and any wide record table scrolls within its own container rather than the whole page.
1. **Given** keyboard-only use, **When** I open and dismiss connection management or confirmation, **Then** controls have accessible names, expanded state is exposed, focus is visible, and confirmation returns focus to its trigger.
1. **Given** asynchronous progress or a result, **When** it changes, **Then** meaningful state changes are announced without moving focus or announcing every polling tick.
1. **Given** the existing Importers visual handoff, **When** the page is compared with it, **Then** headings, spacing, connection panels, actions, status banners, counters and errors follow the established design; additions for durable jobs and account switching are explicitly documented.

### Edge Cases

- Connection or history is loading, unavailable or stale while an action is attempted.
- Another administrator disconnects or changes the account while a dialog or preview is open.
- A status response belongs to a previously selected operation; it must not replace the current selection.
- History retention removes records: absence of retained history does not prove absence of imported data.
- A job has unknown kind, mode, phase or total; the UI uses an honest fallback rather than guessed meaning.
- A job fails before it has a report, or completes with only skipped records or zero processed records.
- CSV file replacement, source switching, reload and historical selection invalidate current-preview confirmation where the existing workflow requires it.
- Permission denial from Harvest is distinguished from a failed connection check; detailed errors are available without exposing credentials.
- A modal action is pending, an identifier is long, or a report contains more errors than the inline limit.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Present loading, unconfigured, unavailable, connected, expired and disconnected connection states truthfully; preserve a visible bound account identity when disconnected.
- **FR-002**: Group connection actions coherently, distinguish Disconnect from Change account, and retain explicit confirmation and visible blocking reasons.
- **FR-003**: Preserve administrator-only, organization-scoped access, account-change safeguards and stale-action rejection; UI convenience MUST NOT weaken server enforcement.
- **FR-004**: Present source selection, preview, confirmation, progress and result as a consistent journey for Harvest connection and CSV import, with at most one visually primary workflow action per state outside an open modal.
- **FR-005**: Empty and introductory messages MUST describe available evidence, not infer import history from the current report selection; a preview MUST be described as leaving business records unchanged, not as writing no operational data.
- **FR-006**: Result copy MUST distinguish preview, full result, partial result and failure without a report, and accurately describe created, updated, skipped and errored counts, including all-skipped and zero-record results.
- **FR-007**: Monitoring MUST show readable job state and phase, processed count and known total; unknown totals or estimates MUST remain unknown, with an indeterminate presentation where appropriate.
- **FR-008**: Monitoring failure MUST be separate from job failure and offer a read-only recovery action; leaving the page MUST NOT be presented as cancelling accepted work.
- **FR-009**: History MUST provide readable source, known mode, date/time, status and selected state, preserve pagination and report access, and label unknown metadata without guessing.
- **FR-010**: Retry, cancellation and confirmation availability MUST reflect known operation and connection eligibility, remain safe when stale, and explain why historical work cannot run against a replacement account.
- **FR-011**: Errors MUST state the affected action and available recovery where known; technical details may be disclosed separately but MUST NOT expose tokens, secrets or session credentials. Preserve complete error downloads.
- **FR-012**: Reuse Horae's established typography, spacing, colors, panels, controls and responsive conventions; document necessary differences from the Importers handoff rather than invent unsupported data or capabilities.
- **FR-013**: Meet the keyboard, focus, screen-reader announcement and narrow/short-viewport scenarios in User Story 4; status MUST be communicated with text, not color alone.
- **FR-014**: Keep current import semantics, limits, history retention, account binding, background execution and local-data preservation unchanged. This feature MUST NOT add secret-management configuration, scheduled sync, another importer or account-data migration.
- **FR-015**: Include automated acceptance coverage for the state/result/action matrix and browser checks for responsive layout and keyboard behavior. Validate an authorized real Harvest dry-run separately; never automatically confirm an import or change the real account as part of testing.

### Key Entities *(include if feature involves data)*

- **Connection**: Current availability, bound account identity and eligibility to disconnect, reconnect or change accounts.
- **Import operation**: Source, known mode, observed state/phase/counts, timestamps, account context and available actions.
- **Import result**: Full or partial report, exact per-entity created/updated/skipped/errored counts and retained errors.
- **Viewing context**: Selected source, selected operation, current-preview eligibility, monitoring state and open connection dialog; separate from the lifetime of accepted work.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All seven connection fixture categories in User Story 1 present correct identity/status/actions, including blocked and stale account-change cases, with zero unintended changes on cancellation.
- **SC-002**: Every result category in User Story 2 uses truthful copy; mixed, all-skipped and zero-record fixtures produce zero claims of writes that did not occur.
- **SC-003**: For queued, running, cancellation-requested, succeeded, failed, cancelled and monitoring-unavailable cases, users can identify the state and permitted next action without reading internal job-kind identifiers.
- **SC-004**: The complete fixture journey is keyboard operable at 360, 768 and 1440 CSS-pixel widths, at 640-pixel height and at 200% zoom, with no clipped essential controls or page-wide horizontal overflow.
- **SC-005**: Reload, source replacement, historical selection and account-switch scenarios produce zero accidental confirmations or duplicate submissions; retained reports remain accessible.
- **SC-006**: An explicitly authorized real Harvest dry-run reaches either a readable preview or an actionable provider error without modifying business records; its result and any permission limitations are recorded separately from fixture test results.

## Assumptions

- Scope is the existing administrator Importers surface, including its Harvest-connection and CSV paths. Existing English product copy is retained; localization is not added.
- The visual handoff is `design/project/app/10_Importers.dc.html`, interpreted with `DESIGN.md`. Current product guarantees take precedence over unsupported prototype copy and interactions.
- Features 005, 006 and 007 provide background jobs, retained reports, remote CLI compatibility and protected account changes; their contracts remain in force.
- Prefer existing data and truthful fallback text. Small read-only presentation metadata additions are permitted if needed to distinguish actions; no new persistence schema or import engine is planned.
- Account identity may remain an identifier if no verified account display name is available. No new external requests are required merely to decorate a card.
- Automated tests use isolated fixtures. Real-account access and any local deployment needed for the dry-run require a separate explicit operator go-ahead at validation time; specifying the check does not execute it.
- Completion requires acceptance evidence, not only green compilation. This planning delivery does not claim implementation or browser validation.
