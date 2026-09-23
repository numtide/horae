# Implementation Plan: New Project

**Branch**: `feat/new-project-screen` | **Date**: 2026-09-21 | **Spec**: [spec.md](spec.md)

**Input**: `specs/011-new-project-screen/spec.md`

## Summary

Implement `/projects/new` in the existing application shell using shared controls and utility CSS. Add private versioned drafts, atomic final creation, real billing/budget settings, task/team permissions and invoice defaults. Preserve existing/imported projects with explicit legacy semantics.

The approved US7 extension adds `/projects/:id/edit` with the same form sections and a distinct explicit-save lifecycle. Load operational settings with current authorization, update the existing graph transactionally and remove the inline Projects editor after its behavior and regression checks are migrated. Keep this in PR #207 and the existing worktree.

Implementation is incremental; the basic form is the first testable slice, not completion of the full goal.

## Technical Context

**Language/Version**: Rust edition 2024, toolchain pinned by the Nix flake.

**Primary Dependencies**: Existing Dioxus 0.7 fullstack, Axum, Tokio, sqlx, serde, chrono, UUID and thiserror. No new crate/framework planned. Optional email delegates to an administrator-configured sendmail-compatible executable, without a shell or a custom SMTP implementation.

**Storage**: PostgreSQL; migrations start at `0030`. New tables use UUID v7 and organization foreign keys. Versioned JSON is appropriate for incomplete drafts; operational data uses typed columns and relations.

**Testing**: Core unit tests, serial sqlx integration tests, existing isolated Playwright runner, server Clippy, WASM check, SQLx cache, formatting and flake CI.

**Target Platform**: Linux server/browser WASM, desktop and mobile.

**Project Type**: Existing fullstack workspace, `crates/core` and `crates/horae`; no new crate.

**Performance Goals**: Autosave after 600 ms idle debounce, one outstanding save per draft with coalesced changes. Catalog lookup pages at most 1,000 rows; server search/pagination beyond that. Draft limits: 256 KiB, 500 selected tasks/people each, 50 tags, 100 milestones.

**Constraints**: Never use imported production data for tests. Integer minutes/cents. New currency choices EUR/CHF/USD/GBP follow the handoff and current two-decimal monetary support. Shared CSS/control defaults stay unchanged; opt-in extensions only. No automatic invoice issuing or external mail during tests.

**Scale/Scope**: Shared creation/editing surface plus necessary downstream consumers. General Project Detail redesign, automatic existing-project conversion and organization/auth redesign are excluded.

## Constitution Check

| Principle | Pre-research | Post-design approach |
|---|---|---|
| Exactness | Pass | Checked core money/time helpers, basis-point adjustments, Rust/SQL parity tests. |
| Domain purity | Pass | Pure validation, rate choice, fee dates and invoice arithmetic in core; no new I/O dependency there. |
| Single datastore | Pass | PostgreSQL drafts/settings/outbox; UUID v7 and org foreign keys on every new table. |
| Server mutations | Pass | Session-authorized Dioxus functions and server-internal workers/importer paths; no ad-hoc browser writes. |
| Reproducibility | Pass | Nix shell, regenerated SQLx cache/utilities, unit/integration/browser/flake gates. |

No constitutional exception is requested.

## Project Structure

### Documentation

```text
specs/011-new-project-screen/
  spec.md
  plan.md
  research.md
  data-model.md
  contracts/new-project.md
  quickstart.md
  checklists/requirements.md
  tasks.md
```

### Source Code

```text
crates/core/src/project.rs                     creation validation/types
crates/core/src/invoice.rs                     rate and adjustment arithmetic
crates/core/src/budget.rs                      period/scope helpers
crates/horae/migrations/0030_*.sql              drafts/configuration
crates/horae/migrations/0031_*.sql              billing/invoice sources
crates/horae/src/models/project_creation.rs    typed request/result projections
crates/horae/src/server_fns/project_creation.rs
crates/horae/src/server_fns/project_creation/tests.rs
crates/horae/src/server_fns/projects.rs         catalog, tags, authorized projections
crates/horae/src/server_fns/time_entries.rs    shared task restriction guards
crates/horae/src/server_fns/reports.rs          rates, currencies, progress privacy
crates/horae/src/server_fns/invoices.rs         fee/default preparation and snapshots
crates/horae/src/notifications.rs               bounded optional mail delivery
crates/horae/src/jobs.rs                        scoped outbox claiming
crates/horae/src/pages/new_project.rs           composition and draft lifecycle
crates/horae/src/pages/new_project/             form sections if needed
crates/horae/src/pages/projects.rs              creation/edit entry links; remove old inline editor
crates/horae/src/pages/invoices.rs              defaults and fee preparation
crates/horae/src/route.rs / src/pages.rs        registration
crates/horae/src/components/                   opt-in shared capabilities
crates/horae/assets/css/horae.css               structural np-* rules/tokens
crates/horae/build.rs                          missing utilities only
crates/horae/tests/browser/new-project.cjs
```

**Structure Decision**: Existing layers and sibling-file module roots; no repository abstraction, form engine, new queue framework or alternate mutation API.

## Execution Design

