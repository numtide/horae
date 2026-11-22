# Research: OAuth 2.0 with PKCE for Horae's APIs

Compiled 2026-11-22 for [spec.md](spec.md) on branch `spec/oauth-pkce` (master `58d5ad4e`). Part
1 records what the code does today. Part 2 records each planning decision with what else was
weighed. Decisions that follow from one of the maintainer's decisions in the spec (D-1 to D-7)
cite it. R-19 compares the design with Harvest's public OAuth documentation.

## 1. Horae today (checked in the code)

### 1.1 Personal access tokens

- `crates/server/migrations/0004_api_tokens.sql`: `api_tokens` with `token_sha256 bytea UNIQUE`,
  `display_prefix`, `expires_at`, `last_used_at`, `revoked_at`, and a composite foreign key
  `(org_id, user_id) → users (org_id, id)`.
- `ops::api_tokens` (`crates/server/src/ops/api_tokens.rs`): `horae_pat_` plus 32 random bytes in
  base64url from `OsRng`; SHA-256 of the whole string stored; `Secret` prints only its prefix in
  `Debug`. `authenticate(pool, token)` runs before the transaction, checks prefix, digest,
  revocation, expiry and that the owner is active, stamps `last_used_at` at most once a minute and
  returns `Caller::token(org_id, user_id, token_id)`.
- Issue and revocation are server functions (`crates/horae/src/server_fns/api_tokens.rs`) with
  pages at `/settings/api-tokens` and `/admin/api-tokens`.

### 1.2 Channels and the actor

- `ops::Channel` (`crates/server/src/ops/actor.rs`) is `Session`, `Token { token_id }`, `Operator`,
  `System`, and is `Copy`. `Actor::load_with` treats `Session` and `Token` as person channels and
  loads the stored grants; it has no notion of a narrowed grant set.
- `Channel` is matched in: `ops::people::add` (a token adds no one), `ops::api_tokens::identity`
  (`/api/v1/me`), the limiter middleware (`horae_api::rate_limit::limit`, keyed by `token_id`), the
  Harvest API's `by_token` and its write guard (`time_entry_writes::write` refuses
  `Channel::Session`).

### 1.3 How the two APIs authenticate

- `/api/v1`: `horae_api::token::ApiCaller` reads `Authorization: Bearer`, calls
  `ops::api_tokens::authenticate`, and the `authenticate` middleware in `crates/api/src/v1.rs` puts
  the `Caller` in the request extensions; then `rate_limit::limit`. Routes are registered through
  `utoipa-axum` (`token_routes()`), each with `security(("token" = []))`.
- Harvest API (`crates/harvest-api/src/auth.rs`): token from the header or the `access_token`
  query parameter, else the session cookie for reads. `Harvest-Account-Id` (header or
  `account_id`) must equal the organization's `api_account_id`. The route table in
  `crates/harvest-api/src/lib.rs::serve` has 14 entries plus `.json` twins.

### 1.4 Sign-in

- `crates/api/src/auth/oidc.rs`: `/auth/oidc/start` stores CSRF, PKCE verifier and nonce in the
  session; `/auth/callback` validates, resolves the user, calls `session.cycle_id()` and always
  redirects to `/`. `dev.rs` (`DEV_LOGIN=1`) also redirects to `/`.
- `login_redirect_guard` sends signed-out HTML page loads outside the public prefixes to
  `/auth/login` and forgets where they were going. There is no return-to.
- Session cookie: Postgres store, `SameSite=Lax`, `Secure` in production, expires with the browser
  session. `cycle_id` keeps the session's data and changes only its id.

### 1.5 Audit

- `audit_log` (baseline: `id, org_id, actor_user_id, action, entity_type, entity_id, data jsonb, created_at`) is written today only by the Harvest client settings
  (`importers/harvest/client_settings.rs`). The audit page (`/admin/audit`,
  `ops::permissions::audit::page`) reads `permission_change_receipts` only. Neither API token
  creation nor revocation is audited today.

### 1.6 Headers

- No route sets `X-Frame-Options` or `Content-Security-Policy` (`rg` over `crates/` and `nix/`).

### 1.7 Dependencies already present

`sha2`, `hmac`, `subtle`, `base64`, `chacha20poly1305` (for its `OsRng`) and `url` are in
`Cargo.lock`. `openidconnect` is used as an OIDC client; it has no server side.

