# Feature Specification: OAuth 2.0 with PKCE for Horae's APIs

**Feature Branch**: `spec/oauth-pkce`

**Created**: 2026-11-22

**Status**: Ready for implementation. The maintainer settled the open questions on 2026-11-22
(*Decisions* D-1 to D-7); requirements that follow from one cite it.

**Input**: User description: "OAuth 2.0 authorization code with PKCE for Horae's APIs (`/api/v1`
and the optional Harvest-compatible API), so native and third-party clients act for a person
without pasted personal access tokens."

Builds on [022 Public API `/api/v1`](../022-public-api-v1/spec.md), whose FR-053 left OAuth2 for
third-party applications to a later, backward-compatible addition, and on
[021 Harvest API v2 compatibility](../021-harvest-api-compat/spec.md), whose User Story 6 and
FR-060 to FR-062 sketched Horae as an OAuth2 provider for Harvest clients. This feature replaces
021's sketch with one authorization server for both APIs (see *Relation to 021*).

## Background

Today a program acts for a person in Horae only with a personal access token: the person creates
it in My settings, copies it and pastes it into the program. That works for a script on the
person's own machine. It does not work for an app installed from a store, which cannot ask
people to paste a 50-character secret on a phone, and it is the wrong tool for a third-party
service, which should never see a credential that lasts as long as the person wants and covers
everything they can do.

The first client in view is a native Android app over `/api/v1`. A study of how a comparable
commercial time tracker's Android app signs in (`.scratch/ESTUDIO-HARVEST-ANDROID.md`) found a
store app that opens the browser for single sign-on, keeps its tokens in the platform keystore
and refreshes them, and recommended that Horae offer the authorization code flow with PKCE to
public clients before anything else. Horae's sign-in already goes through an external OIDC
provider, so the app cannot collect a password either: it must send the person to Horae's own
sign-in in the browser and get back a token.

## Constraints (maintainer decisions, before this specification)

- **C-1 Two APIs, one credential system.** OAuth access tokens authenticate on `/api/v1` and, when
  it is enabled, on the Harvest-compatible API (`/harvest/v2`, `/v2`), as personal access tokens do
  (022 FR-008).
- **C-2 Harvest compatibility stays optional.** It lives in its own crate behind a runtime switch,
  off by default (022 C-5). Nothing in this feature turns it on or depends on it.
- **C-3 `/api/v1` stays English** and keeps its versioning rule: within v1, only additions
  (022 FR-050).
- **C-4 The audit log is for power users only.** Members never see it or a history of their own.
- **C-5 Sign-in does not change.** People keep signing in to Horae through the configured OIDC
  provider (or the development login). Horae does not become an identity provider and does not
  gain passwords.
- **C-6 Mutations go through shared operations** (constitution IV). A token never grants more than
  its owner could do in the web app at the moment of the request.

## Decisions (maintainer, 2026-11-22)

The planning drafts listed these as open questions with recommendations. The maintainer chose
the following; the alternatives that were weighed are in research R-20.

- **D-1 Client registration for the store app.** A store app is built once and used against many
  self-hosted instances, and App Links cannot be verified for hosts unknown when it is built, so
  its redirect is a private-use URI scheme or a loopback address. Horae ships one built-in public
  client for its own Android app, `horae-android`, with a fixed redirect URI. It appears in the
  administrators' app list and stays disabled until an administrator enables it (FR-005).
- **D-2 Token lifetimes.** Access tokens last one hour. Refresh tokens rotate, stop working after
  30 days without use, and stop one year after the authorization was approved (FR-040, FR-043,
  FR-044).
- **D-3 Constitution.** Principle IV says "token issue and revocation are themselves mutations and
  go through server functions", and the token and revocation endpoints are called by programs with
  no session. The maintainer approved the bounded clause in
  [constitution-amendment.md](constitution-amendment.md) (7.1.0): those endpoints may run the
  shared OAuth operations, authorized by a code with its PKCE verifier, a refresh token or a client
  secret, and nothing else. PR A of the plan applies it.
