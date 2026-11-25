# Tasks: OAuth 2.0 with PKCE for Horae's APIs

**Input**: Design documents from `specs/024-oauth-pkce/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: included. The constitution requires tests for correctness-critical changes, and the
spec's success criteria (SC-003, SC-005, SC-006, SC-007, SC-008) are measured by tests.

**Organization**: one phase per pull request (plan.md *PR split*), as in 022, in an order that
respects the dependencies. Each task carries the user story it serves (`[US1]`–`[US6]`) when it
serves one; infrastructure tasks carry none. Each PR ends with a checkpoint that it passes
`nix flake check` on its own. Tasks marked *(D-n)* carry out one of the maintainer's decisions in
the spec.

## Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: can run in parallel with the other [P] tasks of its phase (different files, no
  dependency on an unfinished task)
- **[USn]**: the user story the task serves

## Dependencies between PRs

| PR | Depends on | Can start with |
| --- | --- | --- |
| A | — (text approved, D-3) | B |
| B | — | A |
| C | A, B | D |
| D | B | C |
| E | B (C for end-to-end tests) | C, D |
| F | B, E | D |
| G | C, D, E | F |

______________________________________________________________________

## Phase 1: PR A — Constitution amendment *(D-3)*

**Goal**: Principle IV allows the token and revocation endpoints, within bounds.

**Independent test**: the constitution reads 7.1.0 with the approved clause.

- [ ] T001 [US1] Apply the approved text of `specs/024-oauth-pkce/constitution-amendment.md` to `.specify/memory/constitution.md`: replace the last bullet of the *Token-authenticated APIs* clause, append the rationale sentence, add the Sync Impact Report and bump the version line to 7.1.0 (MINOR, as approved)
- [ ] T002 Checkpoint: `nix fmt -- --ci` clean; PR A opened

______________________________________________________________________

## Phase 2: PR B — Core rules, storage and the narrowed actor

**Goal**: everything the endpoints and pages build on: scopes, redirect and PKCE rules, tables,
`ops::oauth` for clients and grants, `Channel::OAuth`, audit writes, configuration.

**Independent test**: `cargo test -p horae-core oauth` and `cargo test -p horae-server oauth` pass;
an actor loaded with `Channel::OAuth` has `grants ∩ ceiling` and no administrator identity.

### Tests first

- [ ] T003 [P] [US1] Unit tests for `Scope`/`ScopeSet` parsing, printing, `normalize` (`time:write` ⇒ `time:read`) and unknown scopes in `crates/core/src/permissions/oauth/tests.rs`
- [ ] T004 [P] [US1] Proptests: `grants ∩ ceiling(s) ⊆ grants` for every selection and scope set, the result stays closed under `implies`, and no ceiling contains an approval, rate, invoice, people-write, project-write or company permission, in `crates/core/src/permissions/oauth/properties.rs`
- [ ] T005 [P] [US3] Table tests for `RedirectUri::parse` (https, loopback IPv4/IPv6 with and without port, `localhost` refused, fragment refused, userinfo refused, wildcard refused, private-use scheme with and without a dot, http to a public host refused, public vs confidential) and `RedirectUri::matches` (exact; loopback port variance; path, query and case differences refused) in `crates/core/src/permissions/oauth/tests.rs`
- [ ] T006 [P] [US1] Tests for `pkce::verify` with the RFC 7636 Appendix B vector, verifier lengths 42/43/128/129, characters outside the unreserved set, and `plain` never accepted, in `crates/core/src/permissions/oauth/tests.rs`

### Implementation

- [ ] T007 [US1] Add `crates/core/src/permissions/oauth.rs` (module root, registered in `crates/core/src/permissions.rs`) with `Scope`, `ScopeSet`, `ceiling()` (research R-6 table), `RedirectUri`, `ClientKind` and `pkce::verify` (constant-time comparison with `subtle`, or a local constant-time loop if `horae-core` must stay free of the dependency)
- [ ] T008 Write `crates/server/migrations/0005_oauth.sql` (next free number) with the enums `oauth_client_kind` and `oauth_revoke_reason`, the five tables, checks, indexes and cascades of `data-model.md`, the `set_updated_at()` trigger on `oauth_clients`, and one disabled built-in Android client per existing organization (constants in `contracts/configuration.md`) *(D-1)*
- [ ] T009 Create the built-in client for a new organization in the organization creation path used by `init` and the test fixtures (`crates/server/src/init.rs` and `crates/server/src/testing.rs`) *(D-1)*
- [ ] T010 Add `Channel::OAuth { grant_id, token_id, scopes }` in `crates/server/src/ops/actor.rs`; in `Actor::load_with` treat it as a person channel, intersect the loaded grants with `scopes.ceiling()`, make `administrator()` false for it, and add `Actor::scopes()`; add `Caller::oauth(…)` in `crates/server/src/ops.rs`
- [ ] T011 Update every `match` on `Channel` for the new variant: `ops::people::add` refuses it like `Token` (`crates/server/src/ops/people.rs`), `ops::api_tokens::identity` answers not found for it (`crates/server/src/ops/api_tokens.rs`)
- [ ] T012 [P] Tests for the narrowed actor (member and administrator with each scope set; deactivated person; missing organization) in `crates/server/src/ops/actor/tests.rs`
- [ ] T013 Add `crates/server/src/ops/oauth.rs` (module root) with the shared credential generator (prefix + 32 bytes `OsRng` base64url, SHA-256 digest, `Secret` whose `Debug` prints the prefix only), the prefixes of research R-5, and move `ops::api_tokens::Secret::generate`'s body onto it so both kinds share one generator
- [ ] T014 [US3] Implement `crates/server/src/ops/oauth/clients.rs`: `list`, `get`, `register`, `update`, `set_enabled`, `replace_secret`, `delete`, each `require_administrator`, validating names, kinds, redirect URIs (`horae-core`), scopes; built-in client's fixed fields and deletion refused with `Conflict`
- [ ] T015 [US4] Implement `crates/server/src/ops/oauth/grants.rs`: `list_own`, `list_for_client` (administrator), `revoke` (own, or administrator; reason `person`/`administrator`; another person's grant is `NotFound` for a member)
- [ ] T016 Implement `crates/server/src/ops/oauth/audit.rs` writing `audit_log` rows for every action of research R-15, with names, scopes and reasons in `data` and no credential
- [ ] T017 [P] [US3] Tests for client operations (validation, administrator only, built-in rules, disable keeps grants, delete cascades, secret replaced once) in `crates/server/src/ops/oauth/clients/tests.rs`
- [ ] T018 [P] [US4] Tests for grant listing and revocation by person and administrator, with audit rows asserted, in `crates/server/src/ops/oauth/grants/tests.rs`
- [ ] T019 Add `PublicUrl` and `OAuthConfig` (access TTL, refresh idle, refresh max) to `crates/server/src/config.rs` with the bounds and errors of `contracts/configuration.md`, and tests beside the existing config tests *(D-2)*
- [ ] T020 Add the expired-row cleanup for `oauth_codes` and `oauth_access_tokens` (older than a day) to the scheduler's periodic work in `crates/server/src/scheduler.rs`
- [ ] T021 Regenerate `.sqlx/` (AGENTS.md *sqlx query cache*), exclude the new tables from the migration snapshot tests if they compare whole schemas, and run `cargo clippy -p horae-server --all-targets`
- [ ] T022 Checkpoint: `nix flake check` passes; PR B opened

______________________________________________________________________

## Phase 3: PR C — Authorization server (US1, US2)

**Goal**: a native app can authorize, exchange, refresh and revoke.

**Independent test**: quickstart steps 2, 3, 5 and 7 against the dev stack; the threat-model tests
of research R-11 pass.

### Tests first

- [X] T023 [P] [US1] Router tests for `GET /oauth/authorize`: every check of `contracts/oauth-endpoints.md` in order (page error vs redirect error), a missing or `plain` challenge refused for a public and for a confidential client alike *(D-7)*, `iss` and `state` on redirects, the security headers, `Cache-Control: no-store`, session storage of the request (max 5, 10-minute expiry), in `crates/api/src/oauth/authorize/tests.rs`
- [X] T024 [P] [US1] Router tests for `POST /oauth/token` with `authorization_code`: success shape, wrong verifier, missing verifier, verifier for another challenge, code of another client, other `redirect_uri`, expired code, replayed code revokes the grant, public client with a secret, confidential client without or with a wrong secret (Basic and post), in `crates/api/src/oauth/token/tests.rs`
- [X] T025 [P] [US2] Router tests for `refresh_token`: rotation, replay revokes the whole grant, concurrent double use (one success, grant revoked), idle expiry, absolute expiry, narrower scope, wider scope refused, grant revoked, client disabled, person deactivated, in `crates/api/src/oauth/token/refresh_tests.rs`
- [X] T026 [P] [US4] Router tests for `POST /oauth/revoke` (access token, refresh token, unknown, twice, another client's token is not revoked and still answers 200) in `crates/api/src/oauth/revoke/tests.rs`
- [X] T027 [P] [US1] Test that secrets never reach logs: capture `tracing` output across a full flow and assert no `horae_oac_`, `horae_oat_`, `horae_ort_` or `horae_ocs_` string beyond the 8-character prefixes, in `crates/api/src/oauth/tests.rs`

### Implementation

- [X] T028 [US1] Implement `crates/server/src/ops/oauth/authorize.rs`: `validate_request` (read-only, error split of FR-022), `approve` (active actor, enabled client, creates grant and code, audit `grant_approved`), `deny`
- [X] T029 [US1] Implement `crates/server/src/ops/oauth/tokens.rs`: `exchange_code` (research R-9, row lock, reuse revokes with `code_reuse`), `refresh` (R-8, row lock, reuse revokes with `refresh_reuse`), `revoke_token` (reason `client`), `authenticate` (one query joining token, grant, client and person; `last_used_at` once a minute; returns `Channel::OAuth`); client authentication with constant-time digest comparison
- [X] T030 [P] [US2] Server tests for `tokens::authenticate` (each refusal of FR-052) and the `last_used_at` stamp in `crates/server/src/ops/oauth/tokens/tests.rs`
- [X] T031 [US1] Add `crates/api/src/oauth.rs` (router, mounted only when `HORAE_PUBLIC_URL` is set) and `crates/api/src/oauth/{authorize,token,revoke,metadata}.rs`: form parsing, Basic auth parsing, RFC 6749 error bodies, `no-store` headers, the 400 HTML error page with catalog text through `horae-i18n` (a new dependency of `horae-api`; it builds for the server already); merge it in `horae_api::router`; a test that the metadata document matches `contracts/oauth-endpoints.md` and lists only `S256` (FR-095), and that every path answers 404 without `HORAE_PUBLIC_URL`
- [X] T032 [US1] Keep the token and revocation endpoints free of an in-app limit (FR-081, research R-14): refuse a `client_id` over 64 bytes as `invalid_client` in `crates/api/src/oauth/client_auth.rs`, with a test for the body and Basic; document the reverse proxy limit in `contracts/configuration.md`
- [X] T033 [US1] Store `return_to` for signed-out `/oauth/` page loads in `login_redirect_guard` (`crates/api/src/auth.rs`) and redirect to it after sign-in in `crates/api/src/auth/oidc.rs` and `crates/api/src/auth/dev.rs`, re-checking the `/oauth/` prefix; tests that other paths still land on `/` and that `//evil`, `/oauth/../x` and absolute URLs are ignored
- [X] T034 [US1] Add `crates/types/src/oauth.rs` (`ConsentDetails`, page DTOs of `data-model.md`) and register it in `crates/types/src/lib.rs`
- [X] T035 [US1] Implement `oauth_consent_details`, `approve_oauth_request` and `deny_oauth_request` in `crates/horae/src/server_fns/oauth.rs`, reading and removing the pending request from the session
- [X] T036 [US1] Add the route `OAuthConsent` (`/oauth/consent?:..query`) outside the app shell in `crates/horae/src/route.rs`, and the page `crates/horae/src/pages/oauth_consent.rs` per `contracts/server-functions.md` (load the `dioxus-best-practices` skill first); full navigation to the returned URL
- [X] T037 [P] [US1] Add the `## OAuth approval` section (`oauth-*` ids, scope sentences, error page text) to `crates/i18n/locales/en/horae.ftl`
- [X] T038 [US1] Add a layer in `crates/horae/src/main.rs` that sets `X-Frame-Options: DENY`, `Content-Security-Policy: frame-ancestors 'none'` and `Referrer-Policy: no-referrer` on responses under `/oauth/`, and mount the OAuth router
- [X] T039 [P] [US1] UI tests for the approval page (details render, Allow and Deny call the right server function once, expired state, sign-out link) in `crates/horae/tests/oauth_consent_ui.rs`
- [X] T040 [P] [US1] Server-function tests: approve creates a grant and code bound to the session's request, a second approve finds nothing, a request id from another session finds nothing, in `crates/horae/src/server_fns/oauth/tests.rs`
- [X] T041 Set `HORAE_PUBLIC_URL=http://127.0.0.1:8080` in the dev stack's environment block in `process-compose.yaml`
- [X] T042 Run `cargo run -p horae-dioxus-lint`, clippy for `horae-api` and `horae`, regenerate `.sqlx/`
- [ ] T043 Checkpoint: `nix flake check` passes; PR C opened

