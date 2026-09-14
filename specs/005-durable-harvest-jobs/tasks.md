# Tasks: Durable Harvest Import Jobs

## Phase 1 - Spike

- [x] T001 Verify `sqlxmq` 0.6.0 compiles with Horae’s Rust/SQLx versions in the feature worktree.
- [x] T002 Review the library migrations and reject direct adoption because its UUIDv4/`uuid-ossp` schema violates Horae invariants.
- [x] T003 Record the dependency decision and fallback to a small in-house queue adapter.

## Phase 2 - Persistence and worker

- [x] T004 Add a registered job-kind envelope with versioned payload, idempotency key, org scope, and execution policy.
- [x] T005 Persist CSV uploads safely for asynchronous execution without retaining request-body streams.
- [x] T006 Add worker startup, bounded concurrency, lease/heartbeat, retry backoff, and graceful shutdown.
- [ ] T007 Add checkpoint/progress updates around existing Harvest import phases.
  - Durable CSV commits checkpoint every 500 records. API commits persist catalog pages, 500-entity parent batches and individual time-entry pages, with report/cache and watermark state.
  - CSV previews checkpoint rollback-only simulation state every 500 records. API previews retain parent snapshots and entry associations at the same boundaries as API commits. All planned scale scenarios and the optimized API preview passed; measured CSV preview overhead remains unresolved. See `performance.md`.
- [x] T008 Add outbox records and transactional enqueue support for future plugin/webhook/notification delivery.
- [x] T009 Ensure all job and outbox reads/mutations enforce organization and admin authorization.
  - Registered HTTP handlers enforce live administrator sessions and tenant isolation; uploads have a composite job/organization foreign key. Outbox primitives are internal worker APIs, with organization and claim-token checks on acknowledgements.

## Phase 3 - API and UI

- [x] T010 Change API and CSV start functions to enqueue and return job identifiers.
- [x] T011 Add status, history, cancel, and retry server functions.
- [x] T012 Update the importer page to show queued/running progress and terminal reports.
- [x] T013 Add cleanup policy for old terminal jobs and stored upload data.

## Phase 4 - Verification

- [x] T014 Test atomic claims with concurrent workers.
- [x] T015 Test lease recovery after a simulated worker failure.
- [ ] T016 Test retry idempotency and cancellation semantics.
- [x] T017 Test a second synthetic job kind through the same worker boundary.
- [x] T018 Test transactional outbox insertion and idempotent delivery bookkeeping.
- [x] T019 Regenerate `.sqlx` cache and run targeted integration tests, clippy, and formatting.
- [x] T020 Preserve inspectable reports for terminal failures (FR-012/SC-004), including failures after confirmed batches and before the first checkpoint; verify status/history, retention and manual retry.
- [ ] T021 Bound report metadata, checkpoint error state and error retrieval without discarding individual error details; cover legacy reports, atomic archival/recovery, authorization, retention and the complete UI/download path. See the bounded-report follow-up in `plan.md`.
  - Verified: bounded new reports, legacy upgrade/rollback with one connection, claim fencing, retention, authorization, captured download boundaries, lazy page reads and missing-fragment errors.
  - Remaining: broader legacy state coverage, API overflow, socket-level/memory stress and full browser acceptance.

See [adversarial-review.md](adversarial-review.md) for uncovered cases and the
evidence required to close the reopened tasks.