- **D-4 Audit log.** App registrations, approvals, revocations and code or refresh token reuse are
  recorded (FR-100) and shown in this feature, as an "Apps" kind in the audit log's filter with one
  sentence per event. Only people who may read the log see them (FR-101, C-4).
- **D-5 Query parameter.** OAuth access tokens are accepted in the `Authorization` header only, on
  both APIs (RFC 9700). On the Harvest-compatible API the `access_token` query parameter keeps
  working for personal access tokens and refuses OAuth tokens (FR-071).
- **D-6 No Harvest-shaped OAuth endpoints.** This feature offers only the standard endpoints.
  Harvest OAuth clients already need a source change for their embedded client id and identity
  host, and the standard endpoints serve them once they point at Horae; Harvest's account list is
  the one piece they would miss, and 021 US6 stays a later step on top of this feature.
- **D-7 PKCE for every client.** Every authorization request carries a PKCE `S256` challenge,
  from public and confidential clients alike, with no exception (FR-021). Harvest's OAuth does not
  use PKCE; if clients built for Harvest's OAuth are ever supported, 021's step 7 decides how, and
  this rule does not bend for them. Research R-19 records how Harvest's public auth documentation
  differs.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A person signs in to the Android app through the browser (Priority: P1)

A person installs Horae's Android app and types their Horae address. The app opens the
browser at Horae. If they are not signed in, they sign in as they do on the web, through their
organization's identity provider, and come back to a Horae page that names the app, says which
account it will act as and what it will be able to do, in their own words. They approve. The
browser hands control back to the app, which can now read their week and start a timer. They
never see or copy a token.

