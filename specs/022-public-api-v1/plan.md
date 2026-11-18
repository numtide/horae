# Implementation Plan: Public API `/api/v1`

**Branch**: `022-public-api-v1-plan` (plan and tasks); each PR below lands as its own branch and
pull request | **Date**: 2026-11-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/022-public-api-v1/spec.md` (clarified 2026-11-18,
PR #418), the approach the maintainer approved for planning, and the decisions below.

## Summary

Add a public, versioned API at `/api/v1` for third parties, authenticated by personal access
tokens, described by an OpenAPI document generated from the code, and limited to what the web app
already does: identity, time entries with timers (read and write), and the catalogue read-only.
Every endpoint translates its request, calls a shared operation in `horae-server`, and translates
the result; the API holds no rule. A new crate, `horae-api`, takes the existing HTTP surfaces
(exports, `/auth`, `/health`) plus tokens, errors, cursors, the limiter and `v1`. The
Harvest-compatible API moves to `horae-harvest-api`, behind a runtime switch that is off by
default, and adopts the shared tokens and limiter. Ten pull requests, A to I plus D2, with their
dependencies in *PR split*.

## Decisions (maintainer, 2026-11-18)

| # | Question | Decision |
| --- | --- | --- |
| P-1 | `/harvest/v2` needs the shared tokens (C) and limiter (F), but PR D depends only on B | **D2**: a separate PR after C, D and F connects `/harvest/v2` to tokens and the limiter. D stays "move + switch" |
| P-2 | Restart does not exist, and starting a timer while one runs is refused (021 D-6 unimplemented) | PR E adds restart **and** D-6: start and restart stop the running timer, in the web app too |
| P-3 | Shape of the Principle IV amendment | A general clause for Horae's token-authenticated HTTP APIs, and the principle renamed "Mutations Through Shared Operations" ([constitution-amendment.md](constitution-amendment.md)); text approved 2026-11-18 |
| P-4 | Plan approval (2026-11-18) | Plan approved with the recommendations. The R-3 residual risk is accepted (no extra CI job; review of the baseline file mitigates it). Defaults accepted: 24 h cursors, public `openapi.json`, 422 for unknown fields, restart only today's entries (409 otherwise). Change: in production the cursor key comes from a secret file (`services.horae.api.cursorKeyFile`); the random key per process is a development fallback with a start-up warning (research R-5, contracts/configuration.md) |

Earlier decisions this plan follows: the approach handed to planning (utoipa, schema validation
in tests, a Nix breaking-change check, HMAC cursors with keyset pagination, 021's tokens, one
limiter); the crate study's P6 (`horae-api` starts with `sqlx`) and P8 (`server → api → horae`).

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the Nix dev shell), Dioxus 0.7.9
fullstack for the token pages

**Primary Dependencies**: axum 0.8, tower-sessions (moved with `auth/http`), sqlx 0.8
(compile-time macros), `utoipa` 5 and `utoipa-axum` 0.2 (new), `jsonschema` (new, dev only),
`sha2`, `hmac`, `subtle`, `base64` (already in `Cargo.lock`); `oasdiff` packaged in Nix
(research R-3); `openapi-generator-cli` 7.22.0 from nixpkgs for the e2e client (R-14)

**Storage**: PostgreSQL. New table `api_tokens`; `updated_at` on `projects`, `clients`, `users`,
and `created_at`/`updated_at` on `tasks` ([data-model.md](data-model.md))

**Testing**: `cargo test -p horae-server` (token operations, new reads, restart and D-6),
`cargo test -p horae-api` (router tests that validate every response against the generated
schema, cursor and limiter unit tests, the OpenAPI snapshot test), `cargo test -p horae-harvest-api` (the moved Harvest suite), `cargo test -p horae --features server` (server
functions, pages), new Nix checks `api-compat` and `api-v1`, the existing `e2e` and
`harvest-compat` checks with the Harvest switch on

**Target Platform**: Linux server; the WASM client for the token pages

**Project Type**: web service plus web application (Cargo workspace)

**Performance Goals**: a 1,000-row page of time entries in under 1 s on the 100,000-entry fixture
(SC-007); token authentication adds one indexed lookup per request

**Constraints**: no rule in the API layer (FR-020, Principle IV); no `horae-types` DTO in `v1`
(FR-040); integer minutes and minor units on the wire (Principle I); the contract only grows
within `v1` (FR-050); no network in Nix checks; tokens never logged (FR-009)

**Scale/Scope**: 20 operations in `v1` (contracts/api-v1.md); about 13,000 lines (with tests)
moved into `horae-api` and 4,500 into `horae-harvest-api`

## Constitution Check

*GATE: checked before Phase 0 and after Phase 1. Checked against constitution 6.0.0.*

**Status: PASS with one gated item.** Writes over `/api/v1` (PR H) need the amendment in PR A;
H depends on A, so no write ships before it.

- **I. Exactness**: Pass. `/api/v1` carries integer minutes and minor units with a currency code;
  no conversion exists (research R-8). The Harvest API's existing exact decimals are moved, not
  changed.
- **II. Domain Purity**: Pass. Restart eligibility and the start-stops-running rule are domain
  rules; where they need no I/O they live in `horae-core` (`time_entry_timer`), with unit tests.
  Cursor encoding and the limiter are transport concerns and live in `horae-api`.
- **III. Single Datastore**: Pass. `api_tokens` has a UUID v7 key and `org_id`; PostgreSQL only.
- **IV. Mutations Through Server Functions**: **Gated on PR A.** Under 6.0.0 only the
  Harvest-compatible API may call the shared operations with a token-resolved actor. PR A applies
  the amendment that extends the clause to `/api/v1` (P-3). After it:
  - every `/api/v1` write calls `ops::time::apply` or `ops::time::patch`, which the timesheet
    also uses;
  - `/api/v1` refuses cookies outright (stronger than "a cookie-only request MUST NOT mutate");
  - tokens are hashed and bound to one person in one organization (R-6);
  - token issue and revocation are server functions over `ops::api_tokens` (contracts/api-tokens.md);
  - `last_used_at` is stamped by `ops::api_tokens::authenticate`. It is authentication bookkeeping
    on the token row, like the session store the cookie layer writes on every request (021 plan's
    reading, unchanged).
    The moved exports, `/auth` and `/health` keep their current standing (no mutation; the OIDC
    callback's second exception is moved, not widened).
- **V. Reproducible Builds & Formatting Gate**: Pass. New crates join `nix/checks/tests.nix` and
  `clippy.nix`; `oasdiff` is a pinned derivation; the new checks need no network.
- **Data constraints**: Pass. Authorization reads stored permission state through `Actor::load`
  for the new `Channel::Token`; no second model.

**Re-check after design**: unchanged. The design adds `Channel::Token` to `ops::Caller`, which
`Actor::load` treats as a session; `ops::people::add` refuses it (no new mutation path).

## Project Structure

### Documentation (this feature)

```text
specs/022-public-api-v1/
├── spec.md
├── plan.md                       # this file
├── research.md                   # §1 the code today, §2 decisions R-1 to R-14
├── data-model.md
├── quickstart.md
├── constitution-amendment.md     # proposed Principle IV text (PR A)
├── contracts/
│   ├── api-v1.md                 # readable contract of every v1 operation
│   ├── openapi-v1.draft.yaml     # draft, replaced by the generated document in PR F
│   ├── api-tokens.md             # token server functions, operations and pages
│   └── configuration.md          # variables, NixOS options, upgrade note
├── checklists/requirements.md
└── tasks.md
```

### Source Code

```text
crates/api/                                   # horae-api (new, PR B)
├── Cargo.toml
├── openapi/v1.json                           # generated snapshot (PR F)
├── openapi/v1.baseline.json                  # frozen release baseline (PR F)
└── src/
    ├── lib.rs                                # router(options) → axum::Router; mounts the pieces below
    ├── health.rs                             # /health (PR B, from main.rs)
    ├── exports.rs + exports/                 # from crates/horae/src/http/reports* (PR B)
    ├── import_errors.rs, harvest_oauth.rs    # from crates/horae/src/http/ (PR B)
    ├── auth.rs + auth/{dev,oidc,page,session}.rs   # from crates/horae/src/auth/http* (PR B)
    ├── token.rs                              # bearer extractor → ops::api_tokens::authenticate (PR C)
    ├── problem.rs                            # problem+json, OpError mapping (PR F)
    ├── cursor.rs                             # HMAC cursors (PR F)
    ├── rate_limit.rs                         # per-token window, shared (PR F)
    ├── v1.rs                                 # OpenApiRouter, /api/v1/openapi.json (PR F)
    └── v1/{me,time_entries,timers,projects,tasks,clients,people,types}.rs   (PRs F, G, H)
