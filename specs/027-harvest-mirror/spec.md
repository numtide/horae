# Feature Specification: Harvest mirror mode, parity checks and the durable jobs crate

**Feature Branch**: `spec/harvest-mirror` (documents); implementation in the PRs listed in the plan

**Created**: 2026-11-25

**Status**: Draft

**Input**: User description: "Keep the company working in Harvest while Horae mirrors it continuously
and proves, with automatic parity checks, that it computes the same results. Cut over only once
parity has been clean for an agreed period. Redesign the durable job queue as its own crate, because
the mirror, the parity checks and later work depend on it."

Builds on 004 (Harvest importer), 005 (durable jobs), 006 (jobs CLI), 007 (account switch), 019
(people, the Harvest section) and 025 (invoice import). Depends on the second delivery of 026 (task
rate templates) before the validation period starts.

## Background

Horae can import a Harvest account once, or again by hand, through durable jobs. A one-off import
does not let a company switch safely: people keep logging time in Harvest until the day they stop,
and nobody can show beforehand that Horae's reports and invoices would have come out the same.

The plan is to leave Harvest as the place where work is recorded for a while. Horae reads it every
few minutes, keeps a read-only copy, and compares its own results with Harvest's: record by record,
report totals by month, and invoice by invoice. When the comparison has been clean for long enough,
the company switches, Horae's copy becomes editable, and Harvest is kept as an archive.

Harvest's public API v2 lists every entity with an `updated_since` filter, so a mirror can read only
what changed. It has no deletion feed and no documented webhooks, so deletions are found by
comparing id lists. It allows 100 requests per 15 seconds in general and 100 per 15 minutes for the
Reports API, and answers 429 with a `Retry-After` header (research R-1).

The queue that runs imports today (`crates/server/src/jobs.rs`, specs 005 and 006) knows two kinds
of job, runs one at a time per worker, renews a five-minute lease and retries every kind the same
way. A once-a-minute loop in `scheduler.rs` handles long timers, budget alerts and OAuth cleanup.
The mirror needs scheduled, non-overlapping passes per organization, and the inventory of future
background work (research R-4) needs schedules in a person's time zone, per-kind retries and delayed
jobs. This feature moves the queue into its own crate first and adds those mechanisms there.

## Clarifications

### Session 2026-11-25 (decisions taken by the maintainer before specification)

- Mirror direction: Harvest wins. Horae reads Harvest and never writes to it, except in the optional
  write-through phase (FR-039).
- Sync method: a scheduled incremental pass per entity with `updated_since`, in dependency order,
  upserting by Harvest id with the importer's mapping code. One stored watermark per entity, which
  also makes a pass resumable. Not webhooks: they are undocumented for v2 and the production host
  is reachable only on the tailnet.
- A proxy in front of the official Harvest apps is impossible because the apps hard-code their
  hosts. A notify proxy for Horae's own clients is possible future work, not part of this feature.
- Deletions: a periodic reconciliation that compares id sets.
- The mirror may read the production Harvest account through the stored OAuth connection.
- Unsupported entities (expenses until spec 016 is implemented) are skipped and reported, never a
  failure.
- Parity runs as a server job in three layers (records, report totals, invoices), stores its results,
  classifies known divergences by explicit rules, and counts a difference only when it persists across
  two consecutive runs.
- The durable queue moves into a new crate, `horae-jobs`, with no external queue library. It takes
  five ideas from `underway` 0.2.0 (concurrency keys, heartbeats, per-kind retry policies with an
  attempts table and delayed jobs, a single-leader scheduler with time zones, and optionally
  LISTEN/NOTIFY), written anew. Multi-step jobs are not needed.
- Rollout in four phases (see Rollout). Write-through is optional, per person, and needs the
  maintainer's explicit authorization because it writes to the production Harvest account.
- No Horae to Harvest sync after cutover. Rolling back in the first days means going back to Harvest
  and importing again.

### Session 2026-11-26

The maintainer accepted the recommendation on each of the four questions left open at
specification.

- **OQ-1**: Are mirrored records read only in Horae while the mirror is on? → A: Yes. Everything
  Harvest owns is read only (FR-020 to FR-023), so parity can only find differences that Horae
  computed, never ones someone typed.