**Why this priority**: It is the reason for the feature (the study's first server gap) and the
smallest slice that a store app can use. It needs every core part: the authorization page, the
token endpoint, PKCE, a public client and token checks on `/api/v1`.

**Independent Test**: With a test client that behaves like a native app (a loopback redirect or a
private-use scheme, PKCE S256, no secret), run the flow against a running Horae, approve, exchange
the code, call `/api/v1/me` and `/api/v1/time_entries`, and compare with the web app.

**Acceptance Scenarios**:

1. **Given** an enabled public client and a signed-out person, **When** the app opens the
   authorization address, **Then** the person is asked to sign in, and after signing in lands back
   on the approval page for that same request, not on the timesheet.
1. **Given** the approval page, **When** the person looks at it, **Then** it shows the app's name as
   the administrator registered it, the person's name and organization, the access it asks for in
   plain words, a note that Horae will send them back to the app, and two actions: Allow and
   Deny.
1. **Given** the person allows, **When** the browser follows the redirect, **Then** the app receives
   a one-time code and its own `state` back, and exchanging the code with its PKCE verifier gives
   it an access token, a refresh token, their lifetimes and the granted scopes.
1. **Given** the person denies, **When** the browser follows the redirect, **Then** the app receives
   an `access_denied` error and no code.
1. **Given** an access token, **When** the app calls `/api/v1`, **Then** it sees and may do exactly
   what the person may do in the web app, narrowed to the granted scopes.
1. **Given** a request without a PKCE challenge, with the `plain` method, or with a redirect URI
   not registered for the client, **When** it reaches the authorization address, **Then** no code
   is ever issued. A bad redirect URI or unknown client is reported on Horae's own page and the
   browser is not sent anywhere.

______________________________________________________________________

### User Story 2 - The app stays signed in, and a stolen refresh token stops working (Priority: P1)

The app keeps working for weeks without asking the person to sign in again. Each hour its access
token expires and it gets a new one with its refresh token, receiving a new refresh token each
time. When someone copies an old refresh token off a device and uses it after the app has already
used it, Horae notices the reuse and ends that whole authorization, so both the thief and the
app must start again.

**Why this priority**: A phone app that asks for sign-in every hour is unusable, and refresh tokens
of public clients are the main target of the native-app threat model (RFC 9700, RFC 8252). Without
rotation and reuse detection, User Story 1 is not safe to ship.

**Independent Test**: Refresh three times and check that each refresh token works once; replay the
first one and check that every token of the authorization stops working; let a refresh token go
idle past its expiry and check that it is refused.

**Acceptance Scenarios**:

1. **Given** a valid refresh token, **When** the app exchanges it, **Then** it receives a new access
   token and a new refresh token, and the old refresh token no longer works.
1. **Given** a refresh token that has already been exchanged, **When** anyone presents it again,
   **Then** the request is refused and every access and refresh token of that authorization stops
   working at once.
1. **Given** an authorization unused for longer than the idle expiry, or older than the absolute
   limit (D-2), **When** its refresh token is used, **Then** it is refused and the app must send the
   person through the browser again.
1. **Given** the person is deactivated, **When** the app uses any of its tokens, **Then** they are
   refused; reactivation restores tokens that have not expired.
1. **Given** a refresh request that asks for fewer scopes, **When** it is exchanged, **Then** the new
   access token carries only those; asking for more than were granted is refused.

______________________________________________________________________

### User Story 3 - An administrator registers an app (Priority: P1)

An administrator opens the administration area and adds an app: its name, whether it is a public
app (installed on people's devices, no secret) or a confidential one (a server that keeps a
secret), its redirect addresses and the access it may ask for. For a confidential app, Horae shows
the secret once. Later the administrator edits the redirect addresses, issues a new secret, or
disables the app, which stops all its tokens.

**Why this priority**: No flow can start without a registered client, and Horae has no dynamic
registration (FR-012). The built-in Android client of D-1 is one entry in the same list.

**Independent Test**: Register a public and a confidential app, run User Story 1 with each,
disable one and see its tokens refused; as a member, try to open the page and see it refused.

**Acceptance Scenarios**:

1. **Given** an administrator, **When** they register an app with a valid name, type, redirect
   addresses and scopes, **Then** it appears in the list with its client id, and a confidential app's
   secret is shown once, in full, and never again.
1. **Given** a redirect address with a fragment, a wildcard, plain `http` to a host that is not a
   loopback address, `localhost` instead of a loopback IP, or a private-use scheme that is not in
   reverse domain form, **When** the administrator saves it, **Then** it is refused with a reason.
1. **Given** an enabled app with active authorizations, **When** the administrator disables it,
   **Then** every token of that app is refused from the next request and no new authorization
   can start; enabling it again does not bring back authorizations that were revoked.
1. **Given** a confidential app, **When** the administrator issues a new secret, **Then** the new one
   is shown once and the old one stops working.
1. **Given** a member, **When** they try to list or register apps, **Then** it is refused.

______________________________________________________________________

### User Story 4 - A person sees and revokes the apps they authorized (Priority: P2)

In My settings, next to API tokens, a person sees every app they have allowed: its name, the
access granted, when they allowed it and when it was last used. They revoke one they no longer
use, and it stops working at once. An administrator sees the same for everyone, per app, and can
revoke any authorization, for instance when a phone is lost.

**Why this priority**: People need to undo an approval without asking an administrator, and a lost
device must be cut off quickly. The flows work without it, so it follows the P1 stories.

**Independent Test**: Authorize two apps, revoke one from My settings and see its tokens refused
while the other keeps working; as an administrator, revoke another person's authorization.

**Acceptance Scenarios**:

1. **Given** a person with two authorizations, **When** they open the apps page in My settings,
   **Then** they see both with name, scopes, approval time and last use, and no token or secret.
1. **Given** an authorization, **When** its person revokes it, **Then** its access and refresh
   tokens are refused from the next request.
1. **Given** an administrator, **When** they open an app in the administration area, **Then** they
   see its authorizations with each person's name, scopes, approval time and last use, and can
   revoke any of them.
1. **Given** a member, **When** they try to revoke another person's authorization, **Then** it is
   refused as not found.
1. **Given** an app that revokes its own refresh or access token through the revocation endpoint,
   **When** it signs out, **Then** that authorization ends as if the person had revoked it.

______________________________________________________________________

### User Story 5 - A Harvest-dialect tool uses an OAuth token (Priority: P2)

An operator has turned on the Harvest-compatible API. A person connects a tool that speaks
Harvest's dialect and was pointed at Horae's authorization server. The tool gets an access token
through the same flow and calls `/harvest/v2/time_entries` with it in the `Authorization` header
and the `Harvest-Account-Id` header. It works exactly as with a personal access token, within its
scopes.

**Why this priority**: C-1. The Harvest-compatible API is off by default, so fewer deployments
need it than `/api/v1`.

**Independent Test**: With the API on, call a read and a write with an OAuth token of each scope
set, and compare with the same calls made with a personal token.

**Acceptance Scenarios**:

1. **Given** the Harvest-compatible API on and an access token with time read access, **When** a
   tool lists time entries, **Then** it receives what the person may see, as with a personal token.
1. **Given** an access token without time write access, **When** a tool creates a time entry,
   **Then** it is refused for insufficient scope and nothing changes.
1. **Given** an OAuth access token in the `access_token` query parameter, **When** it reaches the
   Harvest-compatible API, **Then** it is refused (D-5).
1. **Given** the Harvest-compatible API off, **When** a request with an OAuth token reaches its
   paths, **Then** it answers 404 as for any unknown path.

______________________________________________________________________

### User Story 6 - Power users trace what apps did (Priority: P3)

An administrator investigating a strange entry sees in the audit log that a person authorized an
app on a given day, that a refresh token of another app was replayed and the authorization ended,
and who disabled an app. Members see none of this.

**Why this priority**: C-4. It helps after an incident but no flow depends on it.

**Independent Test**: Run User Stories 1 to 4, then read the audit records as an administrator and
as a member.

**Acceptance Scenarios**:

1. **Given** app registrations, approvals, revocations and a refresh token reuse, **When** an
   administrator reads the audit log, **Then** each appears once, with who acted, which app and
   which person, and no token or secret (D-4).
1. **Given** a member, **When** they look for any of these records, **Then** there is no view or
   link that shows them.

______________________________________________________________________

### Edge Cases

- **Code replay.** A code works once. A second exchange is refused and revokes the tokens issued
  from the first one (RFC 6749 §4.1.2).
- **Code interception on the device.** Another app that registered the same private-use scheme
  can receive the code. It cannot exchange it without the PKCE verifier, which never left the real
  app.
- **Authorization request forgery.** An attacker's page that sends a signed-in person to the
  authorization address still lands on the approval page; nothing is approved without the
  person's click, and the page cannot be framed (clickjacking).
- **Mix-up.** The redirect carries `iss`, Horae's issuer identifier, so a client that talks to
  several servers can tell which one answered (RFC 9207).
- **Open redirect.** An unknown client or an unregistered redirect URI is shown as an error on
  Horae's page; Horae never redirects to an address it cannot match exactly to the client's
  registration.
- **Loopback ports.** A native app's loopback redirect may use any port: `http://127.0.0.1:{port}/path`
  and `http://[::1]:{port}/path` match the registration whatever the port (RFC 8252 §7.3). Every
  other part must match exactly.
- **The person who started is not the person who approves.** If the browser's session belongs to
  someone else, the approval page names that account and offers "Not you? Sign out"; the code is
  bound to whoever approved.
- **Signing in takes longer than the request lives.** An authorization request is kept for 10
  minutes. After that the approval page says the request expired and to start again from the
  app.
- **Consent every time.** The approval page is shown on every authorization request, even for an
  app the person already allowed, because anyone can claim a public client's identity (RFC 8252
  §8.6). Each approval creates a new authorization; the earlier ones stay until revoked or expired.
