# Contract: Harvest CSV Input Format (secondary source adapter)

CSV is the **secondary, offline source adapter** — the primary source is the live Harvest REST API pull (see `harvest-api.md`, research.md §1). Use CSV for an offline migration, a one-shot host-side load, or when OAuth access is unavailable. The input is Harvest's **detailed time-report CSV** — one row per time entry, denormalized with the parent client/project/task fields on each row. The four Horae entity levels are derived from this single file, and each parsed row becomes the same `SourceRow` the API adapter produces (data-model.md) so the shared engine handles both identically. Column **header names** are matched case-insensitively with surrounding whitespace trimmed; unknown extra columns are ignored.

**Key difference from the API source**: a CSV carries **no stable Harvest ids**, so a CSV run cannot use provenance matching — every record resolves by the composite **natural key** only, and no `harvest_import_map` rows are written. Re-importing the same file is still idempotent (matched by natural key), but matching is not edit-robust the way the API source is.

## Expected columns

| CSV column (Harvest) | Required | Maps to | Conversion |
|---|---|---|---|
| `Date` | yes | entry `spent_date` | parse `YYYY-MM-DD` |
| `Client` | yes | `clients.name` | trim |
| `Project` | yes | `projects.name` | trim |
| `Project Code` | no | `projects.code` | trim; blank → NULL |
| `Task` | yes | `tasks.name` (+ `project_tasks`) | trim |
| `Notes` | no | `time_entries.notes` | blank → NULL |
| `Hours` | yes | `time_entries.minutes` | `round(hours * 60)`, exact integer minutes |
| `Billable?` | yes | entry/project-task `billable` | `Yes`/`No` → bool |
| `Invoiced?` | no | informational only (FR-016) | not mapped to Horae invoice state |
| `First Name` + `Last Name` | see note | separate full-name match key | combine nonblank parts with one space; only used without email |
| `Email` / `User Email` | yes\* | resolve `users` by email (FR-010) | trim, ASCII case-fold |
| `Billable Rate` | no | project-task / task default rate | `round(amount * 100)` → cents |
| `Billable Amount` | no | reconciliation | `round(amount * 100)` → cents |
| `Cost Rate` | no | (user cost rate, informational) | `round(amount * 100)` → cents |
| `Cost Amount` | no | reconciliation | `round(amount * 100)` → cents |
| `Currency` | yes | `clients.currency` / entry money | ISO 4217, 3 letters |

\* **User identity**: at least one email or name column must be present. A nonblank email takes priority and must identify exactly one user in the importing organization. An unknown or ambiguous email is a row error; the importer does not retry it as a name match. If email is absent or blank, the combined name must match exactly one full `users.name` in that organization. A single supplied name part must still match the entire stored name, not a prefix. Matching uses `harvest_norm`: surrounding whitespace is trimmed and ASCII letters are case-folded; internal spacing and non-ASCII case are not normalized. Inactive users remain eligible for historical imports and count when detecting ambiguity. Neither path creates or edits users. This name fallback is CSV-only; API imports always require a unique email (FR-010).

If both `Email` and `User Email` contain values, their normalized values must agree; otherwise that row errors while other valid rows continue. Duplicate identity headers (after trimming and case-folding header names) reject the entire file before writes. Rows with no identity, no matching user, or multiple matches report a clear error and leave no partial client/project/task writes.

## Recognition / rejection

- The file MUST have a header row containing at least the required columns above; otherwise the import is **rejected up front** with a clear message and **no writes** (FR-003).
- An empty file (header only or zero bytes) is rejected the same way.

## Conversion rules (authoritative)

- **Hours → minutes**: `round(hours * 60)` half-up, computed without binary-float error (parse the decimal string), in `horae-core`. Inverse of the existing `hours = minutes / 60` in `crates/horae/src/harvest/`.
- **Money → cents**: `round(amount * 100)` half-up, in `horae-core`. Inverse of the existing `rate = cents / 100`.
- **Booleans**: `Yes`/`No` (case-insensitive) → `true`/`false`.
- All string keys used for matching are trimmed and case-folded before comparison (natural keys, data-model.md).

## Supplementary files (optional, not required)

Dedicated Harvest **Clients** and **Projects** CSV exports MAY later enrich attributes the time report omits (client `address`, project `starts_on`/`ends_on`, budget). When absent, those columns take Horae defaults. Accepting them is out of v1 scope but the mapping is designed to allow it without rework. (The API source already returns these attributes directly, so this gap is CSV-specific.)