crates/harvest-api/                           # horae-harvest-api (new, PR D)
└── src/lib.rs + …                            # from crates/horae/src/harvest* ; router(enabled)
crates/server/src/ops/api_tokens.rs           # PR C
crates/server/src/ops.rs                      # Channel::Token, Caller::token (PR C)
crates/server/src/ops/actor.rs                # load person for Channel::Token (PR C)
crates/server/src/ops/time.rs                 # RestartTimer, D-6 (PR E); patch (PR H)
crates/server/src/ops/time_reads.rs           # list, get (PR H)
crates/server/src/ops/{projects/reads,clients,projects/task_catalog,people}.rs  # page, get (PR G)
crates/server/src/config.rs                   # HORAE_HARVEST_API (D), cursor key file, rate limit (F)
crates/server/migrations/000N_catalogue_updated_at.sql   # PR E (next free number when it merges)
crates/server/migrations/000N_api_tokens.sql             # PR C (next free number when it merges)
crates/core/src/time_entry_timer.rs           # restart eligibility (PR E)
crates/types/src/scoped_time.rs               # TimesheetCommand::RestartTimer (PR E)
crates/horae/src/main.rs                      # mounts horae_api and horae_harvest_api
crates/horae/src/server_fns/api_tokens.rs     # PR C
crates/horae/src/pages/settings/api_tokens.rs # PR C
crates/horae/src/pages/admin/api_tokens.rs    # PR C
crates/horae/Dioxus.toml                      # watch_path for ../api and ../harvest-api (PRs B, D)
nix/modules/nixos/horae.nix                   # harvestApi.enable (D), api.rateLimit and api.cursorKeyFile (F)
nix/packages/oasdiff.nix                      # PR F
nix/checks/{api-compat,api-v1}.nix            # PRs F and I
nix/checks/{tests,clippy}.nix                 # -p horae-api (B), -p horae-harvest-api (D)
docs/api.md                                   # how to use the API (PR I)
```

**Structure Decision**: two new library crates in the workspace, `crates/api` and
`crates/harvest-api`, mounted by the `horae` binary. Module layout follows `foo.rs` + `foo/`.

## Layering

1. **Handlers hold no rule.** A `v1` handler parses and validates the shape of its input
   (types, unknown fields, ranges the schema states), resolves the caller, calls one shared
   operation, and converts its result. Anything that decides what a person may see or do is in
   `ops`. A review checks that `horae-api::v1` contains no `sqlx` and no permission test.
1. **One `Actor` for every entry point.** Sessions, tokens and later OAuth produce a `Caller`;
   `Actor::load` is the only place a caller becomes authority.
1. **Operations return `OpError`;** `problem.rs` is the only mapping for `v1` (research R-4), and
   `horae-harvest-api` keeps its own mapping into Harvest's shapes.
1. **Public types are owned by `v1`.** No `horae-types` type appears in a `v1` signature or schema;
   conversions are explicit (R-8).
1. **The contract only grows.** The snapshot test shows every change; the `api-compat` check
   refuses breaking ones; a change to `v1.baseline.json` outside a release is called out in review.

## PR split

| PR | Content | Depends on | Spec |
| --- | --- | --- | --- |
| **A** | Constitution amendment (Principle IV, P-3); remove the `/api/v2` alias and its tests; update 021's contract *Base paths* | — | FR-022, FR-052 |
| **B** | Create `horae-api` by moving `http/`, `auth/http` and `/health`; `main.rs` merges `horae_api::router`; Nix, sqlx prepare and `Dioxus.toml` follow. No behaviour change | — | FR-070, C-4 |
| **C** | Tokens: migration, `ops::api_tokens`, `Channel::Token`, server functions, settings and admin pages, bearer extractor in `horae-api` | B | FR-001–FR-010; FR-011's choice between several memberships waits for 020 (single membership until then, research R-6) |
| **D** | Create `horae-harvest-api` by moving `harvest/`; `HORAE_HARVEST_API` and `services.horae.harvestApi.enable`, off by default; empty 404 when off; e2e and harvest-compat checks turn it on | B (merge after A, which edits the router D moves) | FR-060, FR-061, FR-063, C-5 |
| **D2** | `/harvest/v2` authenticates with shared tokens (bearer and `access_token`, keeping cookie reads) and takes the shared limiter; Harvest account id shown on the token page when on | C, D, F | FR-008, FR-062 |
| **E** | Shared operations: `RestartTimer`, D-6 in start and restart, web restart action; catalogue `updated_at` migration | — | FR-030, FR-046, P-2 |
| **F** | `v1` base: utoipa, `problem.rs`, cursor, limiter, config, `/api/v1/openapi.json`, `GET /me`, `/me/trackable`, `/me/timer`, snapshot test, schema-validating test helper, `oasdiff` package and `api-compat` check | C | FR-040–FR-051 |
| **G** | Catalogue reads: `page`/`get` operations in `ops` and `/projects`, `/tasks`, `/clients`, `/people` | E, F | FR-030, FR-021 |
| **H** | Time entries: `ops::time_reads::{list,get}`, `ops::time::patch`; list, get, create, patch, delete, timers | A, E, F | FR-030, FR-031, FR-020 |
| **I** | `docs/api.md`, release notes, AGENTS.md surfaces; `api-v1` Nix check with a generated client | G, H | SC-001, SC-002 |

```text
A ──────────────────────────────────────┐
B ─┬─► C ─┬─► F ─┬─► G ─┬─► I           │
   │      │      │      │               │
   │      │      └─► H ◄┼───────────────┘
   └─► D ─┴──────► D2   │
E ──────────────► G, H ─┘
```

Parallel lanes: {A, B, E} start at once; after B, {C, D}; after C, F; after F, {G, H, D2}.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
| --- | --- | --- |
| `/api/v1` mutates through the shared operations (Principle IV, 6.0.0) | Third-party clients cannot call server functions (FR-031) | Read-only v1: the maintainer chose time-entry writes (clarification Q1). Resolved by the amendment in PR A, on which H depends |
