# Feature Specification: Harvest Import and Job Management CLI

**Feature Branch**: `feat/harvest-jobs-cli`

**Created**: 2026-09-14

**Status**: Implemented and validated (PR #199)

**Input**: Add command-line Harvest API/CSV imports and job management: preview, full/incremental synchronization, status, history, report download, cancellation, retry, structured output and optional waiting. Reuse durable jobs; closing the terminal must not cancel accepted work.

## Clarifications

### Session 2026-09-14

The implementation request follows the recommended server-client design. The following are explicit implementation assumptions, not separately answered user questions.

- Access model: use the running server with an active administrator session; never use direct database authority or start an implicit worker.
- Credential provisioning: reuse an existing web-authenticated session in an explicitly selected private, origin-bound file. Do not add a login protocol, browser automation, raw credential argument or long-lived token system. Expired/revoked sessions require renewed credentials through the existing login flow.
- Automation: distinguish command acceptance, completed success with record errors, job failure, cancellation, timeout, interruption and indeterminate submission. Keep partial-success exit status zero, as specified in feature 004.
- Resubmission: an explicit key is scoped to organization and source kind, bound to mode/scope and exact CSV contents. Keys are time-bounded so retention cannot silently turn an old retry into a new import; exact bounds belong in the CLI contract.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start an import from the terminal (Priority: P1)

An authorized operator starts a Harvest import without opening the importer page, receives its job identifier, and can close the terminal while processing continues independently.

**Why this priority**: Completes the terminal import capability already planned in feature 004 while preserving feature 005's recovery guarantees.

**Independent Test**: Submit a preview and a committing import for each supported source, close the submitting process, and inspect the same jobs from a new process and the web history.

**Acceptance Scenarios**:

1. **Given** an existing Harvest connection, **When** the operator starts a full or incremental import, **Then** a durable job identifier is returned without waiting for import completion.
1. **Given** a readable valid CSV, **When** submission is acknowledged, **Then** the job can complete even if the terminal closes and the local file becomes unavailable.
1. **Given** unchanged source and destination data, **When** a full preview is followed by a full commit, **Then** their outcome counts agree and the preview leaves no imported domain changes.
1. **Given** previously imported source records, **When** the same source is imported again, **Then** no duplicate entries are created.
1. **Given** invalid arguments, an unreadable or oversized file, missing Harvest connection, or insufficient authority, **When** submission is attempted, **Then** a clear failure is reported without accepting an import.
1. **Given** an expired session or an inactive, demoted or non-administrator user, **When** a command runs, **Then** it fails without revealing another user's job data or submitting work.
1. **Given** a stopped server, **When** submission is attempted, **Then** the command reports unavailability without opening a database connection or running work locally.
1. **Given** an insecure credential file or a non-local plaintext server address, **When** a command runs, **Then** it refuses to send the credential. Redirects must not forward credentials to another destination.

### User Story 2 - Inspect and export job outcomes (Priority: P1)

An operator can inspect a specific job or recent history and retrieve its retained outcome summary and complete record errors from the terminal.

**Why this priority**: Detached execution is useful only when operators can find its status and understand partial or failed outcomes.

**Independent Test**: Inspect running, successful, failed and cancelled jobs created through either surface and compare summaries and downloaded errors with the web surface.

**Acceptance Scenarios**:

1. **Given** a retained job, **When** its status is requested, **Then** the result identifies its state, progress, cancellation request, confirmed outcomes and latest attempt failure when present.
1. **Given** more retained jobs than one history page, **When** history is paged, **Then** results follow the existing ordering and organization boundary without duplicating or omitting jobs in an unchanged history.
1. **Given** a report with more errors than its inline summary holds, **When** errors are downloaded, **Then** every error at the captured report boundary is included, not just the inline subset.
1. **Given** an interrupted or incomplete download, **When** retrieval ends, **Then** the command fails visibly and does not present the partial output as complete.
1. **Given** a foreign, unknown or expired job identifier, **When** any operation addresses it, **Then** no job data is disclosed and no new job is silently substituted.

### User Story 3 - Control and automate durable work (Priority: P2)

An operator can request cancellation, retry eligible jobs and optionally wait for completion. Scripts receive structured results that distinguish command acceptance from completed import outcomes.

**Why this priority**: Enables unattended workflows and recovery without creating another execution mechanism.

**Independent Test**: Submit a job, interrupt its waiter, request cancellation from a new process, retry after cancellation is acknowledged, and verify the completed outcome without duplicates.

**Acceptance Scenarios**:

1. **Given** a running job, **When** cancellation is requested, **Then** the result distinguishes an accepted request from terminal cancellation and preserves previously confirmed changes.
1. **Given** an eligible failed or cancelled job, **When** retry is requested, **Then** the existing retry policy is enforced and confirmed progress is preserved; retrying an ineligible job fails clearly.
1. **Given** an active wait, **When** its deadline expires or the operator interrupts it, **Then** only observation stops, the job identifier remains available, and the job is not implicitly cancelled.
1. **Given** structured-output mode, **When** any command completes or fails, **Then** standard output follows a documented, versioned JSON result contract and human progress messages do not corrupt it.
1. **Given** a completed import with record errors, **When** the waiter finishes, **Then** it reports partial success distinctly from a failed job; the default exit behavior preserves the existing importer contract that partial success is success.

### Edge Cases

- Lost submission acknowledgement: expose an explicit idempotency identity for safe resubmission of the same request; reuse with different input must not silently select an unrelated job.
- A failed upload or lost connection before acknowledgement must not be reported as an accepted import; an indeterminate acknowledgement must be distinguishable from a definitive rejection.
- Unavailable service or stopped workers must not cause the CLI to start an implicit worker; queued jobs remain queued and waiting is bounded when a deadline is supplied.
- Credentials can expire and administrator access can be revoked between commands; rejected access must not reveal report data or credentials.
- A job can finish while cancellation is being requested; return its actual current state rather than claiming rollback or cancellation occurred.
- Reports can expire during retrieval; missing data must fail retrieval rather than silently truncate it.
- Output to a file must refuse overwriting an existing file unless the operator explicitly requests replacement.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Provide terminal submission of Harvest API and CSV jobs, with preview and commit modes, using the existing durable lifecycle rather than request-bound execution.
- **FR-002**: Support explicit full/incremental API scope and document defaults. Preserve the existing planned defaults: commit when no preview flag is supplied, and incremental when a watermark exists, otherwise full. Reject contradictory flags before submission.
- **FR-003**: Identify the accepted job and distinguish submission acknowledgement from import completion. Acknowledged CSV work must not depend on the submitting process or local file.
- **FR-004**: Provide bounded, paginated history and individual status for retained Harvest jobs from both the web and CLI surfaces.
- **FR-005**: Retrieve confirmed report summaries and stream complete record errors with detectable retrieval failure and safe output-file handling.
- **FR-006**: Support cooperative cancellation and policy-checked retry without changing confirmed data, lease, retention or idempotency guarantees.
- **FR-007**: Provide optional waiting for submission and retry, plus waiting on an existing identifier, with an optional deadline. Interrupting observation must not cancel work.
- **FR-008**: Provide readable terminal output and a documented versioned JSON result contract. Document exit semantics for acknowledgement, success, partial success, rejected commands, failed/cancelled jobs, wait timeout and interruption.
- **FR-009**: Preserve organization isolation and credential confidentiality for every operation, including downloads. Do not introduce an implicit privilege bypass.
- **FR-010**: Operate as a client of the running server with an active administrator session. Read credentials only from an explicitly selected private file bound to that server's origin; enforce secure transport except on loopback, refuse redirects, and never expose credential values in errors or logs. Session renewal uses the existing web login; a CLI login protocol is out of scope.
- **FR-011**: Support safe resubmission after uncertain acknowledgement using an explicit idempotency identity and reject conflicting reuse. Documentation must distinguish resubmission from retrying an existing job.
- **FR-012**: Reuse existing Harvest credentials, source validation, numeric exactness, user matching and import semantics. Do not reconnect accounts or weaken source-account binding.
- **FR-013**: Never launch a worker implicitly. Submission requires the running server. Service unavailability must produce an actionable command failure; worker unavailability must not turn an accepted queued job into a fabricated failure or success.

### Key Entities

- **Operator**: An active organization administrator authenticated with an existing server session, supplied through the private credential file described in FR-010.
- **Import request**: Source, preview/commit mode, API synchronization scope where applicable, and resubmission identity.
- **Durable job**: The existing organization-scoped identifier, state, progress, attempt policy and confirmed outcomes shared with the web surface.
- **Command result**: Versioned machine-readable acknowledgement, observation or failure, separate from the job's eventual state.
- **Retained report**: Confirmed summary and complete record errors, available only within existing retention and authorization boundaries.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance tests, all four source/mode combinations (API/CSV × preview/commit) remain inspectable after the submitting process exits; commits finish and previews leave zero imported domain changes.
- **SC-002**: CLI and web observations agree on identifiers, terminal states, counts and complete record-error contents for the same captured report boundary across success, partial success, failure and cancellation fixtures.
- **SC-003**: Retrying an interrupted import and reimporting the same completed source create zero duplicate entries in the recovery acceptance fixtures.
- **SC-004**: Every documented command has structured-output and exit-status tests covering success and failure, including timeout/interruption for waiting commands; no human progress text contaminates structured output.
- **SC-005**: Authorization acceptance tests demonstrate zero foreign-organization disclosures or mutations and zero secret values in command output and diagnostics.
- **SC-006**: A waiter interrupted after acknowledgement leaves the job available to a new process; cancellation is performed only by an explicit cancellation request.

## Assumptions

- Feature 005 is the authoritative durable job contract. Feature 004's historical inline server contracts do not reintroduce retired import endpoints.
- Harvest OAuth connection setup remains in the existing web flow. This feature does not add a second Harvest login flow or accept raw Harvest tokens on command lines.
- Partial success remains success by default, with explicit error counts. Numeric exit codes and exact command spelling belong in the planned CLI contract.
- Existing upload limits, report retention and retry policy apply unchanged.
- No scheduler, automatic synchronization, new production job kinds, email, webhooks, exports, independent worker command or new workspace crate is included.

## Dependencies and Specification Boundaries

- [Harvest importer](../004-harvest-importer/spec.md) and its [CLI contract](../004-harvest-importer/contracts/importer-api.md#4-cli-subcommands): source behavior and CLI tasks T023, T029, T031, T035 and T039. Their implementation evidence is recorded in this feature's acceptance report; release gates remain tracked here.
- [Durable Harvest jobs](../005-durable-harvest-jobs/spec.md) and [job contract](../005-durable-harvest-jobs/contracts/import-jobs.md): shared lifecycle, authorized operations and reports.
- [Horae Constitution](../../.specify/memory/constitution.md): mandatory invariants, including the authenticated mutation boundary preserved by FR-010. No amendment or privilege bypass is required.
