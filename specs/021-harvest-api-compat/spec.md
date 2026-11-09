# Feature Specification: Harvest API v2 compatibility

**Feature Branch**: `spec/harvest-api-compat` (specification); implementation lands as the
ordered pull requests under *Delivery order*

**Created**: 2026-11-09

**Status**: In progress. The constitution amendment it depends on
([constitution-amendment.md](constitution-amendment.md)) was approved on 2026-11-09 (constitution
5.0.0).

**Input**: User description: "The whole Harvest ecosystem should work without problems against
Horae: apps, timers, browser extensions, integrations, scripts and libraries written for Harvest
API v2."

Extends the read-only Harvest-compatible API of
[001](../001-time-tracking-invoicing/contracts/harvest-api.md). Changes the scope row "Existing
integrations and API" in [harvest-parity.md](../harvest-parity.md), which allowed regression
coverage only and no write endpoints; the maintainer approved the wider scope as D-020. Interacts with
[020 multi-tenant subdomains](https://github.com/numtide/horae/tree/spec/multi-tenant-subdomains/specs/020-multi-tenant-subdomains)
(not merged). The evidence is in [research.md](research.md).

## Background

People who leave Harvest bring tools that speak Harvest API v2: their own scripts, libraries,
command-line timers, editor and launcher extensions, automation flows and data pipelines. Horae
already answers a read-only subset of that API. It authenticates with the web session cookie,
returns UUIDs, has no writes and caps pages at 100, so no real Harvest client works against it
today.

The survey in [research.md §1](research.md#1-ecosystem-survey) sets a hard limit on what
"the whole ecosystem" can mean. Every Harvest client sends its requests to Harvest's own hosts.
Harvest's apps, browser extensions and embeds have those hosts built in and cannot be redirected,
and neither can the hosted connectors (Zapier, Make, Pipedream, Fivetran and others). They are
out of reach whatever Horae does.

What a compatible Horae can serve:

- Code people write themselves, including against the eight libraries that accept a base URL.
- Generic HTTP steps in automation platforms.
- Open-source tools after a one-line host change. That covers about 40 of the 47 surveyed v2
  projects. The two thirds of them that use a personal access token need nothing more.

This specification therefore defines "works without problems" as follows: **a Harvest API v2
client that can be pointed at Horae, and that authenticates with a personal access token,
completes its normal workflows with no other change**. A compatibility suite measures this for
named clients. Pointing clients at Horae is outside this repository; upstream pull requests that
add a base-URL option are recommended to the maintainer as a separate, outward-facing step.

## Open Questions

**Resolved on 2026-11-09**: the maintainer chose option (a) for all three (see *Clarifications*).
The options are kept as the record of what was weighed. Requirements that depended on one are
marked *(OQ-n)*.

- **OQ-1: Horae as an OAuth2 identity provider for Harvest clients.** About a third of the
  open-source clients sign in with OAuth2. All of them have Harvest's identity host built in and
  most embed a client id registered with Harvest, so each needs a source change beyond the API
  host anyway.
  - (a) *Personal access tokens first, OAuth2 later* (recommended): ship tokens, writes and
    fidelity, then build Harvest-shaped authorize, token and account-list endpoints (US6). OAuth
    clients can already be given a token by hand in most cases (hrvst-cli, dedene, n8n, Airbyte).
  - (b) *Tokens only, never OAuth2*: smallest surface. Blocks OAuth-only tools (Singer tap,
    Pipedream and Activepieces forks, yield-app) for good.
  - (c) *OAuth2 in the first slice*: more reach early, at the cost of an authorization server
    before anything can write time.
- **OQ-2: Which integer each record gets.** Clients need integer ids, and many CLIs keep project
  and task ids in their own configuration.
  - (a) *Keep imported Harvest ids, number the rest* (recommended): a record imported from
    Harvest is exposed under its original Harvest id. Any other record gets the next free number
    for its kind in its organization. If an imported id is already taken by a native record, the
    imported record gets a fresh number and the importer reports it. Existing Harvest-era
    configurations and scripts keep working for imported data.
  - (b) *Fresh numbers for everything*: simpler and collision-free. Every id a migrating person
    has written down changes.
- **OQ-3: What the account id is.**
  - (a) *A Horae-assigned number per organization* (recommended): shown to administrators next to
    each token. It stays the same for the organization's lifetime.
  - (b) *The Harvest account id the importer is bound to*: a migrating script's configured account
    id keeps working. It fails for organizations that never imported, is ambiguous if two tenants
    import one Harvest account (020 OQ-3 allows that), and names a Harvest account inside Horae.

Further decisions with a recommended default are under *Decisions with defaults*; they are less
consequential and are recorded there.

## Clarifications

### Session 2026-11-09 (from the request and the survey)

- Q: Must Harvest's own apps, extensions and hosted connectors work? → A: Not achievable: their
  hosts are built in. The target is clients that can be pointed at Horae (see Background).
- Q: Does this feature add new Horae domain features (estimates, expenses, contacts, payments)?
  → A: No. Endpoints for concepts Horae lacks wait for those features; until then their lists are
  empty and the company's feature flags say they are off (FR-043).
- Q: Must the API reuse Horae's permission and domain rules? → A: Yes. Every write runs the same
  operation, rules and permission checks as the equivalent action in the web app (FR-030).

### Session 2026-11-09 (maintainer decisions)

- Q: OQ-1, should Horae be an OAuth2 provider? → A: (a) Personal access tokens first, OAuth2
  later (US6 stays, last in the delivery order).
- Q: OQ-2, which integer does each record get? → A: (a) Imported records keep their Harvest id;
  everything else gets the next free number for its kind; a collision gets a fresh number and is
  reported.
- Q: OQ-3, what is the account id? → A: (a) A Horae-assigned number per organization.
- Q: D-6, does starting a timer stop the running one, in the web app too? → A: Yes.
- Q: D-8, open base-URL pull requests in third-party projects? → A: No. Patches needed to point a
  suite client at Horae stay inside this repository's test suite and are not sent upstream.
- Q: D-9, may reference responses be recorded from the test (evidence) Harvest account? → A: Yes,
  with GET requests only, to record response and error shapes. The authorization is row D-020 in
  `harvest-parity.md`. No real account name or id is written into the repository.
- Q: The other defaults D-1 to D-7? → A: Accepted as written.
- Q: The constitution amendment? → A: Approved verbatim on 2026-11-09 and applied as constitution
  5.0.0 in the step-1 pull request.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Harvest client reads Horae with a personal access token (Priority: P1)

A person creates a personal access token on their Horae profile page and copies it together with
their organization's account id. They configure a Harvest client that has been pointed at Horae,
for example a command-line timer, an editor extension or their own script. The client shows who
they are and lists their projects, tasks and time entries, with integer ids, exactly as it would
against Harvest.

**Why this priority**: Nothing else works without it. Token sign-in and integer ids are what make
every client get past its first request.

**Independent Test**: Create a token, then run the compatibility suite's read workflows (who am I,
my project assignments, my time this week) with two clients that accept a base URL (a Go library
and a PHP library) and see them succeed and decode every id as an integer.

**Acceptance Scenarios**:

1. **Given** a signed-in person on their profile page, **When** they create a token with a name,
   **Then** the token is shown once, the page lists it by name with its creation time, last use
   and expiry, and Horae keeps no copy from which the token could be read back.
1. **Given** a valid token and the matching account id, **When** a client requests who it is,
   **Then** the response describes the token's owner with integer ids.
1. **Given** a valid token and an account id that is not the token's organization, **When** any
   request is made, **Then** it is refused as unauthorized and reveals nothing about the other
   organization.
1. **Given** a revoked or expired token, or a token whose owner has been deactivated, **When** it
   is used, **Then** the request is refused as unauthorized.
1. **Given** a member's token, **When** the client lists time entries, **Then** it sees exactly
   the entries the member sees in Horae's own timesheet, no more.
1. **Given** a record seen through the API, **When** the same record is requested again later,
   by any client, **Then** it has the same integer id.
1. **Given** a record imported from Harvest, **When** a client asks for it, **Then** its id is its
   original Harvest id *(OQ-2)*.

______________________________________________________________________

### User Story 2 - Log time and run timers from a Harvest client (Priority: P1)

The person uses their pointed client to log a duration, log a start and end time, start a timer,
stop it, restart yesterday's entry, fix a note and delete a mistaken entry. Horae applies each one
with the same rules as its own timesheet: daily limits, locked periods, approval locks, project
and task eligibility, and the one-running-timer rule. Each refusal comes back as a Harvest-style
error the client can show.

**Why this priority**: Logging time and timers is what 19 of the 23 surveyed end-user apps do.
With US1, this unlocks the full feature set of about 20 of them.

**Independent Test**: Run the compatibility suite's time workflows (log, start, stop, restart,
edit, delete) through at least one library and one patched end-user client, then open Horae's
timesheet and see the same entries, totals and timer state.