- **Scope request.** A request without `scope` asks for all the scopes the app is registered for.
  A scope the app is not registered for, or an unknown one, is refused with `invalid_scope`.
- **Scopes against permissions.** Scopes only narrow. A member granted `time:write` still writes
  only their own time; a manager's token with only `time:read` cannot write any.
- **Permission changes.** The person's permissions are read on every request (022 FR-003), so a
  permission removed today applies to every token at once; scopes never need to change.
- **A token used on the wrong API.** Access tokens work on both APIs; refresh tokens and codes work
  only at the token endpoint, and a refresh token sent as a bearer token is refused.
- **Client authentication.** A confidential client must authenticate at the token endpoint; a public
  client must not send a secret, and its code exchange is protected by PKCE alone.
- **Clock and expiry.** Expiry times are decided by the server; clients get `expires_in` in seconds.
- **Several organizations** (after multi-tenancy). An authorization belongs to one person in one
  organization; a token never acts in another (022 FR-011).
- **Losing a refresh response.** If the network drops after Horae rotated a refresh token, the app
  holds a used refresh token and the next refresh counts as reuse. The app has to send the person
  through the browser again. Research R-8 weighs a short grace period for this case.
- **Revoking twice** or revoking an unknown token at the revocation endpoint answers success
  (RFC 7009 §2.2) and reveals nothing.
