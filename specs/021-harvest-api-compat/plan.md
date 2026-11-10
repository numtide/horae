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
`arbitrary_precision` feature the `server` build already enables (exact decimal numbers),
`sha2` (already in `Cargo.lock`, becomes a direct dependency in step 2), the OS random source
already used through `chacha20poly1305`

**Storage**: PostgreSQL. New table `api_tokens`; new columns `organizations.api_account_id`,
`harvest_id` (the id in this API) on each exposed table, and `updated_at` on tasks, projects,
clients, users
([data-model.md](data-model.md))

**Testing**: `cargo test -p horae-core` (properties for the wire decimals and unit tests for the
restart rules), `#[sqlx::test]` router tests for the API in `crates/horae/src/harvest/`, a new `harvest-compat` Nix check that runs
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

*GATE: checked before Phase 0 and after Phase 1. Checked against constitution 5.0.0.*

**Status: PASS.** The maintainer approved the amendment
([constitution-amendment.md](constitution-amendment.md)) on 2026-11-09, and
`.specify/memory/constitution.md` is at 5.0.0 with it applied verbatim, in the step-1 pull
request.

- **I. Exactness**: Pass. Storage and computation stay integer. Today's API converts through
  `f64`; this plan replaces that with integer-only decimal text (R-4), which the 5.0.0 paragraph on
  decimal wire formats states explicitly.
- **II. Domain Purity**: Pass. The decimal conversions (`duration::format_hours2`,
  `convert::hours_to_minutes`, `money::format_cents_plain`, `money::parse_cents`) and the restart
  and start-stops-running rules live in `horae-core` with no I/O dependencies.
- **III. Single Datastore**: Pass. New tables have UUID v7 keys and `org_id`; PostgreSQL only.
  Integer ids are a `harvest_id` column beside the UUID primary key, not new primary keys (R-2).
- **IV. Mutations Through Server Functions**: Pass. Under 5.0.0 the Harvest-compatible API may
  call the shared mutation operations server functions call, with the actor resolved from a
  hashed, person-bound token. Steps 3, 6 and 7 write only through those operations and refuse
  cookie-only callers. Token issue and revocation are server functions (contracts/api-tokens.md);
  integer ids are assigned by a trigger on the insert the operations already make. The
  `last_used_at` stamp on a read request is authentication bookkeeping on the token row, like the
  session store the cookie layer already writes on every request.
- **V. Reproducible Builds & Formatting Gate**: Pass. `nix fmt`, clippy, the `.sqlx` cache check,
  the Dioxus lint ratchet, and the new suite inside `nix flake check`. Third-party clients are
  fixed-output derivations.
- **Data constraints**: Pass. UUID v7, `org_id` everywhere, Postgres only.