**Acceptance Scenarios**:

1. **Given** an eligible project and task, **When** a client creates an entry with `hours: 1.5`,
   **Then** Horae stores 90 minutes and the response reports `hours: 1.5` and `is_running: false`.
1. **Given** the same, **When** a client creates an entry without hours, **Then** a timer starts
   and the response reports `is_running: true` and when it started.
1. **Given** a running timer, **When** the client stops it, **Then** elapsed minutes are recorded
   under the same daily-limit clamp as the web app, and the response explains any clamp in a
   field the client can ignore.
1. **Given** a stopped entry, **When** the client restarts it, **Then** the entry runs again and
   continues from its recorded time, and any other running timer of that person stops.
1. **Given** an entry in a submitted, approved or invoiced state, **When** a client updates or
   deletes it, **Then** the request is refused with the same reason the web app gives, and
   nothing changes.
1. **Given** a member's token, **When** the client creates an entry for another person, **Then**
   it is refused unless Horae's own rules let that member write for that person.
1. **Given** a write, **When** it succeeds, **Then** plugins receive the same events and budget
   checks run exactly as for the same action in the web app.
1. **Given** a session cookie and no token, **When** a write is attempted, **Then** it is refused,
   so a page on another site cannot write through a signed-in browser.

______________________________________________________________________

