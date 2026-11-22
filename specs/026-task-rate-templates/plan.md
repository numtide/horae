# Implementation Plan: Task rate templates

**Branch**: `spec/task-rate-templates` (documents); implementation on its own branch | **Date**:
2026-11-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/026-task-rate-templates/spec.md`

**Order**: after PR 1 (`fix/unknown-currency`, migration 0009), the Harvest invoice import (025) and
the next deploy; close to the Tasks screen redesign, which touches `pages/tasks.rs` too.

## Summary

A task's default rate becomes a currency-free template in hundredths. Because Horae stores every
currency in hundredths (research R0), applying a template to a project copies the number into the
project's task rate in the project's currency: no rescaling, no currency check. The refusals that
spec 011 added go. Amounts computed from a rate round once, half up, to the currency's precision
(whole units for zero-decimal currencies), and totals stay the sum of the rounded lines. One
migration drops `tasks.default_rate_currency`, keeps every number, and replaces the SQL
`line_amount_cents`. Interfaces keep their shapes.

The plan follows the recommended answers to the spec's three open questions (research R1 to R3).

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the flake)

**Primary Dependencies**: Dioxus 0.7 fullstack, Axum, sqlx (compile-time checked queries), proptest
and Kani in `horae-core`

**Storage**: PostgreSQL; one migration at the next free number (0010 when written)

**Testing**: `cargo test -p horae-core` (unit, proptest), `cargo kani -p horae-core`, `#[sqlx::test]`
in `horae-server`, `horae-harvest-api` and `horae`, the browser scripts under
`crates/horae/tests/browser/`, `nix flake check`

**Target Platform**: Linux server plus WASM client

**Project Type**: web application (virtual workspace, crates `core`, `types`, `server`, `api`,
`harvest-api`, `horae`, `i18n`)

**Performance Goals**: none new; the SQL line amount stays an `IMMUTABLE` function over row values

**Constraints**: integer arithmetic only; `.sqlx/` regenerated; i18n catalog for every string;
`dioxus-lint` ratchet

**Scale/Scope**: about 60 references to `TaskRateEdit` and the currency fields, about 25 test files,
3 browser scripts (PR 1's measured scope)

## Constitution Check

*Before Phase 0 and again after Phase 1.*

| Principle | Check | Result |
| --- | --- | --- |
| I. Exactness | Templates and amounts are integers; one half-up rounding from the exact value; totals are sums of stored lines. A currency-free integer needs the amendment note in `constitution-amendment.md` | Pass, with the amendment applied first (T001) |
| II. Domain purity | `RateTemplate`, `currency_decimals` and the line amount live in `horae-core`, with unit, property and Kani tests; the SQL twin is tested against them | Pass |
| III. Single datastore | PostgreSQL migration; no new tables | Pass |
| IV. Shared operations | Rate writes stay in `ops/projects/task_writes.rs`; applications in the creation and editor operations; importers keep their paths | Pass |
| V. Builds and formatting | `nix fmt`, `nix flake check`, `.sqlx` check | Pass |

After design: unchanged. No Complexity Tracking entry.

## Project Structure

### Documentation (this feature)

```text
specs/026-task-rate-templates/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── constitution-amendment.md
├── contracts/interfaces.md
├── checklists/requirements.md
└── tasks.md
```

### Source code touched

```text
.specify/memory/constitution.md                 # the amendment, after approval
crates/core/src/money.rs                        # RateTemplate, pub currency_decimals
crates/core/src/invoice.rs                      # line_amount_cents(rate, minutes, currency)
crates/core/src/proofs.rs, crates/core/tests/properties.rs
crates/server/migrations/0010_task_rate_templates.sql
crates/server/src/db.rs                         # migration test
crates/server/src/plugin/database.rs            # function allowlist
crates/server/src/ops/projects/task_writes.rs   # authorize_rate
crates/server/src/ops/projects/task_catalog.rs
crates/server/src/ops/projects/creation/{finalize,options}.rs
crates/server/src/ops/projects/editor/{associations,catalog,projection}.rs, editor.rs
crates/server/src/ops/{reports,invoices/entries}.rs, ops/projects/{reads,budgets}.rs
crates/server/src/importers/harvest/{resolve,parents}.rs, resolve/preview.rs
crates/server/src/seed.rs
crates/types/src/{task,project_creation}.rs
crates/harvest-api/src/{lib,task_reads}.rs
crates/horae/src/pages/tasks.rs, pages/tasks/editor.rs, pages/new_project/{tasks,draft}.rs
crates/horae/src/server_fns/…                   # tests listed in tasks.md
crates/horae/tests/{new_project_screen,task_catalog_ui}.rs, tests/browser/*.cjs
crates/i18n/locales/en/horae.ftl
specs/011-new-project-screen/{research,data-model,quickstart,tasks}.md   # superseded notes
```

## Phases

- **Phase 0** (research.md): storage finding (R0), rounding rule (R1), open questions with
  recommendations (R2, R3), naming (R4), core type (R5), application points (R6), wording (R7),
  interfaces (R8), migration (R9), spec 011 (R10).
- **Phase 1**: data-model.md, contracts/interfaces.md, quickstart.md, constitution-amendment.md.
- **Phase 2**: tasks.md.

## Delivery

One PR, because dropping the column breaks every query that names it, and `.sqlx/` must match in one
commit. Inside it, commits follow the task phases: amendment, core, migration with the SQL function,
server, types and UI, importers and APIs, tests, spec 011 notes. If the maintainer wants the rounding
change (User Story 3) on its own, it can ship first as a smaller PR with its own migration for the
SQL functions (T005, T008 to T011, T027 to T031); it does not depend on templates.

## Complexity Tracking

None.