- **The development login.** With `DEV_LOGIN=1` the flow works the same, signing in through the
  one-click page, so the Android app can be developed against a local stack.

## Requirements *(mandatory)*

### Functional Requirements

#### Clients

- **FR-001**: Administrators MUST be able to register, list, edit, disable, enable and delete OAuth
  clients ("apps") for their organization. An app has a name shown to people, a type (public or
  confidential), one or more redirect URIs and the scopes it may request.
- **FR-002**: A confidential app MUST have a secret, generated by Horae, shown once and stored only
  as a one-way hash. Administrators MUST be able to replace it, which ends the old one. A public app
  has no secret.
- **FR-003**: Disabling or deleting an app MUST make all its tokens fail from the next request and
  refuse new authorization requests. Deleting also removes its authorizations.
- **FR-004**: Each app MUST have a client id that is unique, public and not guessable from another.
- **FR-005**: Horae MUST ship a built-in public client for its own Android app, with a fixed client
  id and redirect URI, listed with the other apps and disabled until an administrator enables it
  (D-1). Administrators can disable it again but not delete it or edit its id, redirect URI
  or type.

#### Redirect URIs

- **FR-010**: Redirect URIs MUST be compared to the registration by exact string match, except
  for the port of a loopback redirect (FR-011). No wildcards, no prefix matching.
- **FR-011**: Allowed redirect URIs are: `https` URIs (any client); `http` only to the loopback IP
  literals `127.0.0.1` or `[::1]`, with any port at request time (public clients); and
  private-use URI schemes in reverse domain name form containing a dot, such as
  `com.example.app:/oauth` (public clients, RFC 8252 §7.1). URIs with a fragment, with `localhost`
  as host, or with user information are refused at registration.
- **FR-012**: Dynamic client registration (RFC 7591) is out of scope. Only administrators register
  apps.

#### Authorization endpoint and approval page

- **FR-020**: Horae MUST offer an authorization endpoint for the authorization code grant
  (`response_type=code`) only. The implicit grant and the resource owner password grant MUST NOT
  be offered.
- **FR-021**: Every authorization request MUST carry a PKCE `code_challenge` with method `S256`,
  for public and confidential clients alike, with no exception for any client (D-7). A missing
  challenge or the `plain` method is refused.