### User Story 3 - Sync everything into a warehouse or archive (Priority: P2)

An operator points a data-pipeline tool (a forked Airbyte connector, a Singer tap, an archive tool,
their own script) at Horae with an administrator's token. It pages through every supported
collection with large pages, follows the response's own next links or page numbers, and on later
runs asks only for records updated since its last run.

**Why this priority**: Data pipelines are the main users of the full read surface, and the most
likely to break on pagination details.

**Independent Test**: Run the suite's sync workflow with one link-following client and one
page-counting client over a seeded organization with more than 2000 entries. Compare record
counts and ids with the database. Then edit one record of each kind and see a second incremental
run return exactly those records.

**Acceptance Scenarios**:

1. **Given** a collection larger than one page, **When** a client follows `links.next`, **Then**
   every link is absolute on the host the request arrived on, and the walk returns every record
   exactly once while nothing changes underneath.
1. **Given** the same, **When** a client walks by `page` and `total_pages`, **Then** it gets the
   same records, and `next_page` is filled on every page that has a successor.
1. **Given** `per_page=2000`, **When** requested, **Then** up to 2000 records are returned;
   **given** `per_page=2001`, **then** the request is refused as Harvest refuses it.
1. **Given** a record of any exposed kind whose content changed, **When** a client asks for
   records updated since a time before the change, **Then** the record is included, whatever the
   kind (time entry, project, client, task, user, assignment).
1. **Given** a kind Horae does not have yet (expenses, estimates, contacts), **When** a pipeline
   lists it, **Then** it gets a well-formed empty page, not an error, so the rest of the sync
   completes.

______________________________________________________________________

### User Story 4 - The reads that end-user apps call around the timer (Priority: P2)

A launcher extension asks for the company settings to choose its time format, then for the
person's project assignments with their tasks, to build its picker. A budget widget asks for the
project budget report; a weekly summary asks for the time report by project. Each answers in
Harvest's shape from Horae's data.

