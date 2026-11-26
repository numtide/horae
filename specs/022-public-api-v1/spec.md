# Feature Specification: Public API `/api/v1`

**Feature Branch**: `022-public-api-v1`

**Created**: 2026-11-18

**Status**: Draft, clarified 2026-11-18; awaiting the maintainer's review before `/speckit-plan`

**Input**: User description: "A public API for third parties at `/api/v1`, with its own stable,
versioned types, an OpenAPI description and personal access tokens as the first authentication
method. Every operation goes through `ops` with the same `Actor`, permissions and errors as the
web app. The API lives in a new `horae-api` crate, together with tokens, pagination, the error
shape and what is in `http/` and `auth/http` today. The Harvest-compatible API moves to its own
`horae-harvest-api` crate, reuses those pieces, and can be turned on or off at runtime; it is off
by default."

Builds on [021 Harvest API v2 compatibility](../021-harvest-api-compat/spec.md). 021 designed
personal access tokens (its delivery step 2: [data-model.md](../021-harvest-api-compat/data-model.md),
[contracts/api-tokens.md](../021-harvest-api-compat/contracts/api-tokens.md)) but they are not
implemented. This feature implements that design once, for both APIs, rather than defining a
second one. 021's step 1 (integer ids for `/harvest/v2`, the organization's account id) is
implemented and is not changed here.

## Background

Horae has two HTTP surfaces today. The web app calls Dioxus server functions, which only the web
app can use. The Harvest-compatible API (`/harvest/v2`, with `/v2` and `/api/v2` aliases) speaks
Harvest's dialect: integer ids, decimal hours and Harvest's field names. It is always mounted and
authenticates with the web session cookie only.

Neither suits someone writing their own integration against Horae: a script that books time from
a calendar, an internal dashboard, a payroll export, an editor plugin. They need an API in Horae's
own terms, described in a document they can generate a client from, that does not change under
them, and that they can call from outside a browser.

## Constraints (maintainer decisions, 2026-11-18)

These were decided before this specification and are not reopened here.

- **C-1 Audience and contract.** `/api/v1` is for third parties. It has its own types, which are
  stable and versioned; it does not expose the web app's internal data shapes as they are. It is
  described by an OpenAPI document.
- **C-2 Authentication.** Personal access tokens are the first authentication method.
- **C-3 One set of rules.** Every operation of the API runs through Horae's shared operations,
  with the same acting person, the same permissions (scoped profiles) and the same errors as the
  web app. The API has no rules of its own.
- **C-4 Where it lives.** `/api/v1` lives in a new `horae-api` crate. That crate also holds the
  tokens, pagination, the error shape, and what today lives in the app's `http/` and `auth/http`
  modules: the exports, `/auth` and `/health`.
- **C-5 Harvest compatibility as an option.** The Harvest-compatible API moves to its own crate,
  `horae-harvest-api`, as a thin layer over the shared operations that reuses the tokens,
  pagination and errors of `horae-api`. It is turned on or off at runtime, by configuration and by
  an option of the NixOS module, and is **off by default**. While off, every `/harvest/v2/*` path
  answers 404. A build option that leaves it out of the binary is out of scope.
- **C-6 Exact values.** Primary keys are UUID v7, durations are integer minutes and money is
  integer minor units with a currency code (constitution I and III). The public format keeps
  these, or justifies any conversion.

## Clarifications

### Session 2026-11-18

- Q: Which resources does v1 expose, and does it write? → A: Identity and time entries with
  reads and writes (create, edit, delete, start, stop and restart timers); projects, tasks,
  clients and people read-only. Catalogue writes, reports, invoices and approvals come later.
- Q: How is a version deprecated and retired? → A: No fixed policy now. v1 only promises not to
  break; the retirement policy is decided when a v2 exists.
- Q: Does v1 limit request rates, and per what? → A: Per token, with the same limiter and
  defaults as 021 for `/harvest/v2`, configurable by the operator.
- Q: What happens to the Harvest-compatible API's `/api/v2` alias, which would collide with a
  future `/api/v2` of Horae's own API? → A: Remove it; `/api/` is reserved for Horae's own API.
  `/harvest/v2` and `/v2` stay.
- Q: Does v1 include OAuth2 for third-party applications or outgoing webhooks? → A: Neither.
  Personal access tokens only; both come later as additions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A script reads Horae with a personal access token (Priority: P1)

A person creates a personal access token in their personal settings, names it after the script
that will use it, and copies it. Their script sends the token with each request and asks who it
is acting as, then lists the person's time entries for this week. It gets the person's own data,
in Horae's own terms: identifiers as Horae shows them, durations in minutes, money in minor units
with a currency.

