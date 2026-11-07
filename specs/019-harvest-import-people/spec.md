# Feature Specification: Harvest people import and the Harvest import section

**Feature Branch**: `feat/harvest-import-people` (US1), stacked `feat/harvest-import-section` (US2)

**Created**: 2026-11-06

**Status**: Draft

**Input**: User description: "Import Harvest people as part of the Harvest import: match them to Horae
accounts by normalized email, create the missing ones as deactivated members with no rates and no
sign-in binding, and report a people entity. Give the importer a Harvest section with real URLs for
the overview, the OAuth app, CSV upload, run history and each run, with grouped errors, a
missing-people table and commit-from-preview."

Amends [005 Durable Harvest Jobs](../005-durable-harvest-jobs/spec.md) (removes users from its
Non-Goals) and [004 Harvest Importer](../004-harvest-importer/spec.md) (FR-010 and the "Users are
matched, never created" assumption). Builds on 006 (jobs CLI), 007 (account switch), 008 (import UX),
013 FR-002 (admin-only administration) and 018 (OAuth client settings).

## Background

The first full Harvest API preview of a real account would have created 1,386 time entries and errored
12,291. Every error was "no Horae user matches email", across 39 Harvest people who have no Horae
account. The importer matches people but never creates them, and the import screen gives no grouped
view of the errors, so finding the 39 people took direct database queries. Runs also have no
addressable page: a reload loses the result and the preview cannot be committed afterwards.

## Clarifications

### Session 2026-11-06

- Q: Should accounts created by an import be active or deactivated? → A: Deactivated. Harvest's
  active flag is recorded in the report only and never applied.
- Q: Should Harvest `default_hourly_rate` / `cost_rate` be imported onto accounts? → A: No.
- Q: How is Harvest's active state reported for created and matched people? → A: A bounded people
  list on the report (one row per resolved Harvest person: email, name, outcome, Harvest active).
- Q: Does an import emit the "user created" plugin event, and how is it marked? → A: Yes, with no
  sign-in `method`, best effort after the data commits, never for a preview.
- Q: Does the CSV path create missing people? → A: Only from rows with an email, never from a name
  alone.
- Q: Which sources does the importer index list? → A: Only real ones (Harvest API, Harvest CSV);
  the placeholder planned sources are removed.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Harvest people come across with their history (Priority: P1)

An administrator migrating from Harvest runs an import. Before any time is imported, Horae reads
every Harvest person, active and archived. A person whose email already belongs to a Horae account is
linked to that account, which is otherwise left exactly as it is. A person with no account gets one,
with their Harvest name and email, as a deactivated member with no rates and no sign-in identity. The
time entries of both kinds of people then import in the same run. The administrator reactivates the
people who still work there; until then nobody gains access to Horae through an import.

**Why this priority**: Without it the import loses almost all historical time (89% of entries in the
observed run). It is the precondition for every other part of the migration being useful.

**Independent Test**: Run a preview and a commit against a Harvest fixture with a mix of matched,
unmatched, email-less and duplicate-email people. Check the people counts, the created accounts'
state, that existing accounts are unchanged, and that every entry of a created person imports.

**Acceptance Scenarios**:

1. **Given** a Harvest person whose email matches a Horae account (case and surrounding whitespace
   ignored, active or deactivated), **When** an import commits, **Then** the person is counted
   "matched" (skipped), the account's name, role, rates, active state and sign-in binding are
   unchanged, and the person's time entries import under that account.
1. **Given** a Harvest person with no matching Horae account, **When** an import commits, **Then** a
   Horae account is created with the Harvest full name and email, the member role, deactivated, with
   no rates and no sign-in binding, the person is counted "created", and their time entries import
   under it in the same run.
1. **Given** the same Harvest account imported a second time, **When** the import commits again,
   **Then** no account is created and every person is counted "matched".
1. **Given** a preview (dry run), **When** it completes, **Then** it reports the same people counts a
   commit would, and no account, link or entry exists afterwards.
1. **Given** a Harvest person with no email, **When** the import runs, **Then** that person is a
   people error saying the email is missing, no account is created, and the run continues.
1. **Given** two Harvest people whose emails normalize to the same value, or a Harvest email that
   matches more than one Horae account, **When** the import runs, **Then** each affected person is a
   people error naming the collision, no account is created or linked for them, and the run continues.
1. **Given** a completed run, **When** the administrator reads the report in the UI or with
   `horae jobs report`, **Then** it shows a people line with created, matched and errored counts,
   and a people list recording, for each resolved person, whether they are active in Harvest.
1. **Given** a Harvest CSV whose rows carry an email column, **When** a CSV import commits, **Then**
   people missing from Horae are created the same way (deactivated member, no rates), using the row's
   first and last name, and rows that identify the person only by name keep the current behaviour.

______________________________________________________________________

### User Story 2 - A Harvest section with addressable runs and actionable results (Priority: P2)

The administrator opens **Importers → Harvest** and finds the connection, the OAuth app settings, the
CSV upload and the run history each at their own address. Starting a preview takes them to that run's
page, which survives a reload and can be bookmarked or shared with another administrator. The page
shows whether the run is a preview or a commit, its live progress, the per-entity summary (including
people), the errors grouped by reason with counts, and a table of the people who still could not be
matched. From there the administrator adds those people to Horae, deactivated by default, one at a
time or all at once, re-runs the preview, and commits the preview without uploading or reconnecting.

**Why this priority**: With US1 most person errors disappear, but the remaining ones (no email,
collisions, CSV name-only rows) and every other error class still need to be found and acted on
without database access, and a preview is only useful if it can be committed after a reload.

**Independent Test**: With seeded jobs, navigate index → Harvest → runs → one run through real URLs,
reload the run page, confirm grouped errors and missing people render from the server-side summary,
add a missing person, and commit a succeeded preview into a new run.

**Acceptance Scenarios**:

1. **Given** an administrator, **When** they visit `/admin/importers`, **Then** they see the import
   sources with Harvest's connection state and a link to `/admin/importers/harvest`.
1. **Given** the Harvest section, **When** the administrator follows its links, **Then** the overview,
   OAuth app (`/admin/importers/harvest/oauth`), CSV upload (`/admin/importers/harvest/csv`), run
   history (`/admin/importers/harvest/runs`) and one run (`/admin/importers/harvest/runs/{id}`) each
   load directly from their URL, and "Importers" stays highlighted in the admin navigation.
1. **Given** a run in progress, **When** the administrator reloads its page, **Then** the page shows
   the same run, still updating, labelled Dry run or Commit.
1. **Given** a finished run with errors, **When** its page loads, **Then** the errors are grouped by
   kind or reason with a count and sample locations per group, people errors list each distinct
   person once with name, email and entry count, and the full error download remains available.
1. **Given** a missing person on the run page, **When** the administrator chooses "Add to Horae",
   **Then** a prefilled form (name editable; email editable only when Harvest gave none) creates a
   deactivated member unless the administrator explicitly chooses active; "Add all as deactivated"
   creates every listed person that has an email in one action.
1. **Given** a succeeded preview and no newer Harvest job of the same source, **When** the
   administrator chooses "Commit this dry run", **Then** a commit job starts for the same source and
   account (CSV: the retained upload) and the page moves to the new run; a repeated click returns the
   same commit job.
1. **Given** a newer job of the same source, a changed Harvest account, or an expired CSV upload,
   **When** the administrator views a succeeded preview, **Then** commit is unavailable and the page
   says why; a direct request is refused with a conflict.
1. **Given** a non-administrator, **When** they request any of these pages or actions, **Then** they
   are refused (013 FR-002).

### Edge Cases

- A Harvest person was created by an earlier import, then renamed or given another email in Horae:
  the Harvest identity link still resolves them on the next run, and nothing on the account is
  overwritten.
- A Harvest person matches a deactivated Horae account: it is linked, not reactivated.
- A Horae account appears with the same email between the preview and the commit: the commit matches
  it instead of creating a duplicate.
- Account creation fails for one person (for example a concurrent sign-in created the email first):
  that person is a people error, their entries resolve by email against whatever exists, and the run
  continues.
- An entry belongs to a Harvest person who errored: the entry errors with the existing person
  message, and the people error explains the cause.
- A created, deactivated account cannot sign in or be bound to a sign-in identity until an
  administrator reactivates it; sign-in provisioning never creates a second account for that email.
- Old run reports without a people line or without the new error fields still display: people show
  as zero and errors group by their reason text.
- A preview is committed after its CSV upload expired: refused with "upload the file again".
- A Harvest account with tens of thousands of entries and errors: the run page loads the grouped
  summary, never every error row.

## Requirements *(mandatory)*

### Functional Requirements

**People import (US1)**

- **FR-001**: An API import MUST read every Harvest person (all pages, active and archived) before it
  imports time entries, within the existing download, checkpoint, retry and rate-limit behaviour.
- **FR-002**: Each Harvest person MUST be resolved in this order: an existing Harvest identity link
  for that person; else exactly one Horae account in the organization whose normalized email equals
  the person's normalized email (any active state); else creation.
- **FR-003**: Matching MUST NOT change any field of an existing account: name, email, role, rates,
  active state or sign-in binding.
- **FR-004**: Creation MUST produce an account with the Harvest full name (first and last name;
  the email when both are blank) and email, the member role, deactivated, no cost or billable rate,
  and no sign-in identity.
- **FR-005**: A matched or created person MUST be linked to its Harvest identity on commit, so later
  runs resolve it without email matching and create nobody new.
- **FR-006**: A person with no email, a person whose normalized email is shared with another Harvest
  person in the same account, and a person whose email matches more than one Horae account MUST each
  be reported as a people error with a reason that names the problem; none of them is created or
  linked.
- **FR-007**: Time entries MUST resolve their person through the people already resolved in the run
  first, then by email as today, so the entries of a person created in the run import in that run.
- **FR-008**: A preview MUST report the people a commit would create and match, and MUST leave no
  account, link or event behind.
- **FR-009**: The report MUST count people as a fifth entity (created, updated, skipped meaning
  matched, errored) in the summary, in the run's processed total and in the CLI and UI output.
  Reports written before this feature MUST still load, with zero people.
- **FR-010**: The report MUST carry a people list with one row per resolved Harvest person (email,
  name, created or matched, and whether the person is active in Harvest), bounded by the number of
  Harvest people, so the administrator can see who to reactivate. Harvest's active state MUST NOT
  change any account's state.
- **FR-011**: Harvest rates (`default_hourly_rate`, `cost_rate`) MUST NOT be imported onto accounts.
- **FR-012**: A CSV import MUST create a missing person the same way when the row carries an email,
  named from the row's first and last name (the email when both are blank). Rows with no email keep
  the current unique-name match and error when no single account matches; the CSV path never creates
  an account from a name alone.
- **FR-013**: Each account created by a committed import MUST emit the existing "user created" plugin
  event, without a sign-in method. The event is queued in the same transaction as the account, so
  it is delivered exactly when the account commits, including when the job later fails, is
  cancelled or is retried; a preview emits none.
- **FR-013a**: An email match MUST NOT link a Harvest person to an account already linked to a
  different Harvest person; that person is reported as ambiguous and their entries are not
  imported.
- **FR-013b**: Creating an account MUST serialize with every other account creation for the same
  email ignoring case, and an email already used by an account the import cannot match (another
  workspace, or one created concurrently with different case) MUST be reported as "email in use".
  This holds whatever the database connection's default isolation level is.
- **FR-013c**: A preview whose simulated person lost their email to a real account between batches,
  in any letter case, MUST report that person once and resolve their later records against current data, never fail
  on a missing account.
- **FR-014**: Each person-related row error MUST carry, besides its reason, a machine-readable kind
  (unmatched email, unmatched name, ambiguous person, missing email, duplicate email, person not
  imported, email in use) and the person's email and name when known. A CSV person reached by
  both name and email rows counts once. Errors written before this feature MUST
  still load without them.
- **FR-014a**: A time entry whose Harvest person errored in the same run MUST be reported as "person
  not imported" and MUST NOT be attached to any account by email.

**Harvest section (US2)**

- **FR-015**: The importer MUST expose `/admin/importers` (source index), `/admin/importers/harvest`
  (overview and connection), `/admin/importers/harvest/oauth`, `/admin/importers/harvest/csv`,
  `/admin/importers/harvest/runs` (paginated history) and `/admin/importers/harvest/runs/{id}` (one
  run). The OAuth callback MUST return to `/admin/importers/harvest`. The index MUST list only
  sources Horae implements (Harvest API, Harvest CSV); no placeholder "planned" sources.
- **FR-016**: Starting a preview, commit, retry or re-sync MUST navigate to the new run's page.
- **FR-017**: A run page MUST show the mode, source, state, progress, per-entity summary and, once the
  run ends, a server-side error summary: groups by kind (person errors) or exact reason (others) with
  entity, count and up to three sample locations, and distinct missing people with name, email and
  entry count, capped at 50 groups and 500 people with the omitted counts stated.
- **FR-018**: From a run page the administrator MUST be able to add a listed person to Horae,
  deactivated by default with an explicit choice to add active, and add all listed people with an
  email as deactivated in one action. Additions MUST use the same validation and events as adding a
  person from People.
- **FR-019**: "Commit this dry run" MUST start a commit for a succeeded preview from the preview alone
  (API: the preview's Harvest account generation; CSV: the retained upload), MUST be idempotent per
  preview, and MUST be refused with a conflict when a newer job of the same source exists, the
  account changed, or the upload expired. The run status MUST say which of these makes commit
  unavailable.
- **FR-020**: Every page and action in the section MUST be administrator-only.

### Key Entities

- **Harvest person**: a Harvest user record: Harvest id, first and last name, email, active flag.
- **Imported account**: a Horae account created by an import: deactivated member, no rates, no
  sign-in identity, linked to one Harvest person.
- **Harvest identity link**: the existing record that ties a Harvest id of a given entity type to the
  Horae record it resolved to; people become a fifth entity type.
- **Run report**: per-entity counts (now including people) plus row errors, each optionally carrying
  a person (email, name) and a kind.
- **Error summary**: a derived, bounded view of one finished run's errors: groups and missing people.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For an account like the observed one, every time entry whose Harvest person has a unique
  email imports in a single commit; entry errors caused by unknown people fall from 12,291 to the
  entries of people with no or colliding email.
- **SC-002**: A second commit of the same account creates zero accounts and zero entries.
- **SC-003**: No existing account changes in any field as a result of an import.
- **SC-004**: No account created by an import can sign in until an administrator reactivates it.
- **SC-005**: An administrator can go from a finished preview to knowing every distinct reason and
  every unmatched person in one page load, without database access, for a run with 15,000 errors.
- **SC-006**: Every run page loads from its URL after a reload, and a succeeded preview can be
  committed after a reload without re-uploading or reconnecting.

## Assumptions

- Accounts are created deactivated because they exist to own imported history; access is an
  administrator decision (confirmed for both import-created accounts and the run-page "Add to
  Horae" default).
- Harvest rates stay out of accounts: they are money and permission-adjacent, and Horae rates are
  configured deliberately. Imported entries keep their own per-entry rates as today.
- Email comparison uses the importer's existing normalization (trim, collapse whitespace, case-fold).
- The "user created" event has no import marker: its `method` field is defined for sign-in events
  only. Plugins that need to distinguish imports can check the account's deactivated state.
- People are a Harvest catalog collection the importer already downloads; no new Harvest permission
  or scope is needed.
- Known limitation: an imported account keeps the email's casing from Harvest. Matching ignores
  case, but binding a sign-in identity by email compares exact text, so a provider that reports the
  email in different case will not bind until the address is corrected in Horae.
- Out of scope: importing Harvest roles, teams, project assignments, capacity, avatars or rates;
  deleting Horae accounts for people removed from Harvest; changing the People page.
