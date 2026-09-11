# Feature Specification: Durable Harvest Import Jobs

**Feature Branch**: `feat/durable-harvest-jobs`
**Created**: 2026-09-11
**Status**: Draft

## User Scenarios & Testing

### User Story 1 - Start an import without holding the request open (Priority: P1)

As an organization administrator, I can start a Harvest API or CSV import and immediately receive a job identifier, so a long import does not depend on a browser request remaining open.

**Independent Test**: Start a valid import, close the page, reopen the importer, and observe the job until it reaches a terminal state with a persisted report.

### User Story 2 - Recover and retry interrupted imports (Priority: P1)

As an administrator, I can see that an import failed or was interrupted, retry it safely, and never create duplicate records when work is retried.

**Independent Test**: Stop the server during an import, restart it, and verify the job is recovered or marked retryable; rerun the same source and verify idempotent results.

### User Story 3 - Monitor progress and cancel work (Priority: P2)

As an administrator, I can view progress, errors, and cancellation state for my imports, so I understand what happened without reading server logs.

**Independent Test**: Run an import with multiple pages, verify progress advances, request cancellation, and verify no new work starts after cancellation is acknowledged.

### User Story 4 - Reuse durable work infrastructure (Priority: P2)

As a developer, I can register other bounded background workloads using the same durable job infrastructure, so future exports, notifications, webhooks, and cleanup tasks do not each invent their own retry and recovery mechanism.

**Independent Test**: Register a test job kind, enqueue it transactionally, process it with the worker, and verify its retry, lease, and terminal-state behavior without changing Harvest code.

## Requirements

- **FR-001**: Starting an API or CSV import MUST enqueue a durable job and return its UUID without waiting for the full import.
- **FR-002**: Jobs MUST be scoped to the requesting organization and MUST be visible only to authorized administrators of that organization.
- **FR-003**: A job MUST have explicit `queued`, `running`, `succeeded`, `failed`, and `cancelled` states.
- **FR-004**: The worker MUST claim jobs atomically so multiple Horae instances cannot process the same job concurrently.
- **FR-005**: A running job MUST have a lease/heartbeat; an expired lease MUST become eligible for retry after a server crash.
- **FR-006**: Retryable failures MUST use bounded exponential backoff and a configurable maximum attempt count.
- **FR-007**: Import checkpoints MUST be persisted often enough that a retry does not restart an already completed page or entity batch.
- **FR-008**: Retried work MUST preserve the importer’s existing idempotency and provenance guarantees.
- **FR-009**: Job progress MUST include the current phase, processed count, total when known, and the latest error when applicable.
- **FR-010**: Cancellation MUST be cooperative and MUST leave committed database changes valid; it MUST NOT roll back work committed before cancellation.
- **FR-011**: Job payloads MUST be versioned and MUST NOT contain OAuth access or refresh tokens.
- **FR-012**: Completed and terminally failed jobs MUST be retained long enough for administrators to inspect their report, then be eligible for cleanup.
- **FR-013**: The worker MUST stop cleanly during application shutdown and MUST release or expire its leases.
- **FR-014**: The job envelope MUST support a registered job kind, versioned payload, organization scope, idempotency key, and bounded execution policy without exposing queue internals to callers.
- **FR-015**: New job kinds MUST be able to use the same retry, lease, cancellation, progress, and retention mechanisms.
- **FR-016**: Enqueuing a job as a consequence of a domain mutation MUST support transactional outbox semantics, so a committed mutation cannot lose its event before delivery.
- **FR-017**: Outbox delivery MUST be idempotent and MUST record delivery attempts and the last error; external side effects MUST NOT be described as exactly-once.
- **FR-018**: The first release MUST implement only Harvest imports; exports, webhooks, notifications, emails, and cleanup jobs are extension points documented for later work.

## Non-Goals

- Queueing interactive CRUD mutations such as timers, time entries, or invoice transitions.
- Adding Redis, RabbitMQ, or another runtime service.
- Scheduled automatic Harvest synchronization.
- Importing Harvest invoices, expenses, users, or teams.
- Guaranteeing exactly-once execution for external Harvest HTTP side effects.
- Implementing every future job kind in this feature.

## Success Criteria

- **SC-001**: An administrator can start an import and receive a response without waiting for import completion.
- **SC-002**: A server restart does not silently lose an enqueued or running import.
- **SC-003**: Retrying the same source produces no duplicate Horae records.
- **SC-004**: An administrator can identify the current state and final report of every import started by their organization.