**Why this priority**: Nothing else in the API is usable without token authentication and one
real resource behind it. It is also the smallest slice that gives a third party something useful.

**Independent Test**: Create a token on the settings page, call the identity and time-entry
endpoints with it from a plain HTTP client, and compare the result with the person's timesheet in
the web app.

**Acceptance Scenarios**:

1. **Given** a signed-in person in their personal settings, **When** they create a token with a
   name and an optional expiry, **Then** the token is shown once, in full, and the list shows it by
   name with a short prefix, its creation time, expiry and last use; Horae keeps nothing from which
   the token could be read back.
1. **Given** a valid token, **When** a client asks who it is, **Then** the answer describes the
   token's owner and organization.
1. **Given** a member's token, **When** the client lists time entries, **Then** it sees exactly the
   entries that member sees in the web app, no more.
1. **Given** a revoked or expired token, or one whose owner has been deactivated, **When** it is
   used, **Then** the request is refused as unauthenticated and reveals nothing else.
1. **Given** a request with no token and only the web session cookie, **When** it reaches
   `/api/v1`, **Then** it is refused as unauthenticated.

______________________________________________________________________

### User Story 2 - A developer builds a client from the published description (Priority: P1)

A developer opens the API's OpenAPI document, served by their Horae instance, and generates a
client in their language from it. Every operation, parameter, response and error in the document
matches what the server does. Months later, after Horae has been upgraded, the generated client
still works.

**Why this priority**: A third-party API is only as useful as its description is accurate and its
contract is stable (C-1). Without this, every integration is a guess that breaks on upgrade.

**Independent Test**: Fetch the published description, generate a client with an off-the-shelf
generator, and run the scenarios of User Story 1 through it. Separately, change a response type in
a way that removes a field and see the build's contract check fail.

**Acceptance Scenarios**:

1. **Given** a running instance, **When** anyone fetches the API description, **Then** they get an
   OpenAPI document that lists every `/api/v1` operation with its parameters, response shapes,
   error shapes and the authentication it needs.
1. **Given** the published description, **When** a response is compared with the schema of its
   operation, **Then** it conforms, for every operation (checked automatically).
1. **Given** a change to the API that removes or renames a field, changes a type or meaning, or
   removes an operation, **When** it is proposed, **Then** the project's checks flag it as a
   breaking change of `v1`.

______________________________________________________________________

### User Story 3 - An integration writes through the same rules as the web app (Priority: P2)

A calendar-sync script creates time entries for its owner with a token. Horae applies the same
rules as the timesheet: eligible projects and tasks, locked periods, approval locks, daily limits.
When it refuses, the script gets the same reason the web app would show, in a stable,
machine-readable form.

**Why this priority**: Logging time is the most requested integration for a time tracker. It is
the only write in v1 (FR-031), and it needs the constitution amendment of FR-022 first.

**Independent Test**: Create, change and delete an entry through the API, then see the same entry
and totals in the web app; repeat on a locked period and compare the refusal with the web app's.

**Acceptance Scenarios**:

1. **Given** a write that the web app allows for the token's owner, **When** the API receives it,
   **Then** the result in Horae is the one the web app produces, including audit records and
   plugin events.
1. **Given** a write that the web app refuses, **When** the API receives it, **Then** nothing
   changes and the error carries the same reason, with a status and machine-readable code that
   identify the kind of refusal.
1. **Given** a token whose owner loses a permission, **When** the next request needs it, **Then**
   it is refused; nothing about the token itself has to change.

______________________________________________________________________

### User Story 4 - An operator turns the Harvest-compatible API on or off (Priority: P2)

An operator deploying Horae leaves the Harvest-compatible API off, so the instance exposes only
Horae's own API. Another operator, whose team migrates from Harvest with tools that speak Harvest's
dialect, turns it on in the NixOS module and restarts Horae. In both cases, the same tokens work
on whichever API is enabled.

**Why this priority**: It changes a default on every deployment (C-5). Existing deployments that
rely on `/harvest/v2` must turn it on on purpose.

**Independent Test**: Start Horae with default settings and request a `/harvest/v2` path, then
start it with the option on and request it again with a token.

**Acceptance Scenarios**:

1. **Given** default configuration, **When** any request reaches `/harvest/v2/*` or one of its
   aliases, **Then** it answers 404, exactly as for a path Horae does not serve.
1. **Given** the option turned on by configuration or by the NixOS module option, **When** a
   Harvest client sends a valid token, **Then** it is served as before this feature, now
   authenticated by the same tokens as `/api/v1`.
