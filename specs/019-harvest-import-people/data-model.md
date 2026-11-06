# Data Model: Harvest people import and the Harvest import section

## Database

### `harvest_entity_type` (enum, extended)

Its values are `client | project | task | time_entry | person`. The new value comes from migration
`0053_harvest_person_entity.sql` (`ALTER TYPE harvest_entity_type ADD VALUE 'person'`), and no
other schema changes.

### `harvest_import_map` (unchanged table, new rows)

`(org_id, 'person', harvest_user_id) → users.id`. The importer writes one on commit for every
matched or created person. Like all provenance, it makes `has_provenance` true and blocks an account
switch (007).

### `users` (unchanged table, new rows)

Accounts created by an import have:

| Column | Value |
|---|---|
| id | UUID v7 |
| org_id | importing organization |
| email | Harvest email, trimmed |
| name | trimmed `first last`; the email when blank |
| org_role | `member` |
| active | `false` |
| cost_rate_cents, billable_rate_cents | NULL |
| oidc_subject, oidc_issuer | NULL |

An import never updates an existing row.

## Core types (`horae-core`, `importers::harvest::types`)

```text
EntityType      += Person                      ("person")
ImportSummary   += people: EntityCounts         #[serde(default)]
ImportReport    += people: Vec<ReportPerson>    #[serde(default)], skipped when empty
ReportPerson     { harvest_id: Option<i64>, horae_id: Uuid, email: Option<String> (absent only
                   for a CSV name match), name: String, outcome: PersonOutcome,
                   harvest_active: Option<bool> }
PersonOutcome    Created | Matched
RowOutcome::Errored += person: Option<PersonError>      (defaulted, skipped when None)
RowError        += person: Option<PersonError>      (defaulted, skipped when None)
PersonError      { kind: PersonErrorKind, email: Option<String>, name: Option<String> }
PersonErrorKind  UnmatchedEmail | UnmatchedName | AmbiguousPerson | MissingEmail
                 | DuplicateEmail | PersonNotImported
```

Invariants:

- `summary.people.processed()` = `people.len()` + `summary.people.errored`.
- `people.len()` ≤ the number of Harvest people for the API, or distinct people for CSV.
- `reconciles()` covers all five entities.

## Server-side (horae)

- `RunCache.users` holds `UserKey::Email(norm harvest email) → horae_id` for every resolved person.
  `failed_parents` holds `(Person, harvest_id)` for every person that errored.
- `ParentSnapshot.users` holds the rows to restore for simulated accounts during durable previews.
- `ApiUser` gains `email: Option<String>` (defaulted) and `is_active: bool` (defaulted to true).

## PR B additions

- `JobStatus` += `mode`, `source` and `commit_availability`, all read from the payload with
  serde defaults.
- `ErrorSummary` = { groups: [{ kind?, reason, entity, count, samples ≤ 3 }], people: \[{ email?,
  name?, entries }\], omitted_groups, omitted_people }
- `NewPerson` { email, name }
- `Activation` = `Deactivated | Active`