- **OQ-2**: How often does the mirror pass run? → A: Every 5 minutes, configurable from 1 to 60
  minutes (FR-011). A pass with few changes uses a handful of requests, far below the limit.
- **OQ-3**: What ends the validation period? → A: Four consecutive weeks with zero unexplained
  differences, one complete month close in which the reports and the invoices reconcile, and zero
  unexplained mirror errors over the same weeks (SC-005).
- **OQ-4**: Who in the company checks Horae, read only, during validation? → A: The person who closes
  the month in Harvest (invoicing) and one project manager, so the reports and invoices they use
  every month are also compared by eye.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Background work keeps behaving the same after the queue moves (Priority: P1)

An administrator runs a Harvest API import, a CSV import, a preview and its commit, cancels one and
retries another, from the web app and from `horae jobs`. Everything behaves as before the queue
moved into its own crate: same states, same reports, same error downloads, same exit codes.

**Why this priority**: every later story runs on the new crate. Moving it without changing behaviour
first keeps the risky part of the change small and reviewable.

**Independent Test**: run the existing job, importer and CLI test suites unchanged against the
extracted crate; they pass without edits to their assertions.

**Acceptance Scenarios**:

1. **Given** the extracted crate, **When** the jobs, importer and CLI tests of specs 005, 006 and
   008 run, **Then** they pass with their assertions unchanged.
1. **Given** a job enqueued before the upgrade, **When** the upgraded server starts, **Then** it
   claims and finishes the job as the old server would have.

______________________________________________________________________

### User Story 2 - Horae stays current with Harvest (Priority: P1)

An administrator turns the mirror on from the Harvest section. From then on, every few minutes,
Horae reads what changed in Harvest (people, clients, projects, tasks, assignments, time entries and
invoices) and updates its copy. A time entry logged in Harvest at 10:02 is in Horae's reports by
10:10.

**Why this priority**: it is what lets the company keep working in Harvest while Horae is validated.

**Independent Test**: against a Harvest test double, turn the mirror on, change records of every
entity in the double, and check that the next pass brings each change into Horae and that a pass
with no changes writes nothing.

**Acceptance Scenarios**:

1. **Given** the mirror is on, **When** a time entry is created, edited or approved in Harvest,
   **Then** the next pass creates or updates it in Horae, and it is in Horae's reports.
1. **Given** a new project with new tasks and assignments created in Harvest, **When** the next pass
   runs, **Then** the project, its tasks and task rates, its managers and its per-person budgets exist
   in Horae before any time entry that references them is applied.
1. **Given** a pass with nothing changed in Harvest, **When** it runs, **Then** it reads only the
   changed-since lists, writes nothing to Horae's records and records the pass as succeeded.
1. **Given** a pass interrupted halfway (server restart), **When** the next pass starts, **Then** it
   continues from the watermarks of the entities already finished and repeats no write.
1. **Given** Harvest answers 429, **When** the pass is running, **Then** it waits the time in
   `Retry-After` and continues, and the status page shows the wait.

______________________________________________________________________

### User Story 3 - Mirrored records are read only and say so (Priority: P1)