- **FR-022**: An unknown client, a disabled client, or a redirect URI that does not match the
  registration MUST be reported on Horae's own page without redirecting. Any other error MUST be
  returned to the client's redirect URI with an `error` code, the client's `state`, and `iss`.
- **FR-023**: A person who is not signed in MUST be sent through Horae's usual sign-in and then
  back to the same authorization request. Only a path on Horae's own host may be a return
  target. Sign-in itself does not change (C-5).
- **FR-024**: The approval page MUST be part of Horae's interface, follow DESIGN.md, take all its
  text from the interface catalog, and show: the app's registered name and whether it was built in
  or registered by the organization, the signed-in person's name and organization with a way to
  sign out, each requested scope as a plain sentence, the host or scheme Horae will return to,
  and Allow and Deny. It MUST NOT be displayable inside a frame.
- **FR-025**: The approval page MUST be shown for every authorization request (Edge Cases,
  *Consent every time*). Allow MUST require an explicit action by the signed-in person and MUST be
  protected against cross-site request forgery.
- **FR-026**: On Allow, Horae MUST create an authorization for that person, app and set of scopes,
  and return a code with the client's `state` and `iss` to the redirect URI. On Deny, it MUST
  return `access_denied`.
- **FR-027**: An authorization request MUST expire 10 minutes after it was made, unused.

#### Codes

- **FR-030**: A code MUST be single-use, expire 60 seconds after issue, and be bound to the client,
  the exact redirect URI of the request, the PKCE challenge, the person and the authorization.
- **FR-031**: A second use of a code MUST be refused and MUST revoke every token issued from it.

#### Token endpoint

- **FR-040**: Horae MUST offer a token endpoint for the `authorization_code` and `refresh_token`
  grants only. Access tokens MUST expire one hour after issue.
- **FR-041**: Exchanging a code MUST require the PKCE `code_verifier` that matches the challenge, the
  same `redirect_uri` as the request, and, for a confidential client, its authentication with its
  secret. A public client MUST NOT be accepted with a secret.
- **FR-042**: Every successful exchange MUST return a new access token and a new refresh token
  together with the token type `Bearer`, `expires_in` and the granted `scope`.
- **FR-043**: Refresh tokens MUST rotate: each one works once. Presenting a refresh token that has
  already been exchanged MUST end the whole authorization (all its access and refresh tokens).
- **FR-044**: A refresh token MUST stop working after 30 days without use or one year after the
  authorization was approved, whichever comes first (D-2). Both values are operator settings.
- **FR-045**: A refresh request MAY ask for a subset of the granted scopes, never more.
- **FR-046**: Token endpoint errors MUST use the OAuth 2.0 error response (RFC 6749 §5.2:
  `invalid_request`, `invalid_client`, `invalid_grant`, `unauthorized_client`,
  `unsupported_grant_type`, `invalid_scope`), never echo a credential, and send
  `Cache-Control: no-store`.

#### Tokens

- **FR-050**: Access tokens, refresh tokens, codes and client secrets MUST carry enough randomness
  that guessing is infeasible, MUST be stored only as one-way hashes, and MUST start with a prefix
  that tells their kind apart, so secret scanners recognise them.
- **FR-051**: Credentials MUST never be written to logs, error messages, audit details or URLs other
  than the code in the redirect (022 FR-009).
- **FR-052**: An access token MUST act as its person with their current permission state in the
  authorization's organization, narrowed by the granted scopes. It MUST stop working when it
  expires, when its authorization ends, when its app is disabled or deleted, or when its person is
  deactivated.
- **FR-053**: Each authorization MUST record when it was last used, at most once a minute.

#### Scopes

- **FR-060**: Horae MUST define these scopes, each with a sentence for the approval page:
  - `time:read`: read time entries the person may see.
  - `time:write`: create, edit, delete and run timers on time entries the person may change.
    Includes `time:read`.
  - `catalog:read`: read the projects, tasks, clients and people the person may see, and the
    projects and tasks they may track time on.