1. **Given** the option off, **When** a request reaches `/api/v1`, exports, `/auth` or `/health`,
   **Then** they are served unchanged.

______________________________________________________________________

### User Story 5 - An administrator oversees the organization's tokens (Priority: P3)

An administrator opens the organization's settings, sees every token with its owner, name, prefix,
creation, expiry and last use, and revokes one that belongs to someone who left a project. They
never see a token's secret.

**Why this priority**: Needed before the API is opened to many people, but not for the first
integration.

**Independent Test**: As an administrator, list and revoke another person's token, then use that
token and see it refused; as a member, try the same and see it refused.

**Acceptance Scenarios**:

1. **Given** an administrator, **When** they list the organization's tokens, **Then** they see
   every token's details except the secret.
1. **Given** a member, **When** they try to list or revoke someone else's token, **Then** it is
   refused.

______________________________________________________________________

### User Story 6 - A sync job walks a large collection (Priority: P3)

A nightly job copies an organization's time entries into a warehouse. It pages through the
collection with the cursor the API returns, and on later nights asks only for what changed since
its last run.

**Why this priority**: Data pipelines are the main consumers of large reads and the most sensitive
to pagination details. Time entries are the largest collection in v1 (FR-030).

**Independent Test**: Seed more than one page of entries, walk the collection, and compare the
records with the database; change one entry and see an incremental run return exactly that one.

**Acceptance Scenarios**:

1. **Given** a collection larger than one page, **When** a client follows the cursor from each page
   to the next, **Then** it receives every record exactly once while nothing changes underneath.
1. **Given** a tampered or expired cursor, **When** it is sent, **Then** the request is refused as a
   bad request, never silently restarted from the beginning.

______________________________________________________________________

### Edge Cases

- **Token sent in a query string.** `/api/v1` accepts tokens only in the `Authorization` header,
  so they do not end up in proxy and browser logs. (The Harvest-compatible API keeps accepting the
  `access_token` query parameter, which Harvest clients use.)
- **Token prefixes.** A token is recognisable as a Horae token by its prefix, so secret scanners can
  find leaked ones.
- **A token whose owner has memberships in several organizations** (after multi-tenancy). The
  request names the organization; a token never acts in an organization where its owner has no
  active membership, and on a tenant's host only in that tenant.
- **Unknown query parameters** are ignored; an unknown filter value of a known parameter is refused.
- **An unknown field in a request body** is refused, so a client's typo does not silently do
  nothing.
- **A permission check that depends on scope** (own, managed, all): reads return only what the
  scope allows, and a single record outside it answers not found, not forbidden, as in the web app.
- **Money in currencies with 0 or 3 decimals.** Amounts are integers in the currency's minor unit
  with the currency code, so no decimal handling is needed by clients.
- **Durations of a running timer.** Reported in whole elapsed minutes as Horae computes them, never
  as fractions.
- **Harvest's `/api/v2` alias.** Removed (FR-052), so `/api/v2` is free for a future version of
  Horae's own API. A Harvest client configured with a base URL ending in `/api` switches to the
  host root or `/harvest`. Until Horae has its own v2, `/api/v2` answers 404 whether or not the
  Harvest-compatible API is on.
- **Deployments that use `/harvest/v2` today.** Turning the default off would break them on
  upgrade; the release notes and the NixOS option description say how to turn it back on.

## Requirements *(mandatory)*

### Functional Requirements

#### Tokens (021 step 2, implemented here for both APIs)

- **FR-001**: Each person MUST be able to create, name, list and revoke their own personal access
  tokens from their personal settings. A token is shown once, in full, at creation. An optional
  expiry may be set at creation.
- **FR-002**: Horae MUST store only a one-way hash of each token, and a token MUST carry enough
  randomness that guessing it is infeasible. A short display prefix tells tokens apart.
- **FR-003**: A token MUST act as its owner, with the owner's current permission state in the
  organization the request is for. Changes to the owner's permissions apply from the next request.
  A token never grants more than its owner could do in the web app at the moment of the request.
- **FR-004**: A token MUST stop working when it is revoked, when it expires, or when its owner is
  deactivated in that organization. Reactivating the owner makes unexpired, unrevoked tokens work
  again.
- **FR-005**: Each token MUST record when it was last used, shown to its owner and administrators.
- **FR-006**: Administrators MUST be able to list every token in their organization and revoke any
  of them, without seeing any token's secret.
- **FR-007**: Issuing and revoking tokens are mutations of the web app and MUST go through the
  same shared operations as every other mutation (constitution IV).