While the mirror is on, anyone opening a client, project, task, person or time entry that came from
Harvest sees "Synced from Harvest", and Horae refuses edits to what Harvest owns. Horae's own
settings (permissions, API tokens, the organization's settings) stay editable.

**Why this priority**: an edit in Horae would be overwritten by the next pass, and it would make parity
report differences that nobody computed. (OQ-1.)

**Independent Test**: with the mirror on, try every edit path (web app, `/api/v1`, `/harvest/v2`) on a
mirrored record of each kind; each is refused with the same message, and nothing changes.

**Acceptance Scenarios**:

1. **Given** the mirror is on, **When** a person opens a mirrored time entry, **Then** it is marked
   "Synced from Harvest" and offers no edit, delete or timer action.
1. **Given** the mirror is on, **When** a program calls `/api/v1` to edit a mirrored project,
   **Then** the call is refused with a conflict that names the mirror, and nothing changes.
1. **Given** the mirror is on, **When** an administrator changes a person's permissions in Horae,
   **Then** the change is accepted, because permissions are Horae's.
1. **Given** the mirror is off (after cutover), **When** a person edits a record that came from
   Harvest, **Then** the edit is accepted and the record no longer says "Synced from Harvest".

______________________________________________________________________

### User Story 4 - An administrator can see and control the mirror (Priority: P2)

The mirror's status page shows when the last pass ran and how long it took, how far behind Harvest
Horae is, the counts of what each pass created, updated and skipped, the errors of the last passes,
and the latest parity result. The administrator can pause and resume the mirror, start a pass now,
and start a full pass.

**Why this priority**: during weeks of validation, someone has to notice a stopped mirror or a
growing error count without reading logs.

**Independent Test**: run passes with and without errors against the test double; check the page's
figures against the jobs table; pause, check that no pass starts, resume, and check that the next
pass catches up.

**Acceptance Scenarios**:

1. **Given** a pass finished at 10:05 and the mirror runs every 5 minutes, **When** the page is
   opened at 10:07, **Then** it shows the pass, its counts and a lag of at most 7 minutes.
1. **Given** the connection to Harvest was revoked, **When** the next pass runs, **Then** the
   mirror pauses itself, the page says the connection needs to be renewed, and no further pass
   starts until it is.
1. **Given** a paused mirror, **When** the administrator resumes it, **Then** the next pass starts
   within one interval and reads everything changed during the pause.
1. **Given** any person without the Administrator identity, **When** they open the page or call its
   functions, **Then** Horae refuses.

______________________________________________________________________

### User Story 5 - Parity proves that Horae computes what Harvest computes (Priority: P1)

Every night, and whenever an administrator asks, Horae compares itself with Harvest in three layers:
each record, the report totals by month, and each invoice. The result is stored with its counts per
class and a downloadable CSV of every difference. Differences that a known rule explains, such as
invoice rounding, are classified apart. A difference that appears in two consecutive runs and no rule
explains raises an alert.

**Why this priority**: it is the evidence the company needs before switching, and the reason to run
the mirror at all.

**Independent Test**: plant one difference of each kind in a test database (a missing entry, an extra
entry, a changed minute count, a changed rate, a moved invoice link, a report total off by one cent,
an invoice total off by a rounding unit) and check that a run detects each one in the right class.

**Acceptance Scenarios**:

1. **Given** a time entry with 90 minutes in Harvest and 75 in Horae, **When** parity runs twice
   with no change in between, **Then** the second run lists it as different, naming the minutes and
   both values, and the alert is raised.
1. **Given** a difference seen once and gone in the next run, **When** the second run finishes,
   **Then** the difference is not counted as unexplained and raises no alert.
1. **Given** an invoice whose total differs from Harvest's by less than its line count times half a
   minor unit, and whose Horae total equals the sum of its rounded lines, **When** parity runs,
   **Then** the invoice is classified as a known rounding divergence, not a failure.
1. **Given** a record changed in Harvest after the last finished mirror pass, **When** parity runs,
   **Then** that record is not compared in this run and is counted as in flight.
1. **Given** a parity run, **When** the administrator downloads its detail, **Then** the CSV lists
   every non-equal finding with its layer, entity, ids, class, rule, fields and both values.
1. **Given** a run, **When** an administrator runs `horae jobs report` on its job, **Then** the
   counts per class are printed, and `horae jobs findings` writes the CSV.

______________________________________________________________________

### User Story 6 - Deletions in Harvest reach Horae (Priority: P2)

Someone deletes a time entry in Harvest. Within the reconciliation period (hourly by default), Horae
finds it missing from Harvest's id list and removes its copy. A client or project that disappears is
archived rather than removed, because Horae records may still point to it.

**Why this priority**: without it the mirror drifts upward forever, and parity reports the same
"extra in Horae" rows every night.

**Independent Test**: delete records of each kind in the test double, run the reconciliation, and
check what is removed, archived and reported.

**Acceptance Scenarios**:

1. **Given** a mirrored time entry deleted in Harvest, **When** the reconciliation runs, **Then** the
   entry is removed from Horae and counted as deleted.
1. **Given** a mirrored project deleted in Harvest that still has time in Horae, **When** the
   reconciliation runs, **Then** the project is archived in Horae and the report says why it was not
   removed.
1. **Given** a deletion not yet reconciled, **When** parity runs, **Then** the record is classified
   as a known divergence (deletion pending), not as extra in Horae.

______________________________________________________________________

### User Story 7 - Validation and cutover (Priority: P2)

The company runs the mirror on the staging deployment (shipyard) for the validation period. When the
exit criterion holds (OQ-3), the administrator runs a final pass and a final parity run, and turns the
mirror off. From that moment Horae's records are editable and are the source; Harvest is kept as an
archive and is no longer written to.

**Why this priority**: it is how the company leaves Harvest. It needs the stories above first.

**Independent Test**: on the test double, run the cutover command after a clean parity run and check
that records become editable, that no further pass is scheduled, and that the cutover is refused
while the last parity run has unexplained differences unless the administrator overrides it.

**Acceptance Scenarios**:

1. **Given** the last parity run is clean and was taken after the last pass, **When** the
   administrator confirms the cutover, **Then** the mirror is off, no pass or reconciliation is
   scheduled, and mirrored records are editable.
1. **Given** the last parity run has unexplained differences, **When** the administrator starts the
   cutover, **Then** Horae refuses unless they confirm a second time, naming the count, and the
   override is recorded in the audit log.
1. **Given** a cutover, **When** it finishes, **Then** the audit log records who did it, the last
   watermarks and the parity run it relied on.

______________________________________________________________________

### User Story 8 - Pilot people log time in Horae while Harvest stays the source (Priority: P3)

With the maintainer's authorization, an administrator turns on write-through for a few pilot people.
When a pilot person creates, edits or deletes a time entry, or starts or stops a timer in Horae,
Horae makes the change in Harvest first, through the stored connection, on that person's behalf. If
Harvest refuses, Horae refuses too and shows Harvest's reason. Everyone else keeps using Harvest.

**Why this priority**: optional. It lets a few people use Horae daily before the cutover without
breaking the rule that Harvest is the source.

**Independent Test**: on the Harvest test account, with disposable fixtures recorded in
`specs/harvest-parity.md`, turn write-through on for one person, perform each action in Horae,
check it in Harvest, then make Harvest refuse one (a locked entry) and check that Horae refuses it.

**Acceptance Scenarios**:

1. **Given** write-through is on for a person, **When** they create a time entry in Horae, **Then**
   it exists in Harvest under their Harvest user, Horae stores its Harvest id, and the next mirror
   pass counts it as unchanged.
1. **Given** Harvest refuses the change (for example, the entry is locked), **When** the person saves
   it in Horae, **Then** nothing changes in Horae and the message gives Harvest's reason.
1. **Given** Harvest accepted the change but Horae failed to save it, **When** the next mirror pass
   runs, **Then** Horae gets the entry from Harvest, so neither side is left with a change the other
   lacks.
1. **Given** a person without write-through, **When** they try to edit a mirrored entry, **Then** the
   read-only rule of User Story 3 applies.

______________________________________________________________________

### User Story 9 - Background work runs once, on time, and recovers fast (Priority: P2)

Operators and later features need jobs that do not run twice for the same purpose, that are picked up
again soon after a server dies mid-run, that retry each kind of failure on its own schedule, that can
wait until a given time, and schedules that fire at a local time in an organization's or a person's
time zone.

**Why this priority**: the mirror needs non-overlapping passes per organization and fast recovery;
reminders and deliveries in the inventory (research R-4) need the rest. It is built before the
mirror, after the extraction.

**Independent Test**: unit and database tests in the jobs crate: enqueue two jobs with the same
concurrency key, kill a worker mid-run, fail a job with a policy, enqueue a delayed job, and step a
schedule across a daylight-saving change.

**Acceptance Scenarios**:

1. **Given** a mirror pass queued or running for an organization, **When** the scheduler or an
   administrator asks for another, **Then** no second pass is created and the caller gets the
   existing one.
1. **Given** a worker that stops sending heartbeats mid-pass, **When** about a minute has passed,
   **Then** another worker claims the job, and a pass that keeps sending heartbeats is never claimed
   again however long it runs.
1. **Given** a kind whose policy is 5 attempts starting at 10 seconds and doubling up to 10 minutes,
   **When** it keeps failing, **Then** the attempts run at about 10 s, 20 s, 40 s and 80 s after each
   failure, each attempt is recorded with its error, and the fifth failure is final.
1. **Given** a schedule "every day at 09:00 in Europe/Madrid", **When** the clocks change, **Then** it
   still fires at 09:00 local time, once a day, and only one server instance enqueues it.

### Edge Cases

- A pass finds a time entry whose project has not been mirrored yet (created in Harvest after the
  projects step started): the entity order makes this rare; the entry is retried in the next pass,
  not reported as an error, and the time-entry watermark is not advanced past it.
- Harvest's `updated_since` inclusivity is undocumented: the watermark overlaps the previous pass by
  a margin, and re-reading a record is a skip (research R-2).
- A Harvest record changed during a pass: the next pass reads it again because the watermark is the
  pass's start minus the margin, not its end.
- An expense, or any entity Horae has no model for: skipped and counted as unsupported in the pass
  report and in parity; never a failure.
- The OAuth token expires mid-pass: refreshed as the importer already does; a refused refresh pauses
  the mirror (User Story 4, scenario 2).
- The administrator who turned the mirror on loses the Administrator identity or is deactivated: the
  mirror pauses with that reason, and another administrator must resume it (passes run on that
  person's authority).
- A manual import is started while the mirror is on: refused, with a pointer to "Start a full pass",
  so two writers never run.
- The organization tries to switch Harvest accounts while the mirror is on: refused, as with any
  provenance today (007).
- A record Harvest reports with a value Horae refuses (an invalid currency, an unknown time zone):
  the record is an error in the pass report, the rest of the pass continues, and the record's entity
  watermark is held back so the next pass tries it again.
- Parity's report layer needs more requests than the Reports API allows in one run: the job paces
  itself under the limit and, for long histories, spreads older months over several nights
  (FR-035).
- Two server instances: only one runs the scheduler at a time; workers on both may claim jobs, and a
  concurrency key still allows one pass per organization.
- Clock skew between Horae and Harvest: the watermark margin covers skews smaller than itself; a
  skew larger than the margin is detected by comparing Harvest's `Date` header and reported on the
  status page.
- Write-through when Harvest is unreachable: the pilot person's change is refused; nothing is
  queued for later (Harvest stays the source).
- A pilot person's entry on a project they are not assigned to in Harvest: Harvest refuses it and so
  does Horae.

## Requirements *(mandatory)*

### Functional Requirements

**Durable jobs crate**

- **FR-001**: The durable queue, its worker, its outbox and its reports MUST move into a crate of
  their own that holds only the mechanism: a job kind, a versioned JSON payload, enqueue inside the
  caller's transaction, claim, progress and checkpoints, retries, cancel, status, list, the outbox,
  and a worker that runs registered handlers. Knowledge of imports, Harvest and the mirror MUST stay
  in the server crate, which registers one handler per kind.
- **FR-002**: The move MUST NOT change any behaviour that specs 005, 006, 007 and 008 define: job
  states, reports, error downloads, idempotency, account generations, retry availability, the CLI and
  its exit codes. Queue tables keep their names and contents; any migration is additive.
- **FR-003**: The crate MUST NOT depend on a queue library, and its tables MUST follow the
  constitution: UUID v7 keys, an `org_id` on every row, migrations in the server's migrations folder.
- **FR-004**: The build time of the server and its tests MUST be measured before and after the
  extraction, with the same commands, and the numbers recorded in the PR.

**Job mechanisms**

- **FR-005**: A job MAY carry a concurrency key. At most one job per organization, kind and key MAY
  be queued or running at a time; enqueuing another MUST return the existing job instead of
  creating one.
- **FR-006**: A running job MUST record a heartbeat at a short interval. A job whose heartbeat is
  older than its stale limit MUST become claimable again; a job whose heartbeat is current MUST NOT,
  however long it runs. The stale limit MUST be well under the current five-minute lease (about one
  minute).
- **FR-007**: Each kind MUST declare a retry policy: maximum attempts, first delay, multiplier and
  maximum delay. Every attempt MUST be recorded with its start, end, outcome and error. A handler
  MUST be able to mark an error as permanent, which ends the job without further attempts.
- **FR-008**: A job MAY be enqueued to run no earlier than a given time, and MAY carry a priority;
  claims take the highest priority first, then the earliest time.
- **FR-009**: A scheduler MUST enqueue jobs from stored schedules: every N seconds, daily at a local
  time, or weekly on a day at a local time, in UTC, in the organization's time zone or in a person's
  time zone. Only one server instance MUST run the scheduler at a time. Each firing MUST be enqueued
  at most once, keyed by the schedule and its due time, and a firing missed while no instance ran
  MUST be enqueued once when the scheduler resumes, not once per missed slot.
- **FR-010**: The existing periodic work (long-timer detection, budget alerts, OAuth cleanup) MUST
  move onto the scheduler with unchanged behaviour.

**Mirror**

- **FR-011**: An administrator MUST be able to turn the mirror on for the organization's connected
  Harvest account, and set its pass interval between 1 and 60 minutes, default 5 (OQ-2). Turning it
  on MUST require a completed full import of that account and spec 026's second delivery.
- **FR-012**: A pass MUST read, with `updated_since`, each entity in this order: people, clients,
  projects, tasks, task assignments, user assignments, time entries, invoices. It MUST upsert by
  Harvest id with the importer's mapping and conversions (hours to integer minutes, amounts to minor
  units with their currency), so that a record imported once and mirrored afterwards is the same
  record. Assignment fields Horae has no place for are counted as unsupported fields, not errors.
- **FR-013**: Each entity MUST have one stored watermark. A pass MUST advance an entity's watermark
  only when that entity's step finished without errors that hold it back, to the time the step
  started minus a safety margin. A pass interrupted part way MUST resume from the stored watermarks.
- **FR-014**: A pass MUST NOT write a record that is unchanged since Horae last wrote it, and MUST
  count it as skipped.
- **FR-015**: Passes MUST stay under Harvest's limits: at most 100 requests per 15 seconds for
  general endpoints and 100 per 15 minutes for reports, shared by every job that calls the same
  Harvest account from the same server. On 429 a pass MUST wait for `Retry-After` and continue.
- **FR-016**: Harvest entities Horae does not model (expenses until spec 016 lands, estimates,
  retainers) MUST be skipped and counted as unsupported, never fail a pass.
- **FR-017**: A pass MUST run under the authority of the administrator who turned the mirror on, as
  imports run under the administrator who started them. If that person is no longer an active
  administrator, or the Harvest connection is refused, the mirror MUST pause itself and record why.
- **FR-018**: While the mirror is on, manual Harvest imports and account switches MUST be refused.
  An administrator MAY start an incremental or a full pass at any time; a full pass ignores the
  watermarks and reads every entity.

**Deletions**

- **FR-019**: A reconciliation job MUST, hourly by default (configurable to daily), list every Harvest
  id of each mirrored entity and compare it with Horae's provenance. A time entry or invoice whose id
  is gone MUST be removed from Horae, its time reopened as spec 025 does for a removed invoice. A
  person, client, project or task whose id is gone MUST be archived (deactivated) and reported, never
  removed. Assignments whose ids are gone MUST be removed. Records not mirrored from Harvest MUST
  NOT be touched.

**Read only while mirrored** (OQ-1)

- **FR-020**: While the mirror is on, every operation that changes a field Harvest owns on a mirrored
  record, or creates a new record of a mirrored kind, MUST be refused with one error that names the
  mirror, on every surface (server functions, `/api/v1`, `/harvest/v2`). The check MUST live in the
  shared operations, so no surface can skip it. The mirror's own writes and write-through (FR-039)
  are the only exceptions.
- **FR-021**: Data Horae owns MUST stay editable: permissions, profiles, sign-in bindings, API tokens
  and OAuth clients, organization settings, plugins, notification preferences.
- **FR-022**: Each mirrored record MUST be marked "Synced from Harvest" wherever it is shown with
  edit actions, and those actions MUST be hidden or disabled with the same reason.
- **FR-023**: When the mirror is turned off, every record MUST become editable at once, with no
  migration or rewrite of the records.

**Status and control**

- **FR-024**: The mirror's status page MUST show: whether the mirror is on, paused (and why) or off;
  the interval; the last pass's start, duration, outcome and counts per entity (created, updated,
  skipped, unsupported, errored); the lag (now minus the oldest entity watermark); the errors of the
  last passes; the next scheduled pass and reconciliation; any wait for Harvest's rate limit; and the
  latest parity run with its counts.
