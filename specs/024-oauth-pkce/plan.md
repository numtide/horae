# Implementation Plan: OAuth 2.0 with PKCE for Horae's APIs

**Branch**: `spec/oauth-pkce` (spec, plan and tasks); each PR below lands as its own branch and
pull request | **Date**: 2026-11-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/024-oauth-pkce/spec.md`, with the maintainer's
decisions D-1 to D-7 (2026-11-22).

## Summary

Make Horae an OAuth 2.0 authorization server for its own APIs: the authorization code grant with
PKCE S256 required, public and confidential clients registered by administrators, rotating refresh
tokens with reuse detection, and revocation. People approve apps on a Horae page and see and revoke
them in My settings; administrators register apps and revoke anyone's authorization. Access tokens
work on `/api/v1` and on the Harvest-compatible API when it is on, beside personal access tokens,
narrowed by three scopes that only ever subtract from the person's permissions. Everything runs
through new shared operations in `ops::oauth`; the token and revocation endpoints need a bounded
constitution amendment, which the maintainer approved (D-3). Seven pull requests, A to G.

## Technical Context

**Language/Version**: Rust 2024 (toolchain from the Nix dev shell), Dioxus 0.7.9 fullstack for
the pages

**Primary Dependencies**: axum 0.8, tower-sessions, sqlx 0.8 (compile-time macros), utoipa 5 and
utoipa-axum 0.2, `sha2`, `subtle`, `base64`, `url` (all in `Cargo.lock`). No new dependency
(research R-1).

**Storage**: PostgreSQL. New tables `oauth_clients`, `oauth_grants`, `oauth_codes`,
`oauth_access_tokens`, `oauth_refresh_tokens` in one migration `0005_oauth.sql`
([data-model.md](data-model.md)). Pending authorization requests live in the session (R-3).

**Testing**: `cargo test -p horae-core` (scopes, ceilings, redirect URI rules, PKCE check;
proptests), `cargo test -p horae-server` (every `ops::oauth` operation, reuse, expiry, intersection
in `Actor::load`), `cargo test -p horae-api` (token, revoke and metadata endpoints, scope
enforcement on every v1 route, OpenAPI snapshot), `cargo test -p horae-harvest-api` (OAuth tokens
on the Harvest routes), `cargo test -p horae --features server` (server functions, pages), a new
Nix check `oauth-flow` that runs a native-style client through the quickstart

**Target Platform**: Linux server; the WASM client for the pages

**Project Type**: web service plus web application (Cargo workspace)

**Performance Goals**: an OAuth-authenticated API request costs one indexed lookup more than
today at most (the token joins its grant and client in one query); the token endpoint answers in
under 100 ms excluding network

**Constraints**: no OAuth rule in `horae-api` or `horae-harvest-api` beyond parsing and rendering
(Principle IV); scopes only narrow (FR-063); secrets never logged; `/api/v1` changes only by
addition (022 FR-050); no network in Nix checks

**Scale/Scope**: 4 HTTP endpoints, 1 approval page, 2 settings pages, about 12 server functions,
1 migration; every one of the 20 `/api/v1` operations and 14 Harvest routes gets a scope

## Constitution Check

*GATE: checked before Phase 0 and after Phase 1. Checked against constitution 7.0.0.*

**Status: PASS, with one item that depends on PR A.** The token and revocation endpoints (PR C)
issue and revoke credentials outside server functions, which Principle IV's API clause in 7.0.0
does not allow. The maintainer approved the amendment (D-3); PR A applies it as 7.1.0, and C
depends on A.

- **I. Exactness**: Pass. No durations or money are introduced. Token lifetimes are durations of
  time but not domain quantities; they are `interval`/`timestamptz` values, never floats.
- **II. Domain Purity**: Pass. Scopes, the permission ceiling, the redirect URI rules and the PKCE
  comparison need no I/O and live in `horae-core` with unit tests and proptests (R-6, R-9).
- **III. Single Datastore**: Pass. Five new tables, each with a UUID v7 key and `org_id`;
  PostgreSQL only. Session data uses the existing Postgres session store.
- **IV. Mutations Through Shared Operations**:
  - Registering, editing, enabling, disabling and deleting apps, replacing secrets, approving,
    denying and revoking from the web app: server functions over `ops::oauth`. Pass.
  - OAuth access tokens on both APIs: already allowed by the 7.0.0 clause ("an OAuth2 access token
    that Horae issued, stored only as a hash, and bound to one person in one organization"). The
    APIs keep calling the same operations; a token never grants more than its person (R-6). Pass.
  - Cookie-only requests still cannot mutate; an OAuth token is not a cookie. Pass.
  - Code exchange, refresh and revocation at `/oauth/token` and `/oauth/revoke`: they issue and
    revoke credentials without a session, so not through server functions. Allowed by the
    amendment the maintainer approved (D-3,
    [constitution-amendment.md](constitution-amendment.md)); **PR C waits for PR A** to apply it.
    Until then the deviation stays in *Complexity Tracking*.
  - `GET /oauth/authorize` writes only to the session (the pending request), like
    `/auth/oidc/start`; no datastore mutation. Pass.
  - The `return_to` session value set by `login_redirect_guard` is session state, not domain data.
    Pass.
- **V. Reproducible Builds**: Pass. No new dependency; the new Nix check runs offline.
- **Technology & Data Constraints**: authentication stays identity-provider based (C-5);
  authorization reads stored permission state, and scopes are a ceiling over it, not a second
  model: they never grant, they only remove. Pass.

**Post-design re-check (after Phase 1)**: unchanged. The data model keeps every rule in `ops` and
`horae-core`; the contracts put only parsing and rendering in `horae-api`; the narrowed actor is
built in `Actor::load`, the one place permissions are loaded.

## Project Structure

### Documentation (this feature)

```text
specs/024-oauth-pkce/
├── spec.md
├── plan.md                     # this file
├── research.md                 # Phase 0
├── data-model.md               # Phase 1
├── quickstart.md               # Phase 1
├── constitution-amendment.md   # approved text (D-3), applied by PR A
├── contracts/
│   ├── oauth-endpoints.md      # /oauth/authorize, /oauth/token, /oauth/revoke, metadata
│   ├── server-functions.md     # approval, apps, authorizations
│   ├── api-changes.md          # /api/v1 and Harvest API changes
│   └── configuration.md        # settings and NixOS options
├── checklists/requirements.md
└── tasks.md                    # Phase 2
```

### Source code (repository root)

```text
crates/core/src/permissions/oauth.rs          # Scope, ScopeSet, ceiling(), redirect URI rules, PKCE check
crates/server/migrations/0005_oauth.sql
crates/server/src/ops/oauth.rs                # module root: shared types, credential generator
crates/server/src/ops/oauth/clients.rs        # register, update, enable, disable, delete, replace_secret, list
crates/server/src/ops/oauth/authorize.rs      # validate_request, approve, deny
crates/server/src/ops/oauth/tokens.rs         # exchange_code, refresh, revoke_token, authenticate
crates/server/src/ops/oauth/grants.rs         # list_own, list_for_client, revoke
crates/server/src/ops/oauth/audit.rs          # audit_log writes
crates/server/src/ops/actor.rs                # Channel::OAuth, narrowed grants
crates/server/src/config.rs                   # public URL, lifetimes
crates/api/src/oauth.rs                       # router: authorize, token, revoke, metadata
crates/api/src/oauth/{authorize,token,revoke,metadata}.rs
crates/api/src/token.rs                       # bearer: personal or OAuth token
crates/api/src/v1.rs                          # oauth2 security scheme, scope map, scope layer
crates/api/src/auth.rs, auth/oidc.rs, auth/dev.rs   # return_to for /oauth/ paths
crates/harvest-api/src/auth.rs, lib.rs        # OAuth tokens in the header, scope per route
crates/horae/src/route.rs                     # OAuthConsent, SettingsApps, AdminApps, AdminApp
crates/horae/src/pages/oauth_consent.rs
crates/horae/src/pages/settings/apps.rs
crates/horae/src/pages/admin/apps.rs (+ apps/ for the detail and form)
crates/horae/src/server_fns/oauth.rs (+ oauth/ tests)
crates/types/src/oauth.rs                     # DTOs for the pages
crates/i18n/locales/en/horae.ftl              # `oauth`, `apps` sections
crates/horae/src/main.rs                      # mount oauth router, frame headers on /oauth/
nix/modules/nixos/horae.nix                   # publicUrl and oauth options
nix/checks/oauth-flow.nix                     # end-to-end native-style client
```

**Structure Decision**: the established layering (`core → server → api → harvest-api → horae`).
Rules in `horae-core` and `ops::oauth`; HTTP parsing and OAuth error rendering in `horae-api`;
pages and server functions in `horae`. New modules use the `foo.rs` + `foo/` layout.

## PR split

| PR | Content | Depends on | Stories |
| --- | --- | --- | --- |
| A | Constitution amendment (text approved by the maintainer in D-3, applied verbatim) | | |
| B | Core and storage: `horae-core` scopes, ceiling, redirect and PKCE rules; migration; `ops::oauth` clients and grants operations; `Channel::OAuth` and the narrowed actor; audit writes; configuration | | US3 (ops) |
| C | Authorization server: `/oauth/authorize`, `return_to`, approval page and its server functions, `/oauth/token`, `/oauth/revoke`, metadata, frame headers | A, B | US1, US2 |
| D | Administration: apps list, register, edit, secret, enable/disable, delete; per-app authorizations with revoke; built-in Android client | B | US3, US4 (admin) |
| E | `/api/v1`: OAuth bearer, scope map from the document, `insufficient_scope`, `Me` addition, rate limit per grant, OpenAPI security scheme and snapshot | B (C for end-to-end tests) | US1, US2 |
| F | My settings "Connected apps" page; Harvest API OAuth tokens with scopes | B, E | US4, US5 |
| G | Audit log "Apps" kind (D-4); the `oauth-flow` Nix check; docs and NixOS options | C, D, E | US6 |

C and D can be reviewed in parallel after B. The first usable slice for the Android app is
A + B + C + D + E.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
| --- | --- | --- |
| Token and revocation endpoints issue and revoke credentials outside server functions (Principle IV, "token issue and revocation … go through server functions") | OAuth clients have no Horae session and cannot call server functions; RFC 6749 defines these as plain HTTP endpoints | Issuing tokens on the approval page and handing them through the redirect is the implicit grant, which RFC 9700 forbids and the spec excludes (FR-020). The maintainer approved the amendment (D-3); this row goes once PR A applies it |
| `return_to` added to sign-in redirects | The person must come back to the approval request after signing in (FR-023) | Encoding the request in the provider's `state` would put client parameters in the identity provider's logs and change the OIDC flow; limited to `/oauth/` paths so sign-in otherwise behaves as before |
