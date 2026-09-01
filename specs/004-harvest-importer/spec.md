# Feature Specification: Harvest Data Importer

**Feature Branch**: `004-harvest-importer`

**Created**: 2026-09-01

**Status**: Draft

**Input**: User description: "Harvest data importer. Let an org admin migrate their existing Harvest data into Horae — clients, projects, tasks, and time entries — so teams switching from Harvest don't start empty. Start with importing Harvest's own CSV exports (no OAuth). Map Harvest records to Horae's model honoring the domain invariants: durations as integer minutes, money as integer minor units (cents) + ISO currency code (never floats), UUID v7 primary keys, single org (every row carries org_id). Import in FK-safe order: clients → projects → tasks → time entries. The importer MUST be idempotent (re-running does not duplicate — define the natural keys used to match existing rows), MUST offer a dry-run that reports what would be created/updated/skipped without writing, and MUST surface a clear summary plus per-row errors that don't abort the whole import. Treat a Harvest REST API pull (OAuth2) as a possible later mode the spec should acknowledge but not require. Note: Horae already ships a read-only Harvest-compatible API at /harvest/v2/\* (see crates/horae/src/harvest/), so Harvest's data shape is already understood in this codebase — reference it."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Migrate a Harvest export into an empty Horae (Priority: P1)

An organization administrator who is switching from Harvest exports their Harvest data as CSV and uploads it to Horae. Horae reads the export, creates the clients, projects, tasks, and time entries it describes — in that dependency order — and reports how many of each were created. When the administrator opens Horae afterward, their historical time sits under the same clients, projects, and tasks it did in Harvest, so the team continues where they left off instead of starting from an empty install.

**Why this priority**: This is the entire reason the feature exists. Getting existing data into Horae in one pass is the smallest slice that delivers standalone migration value; every other story refines or protects this one.

**Independent Test**: On a freshly seeded organization with no clients, upload a representative Harvest time-report CSV; confirm that clients, projects, tasks, and time entries appear with correct names, dates, durations (in exact minutes), billable flags, and monetary rates/amounts (in exact minor units), and that the reported created-counts match what the file contained.

**Acceptance Scenarios**:

1. **Given** an administrator and a valid Harvest CSV export, **When** they run the import, **Then** clients are created first, then projects under their clients, then tasks, then time entries referencing them, and no row is created before the rows it depends on.
1. **Given** a Harvest time entry recording 1.5 hours, **When** it is imported, **Then** the resulting Horae entry stores 90 minutes exactly (never a floating-point hours value).
1. **Given** a Harvest record carrying a billable rate and currency, **When** it is imported, **Then** the amount is stored as integer minor units with the ISO 4217 currency code, and the stored value re-derives the original amount without rounding drift.
1. **Given** a completed import, **When** the administrator reviews the summary, **Then** it reports the count created, updated, and skipped for each entity type (clients, projects, tasks, time entries).
1. **Given** a Harvest project that carries a project code, **When** it is imported, **Then** the code is preserved on the Horae project.

______________________________________________________________________

### User Story 2 - Re-run an import without creating duplicates (Priority: P1)

An administrator runs an import, then discovers the export was incomplete (a later month was missing) or a few rows failed, so they re-export and import again. The second run recognizes the records that already exist and leaves them alone (or updates them in place), adding only the genuinely new rows. Running the same file twice produces the same result as running it once.

**Why this priority**: A migration that duplicates data on a second attempt is worse than useless — it forces manual cleanup and destroys trust. Idempotency is what makes the import safe to retry, which real migrations always require. It ships alongside P1 because the first import is not safe to offer without it.

**Independent Test**: Import a CSV, record the resulting counts, then import the identical CSV again; confirm the second run creates zero new clients, projects, tasks, or time entries and reports them all as skipped or unchanged, with no duplicate rows in Horae.

**Acceptance Scenarios**:

1. **Given** a client that already exists in Horae with the same natural key, **When** an import references it again, **Then** no second client is created and the existing one is reused for its projects.
1. **Given** an identical CSV imported a second time, **When** the run completes, **Then** the created counts are zero for every entity type and the totals in Horae are unchanged.
1. **Given** a previously imported time entry whose source row is unchanged, **When** the file is re-imported, **Then** the entry is matched to the existing one and not duplicated.
1. **Given** a re-export that adds new rows to an already-imported file, **When** it is imported, **Then** only the new rows are created and the previously imported rows are left intact.

______________________________________________________________________

### User Story 3 - Preview an import before committing (dry-run) (Priority: P2)