- **FR-025**: An administrator MUST be able to pause, resume, start a pass now, start a full pass,
  change the interval and start a parity run from the page and from the CLI. Every one of these
  actions MUST be recorded in the audit log.
- **FR-026**: Only administrators MAY see the page or call its functions.

**Parity checks**

- **FR-027**: A parity job MUST run nightly (at a configurable local time in the organization's time
  zone, default 02:30) and on demand, after the mirror's last pass has finished, and compare against
  that pass's watermarks: a record changed in Harvest after them MUST be counted as in flight and not
  compared.
- **FR-028**: Layer 1, records: for people, clients, projects, tasks, assignments, time entries and
  invoices, the job MUST list each entity from Harvest and from Horae, normalize both with the
  importer's conversions, pair them by Harvest id, and classify each pair as equal, missing in Horae,
  extra in Horae or different. A different pair MUST name each differing field with both values.
  Time entries compare date, person, project, task, minutes, billable, rate, notes, locked or
  approved state, invoiced state and invoice.
- **FR-029**: Layer 2, totals: for each month in range, the job MUST compare Harvest's time reports
  by client, project, task and person, and its uninvoiced report, with the same aggregations computed
  by the operations behind Horae's reports: hours, billable hours and billable amount per currency.
- **FR-030**: Layer 3, invoices: for each invoice the job MUST compare number, client, state, issue
  and due dates, subtotal, discount, taxes, total, amount due, line count and the set of linked time
  entries.
