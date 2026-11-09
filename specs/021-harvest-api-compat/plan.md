# Implementation Plan: Harvest API v2 compatibility

**Branch**: `spec/harvest-api-compat` (specification); each delivery step lands as its own branch
and pull request | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/021-harvest-api-compat/spec.md`, with the
maintainer's decisions of 2026-11-09 (spec *Clarifications*).

## Summary

Turn the read-only `/harvest/v2` API into one that Harvest API v2 clients can use once pointed at
Horae, in seven steps ordered by how many surveyed clients each unlocks: integer ids and an
account id, personal access tokens, time-entry writes with `company` and project assignments,
list fidelity, catalogue and report reads, catalogue writes, and an OAuth2 provider last. Writes
reuse the shared mutation operations the web app's server functions call (research R-5), which
needs the proposed constitution amendment. Hours and money are converted at the edge with integer
arithmetic only (R-4). A Nix check runs real open-source clients against a Horae server (R-10).

## Technical Context

**Language/Version**: Rust (workspace toolchain from the Nix dev shell), Dioxus 0.7.9 fullstack

**Primary Dependencies**: axum 0.8, sqlx 0.8 (compile-time macros), serde_json with the
`raw_value` feature (new feature, no new crate), `sha2` (already in `Cargo.lock`, becomes a
direct dependency), the OS random source already used through `chacha20poly1305`

**Storage**: PostgreSQL. New tables `api_tokens`, `api_ids`; new columns
`organizations.api_account_id` and `updated_at` on tasks, projects, clients, users
([data-model.md](data-model.md))

**Testing**: `cargo test -p horae-core` (unit and proptest for `api_decimal` and restart rules),
`#[sqlx::test]` integration tests for the API, a new `harvest-compat` Nix check that runs
third-party clients, and the existing browser checks for the token page

**Target Platform**: Linux server plus the WASM client (the token page)

**Project Type**: Web application (single app crate plus pure `horae-core`)

**Performance Goals**: a 2000-row page of time entries in under 1 s on the 100,000-entry fixture;
a full sync of that fixture under the rate limit (SC-004)

**Constraints**: no floating point for hours or money (Principle I); no write path that bypasses
the shared operations (Principle IV, amended); the compatibility suite runs with no network
(FR-070); no real Harvest account name or id in the repository

**Scale/Scope**: about 70 operations over the seven steps; the 23 surveyed end-user apps call 9
of them most

## Constitution Check

*GATE: checked before Phase 0 and after Phase 1. Checked against constitution 4.0.0.*

**Status: PENDING the proposed amendment** ([constitution-amendment.md](constitution-amendment.md),
4.0.0 → 5.0.0). The maintainer has not approved it. Steps 1, 2, 4 and 5 can be checked against
4.0.0 as it stands except for the decimal conversion; steps 3, 6 and 7 cannot pass without the
amendment.

- **I. Exactness**: PENDING. Storage and computation stay integer. Today's API already converts
  through `f64`, which 2.0.0's follow-up left unassessed. This plan replaces that with integer-only
  decimal text (R-4). That is within the rule, but the amendment's clarifying paragraph is what
  makes it explicit for an external decimal contract. Re-check when the amendment is decided.
- **II. Domain Purity**: Pass. `api_decimal` and the restart and start-stops-running rules live in
  `horae-core` with no I/O dependencies.
- **III. Single Datastore**: Pass. New tables have UUID v7 keys and `org_id`; PostgreSQL only.
  Integer ids are a mapping table, not new primary keys (R-2).
- **IV. Mutations Through Server Functions**: **PENDING, blocks steps 3, 6 and 7.** Under 4.0.0
  the compatibility API "MUST NOT become a second mutation path". The amendment redefines the rule
  as "through the shared mutation operations server functions call" and lets this API call them
  with a hashed, person-bound token. Until it is approved, no write endpoint is implemented.
  Steps 1, 2, 4 and 5 add no mutation through the API: token issue and revocation are server
  functions (contracts/api-tokens.md), the `last_used_at` stamp is bookkeeping on the token row,
  and integer-id allocation happens inside the operations that already create records.
  - Open point for the maintainer: whether the `last_used_at` stamp on a read request counts as a
    mutation under 4.0.0. The plan treats it as authentication bookkeeping, like the session store
    the cookie layer already writes on every request.
