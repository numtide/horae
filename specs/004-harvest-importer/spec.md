# Feature Specification: Harvest Data Importer

**Feature Branch**: `004-harvest-importer`

**Created**: 2026-09-01

**Status**: Draft

**Input**: User description: "Harvest data importer. Let an org admin migrate their existing Harvest data into Horae — clients, projects, tasks, and time entries — so teams switching from Harvest don't start empty. The primary source is a live pull from Harvest's own REST API over OAuth2; a Harvest CSV export is kept as a secondary, offline adapter feeding the same engine. Map Harvest records to Horae's model honoring the domain invariants: durations as integer minutes, money as integer minor units (cents) + ISO currency code (never floats), UUID v7 primary keys, single org (every row carries org_id). Import in FK-safe order: clients → projects → tasks → time entries. The importer MUST be idempotent (re-running does not duplicate), MUST offer a dry-run that reports what would be created/updated/skipped without writing, and MUST surface a clear summary plus per-row errors that don't abort the whole import. Persist Harvest source identifiers as provenance so matching is exact and edit-robust and incremental re-sync is possible. Note: Horae already ships a read-only Harvest-compatible API at /harvest/v2/\* (see crates/horae/src/harvest/) that emits Horae→Harvest JSON; this feature consumes Harvest's real API in the opposite direction (Harvest→Horae) and inverts those same transforms."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect Harvest and pull a migration over the API (Priority: P1)

An organization administrator who is switching from Harvest connects their Harvest account to Horae with one click: Horae sends them through Harvest's OAuth2 sign-in, they authorize the connection, and Horae stores the resulting access so it can read their Harvest data on their behalf. The administrator then runs the import; Horae calls Harvest's REST API and pulls the clients, projects, tasks, users (for reference only), and time entries, creating the corresponding Horae records in dependency order and reporting how many of each were created. When the administrator opens Horae afterward, their historical time sits under the same clients, projects, and tasks it did in Harvest, so the team continues where they left off instead of starting from an empty install — with no file to export or upload.

**Why this priority**: This is the entire reason the feature exists, and pulling directly from the live API is the flow a migrating admin will actually reach for first. It gives Horae stable Harvest record identifiers (impossible to get reliably from a CSV), which in turn make matching exact and enable safe re-runs. It is the smallest slice that delivers standalone migration value; every other story refines or protects it.

**Independent Test**: Connect a Harvest account (OAuth2) on a freshly seeded organization with no clients; run the import; confirm that clients, projects, tasks, and time entries appear with correct names, dates, durations (in exact minutes), billable flags, and monetary rates/amounts (in exact minor units), and that the reported created-counts match what the Harvest account contained.

**Acceptance Scenarios**:

1. **Given** an administrator with no Harvest connection, **When** they start "Connect Harvest", **Then** they are taken through Harvest's OAuth2 authorization and, on returning, Horae has stored a usable connection (access + refresh) bound to their Harvest account and organization, and no Harvest data is read until they explicitly run an import.
1. **Given** a connected Harvest account and a valid dataset, **When** the administrator runs the import, **Then** clients are pulled and created first, then projects under their clients, then tasks, then time entries referencing them, and no row is created before the rows it depends on.
1. **Given** a Harvest time entry recording 1.5 hours, **When** it is imported, **Then** the resulting Horae entry stores 90 minutes exactly (never a floating-point hours value).
1. **Given** a Harvest record carrying a billable rate and currency, **When** it is imported, **Then** the amount is stored as integer minor units with the ISO 4217 currency code, and the stored value re-derives the original amount without rounding drift.
1. **Given** a completed import, **When** the administrator reviews the summary, **Then** it reports the count created, updated, and skipped for each entity type (clients, projects, tasks, time entries).
1. **Given** a Harvest account with more records than fit in one API response, **When** the import runs, **Then** the importer follows Harvest's pagination to completion and respects Harvest's rate limits (backing off and retrying rather than failing) so the full dataset is pulled.

______________________________________________________________________

### User Story 2 - Re-sync without creating duplicates (Priority: P1)

An administrator runs an import, then later re-runs it — because more time was logged in Harvest during the cut-over, or a few rows failed the first time, or a record was edited in Harvest after the first pull. The second run recognizes the records that already exist by their Harvest identity, leaves them alone (or updates them in place), and adds only the genuinely new rows. An incremental re-sync pulls just what changed in Harvest since the last successful run. Running the import twice produces the same result as running it once.