- **FR-031**: Differences that a known rule explains MUST be classified as known divergences with the
  rule's name, not as failures:
  - rounding: an invoice or report amount whose difference from Harvest is at most the number of
    lines (or entries) times half a minor unit, and whose Horae value equals the sum of Horae's
    rounded lines (spec 026, FR-023);
  - unsupported: amounts that come from entities Horae does not model (expenses);
  - deletion pending: a record gone from Harvest since the last reconciliation;
  - in flight: a record changed after the compared watermark (FR-027).
- **FR-032**: A difference MUST count as unexplained only when the same finding (same layer, entity,
  ids, class and values) appears in two consecutive runs. A first sighting MUST be shown as pending.
- **FR-033**: Each run MUST be stored with its date, the watermarks it compared, its counts per class
  and per layer, and a detail of every finding that is not equal, downloadable as CSV. Runs and their
  counts MUST be kept for at least a year; their detail for at least 90 days.
- **FR-034**: When a run has unexplained differences, Horae MUST show an alert to administrators on
  the status page and in the app's navigation, and send an email to administrators when mail is
  configured. A clean run after an alert MUST clear it.
- **FR-035**: The report layer MUST stay under the Reports API limit. A nightly run MUST cover the
  current and previous three months; older months MUST be covered in rotation so the whole history is
  compared at least once a month.
