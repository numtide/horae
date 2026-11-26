# Data Model: Invoices in the public API

## Storage (migration 0011)

`invoices.updated_at timestamptz NOT NULL DEFAULT clock_timestamp()`

- Backfill: `updated_at = created_at`.
- Trigger `invoices_set_updated_at`, BEFORE UPDATE, `set_updated_at()` (stamps
  `clock_timestamp()`, migration 0006) when any of these differ between `OLD` and `NEW`:
  `client_id, number, status, issued_on, due_on, currency, total_cents, notes, po_number, discount_bps, tax1_bps, tax2_name, tax2_bps, discount_cents, tax1_cents, tax2_cents, subject, source`. A new stored column of `invoices` joins this list.
- Index `invoices_org_id_updated_at_idx (org_id, updated_at)`.

## Shared reads (`ops::invoices::reads`)

| Name | Shape |
| --- | --- |
| `InvoiceQuery` | `status: Option<InvoiceStatus>`, `client_id: Option<Uuid>`, `updated_since: Option<DateTime<Utc>>`, `after: Option<Uuid>`, `limit: u32` |
| `InvoiceRecord` | `item: InvoiceListItem`, `updated_at` |
| `InvoiceDetailRecord` | `detail: InvoiceDetailView`, `updated_at` |
| `page(tx, actor, &InvoiceQuery)` | up to `limit + 1` records in `id` order |
| `get(tx, actor, id)` | `detail` plus `updated_at` |
| `require(tx, actor, id)` | the refusals of `get`, for the PDF |

## Public types (`v1::types`)

`Invoice`

| Field | Type | Notes |
| --- | --- | --- |
| `id` | uuid | |
| `number` | string | |
| `status` | `draft` | `sent` | `paid` | `closed` | `void` | `closed`: written off, imported only |
| `source` | `horae` | `harvest` | |
| `client_id`, `client_name` | uuid, string | the client's name now |
| `subject` | string | empty when unset |
| `issued_on`, `due_on` | date | |
| `overdue` | bool | sent, unpaid, past due on the organization's today |
| `currency` | string | ISO 4217 |
| `subtotal`, `discount`, `tax`, `second_tax`, `total` | Money | hundredths |
| `second_tax_name` | string | null | |
| `po_number` | string | empty when unset |
| `notes` | string | null | |
| `created_at`, `updated_at` | date-time | |

`InvoiceDetail`: every field of `Invoice`, and `lines: InvoiceLine[]` in the invoice's order.

`InvoiceLine`: `description`, `time_entry_id` (uuid | null), `minutes` (int | null),
`hourly_rate` (Money | null), `amount` (Money). Fees and imported lines have `null` time entry,
minutes and rate.