**Why this priority**: A migration that duplicates data on a second attempt is worse than useless — it forces manual cleanup and destroys trust. Idempotency is what makes the import safe to retry, which real migrations always require, and cut-overs are rarely a single instant. Because the API gives every Harvest record a stable identifier, Horae can match exactly (via stored provenance) even when a record was edited after the first import. It ships alongside P1 because the first import is not safe to offer without it.

**Independent Test**: Run an import, record the resulting counts, then run it again unchanged; confirm the second run creates zero new clients, projects, tasks, or time entries and reports them all as skipped or unchanged, with no duplicate rows. Then edit one time entry's notes in Horae, re-run, and confirm it is still recognized as the same entry (matched by Harvest identity, not by a now-diverged natural key).

**Acceptance Scenarios**:

1. **Given** a client already imported from Harvest, **When** an import references the same Harvest client again, **Then** it is matched by its stored Harvest identity, no second client is created, and the existing one is reused for its projects.
1. **Given** an unchanged Harvest dataset imported a second time, **When** the run completes, **Then** the created counts are zero for every entity type and the totals in Horae are unchanged.
1. **Given** a previously imported time entry that was edited in Horae after import (e.g. its notes changed), **When** the same Harvest entry is imported again, **Then** it is matched to the existing Horae entry by Harvest identity — not duplicated — even though a pure natural-key match would now differ.
1. **Given** an incremental re-sync since the last successful run, **When** it runs, **Then** only Harvest records changed since that point are fetched and applied, and unchanged records are left intact.

______________________________________________________________________

### User Story 3 - Preview an import before committing (dry-run) (Priority: P2)

Before writing anything, the administrator runs the import in dry-run mode. Horae pulls (or reads) the source, resolves every record against existing data, and reports exactly what a real run would create, update, and skip — and which records would error — without persisting a single change. The administrator reviews the preview, fixes what is needed, and only then runs it for real.

**Why this priority**: Migrations are high-stakes and irreversible-feeling; a dry-run lets an administrator gain confidence and catch mapping problems before touching real data. It depends on the pull/parse and matching logic of P1/P2 but is a distinct, separately valuable capability.

