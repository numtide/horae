# Implementation Plan: Safe Harvest Account Switching

**Branch**: `feat/harvest-account-switch` | **Date**: 2026-09-15 | **Spec**: [spec.md](./spec.md)

## Summary

Add explicit administrator confirmation in the existing importer. Preserve business data and retained reports, refuse changes with provenance or active Harvest work, and fence old requests with durable account generation and connection revision. Reuse the existing OAuth flow, modal, jobs, PostgreSQL locks and CLI transport.

## Technical Context

- Rust 2024, Dioxus fullstack SSR/WASM, PostgreSQL/sqlx, Tokio; existing dependencies only.
- One additive migration: organization-scoped connection state plus captured account generation on jobs. Existing rows start at generation/revision zero.
- Tests: pure state policy, disposable PostgreSQL integration, real-session HTTP, CLI contract, rendered UI and concurrency barriers.
- Short row-lock transactions serialize enqueue/retry and connection changes. The existing import session lock excludes actual API/CSV execution and credential refresh; no lock is held over the external OAuth exchange.
- One importer management flow; no data migration, new crate, scheduler or external permission changes.

## Constitution Check

Pre-research and post-design gates pass: integer time/money untouched; pure eligibility stays in core; PostgreSQL state is keyed by the organization's UUIDv7 FK; administrator mutations stay in server functions; Nix/format/SQLx/server/WASM gates required before delivery. No exception.

## Project Structure

- `crates/horae/migrations/0029_harvest_connection_generations.sql`: persistent generation/revision state and job generation.
- `crates/core/src/importers/harvest/types.rs`: connection metadata and pure eligibility policy.
- `crates/horae/src/importers/harvest/account_switch.rs`: scoped transaction gate, eligibility, atomic change and fencing checks.
- `crates/horae/src/importers/harvest/credentials.rs` and `harvest.rs`: revision-bound OAuth completion and disconnect; execution generation check.
- `crates/horae/src/server_fns/importers.rs`: status, confirm change, capture generation on API submission.
- `crates/horae/src/jobs.rs`: gate enqueue/retry and preserve old-generation identity on conflict.
- `crates/horae/src/cli/imports.rs`: read current connection generation before API submission.
- `crates/horae/src/pages/importers.rs`: connected/disconnected binding explanation and existing accessible Modal.
- Relevant neighboring tests, `.sqlx/` and specs 004/005/006 contracts updated for compatibility.

## Phases

1. RED eligibility and stale-work tests; additive state migration and shared gate.
1. Atomic change and OAuth revision checks; authorization and concurrency acceptance.
1. Generation-bound enqueue/retry/execution and CLI submission; preserved report acceptance.
1. UI confirmation/recovery states and interaction tests.
1. SQLx regeneration, full validation, PR based on `feat/harvest-jobs-cli` while #199 remains open.

## Workflow Notes

Spec Kit local skills and shipped scripts are used directly; standalone `specify` and the agent-context updater are absent. No extension hooks or preset overrides exist. `setup-plan.sh --json` resolved the checked-in template. Its reported BRANCH derives from feature-directory naming; actual Git branch is `feat/harvest-account-switch`. A bounded read-only research agent reviewed races as requested by the plan skill; implementation remains local.

## Complexity Tracking

Two counters are necessary: switching accounts invalidates old work, whereas disconnect/reconnect must invalidate pending OAuth without invalidating same-account job retries. No generalized reset framework or new dependencies.
