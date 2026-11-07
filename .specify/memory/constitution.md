<!--
Sync Impact Report
- Version change: 3.0.0 → 4.0.0
- Bump rationale: MAJOR. Principle IV is redefined a second time: the sign-in callback becomes a
  second bounded mutation path beside `horae user link-oidc`. The governance rule makes a principle
  redefinition MAJOR.
- Modified principles:
  - IV. Mutations Through Server Functions: title unchanged. "The single narrow exception" becomes
    "the two narrow exceptions"; the `link-oidc` exception is unchanged in substance. Adds "OIDC
    sign-in provisioning": after full token validation and before any session, `/auth/callback`
    may (a) bind a verified email's active, unbound account to the token's issuer and subject, and
    (b) when the operator lists the email's domain, create one active member account with no rates
    bound to that issuer and subject. An email the provider marks unverified is never provisioned;
    an absent claim is accepted only where the operator explicitly opts in. No other role, no
    rates, no reactivation, no replaced binding, not reachable from the UI or plugins.
- Added sections: none
- Removed sections: none
- Templates checked:
  - .specify/templates/plan-template.md: Constitution Check takes its gates from this file ✅ (no change)
  - .specify/templates/spec-template.md: no mandatory-section change ✅
  - .specify/templates/tasks-template.md: no new principle-driven task type ✅
  - .specify/templates/constitution-template.md: placeholder template, not a project copy ✅ (no change)
  - .specify/templates/commands/: not present; the speckit-* skills carry no Principle IV text ✅
  - AGENTS.md: states no Principle IV rule; its Auth paragraph is updated with the feature's
    implementation, not with this amendment ✅ (no change here)
  - README.md: links the constitution only ✅ (no change)
  - specs/001-time-tracking-invoicing/oidc-identity-provisioning/plan.md: its Complexity Tracking
    deviation (the callback binding write) is now covered by the second exception. The plan is a
    historical record checked against 3.0.0 and is left as written; the new
    oidc-auto-provisioning plan records the fold-in.
- Follow-up: the existing local CLI commands that write to the datastore (`init`, `user create`,
  `seed`, `migrate`) remain neither covered nor assessed, as recorded in 3.0.0.
-->
<!--
Sync Impact Report (3.0.0)
- Version change: 2.0.0 → 3.0.0
- Bump rationale: MAJOR. Principle IV's "All data mutations MUST go through Dioxus `#[server]`
  functions" is redefined: one narrowly bounded local-operator mutation is now permitted. As in
  2.0.0, the governance rule makes a principle redefinition MAJOR, even though the change is small.
- Modified principles:
  - IV. Mutations Through Server Functions: title unchanged; adds the local operator identity
    binding exception for `horae user link-oidc` only. It may bind an issuer and subject to an
    existing, active, unbound account, or add the issuer to that account's own issuer-less binding
    with the identical subject; it may not replace bindings, create accounts, sessions or provider
    users, change roles, or be reachable over HTTP, the UI or plugins.
- Added sections: none
- Removed sections: none
- Templates checked:
  - .specify/templates/plan-template.md: Constitution Check takes its gates from this file ✅ (no change)
  - .specify/templates/spec-template.md: no mandatory-section change ✅
  - .specify/templates/tasks-template.md: no new principle-driven task type ✅
  - .specify/templates/constitution-template.md: placeholder template, not a project copy ✅ (no change)
  - .specify/templates/commands/: not present; the speckit-* skills carry no Principle IV text ✅
  - AGENTS.md: describes the two API surfaces and lists CLI subcommands; it states no Principle IV
    rule, so it needs no exception note ✅ (no change). The new subcommand is added to its CLI list
    with the feature's implementation, not with this amendment.
  - README.md: links the constitution only ✅ (no change)
  - Earlier feature plans that cite 2.0.0 are historical records and are deliberately left as
    written; later plans check against 3.0.0.
- Follow-up: the exception covers only `horae user link-oidc`. The existing local CLI commands
  that already write to the datastore (`init`, `user create`, `seed`, `migrate`) are neither
  covered nor assessed by this amendment. The existing `/auth/callback` binding write (automatic
  binding by verified email, before any session exists) is also outside this exception; it is
  justified as an existing deviation in the Complexity Tracking of
  specs/001-time-tracking-invoicing/oidc-identity-provisioning/plan.md.
-->
<!--
Sync Impact Report (2.0.0)
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

All data mutations MUST go through Dioxus `#[server]` functions (session-authenticated, role-checked),
except as the two narrow exceptions below. The UI MUST NOT issue ad-hoc client-side fetches for
data reads or writes. Additional non-mutating surfaces (e.g. a read-only compatibility API, exports)
are permitted but MUST NOT become a second mutation path.

**Local operator identity binding (first exception)**: the local `horae user link-oidc` command
MAY bind an OIDC issuer and subject to one existing user account by writing to the datastore
directly, with the operator's database credentials as its authority.

- It MUST only record an issuer and subject on an existing, active account that has no binding,
  or record the issuer on that account's own issuer-less binding when the subject is identical.
  Repeating the exact existing binding is a no-op. It MUST reject a binding held by another
  account and MUST NOT replace a different subject or issuer, or remove a binding.
- It MUST NOT create accounts, sessions or identity-provider users, change roles or any other user
  field, or be reachable over HTTP, from the UI or from plugins.
- The exception does not extend to any other command, field or mutation, and it is not a precedent
  for one.

**OIDC sign-in provisioning (second exception)**: the `/auth/callback` handler MAY write to the
datastore directly, after the provider's response has been fully validated (state, PKCE, ID token
signature, issuer, audience, nonce and expiry) and before any session exists, in exactly two ways:

- Bind: record the token's issuer and subject on an existing, active account that has no binding,
  only when the token asserts that the account's email is verified.
- Provision: when the operator has listed the email's domain for automatic provisioning, create
  one active account with the member role, no rates, and the token's issuer and subject as its
  binding. The provider MUST assert the email verified, or, only where the operator has
  explicitly opted in, assert nothing about it; an email the provider marks unverified MUST NOT be
  provisioned. No account may already hold the email, in any state.
- It MUST NOT create any other role, set rates, reactivate an account, replace or remove a
  binding, change any other user field, or create identity-provider users. It MUST NOT be
  reachable from the UI, from plugins or from any other route or command.
- The exception does not extend to any other handler, field or mutation, and it is not a precedent
  for one.

**Rationale**: A single, typed, authorized mutation path keeps authorization and validation in one
place and keeps client and server types in sync. Some providers never assert a verified email, so
the first administrator cannot sign in to bind their own identity; nobody holds a session yet. The
operator who can already run `horae init` against the database is the only authority available
when the provider never asserts a verified email, so this one binding step is allowed there and
nowhere else. The sign-in callback likewise runs before any session exists, so no server function
can bind or create the account that the session will belong to; its authority is the validated
token together with the operator's own configuration, and it is limited to the least-privileged
account Horae has.

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

**Version**: 4.0.0 | **Ratified**: 2026-07-10 | **Last Amended**: 2026-11-06
