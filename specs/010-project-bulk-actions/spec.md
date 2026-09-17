# Feature Specification: Project bulk actions

**Feature Branch**: `feat/project-bulk-actions`

**Created**: 2026-09-17

**Status**: Implemented; locally verified

**Input**: Add the Projects design's multiple selection and Actions menu in a separate PR after #204; implement archive/reactivate only and preserve the shared design system.

## Clarifications

### Session 2026-09-17

- Q: Should this delivery include bulk tasks, tags and field editing? → A: No. The user confirmed selection plus archive/reactivate; other actions are deferred.

## User Scenarios & Testing

### User Story 1 - Select visible projects (Priority: P1)

As a manager or administrator, I can select individual projects or all matching projects and see how many are selected before choosing an action.

**Why this priority**: A trustworthy selection defines the exact scope of any bulk change.

**Independent Test**: Select two rows with mouse and keyboard, toggle select-all, change search/client/status filters, and verify the selected count and mixed state without changing data.

**Acceptance Scenarios**:

1. **Given** multiple visible projects, **When** one is selected, **Then** its checkbox is checked, the header is mixed and Actions shows one selected project.
1. **Given** a filtered list, **When** select-all is activated, **Then** only visible rows are selected; activating it again clears selection.
1. **Given** a selection, **When** any filter changes, **Then** selection clears; hidden projects cannot be acted on or silently reselected when filters are restored.
1. **Given** a member, **When** Projects is opened, **Then** bulk management controls are absent and existing read-only presentation is preserved.

### User Story 2 - Archive or reactivate a selection (Priority: P1)

As a manager or administrator, I can confirm a single operation to archive active projects or reactivate archived projects without changing their history.

**Why this priority**: Removes repetitive per-project work while preserving deliberate control over the affected projects.

**Independent Test**: Select two projects, cancel an archive confirmation, then confirm it; locate both in Archived and reactivate them together.

**Acceptance Scenarios**:

1. **Given** selected active projects, **When** Archive is chosen, **Then** a confirmation identifies every selected project and the count; cancel changes nothing.
1. **Given** a confirmed action, **When** it succeeds, **Then** all selected projects reach the requested state, the list/counts refresh, selection clears and success is announced.
1. **Given** a pending action, **When** the user clicks repeatedly or presses Escape, **Then** no duplicate submission or premature dismissal occurs.
1. **Given** a failed or interrupted request, **When** an error is displayed, **Then** the confirmation remains usable for retry or cancellation without claiming success.

### Edge Cases

- Empty/loading/failed list: no actionable selection.
- Refresh removes a selected row: exclude it from the active selection before confirmation.
- Missing or foreign-organization project in a batch: reject the entire batch without changing any project.
- Repeated identifiers and already-requested statuses: do not duplicate changes or transition notifications.
- Concurrent edits: preserve project details; concurrent overlapping batches must not deadlock due to inconsistent project ordering.
- Lost response after a commit: a retry is safe; do not promise that a transport error means nothing changed.
- More than 100 selected projects: explain the limit and require narrowing the selection; never silently truncate.

## Requirements

### Functional Requirements

- **FR-001**: Only signed-in managers and administrators may perform bulk status changes, restricted to their organization; enforce this beyond the visible controls.
- **FR-002**: Provide per-row and select-all checkboxes with checked, unchecked and mixed header states and an accurate selected count in Actions.
- **FR-003**: Selection is page-local, clears on filter changes and successful mutation, and never includes currently hidden or unavailable rows in a submitted batch.
- **FR-004**: Actions offers Archive in active/budgeted views and Reactivate in archived view; no selection, unavailable data or an oversized selection disables the operation and explains why.
- **FR-005**: Confirmation identifies the action, count and project names; cancellation makes no change. Pending submissions prevent duplicates and dismissal.
- **FR-006**: A valid batch of 1–100 projects commits all requested status changes together. Any invalid, missing or foreign project rejects the whole batch. Duplicate identifiers are harmless; unchanged statuses produce no transition notification.
- **FR-007**: Only active/archived status changes. Project details, assignments, time entries, invoices and historical totals remain unchanged. Existing single-project actions keep working.
- **FR-008**: Show success or failure explicitly; refresh the list on success. Failures retain a retry/cancel path and transport errors must not imply a guaranteed rollback.
- **FR-009**: Match selection and Actions in the handoff using the shared design system without changing existing component defaults. Support keyboard use, named checkboxes, mixed-state announcement, focus restoration, light/dark themes and existing responsive layouts.
- **FR-010**: Deliver automated checks for selection, authorization, atomicity, idempotency and cross-page styling regressions; test against isolated data, never the real imported workspace.

### Key Entities

- **Project**: Existing organization-owned project with an active/archived status and unchanged business/history fields.
- **Selection**: Temporary set of visible project identities; not persisted or shared across pages.
- **Bulk confirmation**: Snapshot of selected project identities/names and requested action, with pending/error state.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A manager can archive two visible projects and reactivate them together with one confirmation per operation; cancel leaves both unchanged.
- **SC-002**: All negative permission and invalid-batch acceptance cases change zero projects; retrying a successful request produces zero additional status transitions.
- **SC-003**: Selection and confirmation are operable by keyboard at 320, 768 and 1440 pixel widths; both themes retain readable controls and no document-level horizontal overflow.
- **SC-004**: Existing shared controls and eight other audited pages retain their pre-change styling; all existing Projects, menu and responsive checks pass alongside the new tests.

## Assumptions

- Archive/reactivate uses existing project lifecycle semantics, not deletion or synchronization back to Harvest.
- Atomic batches and a 100-project maximum are conservative defaults for a synchronous interactive operation; users can narrow large selections.
- Changing filters clears all selection rather than keeping an invisible cross-filter basket.
- Bulk tasks, tags, edits, deletion and pinning are explicitly outside this delivery.
- Existing authentication, menus, dialogs and CSS utilities remain the foundation; no new infrastructure is required.
