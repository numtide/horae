# Tasks: Durable Harvest Import Jobs

## Phase 1 - Spike

- [x] T001 Verify `sqlxmq` 0.6.0 compiles with Horae’s Rust/SQLx versions in the feature worktree.
- [x] T002 Review the library migrations and reject direct adoption because its UUIDv4/`uuid-ossp` schema violates Horae invariants.
- [x] T003 Record the dependency decision and fallback to a small in-house queue adapter.

## Phase 2 - Persistence and worker

- [x] T004 Add a registered job-kind envelope with versioned payload, idempotency key, org scope, and execution policy.
- [ ] T005 Persist CSV uploads safely for asynchronous execution without retaining request-body streams.
- [ ] T006 Add worker startup, bounded concurrency, lease/heartbeat, retry backoff, and graceful shutdown.
- [ ] T007 Add checkpoint/progress updates around existing Harvest import phases.
- [ ] T008 Add outbox records and transactional enqueue support for future plugin/webhook/notification delivery.
- [ ] T009 Ensure all job and outbox reads/mutations enforce organization and admin authorization.

## Phase 3 - API and UI

- [ ] T010 Change API and CSV start functions to enqueue and return job identifiers.
- [ ] T011 Add status, history, cancel, and retry server functions.
- [ ] T012 Update the importer page to show queued/running progress and terminal reports.
- [ ] T013 Add cleanup policy for old terminal jobs and stored upload data.

## Phase 4 - Verification

- [ ] T014 Test atomic claims with concurrent workers.
- [ ] T015 Test lease recovery after a simulated worker failure.
- [ ] T016 Test retry idempotency and cancellation semantics.
- [ ] T017 Test a second synthetic job kind through the same worker boundary.
- [ ] T018 Test transactional outbox insertion and idempotent delivery bookkeeping.
- [ ] T019 Regenerate `.sqlx` cache and run targeted integration tests, clippy, and formatting.