## 2. Planning decisions

### R-1 Hand-written authorization server over the existing pieces

- **Decision**: implement the endpoints in Horae: operations in `horae-server::ops::oauth`, HTTP
  in `horae-api::oauth`, pages and server functions in `horae`. No OAuth server library.
- **Rationale**: the server needs four endpoints, two grants, one PKCE method and one token format.
  The hard parts (randomness, hashing, constant-time comparison, token lookup) already exist for
  personal tokens. Rust's OAuth server crates (`oxide-auth`) bring their own storage and frontend
  traits that would sit beside `ops` and `Actor` instead of going through them (Principle IV), and
  support flows the spec forbids.
- **Alternatives**: `oxide-auth` (above); an external server such as Keycloak or Ory Hydra in
  front of the APIs (a second service to deploy for every self-hoster, and consent would live
  outside Horae's interface, against FR-024); reusing the OIDC provider (it authenticates people
  for Horae, it does not know Horae's scopes or permissions).

### R-2 Endpoints and paths

- **Decision**:

  | Path | Served by | Purpose |
  | --- | --- | --- |
  | `GET /oauth/authorize` | `horae-api` | validates the request, keeps it in the session, redirects to the approval page |
  | `GET /oauth/consent?request=<id>` | Dioxus route `OAuthConsent` | the approval page |
  | `POST /oauth/token` | `horae-api` | code exchange and refresh |
  | `POST /oauth/revoke` | `horae-api` | RFC 7009 revocation |
  | `GET /.well-known/oauth-authorization-server` | `horae-api` | RFC 8414 metadata |

  The issuer identifier is the instance's public base URL (`HORAE_PUBLIC_URL`, contracts/configuration.md).

- **Rationale**: `/oauth/*` keeps the endpoints out of `/api/` (reserved for the versioned API,
  022 FR-052) and outside `/auth/*` (sign-in to Horae). Splitting authorize from consent lets the
  request be validated before any page renders, so an unknown client or redirect never reaches
  the SPA, and keeps the request's parameters out of the page URL.

- **Alternatives**: the authorize endpoint as the Dioxus route itself (validation in a server
  function after the page loads; an error page would flash the shell, and the request would have
  to survive the sign-in round trip in the URL); `/api/v1/oauth/*` (ties the authorization server
  to one API version although it serves two APIs).

### R-3 Keeping the authorization request across sign-in

- **Decision**: `GET /oauth/authorize` stores the validated request in the session under
  `oauth_requests` (a map from a random 128-bit request id to the request, at most 5 entries, each
  expiring after 10 minutes) and redirects to `/oauth/consent?request=<id>`. When the person is
  signed out, `login_redirect_guard` stores the path and query in the session as `return_to`
  before redirecting to `/auth/login`, **only for paths under `/oauth/`**, and the OIDC callback
  and the development login redirect to `return_to` when present instead of `/`. The value is
  re-checked to start with `/oauth/` and contain no scheme or `//` before use.
- **Rationale**: the session already carries the OIDC flow secrets the same way, survives
  `cycle_id`, and is server side, so the request never sits in a URL the provider sees. Limiting
  `return_to` to `/oauth/` keeps sign-in's behaviour for everything else unchanged (C-5) and rules
  out an open redirect.
- **Alternatives**: a table of pending requests (a write by an unauthenticated route, and cleanup
  work, for data that lives 10 minutes); a general return-to for every page (a UX change outside
  this feature; it can follow on its own).

### R-4 Approval page and its server functions

- **Decision**: `OAuthConsent` is a Dioxus page outside the main shell (no sidebar), rendered with
  SSR like the rest of the app since #451. It calls `oauth_consent_details(request)` (server
  function: reads the pending request from the session, loads the client and the actor, returns
  the app name, built-in flag, scopes, return host or scheme, person and organization) and offers
  Allow and Deny, which call `approve_oauth_request(request)` and `deny_oauth_request(request)`.
  Both remove the request from the session and return the redirect URL; the page then navigates
  the browser to it with a full navigation. Text comes from a new `oauth` section of the catalog.
  Responses under `/oauth/` carry `X-Frame-Options: DENY` and
  `Content-Security-Policy: frame-ancestors 'none'`, added by a layer in the binary.