- **FR-061**: Who the token acts as (`GET /api/v1/me`, Harvest's `/users/me` and `/company`) MUST
  be readable with any granted scope.
- **FR-062**: Every operation of both APIs that accepts OAuth tokens MUST declare the scope it
  requires. A token without it MUST be refused with `insufficient_scope` (RFC 6750 §3.1). An
  operation with no declared scope MUST refuse OAuth tokens.
- **FR-063**: Scopes MUST only narrow the person's permissions: the effective authority of a
  request is what the person may do, limited to the permissions the granted scopes cover. No scope
  covers approvals, invoices, reports, rates, people or project administration, or any write
  other than time entries.
- **FR-064**: Personal access tokens are not affected by scopes; they keep the person's full
  authority on both APIs (022 FR-003).

#### Using tokens on the APIs

- **FR-070**: `/api/v1` MUST accept an OAuth access token as a bearer credential in the
  `Authorization` header, alongside personal access tokens, with the same answers for an invalid,
  expired or revoked one (401 with the problem body) and `WWW-Authenticate: Bearer` carrying
  `error="invalid_token"` or `error="insufficient_scope"`.
- **FR-071**: When enabled, the Harvest-compatible API MUST accept OAuth access tokens in the
  `Authorization` header with the same `Harvest-Account-Id` check as personal tokens. It MUST refuse
  them in the `access_token` query parameter (D-5). Its session-cookie reads are unchanged.
- **FR-072**: `/api/v1/me` MUST say, for an OAuth token, which app and which scopes it carries, as
  a backward-compatible addition (022 FR-050).
- **FR-073**: The published OpenAPI document MUST describe the OAuth flow (authorization and token
  addresses, scopes) and each operation's required scope, as an addition that keeps v1's contract
  check green.

#### Rate limiting

- **FR-080**: Requests made with OAuth access tokens MUST be rate-limited per authorization by the
  shared limiter, with the same limits as personal tokens, across both APIs (022 FR-051). Rotating
  access tokens MUST NOT reset the count.
- **FR-081**: The token endpoint MUST be rate-limited per client, so a client cannot try codes,
  verifiers or refresh tokens at speed. Over the limit it answers 429 with `Retry-After`.

#### Revocation and listing

- **FR-090**: Horae MUST offer a revocation endpoint (RFC 7009) that accepts an access or refresh
  token from the client it was issued to and ends the whole authorization.
- **FR-091**: Each person MUST be able to list and revoke their own authorizations in My settings,
  on a page next to API tokens, without seeing any token.
- **FR-092**: Administrators MUST be able to list every authorization of each app, with the person,
  scopes, approval time and last use, and revoke any of them.
- **FR-093**: Revoking an authorization MUST make its tokens fail from the next request.
- **FR-094**: Registering, editing and disabling apps, replacing a secret, approving and revoking
  authorizations from the web app are mutations and MUST go through shared operations called by
  server functions (constitution IV). The code exchange, refresh and revocation endpoints MUST go
  through shared operations too, under the amendment approved in D-3.

#### Discovery

- **FR-095**: Horae MUST publish authorization server metadata (RFC 8414) at
  `/.well-known/oauth-authorization-server`: issuer, endpoints, supported grant types, response
  types, PKCE methods (`S256` only), client authentication methods and scopes.

#### Audit

- **FR-100**: Horae MUST record who registered, edited, enabled, disabled or deleted an app, who
  replaced a secret, each authorization approved, each authorization revoked and by whom (person,
  administrator, the app itself, or Horae after a reuse), and each refresh token or code reuse.
  Records carry ids and names, never a credential.
- **FR-101**: Only people who may read the audit log MAY read these records. In this feature they
  appear in the audit log under their own kind, "Apps" (D-4).

### Key Entities

- **App (OAuth client)**: belongs to one organization; name, client id, type (public or
  confidential), hashed secret for a confidential one, redirect URIs, allowed scopes, built-in
  flag, enabled flag, who registered it and when.
- **Authorization request**: a pending request between the authorization address and the person's
  answer: client, redirect URI, scopes, PKCE challenge, `state`, expiry. Lives at most 10 minutes.
- **Authorization (grant)**: a person's approval of an app in one organization with a set of
  scopes; approval time, last use, revocation time and reason. All its tokens end with it.
- **Authorization code**: single-use, 60-second credential bound to an authorization, redirect URI
  and PKCE challenge.
- **Access token**: one-hour bearer credential of an authorization, with its scopes.
- **Refresh token**: rotating credential of an authorization; each one is used once, and a reuse
  ends the authorization.
- **Scope**: a named, fixed set of permissions an app may ask for.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person goes from tapping "Sign in" in the Android app to seeing their week in under
  one minute, without typing or copying any credential (excluding the time their identity provider
  takes).
- **SC-002**: A conforming native-app client library (one that implements RFC 8252 and PKCE, such
  as AppAuth) completes sign-in, refresh and sign-out against Horae using only the published
  metadata, with no Horae-specific code.
- **SC-003**: For every operation of both APIs and every scope set, a request with an OAuth token
  is granted exactly when the person is allowed and the scopes cover it (checked by tests over each
  permission scope and each scope set).
- **SC-004**: A copy of the database does not let anyone call either API or the token endpoint as
  any person or app.
- **SC-005**: 100% of replayed codes and refresh tokens in the test suite are refused and end their
  authorization.
- **SC-006**: After a person or administrator revokes an authorization or disables an app, the next
  request with any of its tokens is refused (no grace period).
- **SC-007**: Every redirect-URI and PKCE case of the threat model (research R-11) has a test that
  shows no code or token is issued to an attacker.
- **SC-008**: The v1 contract check passes: no existing `/api/v1` client breaks.

## Assumptions

- Sign-in stays OIDC against an external provider or the development login (C-5). The approval
  page relies on the existing session; the only sign-in change is where a person lands after
  signing in (FR-023).
- Single organization until multi-tenancy (020); an authorization carries its organization, so
  multi-tenancy needs no change here.
- The Android app is a separate project. This feature gives it a server to talk to; the app itself
  is out of scope.
- Access tokens are opaque and checked against the database on each request, like personal tokens,
  so revocation takes effect immediately; self-contained (JWT) access tokens are not needed.
- The default limits of the token endpoint (FR-081) and the lifetimes (FR-040, FR-044) are operator
  settings with the defaults stated here.

## Relation to 021

021 US6 and FR-060 to FR-062 planned an OAuth2 provider in Harvest's shapes for the
Harvest-compatible API. This feature builds the provider once, in the standard shapes, for both
APIs. 021 FR-060 (registration) and FR-062 (people see and revoke) are met here; FR-061's
standard parts (approval screen, code exchange, refresh, rotation, hashed storage) are met here,
and its Harvest-shaped account list stays with 021 (D-6).

## Out of scope

- Horae as an OpenID Connect provider: ID tokens, `userinfo`, OIDC discovery. Clients learn who they
  act as from `/api/v1/me`. No requirement here needs an ID token.
- Dynamic client registration (FR-012), client ID metadata documents, and the device authorization
  grant (for CLIs and TVs).
- The client credentials grant (an app acting as itself, with no person).
- Sender-constrained tokens (DPoP, mutual TLS) and pushed authorization requests (PAR). Refresh token
  rotation is the protection for public clients (RFC 9700 §4.14.2); DPoP can be added later.
- Remembering consent between requests.
- Push notifications, device registration, idempotency keys and conflict control for offline writes,
  which the Android study lists after OAuth.
- The Android app itself.
- Harvest-shaped OAuth endpoints (D-6), and any exception to PKCE for clients built for Harvest's
  OAuth (D-7).
