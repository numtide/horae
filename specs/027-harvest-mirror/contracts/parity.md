# Contract: parity runs, classes, rules, CSV and alerts

Tables are in [data-model.md](../data-model.md#parity-owned-by-horae-server).

## What is compared

| Layer | Entity | Harvest source | Horae source | Fields |
| --- | --- | --- | --- | --- |
| records | person | `/v2/users` | `users` with provenance | first and last name, email, active, timezone, default rates Horae models |
| records | client | `/v2/clients` | `clients` | name, currency, active, address |
| records | project | `/v2/projects` | `projects` | client, name, code, active, billable, bill-by, budget-by, budget, fee |
| records | task | `/v2/tasks` | `tasks` | name, default billable, default rate (template), active |
| records | task_assignment | `/v2/task_assignments` | `project_tasks` | project, task, billable, rate, active |
| records | user_assignment | `/v2/user_assignments` | `project_management_assignments`, `project_member_budgets` | project, person, manager, budget |
| records | time_entry | `/v2/time_entries` (yearly `from`/`to` windows) | `time_entries` | date, person, project, task, minutes, billable, rate, notes, locked or approved, invoiced, invoice |
| totals | report_client, report_project, report_task, report_person | `/v2/reports/time/{clients,projects,tasks,team}` per month, `include_fixed_fee=false` | `ops::reports::time` grouped the same way | hours (as minutes), billable hours, billable amount per currency |
| totals | uninvoiced | `/v2/reports/uninvoiced` per month | the uninvoiced view behind Invoices | uninvoiced hours, uninvoiced amount per currency |
| invoices | invoice | `/v2/invoices` | `invoices` with `source = 'harvest'` | number, client, state, issue and due dates, subtotal, discount, taxes, total, due amount, line count, linked entries |

Values are normalized with the importer's conversions before comparing: decimal hours to minutes
(half up, as Constitution I's wire rule), decimal amounts to minor units, names trimmed, ids mapped
through provenance. Report hours are decimals that may not be whole minutes, so they are compared
at Harvest's precision: Horae's minute total is written as decimal hours with as many decimals as
Harvest's value has, half up, with integer arithmetic, and the two texts are compared.

## Classes and rules

- `equal`: counted only.
- `missing_in_horae`: Harvest has it, Horae does not.
- `extra_in_horae`: Horae has it (with provenance, or of a mirrored kind), Harvest does not.
- `different`: both have it; `fields` lists every differing field.
- `known` with a rule:
  - `rounding`: an amount differs by at most half a minor unit per line or entry, and Horae's value
    is the sum of its rounded lines (FR-031);
  - `unsupported`: the difference is the amount of entities Horae does not model (expenses in
    `uninvoiced_expenses`, expense lines);
  - `deletion_pending`: missing in Harvest, and the last reconciliation ran before the record's
    disappearance could have been seen;
  - `in_flight`: Harvest's `updated_at` is after the compared watermark.

Rules are applied in that order; the first that matches wins. A finding is `confirmed` when its
fingerprint was in the previous completed run. Unexplained = confirmed and not `known`.

## `ParityRunSummary`

```text
id, job_id, started_at, finished_at, status: succeeded | failed | partial
months: [YYYY-MM]
counts: { records|totals|invoices: { equal, missing_in_horae, extra_in_horae, different, known: { rounding, unsupported, deletion_pending, in_flight } } }
pending, unexplained
```

## CSV download

`GET /api/import/harvest/parity/{run_id}/findings.csv`, session-authenticated, administrators only,
in `horae-api` beside the import error download. Columns:

```text
layer,entity,harvest_id,horae_id,aggregate_key,class,rule,confirmed,field,harvest_value,horae_value,first_seen_run_id
```

One row per differing field (a finding with three fields is three rows; a finding without fields is
one row with empty field columns). UTF-8, RFC 4180 quoting, values as exact text. Streams from the
database; no size limit beyond the 90-day retention.

## Alerts

- A completed run with `unexplained > 0` sets `harvest_mirrors.alert_run_id`. A later completed run
  with `unexplained = 0` clears it.
- While set: a banner on the status page, and a dot on the Harvest navigation entry for
  administrators.
- When `MailConfig` is present: one email per run that sets or keeps the alert with a different
  unexplained count, to every active administrator, with the counts and a link to the status page.
  No email when mail is not configured.