- **V. Reproducible Builds & Formatting Gate**: Pass. `nix fmt`, clippy, the `.sqlx` cache check,
  the Dioxus lint ratchet, and the new suite inside `nix flake check`. Third-party clients are
  fixed-output derivations.
- **Data constraints**: Pass. UUID v7, `org_id` everywhere, Postgres only.

Re-check after design: unchanged. Principle IV and the Principle I clarification stay pending;
nothing else changed. Complexity Tracking records the one deviation that depends on the
amendment.

## Project Structure

### Documentation (this feature)

```text
specs/021-harvest-api-compat/
├── spec.md
├── plan.md                    # this file
├── research.md                # §1–3 survey and gaps, §4 planning decisions (R-1–R-12)
├── data-model.md
├── quickstart.md
├── constitution-amendment.md  # proposed, not approved
├── contracts/
│   ├── harvest-api-v2.md      # the API contract, replacing 001's step by step
│   └── api-tokens.md          # token server functions and UI
├── checklists/requirements.md
└── tasks.md
```

### Source Code

```text
crates/core/src/api_decimal.rs                    # minutes⇄hours text, minor units⇄major text
crates/core/src/time_entry_timer.rs               # restart eligibility, start-stops-running rule
crates/horae/migrations/0056_api_ids_and_account_id.sql
crates/horae/migrations/0057_api_tokens.sql
crates/horae/migrations/0058_catalogue_updated_at.sql
crates/horae/src/harvest.rs                       # was harvest/mod.rs (R-12); router and aliases
crates/horae/src/harvest/
├── auth.rs              # extractor: bearer, query, account id, cookie for reads
├── token_auth.rs        # token lookup, last-used stamp
├── ids.rs               # integer id lookup and allocation
├── errors.rs            # Harvest error bodies and status mapping
├── rate_limit.rs        # per-token windows in AppState
├── decimal_json.rs      # RawValue wrappers over api_decimal
├── pagination.rs        # page and cursor envelopes, absolute links
├── time_entry_writes.rs # step 3
├── company.rs, assignments.rs           # step 3
├── reports.rs, invoices.rs, users.rs    # step 5
└── catalogue_writes.rs  # step 6
crates/horae/src/server_fns/api_tokens.rs         # contracts/api-tokens.md
crates/horae/src/server_fns/time_entries/commands.rs   # apply → pub(crate); RestartTimer; D-6
crates/horae/src/models/scoped_time.rs            # TimesheetCommand::RestartTimer
crates/horae/src/importers/harvest/…              # write api_ids with the Harvest id (OQ-2)
crates/horae/src/pages/settings/api_tokens.rs     # token section of personal settings
crates/horae/tests/harvest_api.rs + harvest_api/  # integration tests per step
crates/horae/tests/fixtures/harvest_reference/    # scrubbed recordings (R-11) or hand-written shapes
nix/checks/harvest-compat.nix                     # the suite (R-10)
nix/checks/harvest-compat/{clients,patches}/      # pinned clients and local base-URL patches
scripts/harvest-record.sh                         # GET-only recorder for the test account (R-11)
```

**Structure Decision**: extend the existing `harvest` module, converted to the `foo.rs` + `foo/`
layout in step 1 because the feature rewrites most of it. Integration tests get their own
`harvest_api` test target beside the existing ones in `crates/horae/tests/`.

## PR split

One pull request per delivery step (spec *Delivery order*), each rebased on the previous:

1. Integer ids and account id, `api_decimal` replacing `f64` in reads, suite skeleton.
1. Personal access tokens, error bodies, aliases, rate limits, token page.
1. **Waits for the amendment.** Time-entry writes, restart, D-6, `company`, project assignments.
1. Pagination and sync fidelity, `updated_at` columns.
1. Catalogue and report reads.
1. **Waits for the amendment.** Catalogue writes and invoice status.
1. **Waits for the amendment.** OAuth2 provider.

The recorder (R-11) is its own small pull request whenever test-account credentials are
provided; its fixtures then replace the hand-written shapes.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
| --- | --- | --- |
| The Harvest-compatible API mutates data (steps 3, 6, 7), against Principle IV 4.0.0 | Harvest clients write time over HTTP with a bearer token and cannot call server functions | Read-only stays compliant but meets none of US2, US5 or US6, which are most of the surveyed clients' use. Resolved only by the proposed amendment; not implemented before it is approved |
