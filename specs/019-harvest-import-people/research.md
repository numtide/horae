# Research: Harvest people import and the Harvest import section

## R-1 Where people go in the pipeline

- **Decision**: People are a fourth catalog parent kind (`Parent::Person` in `parents.rs`). They
  are applied after tasks, from `HarvestData.users`, which the catalog download already fills from
  `GET /v2/users`. That request sends no `is_active` filter, so archived people come too.
- **Rationale**: Each person gets its own savepoint. The 500-record parent batches, the
  `parent_offset` resume and the dry-run rollback apply unchanged. `parent_count()` adds
  `users.len()`. A checkpoint saved before this feature still resumes and imports people next. No
  new HTTP request and no checkpoint version bump are needed.
- **Alternatives**: A separate phase between the catalog and the entries needs its own checkpoint
  state for no gain. Creating people lazily from entry rows (the CSV approach) on the API path would
  miss people with no entries and would lose the per-person `is_active` report.

## R-2 Matching and identity

- **Decision**: Resolve each person in this order:

  1. provenance `(person, harvest_id)` whose `horae_id` is still a user in the org;
  1. `harvest_norm(email)` against `users` in the org, any active state, `LIMIT 2`, with exactly
     one match required;
  1. creation.

  Matching and provenance hits record `Skipped` ("matched"), and creation records `Created`. The
  normalized email is computed with `keys::normalize`, which is byte-identical to `harvest_norm`.

- **Rationale**: This mirrors `resolve_client`, and the email query is the one `resolve_user`
  already uses. Provenance keeps a person linked after an admin edits the email in Horae.

- **Collisions**: A pre-pass over the catalog finds normalized emails shared by more than one
  Harvest person. Every such person errors with `duplicate_email`, and so does each person with a
  blank email (`missing_email`). Two Horae matches give `ambiguous_person`. No guessing.

## R-3 Account creation and locking

- **Decision**: Insert the account with `org_role = 'member'`, `active = false`, NULL rates, NULL
  `oidc_subject` and `oidc_issuer`, and a UUID v7 id. The name is the trimmed "first last" name,
  or the email when that is blank. The insert runs inside the person's savepoint under the Shared
  organization lock the import already holds.
- **Rationale**: `begin_user_access_change` takes `FOR UPDATE` on the organization only to
  serialize last-admin counting. An inactive member cannot change that count. A unique-email race
  with a concurrent sign-in or admin add fails the savepoint and becomes a person error, and the
  run continues.

## R-4 Entries of resolved people

- **Decision**: When a person resolves, the run cache stores
  `UserKey::Email(normalized harvest email) → horae_id`. `resolve_user` already checks the cache
  first, so entries resolve without another query, including after a provenance match whose Horae
  email differs. A person who errored is marked failed (`cache.mark_failed(Person, id)`). Their
  entries error with `person {id} failed to import` (kind `person_not_imported`) and are never
  attached by email.
- **Rationale**: The failed-parent check is how projects and tasks already avoid inventing parents.
  Without it, two Harvest people sharing an email would have both histories attached to one Horae
  account.

## R-5 Dry runs

- **Decision**: `ParentSnapshot` gains a `users` array: `id, org_id, email, name, org_role, active`
  of the users the cache references. It is restored with
  `INSERT … FROM jsonb_populate_recordset(NULL::users, $1) WHERE org_id = $2 ON CONFLICT (id) DO NOTHING`.
  The field uses `#[serde(default = "empty_rows")]` so older checkpoints load, and it takes part in
  the organization-ownership check.
- **Rationale**: Durable API and CSV previews roll back after every batch. Simulated accounts must
  exist again in later batches, or every later entry would report a missing person. The inline path
  is one rolled-back transaction and needs nothing.

## R-6 Report shape

- **Decision**:
  - `EntityType::Person`, as `"person"` in both serde and `as_str`. `ALL` has five entries.
  - `ImportSummary.people` uses `#[serde(default)]`.
  - `ImportReport.people` is a `Vec<ReportPerson>` that uses `#[serde(default)]` and is skipped
    when empty. It holds one row per resolved person:
    `harvest_id?, horae_id, email, name, outcome (created|matched), harvest_active?`.
  - `RowError` and `RowOutcome::Errored` gain one optional
    `person: PersonError { kind, email?, name? }` with
    `#[serde(default, skip_serializing_if = "Option::is_none")]`. A kind only exists for person
    errors, so it lives inside `person` rather than beside it. The kinds are `unmatched_email`,
    `unmatched_name`, `ambiguous_person`, `missing_email`, `duplicate_email` and
    `person_not_imported`.
  - `job_report` adds people to `processed`. The report version stays 1/2, because every addition
    is a defaulted field.
