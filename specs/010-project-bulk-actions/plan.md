# Implementation Plan: Project bulk actions

**Branch**: `feat/project-bulk-actions` | **Date**: 2026-09-17 | **Spec**: [spec.md](spec.md)

## Summary

Extend Projects with visible-row selection, shared Actions and confirmation. Add a bounded atomic server function reusing existing single-project transaction operations and event semantics. No migration, dependency, queue or crate.

## Technical Context

**Language/Version**: Rust edition 2024, pinned Nix toolchain.
**Primary Dependencies**: Existing Dioxus, SQLx, Tokio, PostgreSQL, Playwright.
**Storage**: Existing projects.active; page-local selection/confirmation signals.
**Testing**: SSR controls, SQLx transaction tests, real isolated-server authorization and browser regressions.
**Target Platform**: Linux server and browser WASM.
**Project Type**: Fullstack web application.
**Performance Goals**: One request/transaction per batch, at most 100 IDs, no unbounded tasks.
**Constraints**: Org/role checks, sorted locks, postcommit dispatch attempts, unchanged shared defaults.
**Scale/Scope**: Selection and archive/reactivate, both themes at 320/768/1440px.

## Constitution Check

Pre-research and post-design: PASS. No money/time arithmetic changes; no core I/O dependencies; PostgreSQL only with unchanged schema/IDs; all mutations use session/role-checked server functions. Nix formatting, lint, compilation and isolated tests are required; green CI is required before a future merge. No merge in this delivery.

## Project Structure

### Documentation

`specs/010-project-bulk-actions/`: spec, plan, research, data-model, contracts, quickstart, tasks, quality checklist.

### Source Code

- `crates/horae/src/pages/projects.rs`: selection and confirmation.
- `crates/horae/src/components/controls.rs`: optional mixed/disabled/compact checkbox props.
- `crates/horae/assets/css/horae.css`: opt-in selectable grid and existing-size token.
- `crates/horae/src/server_fns/projects.rs`: batch endpoint and shared transaction setter.
- `crates/horae/src/server_fns/projects/bulk_tests.rs`: rollback/no-op/validation/concurrency.
- `crates/horae/tests/trigger_utilities.rs`: checkbox default/state rendering.
- `crates/horae/tests/detail_navigation.rs`: production-page fixture wiring.
- `crates/horae/tests/browser/project-bulk-actions.cjs`: selection, confirmation and real HTTP authorization.
- Existing browser suites/runner: retain assertions while accommodating selection; run new suite in CI.

**Structure Decision**: Extend existing modules. One focused Rust test module; keep single-project contracts intact.

## Complexity Tracking

No exceptions. Independent client mutations cannot provide the required atomic behavior.