- **FR-008**: One token MUST authenticate on both `/api/v1` and, when enabled, the
  Harvest-compatible API. There is one kind of token, not one per API.
- **FR-009**: Tokens MUST never be written to logs, error messages or audit details.

#### Authentication and organization

- **FR-010**: `/api/v1` MUST accept a token as a bearer credential in the `Authorization` header and
  MUST NOT accept the web session cookie as authentication. A request without a valid token is
  refused as unauthenticated.
- **FR-011**: The organization a request acts in MUST be the token owner's only active membership
  when they have one. When they have several, the request MUST name the organization, and a
  request that does not is refused with an error that says so. Under multi-tenant hosting, the
  host's tenant decides and a token whose owner has no active membership there is refused.

#### Shared rules

- **FR-020**: Every `/api/v1` operation MUST run Horae's shared operation for the equivalent web-app
  action, with the same acting person, permission checks, validation, locks, audit records,
  plugin events and errors. The API MUST NOT contain its own copy of any rule (C-3).
- **FR-021**: Reads MUST return exactly what the token's owner may see in the web app under their
  scoped permissions.
- **FR-022**: A write over `/api/v1` is a new transport for the shared operations. Before any
  write ships, the constitution's Principle IV clause that today names only the Harvest-compatible
  API MUST be amended to cover `/api/v1` with the same bounds (token-resolved actor, hashed tokens,
  no writes outside the shared operations, no cookie-only mutation).

#### Resources

- **FR-030**: Version 1 MUST expose:
  - **Identity**: who the token acts as, and in which organization.
  - **Time entries**: list (with the filters the timesheet offers: person, project, task, date
    range, updated since), get, create with a duration, edit, delete, start a timer, stop it and
    restart a stopped entry, each through the operation the timesheet uses. Horae has no restart
    operation yet; it is added as a shared operation the web app can use too, as 021 FR-032
    describes, never as API-only behaviour.
  - **Projects, tasks, clients and people**: list and get, read-only, limited to what the token's
    owner may see. These are what a client needs to choose a project and task for an entry.
- **FR-031**: The only writes in version 1 are the time-entry operations of FR-030. Catalogue
  writes (projects, tasks, clients, people, assignments), reports, invoices and approvals are not
  part of v1; they are added later as backward-compatible additions (FR-050).

#### Contract and format

- **FR-040**: `/api/v1` MUST use its own public types, defined for the API and changed only under
  the versioning rules (FR-050). Internal changes to the web app's data shapes MUST NOT change
  `/api/v1` responses.
- **FR-041**: Identifiers MUST be Horae's UUIDs (v7), written as strings. The API MUST NOT expose
  the integer ids of the Harvest-compatible API.
- **FR-042**: Durations MUST be integer minutes and money MUST be an integer in the currency's ISO
  4217 minor unit (EUR 12.10 is `1210`, JPY 8000 is `8000`, BHD 10.500 is `10500`) with its ISO
  4217 currency code, on input and output. No decimal hours or decimal amounts appear in
  `/api/v1` (constitution I).
- **FR-043**: Timestamps MUST be RFC 3339 with an offset; calendar dates (an entry's day, a filter's
  range) MUST be plain ISO 8601 dates, interpreted as Horae interprets them in the web app.
- **FR-044**: Every error MUST use one documented shape: the status, a stable machine-readable code
  for the kind of refusal, and a human-readable message carrying the same reason the web app
  shows. Each shared-operation error maps to one status and code.
- **FR-045**: Every list MUST be ordered deterministically and paginated with an opaque cursor and a
  page-size parameter with a documented default and maximum. A page includes the cursor of the next
  page when there is one. Cursors MUST be tamper-evident.
- **FR-046**: Lists of records that change MUST accept an "updated since" filter on a real update
  time, so a client can sync incrementally.
- **FR-047**: The server MUST publish an OpenAPI document describing every `/api/v1` operation, its
  parameters, request and response shapes, errors and authentication, at a stable public path.
  It MUST be produced from the same definitions the server uses, so the two cannot disagree.
- **FR-048**: The project's checks MUST verify that every response conforms to the published
  document, and MUST flag any change to the document that breaks existing clients of `v1`.

#### Versioning

- **FR-050**: Within `v1`, only backward-compatible changes are allowed: new operations, new
  optional parameters, new response fields and new error codes that clients can treat as unknown.
  Anything else needs a new major version. This feature commits only to that rule: how a version
  is deprecated and retired (notice, signalling, overlap) is decided when a second version exists,
  and until then `v1` is not retired.

#### Limits and integrations