1. Establish exact domain tests/contracts, then legacy-safe draft/configuration schema with protected private tables.
1. Implement draft read/save/discard/finalization with revision checks and transactional revalidation. Draft ID provides retry identity; name is not an idempotency key.
1. Add static `/projects/new` before dynamic project detail, keep Projects active in navigation, and compose labeled form rows and existing modal/shell. Do not add a separate sidebar item absent from the handoff.
1. Enforce task access in the existing new-entry eligibility view and historical mutation guards. Preserve archived-history edits and safe stopping of an own running timer after revocation.
1. Integrate rate modes, scoped budgets, fixed-fee occurrences and invoice-owned defaults across all consumers. Missing configuration means legacy behavior, including legacy fixed-fee hourly invoicing.
1. Record budget alerts in the existing outbox with unique logical identity; filter claims by event kind. Optional direct sendmail invocation has bounded timeout, sanitized headers and stable Message-ID. Document ambiguous acknowledgement/at-least-once delivery.
1. Finish responsive/accessibility states and adversarial/regression review. Open one scoped reviewed PR, without automatically merging.

## Unified editing execution (US7)

1. Add a failing real-browser Edit → shared prefilled editor → Cancel/Save/reload regression before redirecting existing actions.
1. Add an authorized operational-to-form projection for configured and legacy projects. Recover selected identities independently of catalog pagination; distinguish inherited, absent and explicit-zero values. Preserve association/milestone identity and private-field boundaries.
1. Add an atomic edit mutation with current actor/reference checks, concurrent-edit detection and retry identity. Share parsing/validation and graph helpers where their creation assumptions also hold for existing records; do not implement editing by deleting/recreating the project or finalizing another draft.
1. Reuse the existing sections/layout with mode-specific headings and footer actions. Creation keeps its durable draft lifecycle; editing uses explicit Save changes and unsaved-navigation protection. Guard historical financial changes with visible explanations, not silent conversion.
1. Redirect every Edit entry point and remove the old form, state, obsolete endpoint/DTO paths once no callers remain. Migrate old edit regressions to the new route without reducing their assertions.
1. Verify configured/legacy/no-op round trips, real persisted edits, cancel/conflict/uncertain retries, role changes, historical/invoiced sources, creation and whole-browser/style regressions. Regenerate SQLx and rerun build/lint/format gates before updating PR #207.

## Partial fixed-fee billing execution (US5, 2026-09-23)

The application is not in production. Use one clean balance model, not a parallel compatibility implementation. Preserve the user's imported development data; use a forward migration because development databases have already applied the existing migrations. All mutation tests use disposable databases.

1. Add pure, checked discount allocation in `crates/core/src/invoice.rs`. Allocate the invoice's rounded discount across all time and fee lines proportionally. Floor each share, then award remaining cents in descending fractional-remainder order, breaking ties by stable source order. Callers sort by time-entry UUID or `(project_id, period_key)` before allocation. Return results in input order; zero-subtotal invoices allocate zero. Negative lines and overflowing sums fail.
1. Replace the exclusive occurrence claim with existing invoice lines as the ledger. Persist each line's post-discount, pre-tax contribution; sum contributions of non-void invoices for the occurrence balance. Drafts reserve the balance immediately. Sending does not consume it again; voiding releases only that invoice by status. Keep original gross lines and headers unchanged when migrating existing fixtures.
1. Give preparation stable source identities without materializing occurrences. Show agreed, already invoiced (including draft reservations), remaining and proposed net amounts. Editable fee amounts/descriptions belong to the invoice, never its schedule. Preserve time-backed eligibility. Monthly balances remain per calendar-month occurrence, not a project-wide amount transferable between months.
1. Reuse the organization invoice advisory lock before invoice/source row locks on generation, draft editing and status changes. Validate the current balance and exact confirmed excess under that lock. Draft editing excludes its old contributions before validating replacements. Changed selection, amounts, discount or balance invalidates the prior review/confirmation. Tax changes do not consume additional fees.
1. Use a caller-stable UUID v7 request identity and persisted canonical payload for generation and draft edits. Identical retries acknowledge the same mutation; changed payloads under the same identity conflict. Draft revisions reject stale edits, including replay after a newer edit. Do not emit another creation event for a replay.
1. Reuse invoice form controls and existing tokens/utilities for fee rows and explicit overbilling confirmation. Never rely on color alone for negative balances. Expose the same authorized balance calculation in project context; do not modify global CSS defaults or introduce a new design system.
1. Prove partial billing, discount remainders, mixed time/fee allocations, draft replacement, void, stale confirmation, concurrent writers and uncertain retry behavior. Run browser, export, import, project-edit and authorization regressions before marking T043 or the PR complete.

**Constitution recheck**: Pass. Integer arithmetic remains in the pure core crate; PostgreSQL persists contributions and retry identities; authenticated server functions own writes; existing Nix/test/format gates remain required. No dependency or new crate is needed.

## Agent Context and Hooks

Checked-in Spec Kit provides setup/prerequisite scripts but no `update-agent-context.sh`; record context here rather than claim an absent script ran. No `.specify/extensions.yml` exists, so no hooks apply.

## Complexity Tracking

No constitution violations. Backend changes are driven by displayed controls; compatibility risks and alternatives are in [research.md](research.md).