- **FR-036**: The checker MUST be tested by planting known differences of every class and layer and
  asserting that each is found, and by a rehearsal against the Harvest test account with disposable
  fixtures whose authorization and ids are recorded in `specs/harvest-parity.md` and cleaned up
  afterwards.

**Rollout and cutover**

- **FR-037**: Turning the mirror off for the cutover MUST run a last pass and a last parity run first,
  and MUST refuse while that run has unexplained differences unless the administrator confirms a
  second time with the count shown. The cutover, the watermarks and the parity run it relied on MUST
  be recorded in the audit log.
- **FR-038**: After the cutover Horae MUST NOT write to Harvest or read it on a schedule. Imported
  records keep their provenance, so a rollback (going back to Harvest and importing again) needs no
  special path.

**Write-through** (phase 3, optional)

- **FR-039**: Write-through MUST be off unless the operator enables it for the deployment and an
  administrator enables it for a named person. It MUST cover only time entries and timers: create,
  edit, delete, start and stop.
- **FR-040**: For a pilot person, each covered change MUST first be made in Harvest through the
  stored connection, naming the person's Harvest user, and only then in Horae, storing the Harvest id
  Harvest returned. If Harvest refuses or cannot be reached, Horae MUST refuse the change and show
  Harvest's reason.
- **FR-041**: Write-through MUST be tried first on the Harvest test account, and MUST be enabled
  against the production account only with the maintainer's explicit authorization, recorded in
  `specs/harvest-parity.md`.