**Why this priority**: `users/me/project_assignments` and `company` are called by most end-user
apps right after sign-in, and without them those apps cannot offer a picker.

**Independent Test**: Run the suite's picker and report workflows through a patched launcher
client and a library, and compare the report totals with Horae's own reports for the same range.

**Acceptance Scenarios**:

1. **Given** a member assigned to two projects, **When** their client lists their project
   assignments, **Then** it gets both, each with its client and the tasks the member may track.
1. **Given** an organization, **When** a client reads the company, **Then** it gets the
   organization's name, week start, time and currency display settings, and feature flags that
   are true only for features Horae has.
1. **Given** a date range, **When** a client asks for a time report by project, **Then** each
   row's hours and amounts equal Horae's own report for the same range to the minute and the
   minor unit.

______________________________________________________________________

### User Story 5 - Administrators automate the catalogue and invoices (Priority: P3)

An administrator's script creates clients and projects from a CRM, assigns people and tasks, and
archives finished projects. A finance script lists invoices and marks them sent. All of it runs
through the same rules as the administration screens.

**Why this priority**: Fewer clients do this (broad CLIs, MCP servers, SDK users), and every
operation needs its own mapping onto Horae's request and approval flows.

**Independent Test**: Run the suite's catalogue workflow with a library client using an
administrator token, then check the same records in Horae's screens and audit log. Repeat with a
member token and see each write refused.

**Acceptance Scenarios**:

1. **Given** an administrator token, **When** a client creates a project for an existing client,
   **Then** the project appears in Horae as if created on the New Project screen, with the same
   validation.
1. **Given** a manager or member token, **When** it attempts a write that the web app would not
   allow that person, **Then** it is refused with 403 and nothing changes.

______________________________________________________________________

### User Story 6 - Sign in to a Harvest OAuth2 client with Horae (Priority: P3, OQ-1)

A person runs an OAuth2-only tool (a forked Singer tap, a macOS menu-bar app) that has been
pointed at Horae's identity endpoints and registered with Horae. The tool sends them to Horae.
They sign in as usual, see which tool is asking for which organization, and approve. The tool
gets a token that expires and can be refreshed, and it can list the person's accounts.

**Why this priority**: These tools need a source change beyond the host anyway (an embedded
client id), and tokens cover most of their users today.

**Independent Test**: Register a client in Horae, run the authorization-code and refresh flows
with a patched OAuth client, then revoke the grant and see the next call refused.

**Acceptance Scenarios**:

1. **Given** a registered client, **When** a signed-in person approves its request, **Then** the
   client receives a code it can exchange once for an access token and a refresh token.
1. **Given** an access token from this flow, **When** the client lists accounts, **Then** it sees
   only the organizations the person granted.
1. **Given** a grant the person revoked on their profile page, **When** its tokens are used,
   **Then** they are refused.

______________________________________________________________________

### Edge Cases

- **Hours that do not divide evenly.** 20 minutes is reported as `0.33`. A client that writes
  `0.33` back gets 20 minutes, because input is rounded to the nearest minute. Two-decimal hours
  always come back to the same minute (the error is at most 0.3 minutes).
- **Hours with more precision, or negative or non-numeric hours.** `1.2345` is rounded to the
  nearest minute. Negative or non-numeric values are refused like Harvest refuses them.
- **Money in currencies with 0 or 3 decimals** (JPY, BHD) is written with that currency's number
  of decimals, from integers, without loss.
- **A timer started on one day and stopped after midnight** follows Horae's existing timer rules;
  the API does not add new ones.
- **The account header is missing.** It is accepted when the token belongs to exactly one
  organization (every token does, today). A malformed one is refused as unauthorized.
- **Both `cursor` and `page`.** The cursor wins, as on Harvest. A stale or tampered cursor is
  refused as a bad request, never silently restarted from the beginning.
- **Records deleted between pages.** They drop out of later pages, and no record is returned
  twice. As today, a page walk is not a snapshot.
- **Ids for a record that existed before this feature.** Every existing record gets its number
  when the feature is deployed, so ids never depend on which record a client happened to read
  first.