**Independent Test**: Run a dry-run against a source containing a mix of new and already-existing records plus a few problem records; confirm the reported would-create/would-update/would-skip/would-error counts, verify that no data was written (Horae's row counts are unchanged, and no new provenance rows were persisted), then run for real and confirm the actual outcome matches the preview.

**Acceptance Scenarios**:

1. **Given** a valid source, **When** the administrator runs a dry-run, **Then** the summary reports would-create, would-update, would-skip, and would-error counts per entity type and no rows are written to Horae.
1. **Given** a dry-run and an immediately following real run on the same unchanged source and data, **When** both complete, **Then** the real run's created/updated/skipped counts match the dry-run's preview.
1. **Given** a dry-run that reports records that would error, **When** the administrator inspects the report, **Then** each problem record is identified with its source location (a Harvest record id, or a CSV line) and the reason it would fail.

______________________________________________________________________

### User Story 4 - Survive bad records with a per-record error report (Priority: P2)

The administrator's source contains a handful of problem records — an entry whose user email is not a Horae user, a value that will not convert, a project with no resolvable client. Rather than aborting the whole import, Horae imports every record it can, skips the ones it cannot, and hands back a per-record error list naming each failed record and why it failed, so the administrator can fix just those and re-run.

**Why this priority**: Real datasets are messy. An all-or-nothing import that dies on record 4,000 of 10,000 is unusable at migration scale. Partial success with a precise error report is what makes a large migration tractable. It builds on P1 but is independently demonstrable.

**Independent Test**: Import a source where a known subset of records is invalid; confirm the valid records are imported, the invalid records are skipped (not partially written), the summary counts reconcile (created + updated + skipped + errored = total processed), and each errored record is reported with its source location and a clear reason.

**Acceptance Scenarios**:

1. **Given** a source with some invalid records, **When** the import runs, **Then** valid records are imported and each invalid record is skipped without aborting the run.
1. **Given** a time-entry record whose user cannot be matched to an existing Horae user, **When** it is processed, **Then** that record is reported as an error with its identifying detail and the run continues.
1. **Given** a failed record, **When** the run completes, **Then** the summary's totals reconcile exactly (processed = created + updated + skipped + errored) so no record is silently lost.
1. **Given** a record that fails midway through its own creation, **When** the run continues, **Then** no partial fragment of that record is left behind in Horae.

______________________________________________________________________

### User Story 5 - Import from a Harvest CSV export (offline / no OAuth) (Priority: P3)

An administrator who cannot or prefers not to connect the API — an offline migration, a one-shot host-side load, or an account whose OAuth access is unavailable — exports their Harvest data as CSV and imports the file instead. The same engine parses the denormalized rows, derives the four entity levels, and applies them through the identical mapping, matching, dry-run, and resilience rules; only the source of the records differs.

**Why this priority**: The CSV path keeps a fully offline migration possible and is valuable as a fallback, but it is no longer the default flow: it cannot carry stable Harvest identifiers (so matching falls back to a composite natural key) and it requires the admin to produce and upload a file. It is a secondary source adapter behind the API pull.

**Independent Test**: On a freshly seeded organization, upload a representative Harvest detailed-time-report CSV; confirm clients, projects, tasks, and time entries appear correctly and that a re-import of the same file creates zero duplicates (matched by the composite natural key, since the CSV carries no Harvest ids).

**Acceptance Scenarios**:

1. **Given** a valid Harvest CSV export, **When** the administrator imports it, **Then** the same FK-safe order, exact conversions, dry-run, and per-row error behavior apply as for the API source.
1. **Given** the same CSV imported twice, **When** the second run completes, **Then** it creates zero new records — matched by the composite natural key — because the CSV provides no stable per-record identifier.

### Edge Cases

- A Harvest duration that does not divide evenly into whole minutes (e.g. an odd decimal-hours value) is converted to an exact whole-minute value by a single, defined rounding rule, and the summary makes any such adjustment visible rather than silently dropping precision.
- A client, project, or task that appears many times across the source is created once and reused for every later reference to it, within a single run and across re-runs.
- A project references a client not returned as its own record (or, for CSV, appearing only denormalized on rows): the client is created (or matched) before the project is created.
- A time-entry record names a project or task that could not be created (because its own record errored): the entry is reported as an error rather than inventing a placeholder parent.
- The same task name is used under two different clients/projects: because Horae tasks are an organization-level catalog, one task record is shared and enabled per project rather than duplicated.
- An imported project's currency differs from the currency Harvest recorded on the client: the importer applies a defined precedence (see FR-013) rather than failing.
- A very large dataset (hundreds of thousands of time-entry records) is imported without loading the entire result set into a single unbounded operation and without exhausting memory; API pages are consumed as a stream.
- The Harvest connection's access token has expired when an import runs: the importer transparently refreshes it (using the stored refresh token) and continues; if refresh fails, the run is rejected up front with a clear "reconnect Harvest" message and nothing is written (see FR-024).
- Harvest returns a rate-limit response (HTTP 429): the importer waits per the response's guidance and retries rather than failing the run (see FR-023).
- The uploaded CSV is not a recognizable Harvest export (wrong columns, empty, or a different format): the import is rejected up front with a clear message and nothing is written.
- A time entry marked billed/invoiced in Harvest is imported without being treated as though it were already invoiced inside Horae's own invoicing lifecycle (see FR-016).
- Re-importing after some records previously errored: the now-fixed records are created and the already-succeeded records are still recognized and skipped.

## Requirements *(mandatory)*

### Functional Requirements

**Access & invocation**

- **FR-001**: The system MUST restrict the importer — connecting a Harvest account, running an import, and running a dry-run — to organization administrators; non-administrators MUST NOT be able to perform any of these.
- **FR-002**: The system MUST support two source adapters that feed one shared import engine: (a) a **live pull from Harvest's REST API over OAuth2** as the primary source, and (b) a **Harvest CSV export** as a secondary, offline source. Every imported row MUST be associated with the current single organization (every created row carries its `org_id`).
- **FR-003**: For the CSV source, the system MUST validate that an uploaded file is a recognizable Harvest export (expected columns present) before processing, and MUST reject an unrecognized or empty file with a clear message and no writes. For the API source, the system MUST refuse to run when no usable Harvest connection exists, with a clear message and no writes.

**Harvest API connection (OAuth2)**

- **FR-022**: The system MUST let an administrator connect their organization's Harvest account via Harvest's OAuth2 authorization-code flow, and MUST store the resulting credentials — access token, refresh token, and the associated Harvest account identifier — **encrypted at rest**, scoped to the organization. Credentials MUST NOT be exposed to the browser or logged.
- **FR-023**: When calling Harvest's REST API, the system MUST follow Harvest's pagination to completion and MUST respect Harvest's rate limits — on a rate-limit response it MUST back off and retry (honoring any retry-after guidance) rather than failing the run, and it MUST identify itself per Harvest's API requirements (account-id header and a user agent).
- **FR-024**: The system MUST refresh an expired Harvest access token using the stored refresh token transparently during a run; if refresh fails (revoked/expired connection), the run MUST be rejected up front with a clear "reconnect Harvest" message and no writes.
- **FR-025**: The system MUST support **incremental re-sync**: it MUST record enough state from each successful run (the last-synced watermark per entity type) to request only records changed in Harvest since the previous run, while a full re-sync remains available. Re-sync is **additive/updating only, not a mirror**: because the `updated_since` filter returns only changed or new records, a record **deleted in Harvest after import is NOT removed from Horae** on re-sync (see Assumptions — known limitation). No requirement here implies re-sync reconciles deletions.

**Mapping & domain invariants**

- **FR-004**: The system MUST import records in foreign-key-safe order — clients, then projects, then tasks, then time entries — so that no record is created before the records it references.
- **FR-005**: The system MUST store every imported duration as an exact whole number of minutes, converted from Harvest's decimal hours by a single defined rule, and MUST NOT store any duration as a floating-point value.
- **FR-006**: The system MUST store every imported monetary value (rates, amounts) as integer minor units together with an explicit ISO 4217 currency code, and MUST NOT store money as a floating-point value; a stored value MUST re-derive the source amount without rounding drift.
- **FR-007**: The system MUST assign every created record a time-ordered UUID primary key consistent with the rest of Horae.
- **FR-008**: The system MUST map Harvest client, project, task, and time-entry fields onto the corresponding Horae fields (including client currency and address, project code and billing attributes, task billable-by-default and default rate, and entry date, duration, notes, and billable flag), reusing the Harvest-compatible data shape already modeled in the codebase (`crates/horae/src/harvest/`) as the reference for field meanings — the importer performs the inverse of that module's Horae→Harvest transforms.
- **FR-009**: The system MUST enable each imported task on the projects its entries reference (Horae keeps an organization-level task catalog with per-project enablement), so an imported time entry always references a task that is valid for its project.
- **FR-010**: The system MUST resolve the Horae user for each time-entry record by matching the Harvest person to an existing Horae user (by email); the importer MUST NOT create or provision user accounts, and a record whose user cannot be matched MUST be reported as a per-record error (see FR-018). Harvest users are pulled as reference data for this match only, never written to Horae's `users` table.

**Idempotency (provenance + natural keys)**

- **FR-011**: The system MUST be idempotent: importing the same source data more than once MUST NOT create duplicate clients, projects, tasks, or time entries, and a second identical run MUST report zero creations.
- **FR-012**: The system MUST match an incoming record to an existing Horae record using, in order of preference:
  1. **Provenance** — a stored mapping from `(org, Harvest entity type, Harvest id)` to the Horae record's id. This is the exact, edit-robust match used for the API source, where every record carries a stable Harvest id.
  1. **Composite natural key** (fallback, and the only key available for the CSV source, which carries no ids), all scoped to the organization:
     - **Client** — by client name (case-insensitively, trimmed).
     - **Project** — by project code when the source provides one; otherwise by the pairing of its client and project name.
     - **Task** — by task name (case-insensitively, trimmed) within the organization-level catalog.
     - **Time entry** — by the combination of user, project, task, spent date, and duration, plus notes, so that two genuinely distinct entries on the same day are both kept while an exact re-import of one entry is recognized as the same record.
- **FR-026**: On a successful create or match of an API-sourced record, the system MUST persist (or confirm) its provenance mapping so subsequent runs match it exactly by Harvest id — even if the record is later edited in Horae or in Harvest. Provenance rows MUST be written only in a committing run, never in a dry-run.
- **FR-013**: When an incoming record's currency is ambiguous or conflicts between levels (e.g. project vs. client), the system MUST apply a single defined precedence and record that a fallback was applied, rather than failing the record.

**Modes: dry-run and commit**

- **FR-014**: The system MUST offer a dry-run mode that pulls/parses and resolves the entire input and reports what would be created, updated, and skipped (and what would error) per entity type, while writing nothing to storage — including no provenance and no re-sync watermark.
- **FR-015**: A real (committing) run on the same unchanged input and data MUST produce outcomes consistent with the dry-run's preview.

**Behavior on conflicts & existing data**

- **FR-016**: The system MUST import time entries into Horae's neutral/open state and MUST NOT mark them as invoiced within Horae's own invoicing lifecycle solely because Harvest recorded them as billed or invoiced; the Harvest billed/invoiced fact MAY be preserved as informational data but MUST NOT couple an imported entry to a Horae invoice.
- **FR-017**: For a record that already exists (matched by provenance or natural key), the system MUST either leave it unchanged or update a defined set of safe attributes in place, and MUST count it as skipped or updated accordingly — never as a new creation.

**Errors, partial success & reporting**

- **FR-018**: The system MUST process records resiliently: a record that fails MUST be skipped without aborting the run, and the run MUST continue with the remaining records.
- **FR-019**: The system MUST return a per-record error report identifying each failed record by its source location (a Harvest record id for the API source, or a CSV line number) and a human-readable reason.
- **FR-020**: A record that fails partway through creation MUST NOT leave a partial or orphaned fragment behind; each record's writes (including its provenance row) MUST be all-or-nothing.
- **FR-021**: The system MUST return a summary reporting, per entity type, the counts created, updated, skipped, and errored, and these MUST reconcile against the number of records processed so that no record is silently lost.

### Key Entities *(include if feature involves data)*

- **Harvest Connection**: The stored OAuth2 credentials binding one Horae organization to one Harvest account — access token, refresh token, and Harvest account id, held encrypted at rest, plus the last-sync watermark(s) used for incremental re-sync. Created by the OAuth connect flow; read by the API source adapter.
- **Import Source**: Either a live Harvest API pull (primary) or an uploaded Harvest CSV export (secondary). Both normalize into the same internal source rows fed to the engine. The CSV is denormalized (each time-entry row carries the client/project/task names); the API returns each entity level as its own paginated collection with stable ids.
- **Import Run**: A single execution of the importer against a source, in either dry-run or committing mode, by a specific administrator. Produces a summary and a per-record error list; conceptually the unit that must be idempotent when repeated.
- **Provenance Mapping**: A persisted link from `(org, Harvest entity type, Harvest id)` to a Horae record id, written on commit for API-sourced records, used ahead of the natural-key fallback for exact, edit-robust matching and incremental re-sync.
- **Client (target)**: A Horae client created or matched from the source's client fields; carries name, currency, and address.
- **Project (target)**: A Horae project under a client; carries code, name, billing/type attributes, currency, and dates.
- **Task (target)**: A Horae organization-level task matched or created from the source, enabled on the projects that use it with per-project billable/rate attributes.
- **Time Entry (target)**: A Horae time entry for a matched user against a project and task on a date, with an exact minute duration, notes, and a billable flag.
- **Row Outcome**: The per-record result of a run — created, updated, skipped, or errored — with the reason when errored; the raw material of both the summary and the error report.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An administrator can connect their Harvest account and reach a populated Horae — clients, projects, tasks, and historical time entries present — in a single import pass, without exporting a file and without any manual per-record data entry.
- **SC-002**: Running the import twice against the same Harvest data results in exactly the same data as running it once: zero duplicate clients, projects, tasks, or time entries, and the second run reports zero creations — including when a record was edited in Horae between runs (matched by Harvest provenance).
- **SC-003**: Every imported duration and monetary amount reconciles exactly with its Harvest source (durations to the whole minute by the defined rule, money to the minor unit) with zero rounding drift across the whole import.
- **SC-004**: A dry-run reports would-create/would-update/would-skip/would-error counts that match the outcome of the subsequent real run on the same unchanged input, and a dry-run leaves Horae's stored data — including provenance and re-sync state — completely unchanged.
- **SC-005**: An import of a source containing invalid records still imports 100% of the valid records, and every invalid record appears in the error report with a location and a reason; the summary counts reconcile (processed = created + updated + skipped + errored).
- **SC-006**: A dataset containing at least 100,000 time-entry records imports to completion without exhausting memory and without aborting on the first bad record, following Harvest pagination and rate limits for the API source.
- **SC-007**: After import, reported time and monetary totals in Horae reconcile exactly with the corresponding totals in the Harvest source for the same clients, projects, and periods.
- **SC-008**: An incremental re-sync after the first import fetches and applies only the Harvest records changed since the previous successful run, leaving unchanged records intact.

## Assumptions

- **API first, CSV secondary**: The primary source is a live pull from Harvest's REST API over OAuth2; a Harvest CSV export is a secondary, offline adapter feeding the same engine. Both go through one source-agnostic mapping/matching engine, so adding or changing a source does not rewrite the mapping and matching rules.
- **Harvest API shape**: Harvest's API v2 is authorized via OAuth2 at Harvest's identity host and served from Harvest's data host; data calls carry a bearer token plus the Harvest account identifier and a user-agent, are paginated, and support an "updated since" filter for incremental sync, under a published rate limit. The exact endpoints, headers, pagination, and limits are pinned in `contracts/harvest-api.md`.
- **Credentials at rest**: OAuth tokens and the Harvest account id are stored encrypted in a new table (its own migration), never surfaced to the browser or logs. The encryption key is supplied by deployment configuration alongside Horae's existing secrets.
- **Provenance in scope**: A new mapping table `(org, Harvest entity type, Harvest id) → Horae id` is added by its own migration. It does not alter existing columns. It is written on commit for API-sourced records and looked up ahead of the composite natural key, giving exact, edit-robust matching and enabling incremental re-sync. For the CSV source (no ids), the composite natural key remains the matcher.
- **Expected CSV shape** (secondary source): Harvest's detailed time-report CSV, whose rows are denormalized and expected to include at least: Date, Client, Project, Project Code, Task, Notes, Hours (decimal), Billable?, Invoiced?, First Name, Last Name (or a user email), Billable Rate, Billable Amount, Cost Rate, Cost Amount, and Currency. Exact column-name handling is in `contracts/csv-format.md`.
- **Single organization**: All imported data belongs to the one organization of the deployment; multi-organization import (and connecting multiple Harvest accounts) is out of scope. Every created row carries `org_id`.
- **Users are matched, never created**: The importer maps Harvest people to pre-existing Horae users by email and never provisions accounts. Records whose user is unknown are errored, not silently attached to a placeholder user. Administrators are expected to have created the relevant users (via existing provisioning) before importing their time.
- **Duration conversion rule**: Harvest decimal hours are converted to whole minutes by rounding to the nearest minute (hours × 60, rounded half-up); the same rule is applied everywhere so re-imports are stable. Any adjustment is surfaced rather than hidden.
- **Update policy on re-import**: By default a matched existing record is left unchanged (counted as skipped). Whether re-import should update a defined safe subset of attributes in place (FR-017) — versus always skip — is a policy the plan may settle; either choice must remain idempotent and must never duplicate.
- **Imported entries are open**: Time entries import into Horae's neutral/open state and are not tied to any Horae invoice; Harvest's billed/invoiced flags are treated as informational only (FR-016).
- **Reuses existing infrastructure**: The import runs through Horae's existing authenticated, role-checked server-side mutation path and its single PostgreSQL datastore; it introduces no second write path. The only new external dependency is outbound HTTPS to Harvest's API for the primary source. The delivery surface (an administrator-facing screen and/or a CLI subcommand) is an implementation choice for the plan; at least one administrator-invocable surface is required.
- **Re-sync is additive, not a mirror (known limitation)**: incremental re-sync uses Harvest's `updated_since` filter, which returns only records that were changed or newly created — it never reports deletions. A client, project, task, or time entry **deleted in Harvest after import therefore remains in Horae**; the importer only creates and updates, it does not remove. Administrators who need to remove such records do so manually in Horae. A future "mirror-delete" mode (reconciling Harvest deletions into Horae) is deferred (see below).
- **Scope boundary**: The importer covers clients, projects, tasks, and time entries. Harvest data outside those four (invoices, estimates, expenses, users/people as accounts, roles, teams) is out of scope for this version. Deferred to later versions: **propagating Harvest deletions (a "mirror-delete" re-sync mode that removes records deleted in Harvest)**; scheduled/automatic re-sync jobs (this version re-syncs only when an admin runs it); connecting more than one Harvest account per organization; and importing the out-of-scope Harvest entities above.