### Key Entities

- **Job**: a unit of background work with a kind, a payload, a state, an optional concurrency key, a
  priority, a run-after time, a heartbeat and its progress. Exists today; gains the new fields.
- **Job attempt**: one execution of a job: number, worker, start, end, outcome, error.
- **Schedule**: a stored rule that enqueues a kind with a payload: its cadence, its time zone source
  (UTC, organization, person), the next due time and the last firing.
- **Mirror**: the organization's mirror settings and state: on, paused or off, the pause reason, the
  interval, the administrator whose authority passes run under, and the cutover record.
- **Entity watermark**: per entity, the time up to which Harvest's changes have been applied. The
  importer already stores these per connection; the mirror uses the same store.
- **Parity run**: one comparison: when, which watermarks, counts per class and layer, and its
  findings.
- **Parity finding**: one non-equal result: layer, entity, Harvest and Horae ids or the aggregate's
  key (month and client, project, task or person), class, rule, differing fields with both values,
  and the run it was first seen in.
- **Write-through enrollment**: a person for whom write-through is on, who enabled it and when.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A change made in Harvest appears in Horae within two pass intervals (10 minutes at the
  default) in 99% of passes over a week.
- **SC-002**: A pass with no changes in Harvest completes in under 30 seconds and uses no more than
  one request per entity.