Before writing anything, the administrator runs the import in dry-run mode. Horae parses the file, resolves every row against existing data, and reports exactly what a real run would create, update, and skip — and which rows would error — without persisting a single change. The administrator reviews the preview, fixes the source file if needed, and only then runs it for real.

**Why this priority**: Migrations are high-stakes and irreversible-feeling; a dry-run lets an administrator gain confidence and catch mapping problems before touching real data. It depends on the parsing and matching logic of P1/P2 but is a distinct, separately valuable capability.

**Independent Test**: Run a dry-run against a CSV containing a mix of new and already-existing records plus a few malformed rows; confirm the reported would-create/would-update/would-skip/would-error counts, verify that no data was written (Horae's row counts are unchanged), then run for real and confirm the actual outcome matches the preview.

**Acceptance Scenarios**:

1. **Given** a valid CSV, **When** the administrator runs a dry-run, **Then** the summary reports would-create, would-update, would-skip, and would-error counts per entity type and no rows are written to Horae.
1. **Given** a dry-run and an immediately following real run on the same unchanged file and data, **When** both complete, **Then** the real run's created/updated/skipped counts match the dry-run's preview.
1. **Given** a dry-run that reports rows that would error, **When** the administrator inspects the report, **Then** each problem row is identified with its source location and the reason it would fail.

______________________________________________________________________

### User Story 4 - Survive bad rows with a per-row error report (Priority: P2)

The administrator's export contains a handful of problem rows — an entry whose user email is not a Horae user, a malformed date, a project with no client, a duration that will not parse. Rather than aborting the whole import, Horae imports every row it can, skips the ones it cannot, and hands back a per-row error list naming each failed row and why it failed, so the administrator can fix just those and re-import them.

**Why this priority**: Real exports are messy. An all-or-nothing import that dies on row 4,000 of 10,000 is unusable at migration scale. Partial success with a precise error report is what makes a large migration tractable. It builds on P1 but is independently demonstrable.

**Independent Test**: Import a CSV where a known subset of rows is invalid; confirm the valid rows are imported, the invalid rows are skipped (not partially written), the summary counts reconcile (created + skipped + errored = total processed), and each errored row is reported with its source line and a clear reason.

**Acceptance Scenarios**:

1. **Given** a CSV with some invalid rows, **When** the import runs, **Then** valid rows are imported and each invalid row is skipped without aborting the run.
1. **Given** a time-entry row whose user cannot be matched to an existing Horae user, **When** it is processed, **Then** that row is reported as an error with its identifying detail and the run continues.
1. **Given** a failed row, **When** the run completes, **Then** the summary's totals reconcile exactly (processed = created + updated + skipped + errored) so no row is silently lost.
1. **Given** a row that fails midway through its own creation, **When** the run continues, **Then** no partial fragment of that row is left behind in Horae.

### Edge Cases

- A Harvest duration that does not divide evenly into whole minutes (e.g. an odd decimal-hours value) is converted to an exact whole-minute value by a single, defined rounding rule, and the summary makes any such adjustment visible rather than silently dropping precision.
- A client, project, or task that appears many times across the export's rows is created once and reused for every later reference to it, within a single run and across re-runs.
- A project references a client that does not appear as its own record: the client is created (or matched) from the denormalized fields on the project/entry rows before the project is created.
- A time-entry row names a project or task that could not be created (because its own row errored): the entry row is reported as an error rather than inventing a placeholder parent.
- The same task name is used under two different clients/projects: because Horae tasks are an organization-level catalog, one task record is shared and enabled per project rather than duplicated.
- An imported project's currency differs from the currency Harvest recorded on the client: the importer applies a defined precedence (see FR-013) rather than failing.
- A very large export (hundreds of thousands of time-entry rows) is imported without loading the entire result set into a single unbounded operation and without exhausting memory.
- The uploaded file is not a recognizable Harvest export (wrong columns, empty, or a different format): the import is rejected up front with a clear message and nothing is written.
- A time entry marked billed/invoiced in Harvest is imported without being treated as though it were already invoiced inside Horae's own invoicing lifecycle (see FR-016).
- Re-importing after some rows previously errored: the now-fixed rows are created and the already-succeeded rows are still recognized and skipped.

## Requirements *(mandatory)*

### Functional Requirements

**Access & invocation**

- **FR-001**: The system MUST restrict the importer to organization administrators; non-administrators MUST NOT be able to run an import or a dry-run.
- **FR-002**: The system MUST accept a Harvest CSV export as the input source for the first version and MUST associate every imported row with the current single organization (every created row carries its `org_id`).
- **FR-003**: The system MUST validate that an uploaded file is a recognizable Harvest export (expected columns present) before processing, and MUST reject an unrecognized or empty file with a clear message and no writes.

**Mapping & domain invariants**

- **FR-004**: The system MUST import records in foreign-key-safe order — clients, then projects, then tasks, then time entries — so that no record is created before the records it references.
- **FR-005**: The system MUST store every imported duration as an exact whole number of minutes, converted from Harvest's decimal hours by a single defined rule, and MUST NOT store any duration as a floating-point value.
- **FR-006**: The system MUST store every imported monetary value (rates, amounts) as integer minor units together with an explicit ISO 4217 currency code, and MUST NOT store money as a floating-point value; a stored value MUST re-derive the source amount without rounding drift.
- **FR-007**: The system MUST assign every created record a time-ordered UUID primary key consistent with the rest of Horae.
- **FR-008**: The system MUST map Harvest client, project, task, and time-entry fields onto the corresponding Horae fields (including client currency and address, project code and billing attributes, task billable-by-default and default rate, and entry date, duration, notes, and billable flag), reusing the Harvest-compatible data shape already modeled in the codebase as the reference for field meanings.
- **FR-009**: The system MUST enable each imported task on the projects its entries reference (Horae keeps an organization-level task catalog with per-project enablement), so an imported time entry always references a task that is valid for its project.
- **FR-010**: The system MUST resolve the Horae user for each time-entry row by matching the Harvest person to an existing Horae user (by email); the importer MUST NOT create or provision user accounts, and a row whose user cannot be matched MUST be reported as a per-row error (see FR-018).

**Idempotency (natural keys)**

- **FR-011**: The system MUST be idempotent: importing the same source data more than once MUST NOT create duplicate clients, projects, tasks, or time entries, and a second identical run MUST report zero creations.
- **FR-012**: The system MUST match existing records to incoming rows using defined natural keys, all scoped to the organization:
  - **Client** — by client name (case-insensitively, trimmed).
  - **Project** — by project code when the source provides one; otherwise by the pairing of its client and project name.
  - **Task** — by task name (case-insensitively, trimmed) within the organization-level catalog.
  - **Time entry** — by the combination of user, project, task, spent date, and duration, plus notes, so that two genuinely distinct entries on the same day are both kept while an exact re-import of one entry is recognized as the same row. Where the source provides a stable per-entry identifier, the importer SHOULD prefer that identifier as the entry's natural key (see Assumptions).
- **FR-013**: When an incoming record's currency is ambiguous or conflicts between levels (e.g. project vs. client), the system MUST apply a single defined precedence and record that a fallback was applied, rather than failing the row.

**Modes: dry-run and commit**

- **FR-014**: The system MUST offer a dry-run mode that parses and resolves the entire input and reports what would be created, updated, and skipped (and what would error) per entity type, while writing nothing to storage.
- **FR-015**: A real (committing) run on the same unchanged input and data MUST produce outcomes consistent with the dry-run's preview.

**Behavior on conflicts & existing data**

- **FR-016**: The system MUST import time entries into Horae's neutral/open state and MUST NOT mark them as invoiced within Horae's own invoicing lifecycle solely because Harvest recorded them as billed or invoiced; the Harvest billed/invoiced fact MAY be preserved as informational data but MUST NOT couple an imported entry to a Horae invoice.
- **FR-017**: For a record that already exists (matched by its natural key), the system MUST either leave it unchanged or update a defined set of safe attributes in place, and MUST count it as skipped or updated accordingly — never as a new creation.

**Errors, partial success & reporting**

- **FR-018**: The system MUST process rows resiliently: a row that fails MUST be skipped without aborting the run, and the run MUST continue with the remaining rows.
- **FR-019**: The system MUST return a per-row error report identifying each failed row by its source location (e.g. line number) and a human-readable reason.
- **FR-020**: A row that fails partway through creation MUST NOT leave a partial or orphaned fragment behind; each row's writes MUST be all-or-nothing.
- **FR-021**: The system MUST return a summary reporting, per entity type, the counts created, updated, skipped, and errored, and these MUST reconcile against the number of rows processed so that no row is silently lost.

### Key Entities *(include if feature involves data)*

- **Import Source File**: The uploaded Harvest CSV export. Denormalized — each time-entry row typically carries the client, project, and task names alongside the entry — so the four Horae entity levels are derived from it. Expected to follow Harvest's time-report column layout (see Assumptions for the expected columns).
- **Import Run**: A single execution of the importer against a source file, in either dry-run or committing mode, by a specific administrator. Produces a summary and a per-row error list; conceptually the unit that must be idempotent when repeated.
- **Client (target)**: A Horae client created or matched from the export's client fields; carries name, currency, and address.
- **Project (target)**: A Horae project under a client; carries code, name, billing/type attributes, currency, and dates.
- **Task (target)**: A Horae organization-level task matched or created from the export, enabled on the projects that use it with per-project billable/rate attributes.
- **Time Entry (target)**: A Horae time entry for a matched user against a project and task on a date, with an exact minute duration, notes, and a billable flag.
- **Row Outcome**: The per-row result of a run — created, updated, skipped, or errored — with the reason when errored; the raw material of both the summary and the error report.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An administrator can take a Harvest CSV export and reach a populated Horae — clients, projects, tasks, and historical time entries present — in a single import pass, without any manual per-record data entry.
- **SC-002**: Importing the same export twice results in exactly the same data as importing it once: zero duplicate clients, projects, tasks, or time entries, and the second run reports zero creations.
- **SC-003**: Every imported duration and monetary amount reconciles exactly with its Harvest source (durations to the whole minute by the defined rule, money to the minor unit) with zero rounding drift across the whole import.
- **SC-004**: A dry-run reports would-create/would-update/would-skip/would-error counts that match the outcome of the subsequent real run on the same unchanged input, and a dry-run leaves Horae's stored data completely unchanged.
- **SC-005**: An import of a file containing invalid rows still imports 100% of the valid rows, and every invalid row appears in the error report with a location and a reason; the summary counts reconcile (processed = created + updated + skipped + errored).
- **SC-006**: An export containing at least 100,000 time-entry rows imports to completion without exhausting memory and without aborting on the first bad row.
- **SC-007**: After import, reported time and monetary totals in Horae reconcile exactly with the corresponding totals in the Harvest source for the same clients, projects, and periods.

## Assumptions

- **CSV first, API later**: The first version imports Harvest's CSV exports only. A Harvest REST API pull over OAuth2 is acknowledged as a plausible later mode (Horae already models Harvest's data shape via its read-only `/harvest/v2/*` API) but is explicitly out of scope here; the import logic should be structured so that source (CSV rows vs. API records) can vary without rewriting the mapping and matching rules.
- **Expected CSV shape**: The primary input is Harvest's detailed time-report CSV, whose rows are denormalized and expected to include at least: Date, Client, Project, Project Code, Task, Notes, Hours (decimal), Billable?, Invoiced?, First Name, Last Name (or a user email), Billable Rate, Billable Amount, Cost Rate, Cost Amount, and Currency. Dedicated Harvest client/project CSV exports MAY be accepted as optional supplementary sources to enrich attributes (client address, project budget/dates) the time report omits; when absent, those attributes are left at Horae's defaults. Exact column-name handling is an implementation detail for the plan.
- **Single organization**: All imported data belongs to the one organization of the deployment; multi-organization import is out of scope. Every created row carries `org_id`.
- **Users are matched, never created**: The importer maps Harvest people to pre-existing Horae users by email and never provisions accounts. Rows whose user is unknown are errored, not silently attached to a placeholder user. Administrators are expected to have created the relevant users (via existing provisioning) before importing their time.
- **Duration conversion rule**: Harvest decimal hours are converted to whole minutes by rounding to the nearest minute (hours × 60, rounded half-up); the same rule is applied everywhere so re-imports are stable. Any adjustment is surfaced rather than hidden.
- **Time-entry natural key & provenance**: Because Horae's time-entry schema has no built-in external-identity column today, the default idempotency key for entries is the composite of (user, project, task, spent date, duration, notes). If the plan chooses to persist the Harvest source identifier (e.g. via a provenance/mapping record) to make matching exact and robust against edits, that is preferred; introducing any such storage is a design decision for the plan/data-model phase, not assumed here.
- **Update policy on re-import**: By default a matched existing record is left unchanged (counted as skipped). Whether re-import should update a defined safe subset of attributes in place (FR-017) — versus always skip — is a policy the plan may settle; either choice must remain idempotent and must never duplicate.
- **Imported entries are open**: Time entries import into Horae's neutral/open state and are not tied to any Horae invoice; Harvest's billed/invoiced flags are treated as informational only (FR-016).
- **Reuses existing infrastructure**: The import runs through Horae's existing authenticated, role-checked server-side mutation path and its single PostgreSQL datastore; it introduces no second write path and no external service dependency. The delivery surface (an administrator-facing upload screen and/or a CLI subcommand) is an implementation choice for the plan; at least one administrator-invocable surface is required.
- **Scope boundary**: The importer covers clients, projects, tasks, and time entries. Harvest data outside those four (invoices, estimates, expenses, users/people as accounts, roles, teams) is out of scope for this version.
