# Feature Specification: Safe Harvest Account Switching

**Feature Branch**: `feat/harvest-account-switch`

**Created**: 2026-09-15

**Status**: Specification

**Input**: Provide an explicit Change account flow in the Harvest importer. Disconnect currently removes credentials but preserves the original account binding, leaving administrators unable to recover from connecting the wrong account without operator intervention.

## Clarifications

### Session 2026-09-15

The user approved a scoped Spec Kit workflow and preserving the protection against mixing accounts. These are conservative implementation assumptions, not separately answered questions:

- Switching is available only before any Harvest API data has been imported and while no Harvest import is queued or running. Migrating or deleting imported business data is outside this feature.
- Preserve retained reports and history rather than deleting them. Work belonging to an earlier connection must remain inspectable but cannot execute against the replacement account.
- Changing account first releases the existing connection after explicit confirmation, then uses the normal authorization flow. Cancelling authorization leaves Horae disconnected and able to start a fresh connection.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Correct an account connected by mistake (Priority: P1)

An administrator who connected the wrong Harvest account can change it from the importer without editing the database, including after disconnecting or a failed preview.

**Independent Test**: Connect account A, finish a failed preview, disconnect, confirm Change account, and connect account B. No business data changes; the earlier report remains accessible.

**Acceptance Scenarios**:

1. **Given** an existing binding without imported data or active work, **When** the administrator opens Change account, **Then** the current account and the effects on credentials, history and retries are explained before confirmation.
1. **Given** the confirmation is cancelled, **When** the dialog closes, **Then** the connection, binding and history remain unchanged.
1. **Given** confirmation succeeds, **When** replacement authorization completes, **Then** a different account can be connected and a new preview uses only that account.
1. **Given** replacement authorization is cancelled or fails, **When** the administrator returns, **Then** the disconnected state and a fresh connection action remain available without silently restoring the old account.

### User Story 2 - Understand why a change is blocked (Priority: P1)

An administrator sees why changing accounts would be unsafe and what to do next, without losing access to the current connection.

**Independent Test**: Attempt a change with imported identities, queued work and running work separately. Every attempt is rejected without modifying credentials, jobs, reports or business data.

**Acceptance Scenarios**:

1. **Given** previously imported API data, **When** Change account is inspected or confirmed, **Then** it is blocked with an explanation that account migration requires a separate operation; Disconnect still retains the binding.
1. **Given** queued or running Harvest work, **When** Change account is inspected or confirmed, **Then** it is blocked with a request to finish or explicitly cancel that work first.
1. **Given** a disconnected but bound integration, **When** the importer opens, **Then** it distinguishes reconnection to the original account from changing the account.
1. **Given** the state changed after opening confirmation, **When** confirmation is submitted, **Then** stale confirmation is rejected and the administrator is asked to review the current state.

### User Story 3 - Keep old work separate from a replacement account (Priority: P1)

Administrators can inspect prior attempts without letting old jobs, resubmissions or authorization callbacks affect the new account.

**Independent Test**: Race account change against submission, retry and authorization callbacks from another session. Old requests either finish before the change and prevent it, or are rejected; none act on the replacement account.

**Acceptance Scenarios**:

1. **Given** a retained terminal job from account A, **When** it is retried after switching, **Then** retry fails clearly without creating or executing replacement work; its report remains readable.
1. **Given** an authorization started before switching, **When** its callback arrives afterward, **Then** it cannot reconnect the former account or replace the new credentials, even from another browser session.
1. **Given** a submission acknowledgement was lost before switching, **When** that request is resubmitted afterward, **Then** it cannot become a new import under the replacement account, including after ordinary report retention.
1. **Given** multiple administrators act concurrently, **When** one confirms a change, **Then** only the intended connection is changed; a stale second confirmation cannot disconnect a newly connected account.

### Edge Cases

- No connection or binding exists yet: ordinary Connect remains available; Change account must not invent an account to remove.
- Credentials are expired or were already disconnected: the account binding remains visible and the same eligibility rules apply.
- A job finishes or starts between eligibility inspection and confirmation: eligibility is checked again at confirmation.
- Disconnect races a pending authorization: stale authorization must not undo the administrator's action.
- Legacy jobs, callbacks and clients predate this feature: compatibility must fail closed after a change, not silently reinterpret old work.
- Missing, inactive, demoted, non-administrator or foreign-organization sessions cannot inspect private state or perform changes.
- CSV history and locally entered data are not deleted or reassigned by a Harvest API account change.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Provide a separately labelled Change account action for active administrators in connected and disconnected-but-bound states; explain how it differs from Disconnect.
- **FR-002**: Show the bound account identity, eligibility and specific blocking reasons without revealing secrets.
- **FR-003**: Require explicit confirmation describing connection removal, retained history and invalidated old work. Cancel must have no side effects.
- **FR-004**: Block changes when imported Harvest API identities exist or Harvest work is queued/running. Re-evaluate these conditions when committing the change.
- **FR-005**: Release only the eligible connection and binding, leaving all business records and retained reports intact. Partial changes must not persist after failure.
- **FR-006**: Continue through the existing account authorization flow. Cancellation or failure leaves a recoverable disconnected state.
- **FR-007**: Fence old jobs, request identities and pending authorizations from later connections, including across sessions, restarts, retention and repeated A-to-B-to-A changes.
- **FR-008**: Serialize competing connection changes and work acceptance so that stale confirmation, callback, enqueue or retry cannot overwrite or use a replacement connection.
- **FR-009**: Keep Disconnect's binding-preserving semantics and same-account reconnect behavior; improve the explanations instead of weakening that protection.
- **FR-010**: Enforce active administrator and organization boundaries for every new operation and render loading, success, failure and blocked states accessibly.
- **FR-011**: Require automated tests for eligibility, preservation, old-work fencing, concurrency, authorization and the rendered confirmation states before delivery.

### Key Entities

- **Bound Harvest account**: The source identity attached to one Horae organization, independently of whether credentials are currently present.
- **Connection generation**: An identity boundary separating successive authorizations and account changes from old work.
- **Change confirmation**: Administrator acknowledgement of the specific connection and consequences inspected before submitting a change.
- **Prior import attempt**: Retained work and reports associated with an earlier connection, available for inspection but not execution against another one.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An administrator can change an eligible wrong account through the importer without database access, including after disconnecting and a failed preview.
- **SC-002**: All blocked, cancelled and stale change attempts preserve the original connection and data in acceptance fixtures; successful changes preserve every business record and retained report.
- **SC-003**: Every old callback, job retry and resubmission in the acceptance matrix is prevented from affecting a replacement account, including concurrent and restart scenarios.
- **SC-004**: Authorization tests produce zero cross-organization disclosures, unauthorized changes or secret values in responses.
- **SC-005**: UI acceptance covers keyboard-accessible confirmation/cancellation, both eligible connection states and every blocking reason with an actionable explanation.

## Assumptions

- This extends features 004, 005 and 006; it does not introduce multi-account imports, account-data migration, imported-data deletion, permission escalation in Harvest or a new login protocol.
- Existing report retention and CSV behavior remain in place. Archived or retained old work must never be silently reused for a new source.
- Deployment includes coordinated server/web updates; any protocol compatibility restriction must be documented and tested.
- Existing integer time/money, organization isolation and authenticated mutation constraints remain unchanged.