______________________________________________________________________

## Phase 4: PR D — Apps in administration (US3, US4)

**Goal**: administrators register and manage apps, enable the built-in client, and revoke anyone's
authorization.

**Independent test**: quickstart step 1 and the administrator part of step 6; a member is refused.

- [ ] T044 [US3] Implement `list_oauth_apps`, `create_oauth_app`, `update_oauth_app`, `set_oauth_app_enabled`, `replace_oauth_app_secret`, `delete_oauth_app` in `crates/horae/src/server_fns/oauth.rs` over `ops::oauth::clients`
- [ ] T045 [US4] Implement `list_oauth_app_grants` and `revoke_oauth_grant` in `crates/horae/src/server_fns/oauth.rs` over `ops::oauth::grants`
- [ ] T046 [US3] Add the routes `AdminApps` (`/admin/apps`) and `AdminApp` (`/admin/apps/:id`) in `crates/horae/src/route.rs` and the admin subnavigation entry after API tokens
- [ ] T047 [US3] Build `crates/horae/src/pages/admin/apps.rs` (list, built-in client first with Enable/Disable, "Register app", notice when `HORAE_PUBLIC_URL` is unset) and `crates/horae/src/pages/admin/apps/{detail,form}.rs` (detail with Actions menu, authorizations table with "Revoke…", register/edit `Modal`, one-time secret reveal reused from the API tokens page), following DESIGN.md *Destructive actions* and *Dialogs*
- [ ] T048 [P] [US3] Add the `## Apps` section (`apps-*` ids) to `crates/i18n/locales/en/horae.ftl`
- [ ] T049 [P] [US3] UI tests for the admin pages (list, register public and confidential, secret shown once, built-in client cannot be edited or deleted, revoke through `ConfirmDialog`) in `crates/horae/tests/oauth_apps_ui.rs`
- [ ] T050 [P] [US3] Server-function tests for administrator-only access and error mapping in `crates/horae/src/server_fns/oauth/tests.rs`
- [ ] T051 Add the `/admin/apps` rows to the Pages table and an *Apps* subsection to DESIGN.md
- [ ] T052 Checkpoint: `nix flake check` passes; PR D opened

