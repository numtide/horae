# Requirements Checklist: Durable Harvest Import Jobs

**Purpose**: Validate completeness and consistency before implementation.

- [x] CHK001 User stories are independently testable.
- [x] CHK002 Requirements define authorization and organization scoping.
- [x] CHK003 Requirements define all lifecycle states and terminal behavior.
- [x] CHK004 Crash recovery and retry semantics are explicit.
- [x] CHK005 Idempotency and external-side-effect limitations are explicit.
- [x] CHK006 Cancellation semantics preserve committed database work.
- [x] CHK007 Payloads exclude credentials and unbounded request data.
- [x] CHK008 Data model defines UUIDv7, lease, progress, checkpoint, and retention fields.
- [x] CHK009 API contract covers start, status, history, cancel, and retry.
- [x] CHK010 Plan, tasks, and quickstart reference the same first MVP: Harvest imports only.
- [x] CHK011 Queue mechanics are reusable without committing future job handlers to this feature.
- [x] CHK012 Outbox semantics are distinguished from ordinary background jobs.
- [x] CHK013 Future exports, delivery, email, and cleanup use cases are documented as deferred extension points.

## Analysis findings

- No unresolved `[NEEDS CLARIFICATION]` markers remain.
- No requirement conflicts with existing Harvest idempotency or organization invariants.
- The first implementation intentionally does not queue interactive CRUD mutations or plugin events.
- Plugin/webhook/notification delivery is represented as a future outbox consumer, not as an immediate implementation obligation.
