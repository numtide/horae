<!--
Sync Impact Report
- Version change: 1.0.0 → 2.0.0
- Bump rationale: MAJOR. Principle I's blanket "Floating-point MUST NOT be used for any
  duration" is redefined: one narrowly bounded floating-point conversion is now permitted.
  The governance rule makes a principle redefinition MAJOR, even though the change is small.
- Modified principles:
  - I. Exactness (NON-NEGOTIABLE): title unchanged; adds the XLSX (Excel) duration cell
    exception. Integer storage, calculation, rounding and totals stay mandatory, and money
    is excluded.
- Added sections: none
- Removed sections: none
- Templates checked:
  - .specify/templates/plan-template.md: Constitution Check takes its gates from this file ✅ (no change)
  - .specify/templates/spec-template.md: no mandatory-section change ✅
  - .specify/templates/tasks-template.md: no new principle-driven task type ✅
  - .specify/templates/commands/: not present; the speckit-* skills carry no Principle I text ✅
  - AGENTS.md: domain invariants note the exception ✅
  - README.md: describes integer storage only, which is unchanged ✅ (no change)
  - Earlier feature plans that cite 1.0.0 are historical records and are deliberately left as
    written; later plans check against 2.0.0.
- Follow-up: the exception covers only XLSX duration cells. Other existing
  floating-point conversions (money cells in XLSX exports, hours and amounts in the
  Harvest-compatible API, string formatting helpers) are neither covered nor assessed by
  this amendment.
-->

# Horae Constitution

## Core Principles

### I. Exactness (NON-NEGOTIABLE)

Time is stored and computed as **integer minutes**; money as **integer minor units (cents) with an
explicit ISO 4217 currency code**. Floating-point MUST NOT be used for any duration or monetary value,
except as the single narrow exception below. Every reported total MUST equal the exact sum of its
parts across any grouping or period.

**XLSX duration cells (the only exception)**: when an XLSX (Excel) export writes a duration as a
numeric hours cell and the writing library accepts only floating-point numbers, the export MAY
convert one integer-minute value to hours (minutes ÷ 60) as the final step that writes the cell.

- The minutes MUST already be final: stored, rounded and totalled as integers beforehand.
- Within Horae's code paths, the converted value MUST NOT be summed, rounded, compared,
  persisted, parsed back or used in any further calculation. Writing it into the cell is the
  permitted use. A regression test that inspects the serialized cell text, and a spreadsheet
  user's own formulas, are not reuse.
- The cell is an approximate presentation. The integer minutes remain the authority. No exact
  fractional-hours value or round trip back to minutes may be claimed unless a test proves it.
- The exception does not extend to money, text or CSV output, APIs, domain logic or storage.

**Rationale**: Billing and invoicing are financial operations; floating-point accumulation produces
rounding drift that corrupts totals and erodes trust. Integer representation makes correctness verifiable.
The XLSX library's numeric-cell API takes an `f64`, so the value is converted at that call; most
n/60 values have no exact finite representation. Users need hours they can sum in a spreadsheet.
Confining the conversion to that last call keeps every Horae calculation in integers.

### II. Domain Purity

Correctness-critical logic — duration parsing, rounding, money, totals, and entry/invoice state
transitions — MUST live in the `horae-core` crate. `horae-core` MUST NOT depend on `sqlx`, `axum`,
`dioxus`, or any I/O framework, and MUST be unit-tested in isolation.

**Rationale**: Isolating the rules that must be exact keeps them fast to test and impossible to break
by unrelated I/O or UI changes.

### III. Single Datastore

PostgreSQL is the only supported datastore. Primary keys MUST be UUID v7 (time-ordered). Every table
MUST carry an `org_id` foreign key even while the product is single-organization, so multi-organization
support is a later flip rather than a migration rewrite.

**Rationale**: One well-understood datastore avoids dialect-portability tax; v7 keys give ordered,
index-friendly identifiers; retaining `org_id` preserves an obvious future path.

### IV. Mutations Through Server Functions

All data mutations MUST go through Dioxus `#[server]` functions (session-authenticated, role-checked).
The UI MUST NOT issue ad-hoc client-side fetches for data reads or writes. Additional non-mutating
surfaces (e.g. a read-only compatibility API, exports) are permitted but MUST NOT become a second
mutation path.

**Rationale**: A single, typed, authorized mutation path keeps authorization and validation in one
place and keeps client and server types in sync.

### V. Reproducible Builds & Formatting Gate

The project MUST build and run through the Nix dev shell and flake. `nix fmt` (treefmt) and
`nix flake check` MUST be green before merge; formatting is enforced in CI via `nix fmt -- --ci`.
Toolchain versions are pinned via the flake, not assumed from the host.

**Rationale**: Reproducible environments eliminate "works on my machine" drift and make CI results
trustworthy; a formatting gate keeps diffs about substance.

## Technology & Data Constraints

- Language: Rust (edition 2024); the web UI compiles to WASM via Dioxus fullstack.
- Persistence: PostgreSQL 15+ via `sqlx`; schema changes ship as ordered migrations under `migrations/`.
- Authentication is credential/identity-provider based with role-based authorization
  (administrator / manager / member); a local development bypass is permitted but MUST be off by default.
- Plugins (when present) run sandboxed and MUST NOT bypass these principles — in particular, they have
  no direct datastore write access (they use granted host capabilities only).

## Development Workflow & Quality Gates

- Feature work follows the spec-driven flow: `spec.md` → `plan.md` → `tasks.md`, with artifacts under
  `specs/<NNN-feature>/`.
- Correctness-critical changes MUST include tests in `horae-core` and/or `#[sqlx::test]` integration
  tests; the NixOS e2e check exercises the deployed surface.
- A change MUST NOT merge with a red `nix flake check` or unformatted files.
- Any deviation from a principle MUST be justified in the plan's Complexity Tracking (or rejected).

## Governance

This constitution supersedes ad-hoc conventions. Amendments MUST be made by editing this file with a
Sync Impact Report and a semantic version bump: MAJOR for principle removals/redefinitions, MINOR for
added principles or materially expanded guidance, PATCH for clarifications. Every plan's Constitution
Check gate MUST verify compliance before Phase 0, and again after design. Unjustified violations block
merge. Runtime working guidance for agents lives in `AGENTS.md`.

**Version**: 2.0.0 | **Ratified**: 2026-07-10 | **Last Amended**: 2026-10-28