- **SC-003**: Parity finds every planted difference in the test suite (100%), and reports zero
  unexplained differences on an account that was imported and then left unchanged.
- **SC-004**: After a server is killed mid-pass, another worker resumes the pass within 2 minutes,
  and no record is written twice.
- **SC-005**: The validation period ends only when the exit criterion holds (OQ-3): four consecutive
  weeks with zero unexplained differences, one complete month closed with reports and invoices
  reconciled, and zero unexplained mirror errors.
- **SC-006**: No edit to a mirrored record succeeds through any surface while the mirror is on, apart
  from write-through.
- **SC-007**: Every existing job, importer and CLI test passes unchanged after the extraction, and the
  measured incremental rebuild time of the server does not grow by more than 10%.

## Rollout

1. **Mirror and parity, read only.** Ship the jobs crate (US1, US9), then the mirror, deletions,
   read-only marking and status page (US2 to US4, US6), then the parity checker (US5).
1. **Validation period on shipyard.** Take a Postgres backup routine in place first (needed before
   the cutover, not before the mirror). Reset the database, run a full import of the production
   account, turn the mirror on. The person who closes the month in Harvest and one project manager check Horae read
   only (OQ-4). The period ends when
   the exit criterion holds (OQ-3).
1. **Write-through, optional** (US8). Test account first; then, with the maintainer's authorization,
   a few pilot people on the production account; time entries and timers only.
1. **Cutover** (US7). Final pass, clean parity, mirror and write-through off, records editable.
   Harvest stays as a read-only archive. Rolling back in the first days means going back to Harvest
   and importing again; time recorded in Horae after the cutover would have to be re-entered there.

## Assumptions

- Spec 025 (invoice import) has landed, so invoices and their time links can be mirrored and
  compared. Payments are not records in Horae; an invoice's amount due and paid date are mirrored
  with the invoice, and a payment in Harvest changes the invoice's `updated_at` (to be confirmed in
  the rehearsal, research R-2).
- The stored Harvest connection belongs to a Harvest administrator, who can read invoices, every
  person's time and the billable amounts in reports.
- One Horae server instance runs on shipyard; the design still holds with more (FR-009, FR-015).
- Expenses remain unsupported until spec 016 is implemented.
- A notify proxy for Horae's own clients, calendar and accounting syncs, reminders and recurring
  invoices are out of scope; they appear only in the background work inventory (research R-4).
- LISTEN/NOTIFY wake-ups are optional and may be left out of this feature.