- **A deleted record's number** is never given to another record.
- **The token appears in a query string** (`access_token=`). It is accepted, as on Harvest, and
  never written to logs.
- **A token is used from two clients at once.** Both share the rate limit of that token.
- **A token is used on another tenant's host** (multi-tenant). It is refused as unauthorized.
- **Legacy paths.** `/api/v2/...` and a `.json` suffix answer like `/v2/...`.
- **Unknown query parameters** are ignored, as today.

## Requirements *(mandatory)*

### Functional Requirements

#### Authentication and tokens

- **FR-001**: Each person MUST be able to create, name, list and revoke their own personal
  access tokens from their profile page. A token is shown once, in full, at creation.
- **FR-002**: Horae MUST store only a one-way hash of each token, so that a stolen database does
  not yield usable tokens. A token MUST carry enough randomness that guessing is infeasible.
- **FR-003**: A token MUST act as its owner, with exactly the owner's current role and
  permissions in the owner's organization. A change of role or permissions applies to the next
  request. A token never grants more than its owner could do in the web app.
- **FR-004**: A token MUST stop working when it is revoked, when it expires (if an expiry was
  chosen), or when its owner is deactivated. Administrators MUST be able to list and revoke any
  token in their organization without seeing the token itself.
- **FR-005**: Each token MUST record when it was last used, shown to its owner and to
  administrators.
- **FR-006**: The API MUST accept a token as `Authorization: Bearer <token>` or as the
  `access_token` query parameter. The account MUST be accepted as the `Harvest-Account-Id` header
  or the `account_id` query parameter. Tokens MUST never be written to logs.
- **FR-007**: The existing session-cookie authentication MUST keep working for reads, so nothing
  that relies on it today regresses. Writes MUST require a token *(see Decisions with defaults,
  D-7)*.
- **FR-008**: The API MUST NOT require a `User-Agent` header.

#### Accounts and tenants

- **FR-010**: Each organization MUST have one integer account id that never changes and is never
  reused *(OQ-3)*. Administrators MUST see it beside the token list.
- **FR-011**: A request whose account id is not the token owner's organization MUST be refused as
  unauthorized. A request with no account id MUST be served for the token's own organization.
- **FR-012**: Under multi-tenancy (020), the request's host MUST select the tenant first. A token
  MUST work only on its own organization's host, and the account id MUST match that tenant.
  Links in responses MUST use the host the request arrived on.

#### Integer ids

- **FR-015**: Every record exposed by the API MUST have an integer id. It is unique within its
  kind and organization, never changes and is never reused. Primary keys stay UUID v7.
- **FR-016**: A record imported from Harvest MUST keep its original Harvest id as its integer id.
  If that number is already held by another record of the same kind, the record gets a fresh
  number and the importer reports the change *(OQ-2)*.
  - *Implementation note (step 1)*: "imported" means created by the import. A record the import
    only matches (it existed in Horae before) keeps the number it already has, because FR-015's
    "never changes" wins: clients may already know that number. The report line for a collision
    is not implemented yet (tasks.md T015); until it is, a collision keeps the fresh number
    silently, so FR-016 is only partly met. Native numbers come from a sequence per kind that
    starts at 1, far below Harvest's ids, so collisions are rare in practice.
- **FR-017**: Every record that exists when this feature is deployed MUST receive its number at
  deployment, and every new record when it is created.
- **FR-018**: Every id in a request (path, filter or body) MUST be accepted as an integer. For a
  transition period announced in the release notes, the existing UUID form MUST also be accepted
  in paths and filters. Responses MUST use integers only.

#### Units, fields and errors

- **FR-020**: Hours on the wire MUST be decimal numbers derived from integer minutes, rounded half
  up to two decimals. Hours received MUST be parsed as exact decimals and rounded half up to whole
  minutes, never through floating point. *(Constitution Principle I; see the amendment.)*
- **FR-021**: Money on the wire MUST be decimal numbers in major units, derived from integer minor
  units with the currency's own number of decimals. Amounts received MUST convert exactly or be
  refused if they have more decimals than the currency allows.
