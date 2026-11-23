# Data model: Import Harvest invoices and link the time they billed

One migration, numbered after the last one on master when it is written (PR-I1 adds `0005`). Column
types follow the baseline.

## Enum changes

```sql
ALTER TYPE invoice_status ADD VALUE 'closed';        -- written off: not due, not paid, time stays invoiced
ALTER TYPE harvest_entity_type ADD VALUE 'invoice';  -- provenance of imported invoices
CREATE TYPE invoice_source AS ENUM ('horae', 'harvest');
```

`horae_core::types::InvoiceStatus` gains `Closed` (`"closed"`) in its display, parse and `pg_enum`
lists. `EntityType` gains `Invoice` (`"invoice"`).

## `invoices` (changed)

| Column | Change |
| --- | --- |
| `source invoice_source NOT NULL DEFAULT 'horae'` | new; only the importer writes `'harvest'` |

Checks:

- `invoice_discount_exact`, `invoice_tax1_exact`, `invoice_tax2_exact` become
  `CHECK (source = 'harvest' OR <current expression>)`.
- New `invoices_closed_imported CHECK (status <> 'closed' OR source = 'harvest')`: Closed exists only
  for imported invoices in this feature (R-5).
- New `invoices_harvest_no_rates CHECK (source = 'horae' OR (discount_bps = 0 AND tax1_bps = 0 AND tax2_bps IS NULL AND tax2_name IS NULL))`: an imported invoice's rates live in `harvest_invoices`.
- Unchanged: unique `(org_id, number)`, non-negative totals and components, `subtotal_cents`
  generated.

For an imported invoice: `number` is Harvest's; `issued_on`, `due_on`, `currency`, `po_number`,
`subject` (PR-I1), `notes` and the amount columns are Harvest's; `status` follows R-5; `created_at` is
the import time.

## `harvest_invoices` (new, one per imported invoice)

| Column | Type | Notes |
| --- | --- | --- |
| `invoice_id` | `uuid PRIMARY KEY` | FK `(invoice_id, org_id)` → `invoices (id, org_id)` |
| `org_id` | `uuid NOT NULL` | FK `organizations` |
| `harvest_invoice_id` | `bigint NOT NULL` | unique per `org_id`; also in `harvest_import_map` |
| `due_cents` | `bigint NOT NULL` | `CHECK (due_cents BETWEEN 0 AND <total>)` checked by the import |
| `paid_on` | `date` | Harvest's paid date, when given |
| `discount_rate` | `numeric` | percent as Harvest wrote it; NULL when none; display only |
| `tax1_rate` | `numeric` | same |
| `tax2_rate` | `numeric` | same |
| `period_start`, `period_end` | `date` | Harvest's period, display only |
| `harvest_updated_at` | `timestamptz NOT NULL` | to count a re-import as skipped |

Paid amount shown = total − `due_cents`. Step 3 (payments) can create one payment of that amount on
`paid_on` per imported invoice.

## `harvest_invoice_lines` (new)

| Column | Type | Notes |
| --- | --- | --- |
| `id` | `uuid PRIMARY KEY` | v7 |
| `org_id` | `uuid NOT NULL` | |
| `invoice_id` | `uuid NOT NULL` | FK `(invoice_id, org_id)` → `invoices`, `ON DELETE CASCADE` |
| `position` | `integer NOT NULL` | Harvest's order; unique per invoice |
| `harvest_line_id` | `bigint NOT NULL` | |
| `kind` | `text NOT NULL` | 1–100 characters |
| `description` | `text NOT NULL` | |
| `quantity` | `numeric NOT NULL` | display only |
| `unit_price` | `numeric NOT NULL` | exact decimal as Harvest wrote it (`95.2381`, JPY `80.50`); display only, never used in arithmetic (FR-004) |
| `amount_cents` | `bigint NOT NULL` | may be negative; authoritative |
| `taxed`, `taxed2` | `boolean NOT NULL` | |
| `project_id` | `uuid` | the imported project when Harvest names one and it resolved |

A re-import replaces an invoice's lines (delete and insert) only when `harvest_updated_at` moved.

## `time_entries` (unchanged schema)

States used by the import (R-6):

| Meaning | `state` | `invoice_id` |
| --- | --- | --- |
| billed on an imported invoice | `invoiced` | the imported invoice |
| billed in Harvest, invoice not imported | `invoiced` | NULL |
| not billed | `open` (as created) | NULL |

`rounded_minutes` is frozen on link and cleared on unlink, as generation and void do.

## `harvest_import_map`

Imported invoices get rows with `harvest_entity_type = 'invoice'`, `horae_id` = the invoice id and
`harvest_updated_at`, like the other entity types. The account switch's provenance check therefore
covers them with no code change (FR-021).

## `harvest_credentials.synced_watermark`

Gains the key `invoice`. Absent means invoices were never imported: the run reads invoices and time
entries in full (R-7).

## Domain types (`horae-core`)

- `importers::harvest::invoice::map_state(&str) -> Result<Mapped, …>` where `Mapped` is `Skip`
  (draft), `Import(InvoiceStatus)` or an error for an unknown state.
- `importers::harvest::invoice::check_amounts(lines, subtotal, discount, tax1, tax2, total, due)`:
  the lines sum to the subtotal, `subtotal − discount + tax1 + tax2 = total`, `0 ≤ due ≤ total`,
  `total ≥ 0`; errors name the difference in minor units.
- `importers::harvest::convert::money_exact(text, currency) -> Result<i64, …>`: signed, refuses more
  decimals than the currency has and unknown currencies.
- `importers::harvest::link::decide(harvest: Billing, current: Current) -> LinkAction` where
  `Billing` is `NotBilled | Draft | Imported { invoice: Uuid, voided_in_horae: bool } | Missing` and
  `Current` describes the entry's state and whether its invoice is imported or Horae-made; `LinkAction` is `Link(Uuid)`,
  `MarkBilled`, `Unlink`, `Keep`, `Conflict`.
- `types::ImportSummary` gains `invoices: EntityCounts` and `links: LinkCounts { linked, unlinked, billed_without_invoice, conflicts }`, both `#[serde(default)]`.
- `types::SourceRow` gains `harvest_invoice_id: Option<i64>`; its `invoiced` flag keeps its meaning
  (Harvest's billed flag) and now drives FR-011/FR-022.

## Wire types (`horae-types`)

- `Invoice` gains `imported: Option<ImportedInvoice>` with `due_cents`, `paid_on`, the three rates as
  decimal text and the period.
- `ImportedInvoiceLine { kind, description, quantity: String, unit_price: String, amount_cents, taxed, taxed2, project_id }`; `quantity` and `unit_price` are exact decimal text for display.
- `InvoiceDetailView` gains `imported_lines: Vec<ImportedInvoiceLine>` (empty for Horae invoices).
