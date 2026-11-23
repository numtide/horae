# Contract: import report and CLI output

Extends the report in [005](../../005-durable-harvest-jobs/contracts/) and the CLI JSON of
[006](../../006-harvest-jobs-cli/quickstart.md). All additions are optional on read
(`serde(default)`), so reports stored before this feature still load and older CLI parsers ignore the
new keys.

## Summary

```json
{
  "summary": {
    "clients":      {"created": 0, "updated": 0, "skipped": 12, "errored": 0},
    "projects":     {"created": 0, "updated": 0, "skipped": 40, "errored": 0},
    "tasks":        {"created": 0, "updated": 0, "skipped": 9,  "errored": 0},
    "people":       {"created": 0, "updated": 0, "skipped": 45, "errored": 0},
    "invoices":     {"created": 310, "updated": 0, "skipped": 0, "errored": 2},
    "time_entries": {"created": 0, "updated": 0, "skipped": 13700, "errored": 0},
    "links": {
      "linked": 11850,
      "unlinked": 0,
      "billed_without_invoice": 14,
      "conflicts": 0
    }
  },
  "invoice_scan": "full"
}
```

- `invoices` counts each Harvest invoice once. Drafts are not counted in it; they are counted in
  `invoices_skipped_drafts` (integer, next to `summary`).
- `links.linked` counts entries newly linked or relinked; an entry already linked to the same invoice
  counts in neither.
- `invoice_scan` is `"full"` when the run read invoices and entries in full because invoices had never
  been imported (R-7), otherwise `"incremental"` or `"full"` as asked.
- The CSV source reports `invoices` all zero and uses `billed_without_invoice`.

## Errors

An invoice error is a `RowError` with `entity: "invoice"`, a `source_location` naming the Harvest
invoice, a `reason` in plain words, and a new optional `invoice_kind`:

```json
{"source_location": "invoice 998877 (1042)", "entity": "invoice",
 "reason": "An invoice made in Horae already uses number 1042", "invoice_kind": "number_taken"}
```

`invoice_kind` is one of `number_taken`, `amounts_mismatch`, `amount_precision`, `unknown_currency`,
`negative_total`, `client_missing`, `unknown_state`. The error download and `horae jobs errors`
include these rows beside entry errors.

## Link conflicts

A conflict is not an error, because the entry itself imported. Conflicts go in a bounded list next to
`people`, kept up to the same limit as `id_notices` with an omitted count:

```json
"link_conflicts": [
  {"harvest_time_entry_id": 5000, "harvest_invoice_id": 998877,
   "kind": "billed_in_horae", "horae_invoice_number": "INV-202611-003"}
]
```

`kind` is `billed_in_horae` (the entry is on a Horae-made invoice) or `submitted_in_horae`.

## Exit codes

Unchanged: invoice errors are per-record errors like entry errors and do not fail the job.