- **FR-022**: Each exposed object MUST carry every field Harvest documents for it that Horae can
  fill. A field Horae has no data for MUST be present with Harvest's empty value (`null`, `false`
  or an empty list), not omitted, unless omission is how Harvest itself hides it from the caller's
  role.
- **FR-023**: Every exposed kind MUST have a real update time, which changes whenever anything
  shown about the record changes. `updated_since` MUST filter on it for every kind.
- **FR-024**: Errors MUST use Harvest's status codes and body shapes. Until the shapes are
  confirmed (see *Assumptions*), a failed authentication answers 401 with `error` and
  `error_description`, and any other refusal answers with `message`. A refusal by a Horae rule
  MUST carry the same reason the web app shows.
- **FR-025**: Request parameters for POST and PATCH MUST be accepted as a JSON body, a form body
  or query parameters.
- **FR-026**: `/api/v2/...` and a `.json` suffix MUST be served the same as the canonical path.

#### Writes

- **FR-030**: Every write MUST run the same Horae operation as the equivalent web-app action, with
  the same validation, permission checks, locks, audit records, plugin events and budget checks.
  The API MUST NOT contain its own copy of any rule. *(Constitution Principle IV; see the
  amendment.)*
- **FR-031**: Time entries MUST support every Harvest operation:
  - create with a duration;
  - create with start and end times, regardless of the company setting;
  - create a running timer;
  - update, delete, stop and restart.
- **FR-032**: Restart MUST be a Horae domain operation available to the web app too, not an
  API-only behaviour. When a person starts or restarts a timer while another of theirs is
  running, the running one MUST stop first, as on Harvest *(see D-6)*.
- **FR-033**: Catalogue writes (clients, projects, tasks, user and task assignments) and user
  administration MUST each map onto the existing administration operation for that action. An
  operation that needs a request and approval in Horae answers with that request's outcome, not
  a bypass.
- **FR-034**: Invoice reads and the status changes Horae already supports (draft, sent, closed)
  MUST be exposed. Invoice creation MUST go through Horae's invoice generation request.

#### Reads and pagination

- **FR-040**: Every list MUST return Harvest's envelope with `per_page` from 1 to 2000, defaulting
  to 2000. A larger value MUST be refused the way Harvest refuses it. `page`, `next_page`,
  `previous_page`, `total_pages` and `total_entries` MUST always be filled where they have a
  value, including on lists served with cursors.
- **FR-041**: Lists that Harvest pages with cursors MUST accept `cursor`, and their `links.next`
  and `links.previous` MUST carry one. A cursor MUST be opaque, tamper-evident and refer to a
  position, not a page number. Every link MUST be an absolute URL on the request's host.
- **FR-042**: Every filter Harvest documents for an implemented list MUST be implemented or
  explicitly listed as unsupported in the contract. An unsupported filter must not silently
  return a wider result: requesting it MUST be refused.
- **FR-043**: Lists for kinds Horae does not have (expenses, expense categories, estimates and
  their messages and categories, contacts, invoice payments, invoice item categories, roles) MUST
  return an empty page. Gets and writes on them MUST answer not found. The company's feature
  flags MUST say these features are off.
- **FR-044**: The read surface MUST cover, in this order of priority:
  1. `users/me` and `users/me/project_assignments`, `company`, and time entries.
  1. Projects, tasks, clients, users, user and task assignments, project assignments per user,
     teammates.
  1. The time reports (clients, projects, tasks, team), the uninvoiced report and the project
     budget report.
  1. Invoices and tags.
- **FR-045**: Report totals MUST be computed from integer minutes and minor units, and only the
  final values converted for the wire (Principle I).

#### Limits

- **FR-050**: Requests MUST be rate-limited per token: by default 100 requests per 15 seconds,
  and 100 per 15 minutes for reports. Operators can change both. Over the limit, the answer is
  429 with a `Retry-After` header and a `retry_after` field in the body.

#### OAuth2 provider *(OQ-1, US6)*

- **FR-060**: Administrators MUST be able to register OAuth2 clients with a name and redirect
  addresses, and see and revoke them.