______________________________________________________________________

## Phase 5: PR E — OAuth tokens on `/api/v1` (US1, US2)

**Goal**: `/api/v1` accepts OAuth access tokens with scopes, as an addition to v1.

**Independent test**: quickstart step 4 for `/api/v1`; the contract check passes.

- [X] T053 [US1] Dispatch on the prefix in `ApiCaller` (`crates/api/src/token.rs`): `horae_pat_` to `ops::api_tokens::authenticate`, `horae_oat_` to `ops::oauth::tokens::authenticate`
- [X] T054 [US1] Add the `oauth2` security scheme (authorization code flow, relative URLs, the three scopes) and the `{"oauth2": [scope]}` alternative to each operation's `security` in `crates/api/src/v1.rs` and the handler modules under `crates/api/src/v1/`, per `contracts/api-changes.md`
- [X] T055 [US1] Build the scope map from the generated document at start-up and add the route layer that answers 403 `insufficient_scope` with `WWW-Authenticate` for OAuth callers (research R-7), and `WWW-Authenticate: Bearer error="invalid_token"` on 401, in `crates/api/src/v1.rs` and `crates/api/src/problem.rs`. The same layer calls `Actor::unlock_scope` with the route's scope: an OAuth actor has no authority until it does (PR B)
- [X] T055a [US1] Stop reading the stored Administrator flag outside the actor where an OAuth caller can reach it: `ops::time::choices` ("may track on every project") and the Harvest API's `is_admin` queries in `crates/harvest-api/src/lib.rs` take `Actor::administrator()` instead, with a test that an Administrator's OAuth token sees neither (PR B review L9)
- [X] T056 [US1] Test that every token route declares an `oauth2` requirement or is on the allow-list, in `crates/api/src/v1/tests.rs`
- [X] T057 [US1] Add `kind` and `oauth` to `MeToken` in `crates/api/src/v1/types.rs` and fill them in `crates/api/src/v1/me.rs` (a read in `ops::oauth::tokens` for the app name and scopes); `ops::api_tokens::identity` answers not found for any channel but `Token`, so `/me` needs an OAuth arm (FR-061)
- [X] T058 [US2] Key the limiter by `grant_id` for `Channel::OAuth` in `crates/api/src/rate_limit.rs`, with a test that rotating access tokens does not reset the window
- [X] T059 [P] [US1] Per-scope tests over every v1 operation for a member, a manager and an administrator (SC-003), with responses validated against the schema, plus a personal access token on the same routes keeping the person's full authority (FR-064), in `crates/api/src/v1/oauth_tests.rs`
- [X] T060 [US1] Refresh `crates/api/openapi/v1.json` with `UPDATE_OPENAPI=1 cargo test -p horae-api openapi`; run the `api-compat` check and, if oasdiff flags the added security alternative, record the decision in research R-18 before touching `v1.baseline.json`
- [X] T061 Checkpoint: `nix flake check` passes; PR E opened