- **FR-051**: Requests MUST be rate-limited per token, with the same limiter and defaults as the
  Harvest-compatible API (021 FR-050): 100 requests per 15 seconds, and 100 per 15 minutes for
  reports when they exist. Operators can change the limits. A token's requests to either API count
  against the same limit. Over the limit, the answer is 429 with a `Retry-After` header and the
  standard error shape (FR-044).
- **FR-052**: The `/api/` prefix MUST belong to Horae's own API only. The Harvest-compatible API's
  `/api/v2` alias (and its `.json` variants) MUST be removed; it stays at `/harvest/v2` and the
  `/v2` alias. This amends 021 FR-026 and its contract's *Base paths*.
- **FR-053**: Personal access tokens are the only authentication method of this feature. OAuth2
  authorization for third-party applications (021's last step) and outgoing webhooks are not part
  of it; when they come, they arrive as backward-compatible additions to `v1` (FR-050), and
  OAuth2 access tokens resolve to the same acting person as personal tokens.

#### Harvest-compatible API toggle

- **FR-060**: The Harvest-compatible API MUST be enabled or disabled at startup by configuration,
  and the NixOS module MUST offer an option for it. It MUST be disabled by default.
- **FR-061**: While disabled, every path of the Harvest-compatible API, including its aliases, MUST
  answer 404 as an unknown path does, without revealing that the API exists.
- **FR-062**: While enabled, it MUST authenticate with the shared tokens (FR-008), keeping 021's
  session-cookie reads (021 D-7), and use the shared token checks, rate limiter, pagination core
  and operation-error mapping of the API layer, rendered in Harvest's dialect as 021 defines it.
  It MUST hold no data access or rule of its own.
- **FR-063**: Disabling the Harvest-compatible API MUST NOT affect the Harvest importer, which is a
  client of Harvest, not a server.

#### Existing surfaces

- **FR-070**: The exports, the `/auth` routes and `/health` MUST keep their paths, behaviour and
  responses when they move into the API layer (C-4).

### Key Entities

- **Personal access token**: belongs to one person's account; acts as that person's membership in
  the organization of the request. Has a name, a display prefix, a hash of the secret, a creation
  time, an optional expiry, a last-use time and a revocation time. Shared by both APIs.
- **API version**: a named, stable contract (`v1`) with its own types and published description.
- **API description**: the OpenAPI document for one version, published by the server.
- **API error**: a status, a stable code and a message.
- **Page**: a slice of an ordered list with an opaque cursor to the next slice.
- **Harvest-compatible API setting**: an operator's on/off choice, off by default.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person goes from opening their personal settings to a first successful API call
  from a script in under 5 minutes.
- **SC-002**: A client generated from the published description with an off-the-shelf generator
  completes the identity and time-entry scenarios with no hand-written changes.
- **SC-003**: 100% of `/api/v1` responses in the test suite conform to the published description.
- **SC-004**: For every operation, the API grants and refuses exactly what the web app grants and
  refuses to the same person (checked by tests over each permission scope).
- **SC-005**: A stolen copy of the database does not let anyone call either API as any person.
- **SC-006**: With default settings, 100% of Harvest-compatible paths answer 404; with the option
  on, the existing Harvest compatibility suite passes, changed only for token authentication and
  the removed `/api/v2` alias.
- **SC-007**: A full walk of a collection of 100,000 time entries returns every record exactly once.
- **SC-008**: No release changes `v1` in a way that breaks a client built against an earlier `v1`
  description (checked on every change).

## Assumptions

- Single organization in deployment until multi-tenant hosting (020) lands; FR-011 then takes
  effect without changing the contract.
- The session cookie keeps authenticating the web app's server functions, exports and `/auth`.
  `/api/v1` refuses it; the Harvest-compatible API keeps accepting it for reads (021 D-7).
- Default page size 100, maximum 1,000; the plan may tune both before release.
- The token format, hashing and storage are those of 021's data model; this feature implements them
  and does not redesign them.
- The OpenAPI description is public (it reveals the API's shape, not any data).
- Existing deployments that use `/harvest/v2` set the new option when they upgrade.

## Out of scope

- A build option that removes the Harvest-compatible API from the binary (C-5).
- New domain features; the API exposes what Horae already does (the timer restart of FR-030 is
  a shared operation 021 already specified).
- OAuth2 for third-party applications and outgoing webhooks (FR-053).
- Catalogue writes, reports, invoices and approvals over `/api/v1` (FR-031).
- A deprecation and retirement policy for API versions (FR-050).
- Changes to the Harvest-compatible API's dialect or the remaining 021 delivery steps, beyond moving
  it and switching it to shared tokens.
- Exposing the web app's server functions as a public API.
