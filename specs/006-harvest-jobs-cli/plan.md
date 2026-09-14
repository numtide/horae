# Implementation Plan: Harvest Import and Job Management CLI

**Branch**: `feat/harvest-jobs-cli` | **Date**: 2026-09-14 | **Spec**: [spec.md](./spec.md)

## Summary

Extend the server binary with network-only import/job commands, dispatched before deployment configuration is loaded. Reuse authenticated server functions, shared report types and the existing HTTP dependency. Name stable routes explicitly and enforce content-bound idempotency server-side. No new importer, worker, login protocol or crate.

## Technical Context

**Language/Version**: Rust 2024, Nix-pinned toolchain.
**Primary Dependencies**: Existing Clap, Tokio, serde, Dioxus fullstack reqwest re-export, sqlx and UUID.
**Storage**: Existing PostgreSQL jobs/uploads; private local session and report files. No CLI database access.
**Testing**: Unit/parser tests, HTTP fixtures, registered server functions with real sessions/PostgreSQL, executable acceptance and Nix gates.
**Target Platform**: Linux operators/deployments; shared code must retain WASM compilation.
**Project Type**: Existing CLI/fullstack app.
**Performance Goals**: Stream uploads/downloads; bound ordinary JSON to 4 MiB, session input to 16 KiB and history to 100 items. Poll every second; request timeout 30 seconds, upload/download timeout five minutes, optional overall wait deadline.
**Constraints**: HTTPS except loopback, no redirects/implicit proxies, no credential logging, existing 50 MiB CSV limit and job policies.
**Scale/Scope**: Two import commands and seven job commands; no scheduler or additional job kinds.

## Constitution Check

Pre-research and post-design checks pass. I: reuse exact integer values. II: no I/O in core. III: existing org-scoped PostgreSQL jobs and UUIDv7 identities. IV: all mutations remain administrator-checked Dioxus functions; no DB/auth bypass. V: Nix formatting, core/server tests, Clippy, WASM, fresh SQLx metadata and full flake checks are required before completion.

## Project Structure

- `crates/horae/src/cli.rs`: parsing and remote options.
- `crates/horae/src/cli/imports.rs`: dispatch, output and waits.
- `crates/horae/src/cli/imports/transport.rs`: private session loading and bounded HTTP/files.
- `crates/horae/src/cli/imports/tests.rs`: transport/output/parser tests.
- `crates/horae/src/main.rs`: dispatch before deployment configuration.
- `crates/horae/src/server_fns/importers.rs`: stable routes, optional idempotency header and upfront validation.
- `crates/horae/src/jobs.rs`: atomic content-bound enqueue; lifecycle unchanged.
- `crates/horae/src/server_fns/importers/authorization_tests.rs`: extend real-session harness without another global AppState.
- `crates/horae/tests/cli_imports.rs`: actual executable acceptance.
- `.sqlx/`: regenerate changed macros with disposable migrated PostgreSQL.
- `specs/006-harvest-jobs-cli/`: research, model, contract, quickstart, tasks and acceptance evidence.

## Phases

1. Secure session/transport foundation, explicit routes, parser and result contracts; tests first.
1. API/CSV submission, validation, content-bound resubmission and collision tests.
1. Status/history/report and safe complete-error files.
1. Cancel/retry/wait, signals/deadlines and exit behavior.
1. Real-session/executable acceptance for both sources/modes, recovery, authorization and archives; documentation and all Nix gates.

## Workflow Notes

Spec Kit skills/scripts are used directly; the standalone executable is absent. `setup-plan.sh --json` resolved the shipped template. No hooks/preset overrides are configured. The agent-context update script mentioned by the skill is absent; feature context stays in these documents rather than inventing a script.

## Complexity Tracking

No constitution exceptions. Content-bound enqueue is required for safe resubmission; no new schema or generalized job abstraction is planned.
