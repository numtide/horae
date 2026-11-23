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
(whole units for zero-decimal currencies), and totals stay the sum of the rounded lines. Interfaces
keep their shapes.

The work ships in two PRs. The rounding PR comes first: it replaces the SQL `line_amount_cents` and
rounds every per-entry amount to the currency's precision. The templates PR follows: it applies the
constitution amendment, drops `tasks.default_rate_currency`, keeps every number, and makes the screens
and importers treat the rate as a template.

The plan follows the maintainer's answers of 2026-11-23 (spec.md, Clarifications; research R1 to R3).

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the flake)

**Primary Dependencies**: Dioxus 0.7 fullstack, Axum, sqlx (compile-time checked queries), proptest
and Kani in `horae-core`

**Storage**: PostgreSQL; one migration per PR, each at the next free number when written (0010 and
0011 if nothing lands between them)

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
| I. Exactness | Templates and amounts are integers; one half-up rounding from the exact value; totals are sums of stored lines. A currency-free integer needs the approved amendment (7.2.0) in `constitution-amendment.md`; the rounding PR only rounds money and does not | Pass: the templates PR applies the amendment as its first task (T019). Open: review finding H3 proposes two added clauses for the maintainer (HANDOFF.md) |
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

Rounding PR:

```text
crates/core/src/money.rs                        # pub currency_decimals
crates/core/src/invoice.rs                      # line_amount_cents(rate, minutes, currency)
crates/core/src/proofs.rs, crates/core/tests/properties.rs
crates/server/migrations/NNNN_currency_line_amounts.sql
crates/server/src/db.rs                         # SQL parity test
crates/server/src/plugin/database.rs            # function allowlist
crates/server/src/ops/{reports,invoices/entries}.rs, ops/projects/{reads,budgets}.rs
```

Templates PR:

```text
.specify/memory/constitution.md                 # the approved amendment, 7.2.0
crates/core/src/money.rs                        # RateTemplate
crates/server/migrations/NNNN_task_rate_templates.sql
crates/server/src/db.rs                         # migration test
crates/server/src/plugin/event.rs               # TaskCreated doc comment
crates/server/src/ops/projects/task_writes.rs   # authorize_rate
crates/server/src/ops/projects/task_catalog.rs
crates/server/src/ops/projects/creation/{finalize,options}.rs
crates/server/src/ops/projects/editor/{associations,catalog,projection}.rs, editor.rs
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

- **Phase 0** (research.md): storage finding (R0), rounding rule (R1), the maintainer's answers on
  zero and foreign-currency rates (R2, R3), naming (R4), core type (R5), application points (R6), wording (R7),
  interfaces (R8), migration (R9), spec 011 (R10).
- **Phase 1**: data-model.md, contracts/interfaces.md, quickstart.md, constitution-amendment.md.
- **Phase 2**: tasks.md.

## Delivery

Two PRs, in this order, as the maintainer chose.

1. **Rounding PR** (User Story 3; tasks.md Delivery 1, T001 to T018). Public `currency_decimals`,
   `line_amount_cents` with a currency in Rust and SQL, its own migration, the plugin allowlist, and
   the four places that compute per-entry amounts. It does not depend on templates or on the
   amendment, and it can start once PR 1 has merged. It changes uninvoiced amounts in zero-decimal
   currencies only; stored invoices and 2-decimal figures stay the same.
1. **Templates PR** (User Stories 1, 2, 4 and 5; tasks.md Delivery 2, T019 onwards). Starts after
   the rounding PR and the OAuth amendment (7.1.0) have merged. It stays one PR, because dropping
   the column breaks every query that names it, and `.sqlx/` must match in one commit. Its commits
   follow the task phases: amendment, core type, migration, types, server, UI, importers and APIs,
   spec 011 notes.

## Complexity Tracking

None.
