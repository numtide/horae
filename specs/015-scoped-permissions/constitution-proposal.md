# Preserved authorization constitution proposal

Status: proposal only; not ratified or applied. The authoritative constitution
remains [.specify/memory/constitution.md](../../.specify/memory/constitution.md)
version 1.0.0. This archive neither activates permission policy nor approves a
migration. Adoption requires its own explicit governance review.

The following is the complete original text from PR #212, commit
`db3935db364f2a8aa193f0e938ce40ecc01a2f92`, path
`.specify/memory/constitution.md` (blob
`827f6ac88fa4c774721b2fef44843b80581d9e1e`). Its dates, impact report and version
line describe the original proposal, not a new ratification. Preserving the text
here makes it reviewable without checking out the retained draft branch.

```markdown
<!--
Sync Impact Report
- Version change: 1.0.0 → 1.1.0
- Basis: user-confirmed six-profile/custom-permission scope in feature 015,
  Clarifications, 2026-09-30. MINOR expands authorization guidance without removing
  or redefining any of the five core principles.
- Modified principles: none; Principle IV's role checks follow the effective
  capability/scope constraints below rather than an ordered role-name comparison.
- Added sections: none; expanded Technology & Data Constraints and quality gates
- Removed sections: none
- Consistency propagation:
  - .specify/templates/plan-template.md — checked; dynamic Constitution Check remains applicable
  - .specify/templates/spec-template.md — checked; existing requirements/acceptance sections suffice
  - .specify/templates/tasks-template.md — checked; security/test tasks support these gates;
    generic optional-test examples do not override constitutional correctness tests
  - .specify/templates/commands/ — absent; no command templates to update
  - AGENTS.md / README.md — checked; retain accurate current-runtime descriptions,
    not a claim that six profiles are already deployed
  - specs/015-scoped-permissions/spec.md / plan.md / research.md — updated
- Follow-up: reconcile pending dashboard/client role-based acceptance through
  feature 015's verified matrix; finish policy persistence, migration review and
  cross-surface verification. T008 remains open for those dependencies.
- No unexplained placeholders; this amendment does not activate runtime policy.
-->

# Horae Constitution

## Core Principles

### I. Exactness (NON-NEGOTIABLE)

Time is stored and computed as **integer minutes**; money as **integer minor units (cents) with an
explicit ISO 4217 currency code**. Floating-point MUST NOT be used for any duration or monetary value.
Every reported total MUST equal the exact sum of its parts across any grouping or period.

**Rationale**: Billing and invoicing are financial operations; floating-point accumulation produces
rounding drift that corrupts totals and erodes trust. Integer representation makes correctness verifiable.

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
- Authentication is credential/identity-provider based; a local development bypass is permitted
  but MUST be off by default. Permission changes MUST NOT rebind identity or turn imports into
  admission or privilege grants.
- The target authorization model MUST provide Member, Project Manager, People Admin, Accounting,
  Executive Manager and Administrator profiles, reusable custom profiles and per-person adjustments.
  Access MUST depend on effective capabilities and applicable own/person/project/organization scope,
  not profile-name ordering. Membership, project management and person management MUST be distinct.
- Current authorization MUST be enforced across mutations, reads, aggregates, exports and delegated
  work. Sensitive fields MUST be withheld from unauthorized responses, not merely hidden in the UI.
  Organization isolation, inactive-account denial and independent business-state locks MUST remain
  mandatory. Service jobs/plugins MUST use explicitly bounded service authority, not assumed user grants.
- Profile/privilege changes MUST require administrator authority and protect the last active
  administrator under concurrent changes. Assignment authority MUST follow the verified operation
  contract and MUST NOT become an indirect privilege-escalation path.
- Plugins (when present) run sandboxed and MUST NOT bypass these principles — in particular, they have
  no direct datastore write access (they use granted host capabilities only).

## Development Workflow & Quality Gates

- Feature work follows the spec-driven flow: `spec.md` → `plan.md` → `tasks.md`, with artifacts under
  `specs/<NNN-feature>/`.
- Correctness-critical changes MUST include tests in `horae-core` and/or `#[sqlx::test]` integration
  tests; the NixOS e2e check exercises the deployed surface.
- Authorization changes MUST include allowed/denied, record-scope, sensitive-payload and revocation
  verification across affected delivery paths. Before replacing legacy Admin/Manager/Member checks,
  the operation matrix, existing-data migration differences and concurrent activation/recovery
  contract MUST be reviewed. Preserve business records; no silent role remapping or database reset.
  The legacy roles describe the pre-cutover implementation, not an accepted substitute for the target.
- A change MUST NOT merge with a red `nix flake check` or unformatted files.
- Any deviation from a principle MUST be justified in the plan's Complexity Tracking (or rejected).

## Governance

This constitution supersedes ad-hoc conventions. Amendments MUST be made by editing this file with a
Sync Impact Report and a semantic version bump: MAJOR for principle removals/redefinitions, MINOR for
added principles or materially expanded guidance, PATCH for clarifications. Every plan's Constitution
Check gate MUST verify compliance before Phase 0, and again after design. Unjustified violations block
merge. Runtime working guidance for agents lives in `AGENTS.md`.

**Version**: 1.1.0 | **Ratified**: 2026-07-10 | **Last Amended**: 2026-10-01
```