- **FR-061**: Horae MUST offer authorization-code sign-in with an explicit approval screen, code
  exchange, refresh and an account list, in the request and response shapes Harvest uses. Codes
  are single-use and short-lived. Access tokens expire, refresh tokens rotate, and everything is
  stored as hashes.
- **FR-062**: People MUST be able to see and revoke the clients they have approved.

#### Compatibility suite

- **FR-070**: A compatibility suite MUST run in `nix flake check` without network access. It
  starts Horae with seeded data and runs named workflows (FR-071) through real Harvest clients
  pointed at it by configuration or by a recorded one-line patch.
- **FR-071**: The workflows are the user stories: read identity and picker (US1, US4), log, edit
  and time (US2), full and incremental sync (US3), and catalogue administration (US5). Each
  client in the suite declares which workflows it covers.
- **FR-072**: The suite MUST start with at least the clients in *Assumptions*, and record per
  client and workflow whether it passes. The result is published in the contract document, so
  "works" is a measured, named list rather than a claim.
- **FR-073**: Response shapes MUST additionally be checked field by field against reference
  responses that describe Harvest's documented shapes (names, types, nullability). The reference
  responses are recorded from the test (evidence) Harvest account with GET requests only, as
  authorized in D-9. Where no recording exists yet, they are written by hand from the public
  documentation. Recorded files carry no real account name, account id, person or business data.

### Decisions with defaults

The maintainer accepted D-1 to D-7 as written, and decided D-8 and D-9 on 2026-11-09.

- **D-1 Existing UUID consumers.** Switch responses to integers in one release, and accept UUIDs
  in paths and filters for a transition period (FR-018). Alternative: a versioned path. Not
  recommended, because nothing outside this repository is known to consume the UUID form.
- **D-2 Absent domains.** Return empty lists and switch the feature flags off (FR-043).
  Alternative: 404 for the whole kind. That breaks pipelines that sync every stream.
- **D-3 Rate limits.** Limit per token (FR-050). Alternative: per organization as Harvest does.
  That lets one busy pipeline starve a person's timer.
- **D-4 Page size default.** Default to 2000, as on Harvest (FR-040). This changes today's
  default of 100; callers that relied on 100 must ask for it.
- **D-5 Hours precision.** Two decimals (FR-020). Every value comes back to the same minute.
  Whether Harvest itself reports more precision for running timers is an open fact; see
  *Assumptions*.
- **D-6 Starting a timer while another runs.** Stop the running one, as Harvest clients expect.
  Today Horae refuses with a conflict. This changes the web app's behaviour too, because the
  operation is shared. Confirmed by the maintainer.
- **D-7 Cookie writes.** Writes need a token. Session-cookie reads stay for compatibility.
- **D-8 Upstream base-URL patches.** Not opened. The maintainer decided against pull requests in
  third-party projects. A suite client that needs a base-URL change gets it from a patch kept in
  this repository's test suite.
- **D-9 Recording reference responses.** Authorized: record reference shapes from the test
  (evidence) Harvest account, with GET requests only, including error responses. Recorded on
  `harvest-parity.md` as D-020. Recordings are scrubbed of account names, account ids, people and
  business data before they are committed.

### Key Entities

- **Personal access token**: belongs to one person in one organization. It has a name, a hash,
  a creation time, an optional expiry, a last-use time and a revocation time.
- **Account id**: an integer per organization, stable and never reused.
- **Integer id**: per organization, per kind of record, links one integer to one record. It is
  stable and never reused, and it records whether the number came from Harvest.
- **OAuth2 client** *(OQ-1)*: name, redirect addresses, secret hash, registering administrator.
- **OAuth2 grant** *(OQ-1)*: a person's approval of a client for an organization, with its
  single-use codes and its expiring, rotating access and refresh tokens, all stored as hashes.
- **Compatibility result**: per client and workflow, pass or fail, published with the contract.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After the first two delivery steps, at least 6 named clients pass the identity,
  picker and time workflows in the compatibility suite. They must include at least one library,
  one command-line timer and one editor or launcher extension.
- **SC-002**: A person who already uses a pointed client switches it to Horae in under 5 minutes,
  from opening their profile page to their first logged entry.