______________________________________________________________________

## Phase 6: PR F — Connected apps in My settings; Harvest API (US4, US5)

**Goal**: people see and revoke their authorizations; the Harvest API accepts OAuth tokens.

**Independent test**: quickstart step 6 (person part) and the Harvest part of step 4.

- [ ] T062 [US4] Implement `list_my_oauth_grants` and `revoke_my_oauth_grant` in `crates/horae/src/server_fns/oauth.rs`
- [ ] T063 [US4] Add the route `SettingsApps` (`/settings/apps`) in the SETTINGS group after API tokens in `crates/horae/src/route.rs` and the settings side menu, and the page `crates/horae/src/pages/settings/apps.rs` per `contracts/server-functions.md`
- [ ] T064 [P] [US4] Catalog entries for the page under `## Apps` in `crates/i18n/locales/en/horae.ftl`
- [ ] T065 [P] [US4] UI tests for the page (list, revoke through `ConfirmDialog`, empty state, no audit link) in `crates/horae/tests/oauth_settings_ui.rs`
- [ ] T066 [US5] In `crates/harvest-api/src/auth.rs`, accept `horae_oat_` tokens from the header only (D-5), refuse them from `access_token`, keep the `Harvest-Account-Id` check, key the limiter by grant, and unlock the actor for the route's scope (T067) before calling any operation
- [ ] T067 [US5] Add a scope column to the route table in `crates/harvest-api/src/lib.rs` per `contracts/api-changes.md`, enforce it for OAuth callers with 403 and `WWW-Authenticate`, and add a test that every route names a scope
- [ ] T068 [P] [US5] Tests: OAuth reads and writes per scope against the personal-token results, query parameter refused, switch off answers 404, in `crates/harvest-api/src/oauth_tests.rs`
- [ ] T069 Checkpoint: `nix flake check` passes; PR F opened

