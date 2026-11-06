# Contract: import report additions (PR A)

`horae jobs report`, `get_harvest_import_job` and the error NDJSON all carry these fields. Every
field is additive: a report written before this feature still loads, with no people line and
no person fields on its errors.

```json
{
  "version": 1,
  "source": "harvest_api",
  "mode": "Commit",
  "summary": {
    "clients":      {"created": 58, "updated": 0, "skipped": 0, "errored": 0},
    "projects":     {"created": 136, "updated": 0, "skipped": 1, "errored": 0},
    "tasks":        {"created": 10, "updated": 0, "skipped": 0, "errored": 0},
    "people":       {"created": 39, "updated": 0, "skipped": 2, "errored": 1},
    "time_entries": {"created": 13600, "updated": 0, "skipped": 0, "errored": 77}
  },
  "people": [
    {"harvest_id": 1000, "horae_id": "0193…", "email": "dana@example.com", "name": "Dana Dev",
     "outcome": "created", "harvest_active": false}
  ],
  "row_errors": [
    {"source_location": "person 1001", "entity": "person",
     "reason": "Harvest person has no email; add one in Harvest, or add the person in Horae",
     "person": {"kind": "missing_email", "name": "No Mail"}},
    {"source_location": "time_entry 5000", "entity": "time_entry",
     "reason": "person 1001 failed to import",
     "person": {"kind": "person_not_imported", "name": "No Mail"}}
  ]
}
```

- `summary.people.skipped` means matched to an existing account. `updated` is always 0.
- `people[].outcome`: `created` or `matched`. `harvest_active` is Harvest's flag and is never
  applied to the account.
- The `person.kind` values are `unmatched_email`, `unmatched_name`, `ambiguous_person`, `missing_email`,
  `duplicate_email`, `person_not_imported` and `email_in_use`. The `reason` text stays human-readable and is
  unchanged for existing error classes.
- Exit codes and CLI flags are unchanged.