- **SC-003**: For every write the API accepts, the web app shows the same entry, totals and timer
  state. For every write it refuses, the web app would refuse the same action for the same reason
  (checked by the suite, 100% of cases).
- **SC-004**: A full sync of an organization with 100,000 time entries completes through a
  link-following client and through a page-counting client, with no record missing or repeated.
- **SC-005**: Every hour and money value returned converts back to Horae's stored integer without
  loss, and report totals match Horae's own reports exactly (checked by property tests over all
  minute values of a day and all currencies Horae supports).
- **SC-006**: A stolen copy of the database does not let anyone call the API as any person.
- **SC-007**: The published compatibility list names every tested client and its result, and is
  regenerated on every merge.

## Assumptions

- The first suite clients are those that build offline in Nix and can be pointed at Horae without
  a fork:

  - libraries: becoded/go-harvest, adlio/harvest, the minutes-harvest crate, jolicode or
    wearerequired PHP, and 3AP-AG Java;
  - tools with a dormant base-URL hook, enabled by a small recorded patch: harvest-tui,
    dedene/harvest-cli, ianaleck MCP server;
  - one-line patched clients: hrvst-cli and sheaves.

  The plan confirms which build offline.

- Harvest's error body shapes and its exact hours precision are not documented. The defaults in
  FR-024 and D-5 follow commonly observed behaviour and are verified before release with GET
  requests to the test account (D-9).

- Horae stays single-organization in deployment until 020 lands. FR-012 then takes effect with no
  change to the API's contract.

- The importer's record of original Harvest ids is complete for clients, projects, tasks, time
  entries and people imported since spec 004. Records imported before a kind was tracked get
  fresh numbers.

- Tokens are sufficient for every non-OAuth client in the survey. No client needs Harvest's
  identity host just to read the account list.

## Delivery order

Each step is one or a few small pull requests and leaves the API working. The order follows how
many surveyed clients each step unlocks ([research.md §1.5](research.md#15-most-called-endpoints)).

1. **Integer ids and account id** (FR-010, FR-015–FR-018). The numbering table, backfill,
   importer reuse of Harvest ids and responses in integers. The suite skeleton runs with one
   library client using session reads. Unlocks typed clients for reads.
1. **Personal access tokens** (FR-001–FR-008, FR-011, FR-024–FR-026, FR-050). Token management
   on the profile page, bearer and query authentication, Harvest error bodies, leniency aliases
   and rate limits. With step 1, every token-based client can read.
1. **Time-entry writes and the timer reads around them** (FR-020, FR-030–FR-032, FR-044 item 1).
   Create (both modes), update, delete, stop and restart (with restart added to the domain);
   `company`; `users/me/project_assignments`; exact unit conversion replacing floating point.
   Unlocks about 20 of the 23 end-user apps.
1. **Pagination and sync fidelity** (FR-022, FR-023, FR-040–FR-043). Page size 2000, cursors,
   absolute links, real update times on every kind, all time-entry filters and fields, empty
   lists for absent kinds. Unlocks pipelines and archive tools.
1. **Catalogue and report reads** (FR-044 items 2–4). Assignments, teammates, users by id,
   reports, invoices, tags.
1. **Catalogue writes and invoice status** (FR-033, FR-034).
1. **OAuth2 provider** (FR-060–FR-062), last, as OQ-1 decided.

Steps 1–3 relied on the constitution amendment (approved, 5.0.0) for writes (step 3) and for the
unit conversion (steps 1–3 replace today's floating-point output). The suite grows with each step
(FR-070–FR-073).

## Out of scope

- Harvest's own apps, browser extensions, Button and Widget embeds and hosted connectors. They
  cannot be pointed at Horae (research §1.2–1.3). Their private endpoints are not reproduced.
- New domain features: estimates, expenses (spec 016), contacts (spec 012), payments, roles,
  effective-dated rates, PTO, approvals API, reimbursements, e-invoicing.
- Harvest-format CSV exports for CSV-based importers (a separate, file-format question).
- Opening pull requests in third-party projects (D-8).