______________________________________________________________________

## Phase 7: PR G — Audit log, end-to-end check, docs (US6)

**Goal**: administrators see app events; a native-style client runs the whole flow in CI.

**Independent test**: quickstart step 5's audit part; `nix build .#checks.x86_64-linux.oauth-flow`.

- [ ] T070 [US6] Add an "Apps" kind to `AuditFilter` (`crates/types/src/permission_audit.rs`) and read `audit_log` rows of `entity_type` `oauth_client`/`oauth_grant` as a second source merged by time in `crates/server/src/ops/permissions/audit.rs` *(D-4)*
- [ ] T071 [US6] Render one sentence per app event in `crates/horae/src/pages/permission_audit.rs` with catalog messages under `## Audit log`, and extend DESIGN.md *Audit log* *(D-4)*
- [ ] T072 [P] [US6] Tests: each event appears once with actor, app and person; members get nothing, in `crates/server/src/ops/permissions/audit/tests.rs` and `crates/horae/src/pages/permission_audit/tests.rs` *(D-4)*
- [ ] T073 Add `nix/checks/oauth-flow.nix`: start PostgreSQL and the server as `nix/checks/tests.nix` does with `HORAE_PUBLIC_URL` and `DEV_LOGIN=1`, register a public client through the seed, drive the approval with `curl` and the session cookie, then run quickstart steps 3 to 7 with `curl` and `jq`; register it in the flake checks
- [ ] T074 [P] Add `services.horae.publicUrl` and `services.horae.oauth.*` to `nix/modules/nixos/horae.nix` with descriptions, passing the variables of `contracts/configuration.md`
- [ ] T075 [P] Update AGENTS.md (*Auth* paragraph, the HTTP surfaces list) and README.md with the authorization server and its settings
- [ ] T076 Mark 021 FR-060 and FR-062 as met by 024 and FR-061 as met except the account list, with a link, in `specs/021-harvest-api-compat/spec.md` *(D-6)*
- [ ] T077 Checkpoint: `nix flake check` passes; PR G opened

______________________________________________________________________

## Parallel work

- PR B: T003–T006 together; then T012, T017, T018 together once their operations exist.
- PR C: T023–T027 together before the implementation; T037, T039, T040 beside the page.
- PR D and PR C can be built at the same time after B; E can start after B with unit tests and
  finish its end-to-end tests once C lands.

## Implementation strategy

1. Land A first: C needs the approved amendment applied.
1. MVP for the Android app: A, B, C, D, E. At that point a registered or built-in public client
   can sign a person in and use `/api/v1`.
1. Then F (people manage their own apps; Harvest tools) and G (audit, the Nix check, docs).