- **CSRF**: server functions are same-origin POSTs with a non-form content type, so a cross-site
  form cannot call them, and the session cookie is `SameSite=Lax`. The request id, bound to the
  session and unguessable, is the second check: a forged call without it finds nothing.
- **Alternatives**: a server-rendered page in `horae-api` like `/auth/login` (outside the catalog
  and DESIGN.md components, against FR-024); a plain HTML form post (would need its own CSRF token).

### R-5 Credentials

- **Decision**: one generator for every OAuth credential, shared with personal tokens: a prefix plus
  32 bytes from `OsRng` in base64url; SHA-256 of the whole string stored and looked up by its
  unique index.

  | Credential | Prefix | Lifetime |
  | --- | --- | --- |
  | Access token | `horae_oat_` | 1 hour |
  | Refresh token | `horae_ort_` | single use; idle 30 days, absolute 1 year (D-2) |
  | Authorization code | `horae_oac_` | 60 seconds, single use |
  | Client secret | `horae_ocs_` | until replaced |

  Client ids are `horae_app_` plus 16 random bytes in base64url (public, not secret, not
  sequential). The built-in Android client's id is the fixed string `horae-android` (D-1).
  A confidential client's secret is compared by digest with `subtle::ConstantTimeEq`.

- **Rationale**: personal tokens already use this scheme (022 R-6), and SHA-256 is enough for
  256-bit random secrets: there is nothing to brute-force, so a slow password hash buys nothing
  (SC-004).

- **Alternatives**: JWT access tokens (revocation would wait for expiry, against SC-006, and the
  lookup they save is one indexed query); Argon2 for client secrets (for low-entropy secrets
  people choose; these are generated).

### R-6 Scopes and the narrowed actor

- **Decision**:

  - `horae-core::permissions::oauth` defines `Scope` (`TimeRead`, `TimeWrite`, `CatalogRead`), a
    `Copy` set `ScopeSet` (a bit set), parsing and printing of the space-separated `scope`
    parameter, and `ScopeSet::ceiling() -> PermissionSelection`:

    | Scope | Permissions in the ceiling |
    | --- | --- |
    | `time:read` | `TimeReadOwn`, `TimeReadManaged`, `TimeReadAll` |
    | `time:write` | `time:read`'s, plus `TimeWriteOwn`, `TimeWriteManaged`, `TimeWriteAll` |
    | `catalog:read` | `ProjectReadManaged`, `ProjectReadAll`, `ClientReadAll`, `TaskReadAll`, `PeopleReadManaged`, `PeopleReadAll` |

  - `Channel::OAuth { grant_id, token_id, scopes: ScopeSet }` is added. `Actor::load_with` treats
    it as a person channel and stores `grants ∩ scopes.ceiling()` as the actor's grants. The
    Administrator identity is never carried by an OAuth actor: `administrator()` answers `false`
    for this channel.

  - Each API endpoint declares its scope (R-7). The check is at the endpoint; the narrowed grants
    are the second line, for operations reached through it that check permissions themselves.

  - Denied by default: an OAuth actor loads with no authority, answering like an inactive person,
    until the adapter calls `Actor::unlock_scope` with its endpoint's scope, which fails unless
    the token holds it. Operations whose only guard is "an active person" (profile, preferences,
    and the like) therefore refuse OAuth callers unless an endpoint with a declared scope reaches
    them. Managing personal tokens, apps and authorizations also requires `Channel::Session`
    (`Actor::require_session`), so no token, personal or OAuth, can mint or revoke another.

- **Rationale**: FR-063 asks that scopes only narrow. Intersection makes it true by construction:
  whatever an operation checks, it checks against a subset of the person's grants. Some reads need
  no permission at all (any active person sees the people list for pickers), so the endpoint check
  is still needed. Dropping the Administrator identity keeps "no scope covers administration"
  (FR-063) true for every operation that asks `administrator()` rather than a permission. Putting
  the mapping in `horae-core` keeps it pure and testable (Principle II): a proptest checks that
  `grants ∩ ceiling ⊆ grants` and that implied permissions stay closed.

- **Alternatives**: scopes checked at endpoints only (an operation reached through an allowed
  endpoint would use the full grants); per-permission scopes (dozens of entries on a consent page
  nobody reads); `offline_access` as a scope for refresh tokens (every client here needs a refresh
  token, so it would be noise).

### R-7 Scope enforcement from the OpenAPI document

