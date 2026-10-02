# Implementation Plan: Clients MVP

**Branch**: `feat/clients-mvp` | **Date**: 2026-10-02 | **Spec**: [spec.md](spec.md)

## Summary

Deliver all four authorized stories using narrow authorized projections, a shared
form and current project/invoice editors. Preserve permissions, financial
invariants, recovery and shared styling. No schema/dependency change planned.

## Technical Context

- Rust edition 2024; compiler/dependencies pinned by flake.lock/Cargo.lock.
- Existing Dioxus SSR/WASM, Axum, Tokio, sqlx/PostgreSQL, serde and horae-core.
- Existing client/project/invoice tables and editor recovery stores.
- Tests: core unit tests, disposable PostgreSQL `#[sqlx::test]`, Nix browser/e2e,
  clippy, formatting, SQLx cache and flake/CI checks.
- Web: desktop/mobile, both themes, keyboard, enlarged text.
- Performance: no per-client N+1 reads. Use collection projections admitting
  authorized rows only; no new pagination product policy in this increment.
- Scope: client list/detail/editor and contextual navigation, no contacts,
  lifecycle policy change or billing ledger.

## Constitution Check

Pre-research and post-contract architectural gates: PASS. Runtime tests pending.

| Principle | Contract |
| --- | --- |
| Exactness | Integer minutes/cents + ISO currency; nullable rate differs from zero; no mixed-currency sum. |
| Domain purity | Reuse core money rules; new correctness-critical validation in core with tests. |
| Datastore | Existing PostgreSQL schema, UUID v7, session-org scoping. |
| Mutation surface | Authorized Dioxus server functions only; no ad-hoc browser data fetches. |
| Reproducibility | Nix, locked dependencies, format/clippy/cache/tests and green CI. |

## Project Structure

Artifacts stay in `specs/012-clients-design/increments/mvp/` (spec, research, plan,
data-model, contracts, quickstart, tasks, progress).

- `crates/core/src/client.rs` and `lib.rs`: shared validation if existing helpers
  do not already provide it.
- `crates/horae/src/models/client.rs`: catalog-compatible view/form contracts.
- `crates/horae/src/server_fns/clients.rs` and `clients/tests.rs`: authorized reads,
  shared saves and lifecycle/history regression tests.
- `crates/horae/src/pages/clients.rs` plus sibling `clients/` modules: list/detail,
  reusable form and presentation helpers, no `mod.rs` layout.
- `crates/horae/src/route.rs`, `pages/new_project.rs`, `pages/new_project/draft.rs`,
  `pages/invoices.rs`, `pages/invoices/preparation.rs`: contextual entry only.
- Existing components and `crates/horae/assets/css/horae.css`: reuse tokens and
  utilities; add scoped styles only where necessary.
- `nix/checks/` browser fixtures/checks and `.sqlx/`: regression coverage/cache.

## Execution phases

1. Reconcile/clarify increment and record reference evidence/readiness.
1. Tests first for access/validation; initialize disposable PostgreSQL only.
1. US1 list/filter/states, preserving import/lifecycle destinations.
1. US2 useful detail with authorized work and persisted billing fields.
1. US3 shared form, trusted validation and explicit rate intent.
1. US4 context parsing/prefill, draft/recovery precedence, navigation guards.
1. Browser matrix/regressions, adversarial review/corrections, Nix/cache checks,
   unsigned scoped PR(s), green CI. No merges.

The agent-context update script is absent from this checkout; no replacement or
unrelated AGENTS.md change. Spec Kit helpers report `mvp` as logical branch from
the nested feature pointer; actual Git branch remains `feat/clients-mvp`.

## Complexity Tracking

No constitutional exception, new architecture, generic repository layer or
dependency. Keep pickers compatible and sensitive rates/invoices separately gated.
