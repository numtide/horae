# Implementation Plan: Durable Harvest Import Jobs

## Summary

Move Harvest API/CSV imports from request-bound execution to durable PostgreSQL-backed jobs. Use a small Horae-owned adapter with the queue semantics validated during the `sqlxmq` spike; preserve the existing importer engine, provenance, and idempotency behavior.

## Technical Context

Horae is a Rust/Dioxus/Axum application using PostgreSQL and SQLx. Current Harvest server functions execute the import inline and the importer already provides bounded in-memory streaming, locking, retry, and idempotent provenance behavior. This feature changes delivery and lifecycle, not the importer’s domain transformations.

**Language/Version**: Rust edition 2024 (Nix-pinned toolchain)

**Primary Dependencies**: Dioxus fullstack, Axum, Tokio, SQLx 0.8; no queue dependency after the UUIDv7 schema spike

**Storage**: PostgreSQL 15+, including queue tables and CSV upload retention data

**Testing**: `cargo test`, `#[sqlx::test]`, serial integration tests, `nix flake check`

**Target Platform**: Self-hosted Linux server with WASM client

**Project Type**: Fullstack web application

**Performance Goals**: Start-job response remains interactive; worker concurrency is bounded; import throughput remains within current importer limits

**Constraints**: No Redis; no credentials in payloads; existing UUIDv7/org isolation/idempotency invariants remain mandatory

**Scale/Scope**: One worker process per Horae instance initially; one durable job family (Harvest import) in the MVP

## Constitution Check

- **I. Exactness**: Pass — queue metadata does not alter integer minute or money calculations; importer remains the source of truth.
- **II. Domain Purity**: Pass — no queue or persistence code enters `horae-core`.
- **III. Single Datastore**: Pass — queue and upload state use the existing PostgreSQL datastore; UUIDv7 and `org_id` are retained.
- **IV. Server Functions**: Pass — start/status/cancel/retry operations remain authenticated server functions.
- **V. Reproducible Builds**: Pass pending — dependency versions, SQLx cache, formatting, and flake checks are required before merge.

## Phases

1. **Dependency spike**: compile a minimal `sqlxmq` runner against the pinned Horae toolchain; inspect its migrations; reject direct adoption because its UUIDv4/`uuid-ossp` schema violates Horae invariants.
1. **Reusable job boundary**: define a registered job-kind envelope with versioned payload, org scope, idempotency key, execution policy, progress, and cancellation. Keep handlers in application modules.
1. **Persistence and outbox**: define job/upload persistence and an outbox delivery model that can be written in the same transaction as domain mutations.
1. **Worker**: start one bounded worker runner during server startup; configure lease, retry, shutdown, and cancellation behavior; invoke the existing importer engine.
1. **Server/UI contract**: change start functions to return job status, add status/list/cancel/retry functions, and update the importer page to poll persisted state.
1. **Verification**: add database tests for atomic claims, authorization, retry/recovery, cancellation, idempotency, and a second synthetic job kind; run formatting, clippy, and SQLx cache preparation.

## Project Structure

```text
specs/005-durable-harvest-jobs/
├── spec.md
├── research.md
├── plan.md
├── tasks.md
├── data-model.md
├── quickstart.md
├── contracts/import-jobs.md
└── checklists/requirements.md

crates/horae/
├── migrations/                 # queue and upload tables
├── src/jobs.rs                 # queue adapter and worker lifecycle
├── src/server_fns/importers.rs # start/status/cancel/retry functions
├── src/pages/importers.rs       # progress/history UI
└── tests/                      # SQLx integration coverage
```

**Structure Decision**: Keep the first implementation in `crates/horae`; the queue is application infrastructure and does not belong in `horae-core`. Extracting a worker crate is deferred until independent deployment is justified.

## Future Job Catalog

| Job family | Queue-ready? | Initial implementation |
|---|---:|---|
| Harvest API/CSV import | Yes | This feature |
| Large report/document export | Yes | Later |
| Plugin/webhook/notification delivery | Yes, via outbox | Later |
| Email delivery | Yes | Later |
| Cleanup/reconciliation | Yes | Later |
| Interactive CRUD mutation | No | Synchronous server function |

## Constraints

- No Redis or new deployment service.
- No new workspace crate in the first PR.
- No changes to core domain invariants.
- Never serialize OAuth tokens into a job payload.