- **Decision**: `/api/v1` operations declare `security(("token" = []), ("oauth2" = ["time:read"]))`
  in their `#[utoipa::path]`. At start-up, `v1::router` reads the generated document and builds a
  map from method and path template to required scope; a route layer looks up the request's
  `MatchedPath` and refuses an OAuth caller without the scope with 403 `insufficient_scope`. An
  operation with no `oauth2` requirement refuses OAuth callers (FR-062). The Harvest API's route
  table gains a scope column per entry and the same check in `AuthUser`.
- **Rationale**: one declaration feeds both the document (FR-073) and the check, so they cannot
  disagree. A unit test walks the document and fails if a token route has no `oauth2` entry and
  is not on an explicit allow-list (only `openapi.json`, which is public).
- **Alternatives**: a `RequireScope<S>` extractor per handler (the document and the check could
  drift); scopes on the route builder (utoipa would not see them).

### R-8 Refresh token rotation and reuse

- **Decision**: refresh tokens form a chain per grant. Exchanging one marks it `used_at`, issues a
  new access and refresh token, and stamps the grant's `last_used_at`. Presenting a refresh token
  whose `used_at` is set ends the grant (`revoked_at`, reason `refresh_reuse`), which kills every
  token of it, and writes an audit record. The exchange locks the refresh token row
  (`FOR UPDATE`), so two concurrent uses of one token cannot both succeed: the second sees
  `used_at` and counts as reuse.
- **No grace period** in this feature: a client that lost the response must sign in again
  (spec Edge Cases). A grace window (accept the immediately previous token once within N seconds,
  returning the same successor) avoids that on bad networks but means storing the successor's
  secret or re-issuing it, and makes reuse detection weaker. It is a later addition if the Android
  app sees it in practice.
- **Rationale**: RFC 9700 §4.14.2 requires sender-constrained or rotated refresh tokens for public
  clients; rotation with family revocation is what native libraries expect.

### R-9 Code exchange

- **Decision**: a code row stores the digest, grant, client, exact redirect URI, `code_challenge`,
  `expires_at` (60 s) and `used_at`. The exchange locks it, checks client, redirect URI, expiry
  and `BASE64URL(SHA256(code_verifier)) == code_challenge` (constant time), verifier length 43 to
  128 characters from the unreserved set (RFC 7636 §4.1), then marks it used and issues the first
  tokens. A used code ends the grant it created (FR-031); the grant is the unit of revocation, so
  this revokes exactly the tokens issued from it.

### R-10 Grants: one per approval

- **Decision**: each Allow creates a new grant (person, client, scopes, approval time). Grants are
  not merged per client.
- **Rationale**: with consent on every request (FR-025), merging would let a second, narrower
  approval silently keep the first one's wider scopes alive under one name. Separate grants also
  map to devices: two phones, two grants, each revocable alone.
- **Display**: My settings groups grants by app and shows each with its approval time and last use.

### R-11 Threat model (RFC 9700, RFC 8252, RFC 6819)

| Threat | Countermeasure | Spec | Test |
| --- | --- | --- | --- |
| Code interception by another app on the device (shared custom scheme) | PKCE S256 required for every client; verifier never leaves the app | FR-021, FR-041 | exchange without or with a wrong verifier fails |
| PKCE downgrade | `plain` and missing challenge refused; metadata lists only `S256` | FR-021, FR-095 | |
| Code replay | single use, 60 s, reuse ends the grant | FR-030, FR-031 | |
| Code injection (a code from another session) | code bound to client, redirect URI and challenge | FR-030 | code from client A redeemed by B fails |
| Open redirector, redirect URI manipulation | exact matching, loopback port only; errors on Horae's page for unmatched URIs | FR-010, FR-011, FR-022 | prefix, suffix, case, extra query, other port on non-loopback, `localhost` |
| CSRF on the client's redirect | client's `state` echoed; PKCE also binds | FR-026 | |
| Mix-up between servers | `iss` in every authorization response (RFC 9207); `authorization_response_iss_parameter_supported` in metadata | FR-022, FR-026, FR-095 | |
| Clickjacking of the approval page | `frame-ancestors 'none'` and `X-Frame-Options: DENY` | FR-024 | |
| Forged approval (CSRF on Allow) | server function, `SameSite=Lax`, session-bound request id | FR-025 | Allow without the session's request id fails |
| Client impersonation of a public client | consent every time, app name as registered, built-in vs organization label | FR-024, FR-025 | |
| Refresh token theft | rotation, reuse ends the grant, idle and absolute expiry | FR-043, FR-044 | |
| Access token theft | 1 hour lifetime, header only (no query, D-5), revocation immediate | FR-040, FR-070, FR-071 | query parameter refused |
| Token leakage through logs | secrets never logged; `Debug` prints prefixes; `Cache-Control: no-store` | FR-046, FR-051 | |
| Brute force at the token endpoint | 256-bit secrets; per-client rate limit | FR-050, FR-081 | |
| Database theft | only digests stored | FR-050 | |
| Privilege escalation through scopes | intersection with the person's grants; no administrator identity | FR-063 | proptest and per-scope endpoint tests |
| Implicit or password grant downgrade | not offered | FR-020, FR-040 | |
| Lost device | person or administrator revokes the grant | FR-091, FR-092 | |
| Deactivated person | `authenticate` and `Actor::load` both check `active` | FR-052 | |
| Disabled app | `authenticate` checks the client's `enabled` | FR-003 | |