Re-check after design: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/021-harvest-api-compat/
├── spec.md
├── plan.md                    # this file
├── research.md                # §1–3 survey and gaps, §4 planning decisions (R-1–R-12)
├── data-model.md
├── quickstart.md
├── constitution-amendment.md  # approved 2026-11-09, applied as 5.0.0
├── contracts/
│   ├── harvest-api-v2.md      # the API contract, replacing 001's step by step
│   └── api-tokens.md          # token server functions and UI
├── checklists/requirements.md
└── tasks.md
```

### Source Code

```text
crates/core/src/time_entry_timer.rs               # restart eligibility, start-stops-running rule
crates/horae/migrations/0057_api_account_id.sql   # 0056 is taken by the time-zone branch
crates/horae/migrations/0058_harvest_import_id.sql  # harvest_import_map.harvest_id renamed
crates/horae/migrations/0059_harvest_ids.sql        # harvest_id columns, sequences, trigger, lookup indexes
crates/horae/migrations/0060_harvest_ids_backfill.sql
crates/horae/migrations/0061_harvest_ids_check.sql  # then 0062: NOT NULL and unique indexes
crates/horae/migrations/0062_harvest_ids_not_null.sql
crates/horae/migrations/<next>_api_tokens.sql     # step 2
crates/horae/migrations/<next>_catalogue_updated_at.sql   # step 4
crates/horae/src/harvest.rs                       # was harvest/mod.rs (R-12); router and aliases
crates/horae/src/harvest/
├── auth.rs              # extractor: bearer, query, account id, cookie for reads
├── token_auth.rs        # token lookup, last-used stamp
├── ids.rs               # integer id → record lookup (harvest_id columns)
├── errors.rs            # Harvest error bodies and status mapping
├── rate_limit.rs        # per-token windows in AppState
├── decimal_json.rs      # Hours and Amount as exact JSON numbers
├── api_tests.rs         # router tests per step, beside pagination_tests.rs
├── pagination.rs        # page and cursor envelopes, absolute links
├── time_entry_writes.rs # step 3
├── company.rs, assignments.rs           # step 3
├── reports.rs, invoices.rs, users.rs    # step 5
└── catalogue_writes.rs  # step 6
crates/horae/src/server_fns/api_tokens.rs         # contracts/api-tokens.md
crates/horae/src/server_fns/time_entries/commands.rs   # apply → pub(crate); RestartTimer; D-6
crates/horae/src/models/scoped_time.rs            # TimesheetCommand::RestartTimer
crates/horae/src/importers/harvest/…              # insert the Harvest id as harvest_id when free (OQ-2)
crates/horae/src/pages/settings/api_tokens.rs     # token section of personal settings
crates/horae/tests/fixtures/harvest_reference/    # scrubbed recordings (R-11) or hand-written shapes
nix/checks/harvest-compat.nix                     # the suite (R-10)
nix/checks/harvest-compat/{clients,patches}/      # pinned clients and local base-URL patches
scripts/harvest-record.sh                         # GET-only recorder for the test account (R-11)
```

**Structure Decision**: extend the existing `harvest` module, converted to the `foo.rs` + `foo/`
layout in step 1 because the feature rewrites most of it. Integration tests get their own
`harvest_api` test target beside the existing ones in `crates/horae/tests/`.

## Layering

`/harvest/v2` is one adapter over Horae's operations. A native `/api/v1` and multi-tenant
deployments (`specs/020-multi-tenant-subdomains`) will reuse the same operations, so every step
follows these rules:

1. **Handlers hold no business logic.** A `/harvest/v2` handler translates input (integer id →
   UUID, hours → minutes, decimal → cents), calls a shared operation in the
   `*_record(tx, actor, input)` style the server functions already use, and translates the
   output. An operation the API needs and that exists only inside a server function is extracted
   first; the server function then calls it. The rule binds every write from step 3 on and every
   read added from step 5 on. Step 1's reads predate it: `harvest.rs` still scopes users and time
   entries itself (`scoped_user_filter`, the manager and member split). That is recorded debt, to
   move into a shared read operation when step 5 touches those reads.
1. **One `Actor` for every entry point.** A session, a token and later `/api/v1` all produce the
   same `horae_core::permissions::Actor` (person, organization, active), loaded by the
   authorization service from a membership in one organization; its permissions are checked
   in `horae-core`. A review checks that no entry point builds its own.
1. **Operations return domain errors**; each adapter maps them to its own wire format
   (`harvest/errors.rs` for this API).
1. **One pagination core.** Each list is one ordered query (sort key, then `id`) with its filters
   and `updated_since`, taking a keyset position (the last row's sort key and `id`) or an offset,
   and a limit. `/harvest/v2` maps `page`/`per_page` to an offset into that same ordering and
   `cursor` (step 4) to the keyset position, so both walk the same rows in the same order.

**Tokens (step 2) belong to the account, not to an organization.** Each request names the
organization with `Harvest-Account-Id` (= `organizations.api_account_id`), which resolves to the
account's membership there. Without the header the account's only membership is used; an
account with several is refused with 403 and
`{"error": "account_required", "error_description": "…send Harvest-Account-Id…"}`, so a
client that worked without the header gets an answer that names it once its account gains a
second membership. Under multi-tenant hosting the request's host names the tenant and wins: the
membership used is the one in the host's tenant, a header that names another organization is
403, and a token whose account has no membership there is 401. A header-less token never acts
in another tenant than the host's. Tokens are Horae credentials, shared by `/harvest/v2` and the
future `/api/v1`, not Harvest-specific. Until the multi-tenant split of accounts and memberships
lands, an account is a `users` row and has exactly one membership.

**Crates.** Database-free Harvest pieces live in `horae-harvest` (`crates/harvest`): the compat
API's output types and exact decimals (today `harvest/types.rs`, `harvest/decimal_json.rs`),
sharing resource types with the importer where the schema is the same. Handlers, id resolution
(`harvest/ids.rs`), reads and anything with sqlx stay in the app crate.

## PR split

One pull request per delivery step (spec *Delivery order*), each rebased on the previous:

1. The approved constitution amendment, integer ids and account id, exact decimals
   replacing `f64` in reads, error bodies, path aliases, suite skeleton.
1. Personal access tokens, rate limits, token page.
1. Time-entry writes, restart, D-6, `company`, project assignments.
1. Pagination and sync fidelity, `updated_at` columns.
1. Catalogue and report reads.
1. Catalogue writes and invoice status.
1. OAuth2 provider.

The recorder (R-11) is its own small pull request whenever test-account credentials are
provided; its fixtures then replace the hand-written shapes.

## Complexity Tracking

No deviation. The one this plan carried (API writes against Principle IV 4.0.0) is resolved by
the approved 5.0.0 amendment.