- **Rationale**: Old reports and archived NDJSON load unchanged. The people list is bounded by the
  number of Harvest people (FR-010), so it does not grow with entries. An old remote CLI cannot
  parse the `"person"` entity, but the CLI ships with the server.

## R-7 CSV

- **Decision**: In `resolve_user`, when the source is CSV, the row has an email and nothing
  matches, create the person with `create_person` (the same helper) and return `Created`. Rows
  matched by email or name on a cache miss return `Skipped`. `apply.rs` records a `Person` outcome
  for CSV resolutions only, when the row commits. Rows with only a name keep the unique-name match
  or the error.
- **Rationale**: Outcomes fold only on row commit and cached hits return none, so each distinct
  person counts exactly once, the same as clients. A rolled-back row rolls back its account too.
  The API path records people only in the people phase.

## R-8 "User created" event

- **Decision**: `create_person` queues `AppEvent::UserCreated` (`method: None`, role `member`) in
  `horae_outbox` (kind `user_created`) inside the savepoint that inserts the account. The existing
  outbox worker, which delivered only `project_created`, now also claims `user_created`.
- **Rationale**: A durable commit commits each batch separately. The event therefore commits or
  rolls back with its account, whether the job then succeeds, fails, is cancelled, or loses its
  lease, and a preview's rollback discards it. `UserPayload.method` is documented as login-only.
- **Rejected**: dispatching after job completion (first version). A failed job never announced the
  accounts its committed batches had created, and later runs match them by provenance.

## R-10 Review fixes (2026-11-06)

- **One account per Harvest person**: before linking an email match, the people phase refuses an
  account already linked to a different Harvest person id (`ambiguous_person`). Otherwise a person
  who inherited a former colleague's email would get that colleague's history.
- **Case-insensitive creation**: `create_person` takes
  `pg_advisory_xact_lock(hashtextextended('horae.signup-email:' || lower(email), 0))`, the key
  sign-in provisioning (PR #340) uses. It then checks again for any account with that email
  ignoring case: the same workspace is matched, anything else is `email_in_use`. A unique
  violation maps to `email_in_use` too. Follow-up: once #340 lands, share its helper.
- **READ COMMITTED imports**: `begin_import_transaction` sets READ COMMITTED first. Under a
  REPEATABLE READ connection default the snapshot would predate the lock wait, so the recheck
  would miss an account committed meanwhile. Nothing in the import relies on a snapshot spanning
  statements: every guard is a lock followed by a read, which needs the newest committed rows.
- **Lost preview people**: `ParentSnapshot::restore` returns the simulated people it could not
  restore, including when the real account's email differs only in case. They are removed from the run cache and reported once as `email_in_use`.
- **CSV people count once** per account, tracked in the run cache.

## R-11 PR B review fixes (2026-11-07)

- **Adding people is not all or nothing on existing emails**: `add_harvest_people` leaves an email
  any account already uses (checked under its lock, ignoring case) and reports it as present, so
  "Add all as deactivated" after a reload or from a second tab adds the rest. An email listed twice
  in one request, compared as `trim` + lowercase like the server, is still refused. Adding one
  person from People still refuses an email in use.
- **What replaces a preview**: only a later run of the same source that has not failed or been
  cancelled, and is not the preview's own commit. A committed preview says so and links to its
  commit (`JobStatus.committed_as`, `CommitAvailability::Committed`).
- **Run pages re-read the run** after every action and whenever the tab or window is shown again,
  one status read each, so a finished run's commit and retry offers stay current.
- **The error summary takes no organization lock**: it checks the administrator and scopes every
  read by `org_id`, so it never queues behind a running import or an access change. Adding people
  still takes the access-change lock, like adding a person from People.
- **Known gap, accepted**: starting a new preview does not lock an older preview's row, so a new
  preview can be created while the old one is being committed. The commit then still runs; the new
  preview reports the commit as a newer run.

## R-9 US2 design (adopted)

The design is taken unchanged from the harvest-ux plan:

- the route map;
- one component per file under `pages/importers/harvest/`;
- the server-side `summarize` over archived chunks with line reassembly, capped at 50 groups and
  500 people;
- `commit_harvest_import_preview` with the `preview-commit:{job_id}` idempotency key, the account
  generation fence and CSV upload reuse;
- `JobStatus.mode/source/commit_availability`.

User-approved answers:

1. Missing people are added in place, deactivated by default. There is an "Add all as deactivated"
   action and an explicit "add as active" choice, served by `add_harvest_people(people, activation)`
   with an `Activation` enum.
1. Committing a preview is refused when a newer job of the same source exists.
1. OAuth and CSV get separate URLs.

Added for this feature: the importer index lists only Harvest API and Harvest CSV, and the
`PlannedSource` placeholders (Toggl Track, Clockify) and their tests are removed.