Residual risks: a malicious app that claims the built-in client's scheme on a device can start a
flow and, if the person clicks Allow on a page that names Horae's own app, get tokens. The verifier
protects codes from *other* flows, not a flow the attacker starts itself. RFC 8252 §8.6 accepts
this for private-use schemes; claimed `https` redirects (App Links) close it but cannot be
verified for arbitrary self-hosted hosts (spec D-1). The approval page names the app and the
person, which is the mitigation.

### R-12 Token endpoint client authentication

- **Decision**: confidential clients authenticate with `client_secret_basic` (HTTP Basic, RFC 6749
  §2.3.1) or `client_secret_post`. Public clients send `client_id` in the body and no secret; a
  public client that sends a secret is refused with `invalid_client`.
- **Alternatives**: `private_key_jwt` (no client in view needs it).

### R-13 Errors at the protected resources

- **Decision**: on `/api/v1`, an invalid OAuth token is 401 `unauthenticated` with the problem body,
  as for personal tokens, plus `WWW-Authenticate: Bearer error="invalid_token"`. A missing scope
  is 403 with problem code `insufficient_scope` and `WWW-Authenticate: Bearer error="insufficient_scope", scope="time:write"`. The Harvest API answers in its own error shape
  with the same statuses and header.
- **Rationale**: RFC 6750 §3.1. Adding a header and a problem code is backward compatible within v1.

### R-14 Rate limits

- **Decision**: the limiter (`horae_api::rate_limit`) is keyed by a `Uuid`. For OAuth callers the
  key is the `grant_id`, so rotating access tokens does not reset the window (FR-080). The token
  and revocation endpoints get their own limiter instance keyed by client id, default 30 requests
  per minute (`HORAE_OAUTH_TOKEN_RATE_LIMIT`), so a busy app does not starve its own API calls.
- **Alternatives**: per IP (people behind one NAT share it; the client id is what an attacker must
  name anyway).

### R-15 Audit records

- **Decision**: `ops::oauth` writes to the existing `audit_log` table, `entity_type` `oauth_client`
  or `oauth_grant`, `action` one of `client_registered`, `client_updated`, `client_enabled`,
  `client_disabled`, `client_deleted`, `client_secret_replaced`, `grant_approved`,
  `grant_revoked`, `refresh_reused`, `code_reused`, with `data` holding names, scopes and the
  revocation reason, never a credential. Showing them on `/admin/audit` (D-4) adds an "Apps"
  kind to `AuditFilter` and a second source in `ops::permissions::audit::page`, merged by time.
  The page keeps its existing permission check, so only people who may read the log see them.
- **Rationale**: `audit_log` exists for exactly this and is already written by the Harvest
  client settings; permission receipts are a separate, typed history that should not take
  unrelated events.

### R-16 Constitution

- **Decision**: Principle IV already allows "an OAuth2 access token that Horae issued, stored only as
  a hash, and bound to one person in one organization" on the token-authenticated APIs (7.0.0).
  What it does not allow is issuing and revoking tokens outside server functions, which the token
  and revocation endpoints must do. A MINOR amendment adds a bounded clause for them
  ([constitution-amendment.md](constitution-amendment.md), D-3). The maintainer approved the
  text on 2026-11-22; PR A applies it, and this feature's planning does not edit the
  constitution.

### R-17 Harvest-shaped OAuth (021 US6)

- **Decision**: not in this feature (D-6). Harvest OAuth clients can use the standard
  endpoints once their identity host and client id are changed, which they need anyway (021
  OQ-1). What they lack is Harvest's account list, which would be one read endpoint under the
  Harvest API, added later without touching the authorization server.

### R-18 Discovery and the OpenAPI document

- **Decision**: the metadata document is static per instance (built from `HORAE_PUBLIC_URL` and the
  scope list) and public. The v1 OpenAPI document gains an `oauth2` security scheme with the
  `authorizationCode` flow (`authorizationUrl`, `tokenUrl`, `refreshUrl`, the three scopes). Its
  URLs are relative (`/oauth/authorize`), since the document is served by each instance.
- **Risk**: `oasdiff breaking` may treat a new alternative security requirement on existing
  operations as a change. Adding an alternative (OR) keeps every existing request valid, so it is
  not breaking; the first task of PR F checks oasdiff's verdict and, if it flags it, records the
  baseline refresh as the deliberate exception.

### R-19 How Harvest's OAuth differs, and why Horae does not follow it

Harvest's public API documentation, as summarized in
[021 research](../021-harvest-api-compat/research.md) (*Authentication*), describes this:

| Harvest | Horae (this feature) | Why Horae differs |
| --- | --- | --- |
| Authorization code access tokens last 14 days | 1 hour (D-2) | A stolen access token works until it expires or someone revokes the grant, and a theft is rarely noticed. With refresh tokens available, a short access token costs the app one refresh an hour. |
| A refresh token comes with the access token, with no rotation described | Refresh tokens rotate, and a replay ends the grant (D-2, FR-043) | Public clients cannot keep a secret, so RFC 9700 §4.14.2 requires rotation or sender-constrained tokens for them. Rotation is the one that needs no client key management. |
| The implicit flow is offered | Not offered (FR-020) | The implicit grant puts the access token in the redirect URI, where it leaks through history and logs; RFC 9700 §2.1.2 says not to use it. |
| Scopes name accounts: `harvest:{ACCOUNT_ID}`, `harvest:all`, `all` | Scopes name access to Horae's data (`time:read`, `time:write`, `catalog:read`), and a token is bound to one person in one organization | Horae has one organization per instance today, so an account scope would select nothing. Harvest's scopes do not narrow what a token may do inside an account; Horae's do (FR-060 to FR-063). |
| Authorize, token and account-list endpoints on a separate identity host (`id.getharvest.com`) | The endpoints are on the instance itself, under `/oauth/` (R-2) | A self-hosted instance is its own issuer (`HORAE_PUBLIC_URL`). A second host would add deployment work and gain nothing for a single instance. |
| No PKCE | PKCE `S256` required for every client, public and confidential, with no exception (D-7, FR-021) | PKCE binds the code to the app that started the flow, which protects against code interception and injection (RFC 9700 §2.1.1, RFC 8252 §8.1). Requiring it for confidential clients as well costs them one parameter and keeps a single rule. |

A client built for Harvest's OAuth therefore does not work against Horae unchanged: besides the
host and the client id it would need PKCE. If such clients are ever supported, 021's delivery
step 7 (OAuth2 provider) decides how. That decision must not relax D-7 on Horae's own endpoints.

### R-20 Alternatives weighed for the maintainer's decisions

The spec's planning draft listed D-1 to D-6 as open questions. The options not chosen:

- **D-1**: a built-in client enabled by default (works on install, but leaves an enabled public
  client on every instance, including those that never use the app); no built-in client, with each
  administrator registering the app by hand.
- **D-2**: idle expiry of 30 days with no absolute limit (an app used monthly never asks again);
  idle 14 days and absolute 90 days (people re-approve each quarter).
- **D-3**: no amendment, with the deviation repeated in the Complexity Tracking of every plan that
  touches the token or revocation endpoints.
- **D-4**: record the events now and show them in the audit log in a later change.
- **D-5**: accept OAuth tokens in the Harvest API's `access_token` query parameter, as for
  personal tokens.
- **D-6**: an account-list endpoint in Harvest's shape under the Harvest-compatible API in this
  feature.
