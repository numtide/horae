# Contract: invoices on `/api/v1`

Additions to [022's contract](../../022-public-api-v1/contracts/api-v1.md); its conventions (base
path, bearer token, media types, ids, money, instants, lists, errors) hold. The authoritative
description is the generated document, `crates/api/openapi/v1.json`, tag `invoices`.

## Authentication

A personal access token (`Authorization: Bearer horae_pat_…`) whose owner may view all invoices.
OAuth access tokens are refused: 403 `insufficient_scope` with
`WWW-Authenticate: Bearer error="insufficient_scope"` (no scope names invoices).

## `GET /invoices` (`listInvoices`)

| Query | Type | Notes |
| --- | --- | --- |
| `limit` | 1–1000, default 100 | |
| `cursor` | string | from `next_cursor`; bound to the other filters |
| `status` | `draft` | `sent` | `paid` | `closed` | `void` | another value: 400 `bad_request` |
| `client_id` | uuid | |
| `updated_since` | RFC 3339 | invoices whose `updated_at` is at or after it |

200: `Page_Invoice`, `{ "data": [Invoice…], "next_cursor": "…" | null }`, in `id` order.

```json
{
  "id": "019abc…",
  "number": "2026-014",
  "status": "sent",
  "source": "horae",
  "client_id": "019abd…",
  "client_name": "Example Ltd",
  "subject": "November",
  "issued_on": "2026-11-30",
  "due_on": "2026-12-30",
  "overdue": false,
  "currency": "EUR",
  "subtotal": { "amount_minor": 120000, "currency": "EUR" },
  "discount": { "amount_minor": 0, "currency": "EUR" },
  "tax": { "amount_minor": 0, "currency": "EUR" },
  "second_tax_name": null,
  "second_tax": { "amount_minor": 0, "currency": "EUR" },
  "total": { "amount_minor": 120000, "currency": "EUR" },
  "po_number": "",
  "notes": null,
  "created_at": "2026-11-30T10:00:00Z",
  "updated_at": "2026-11-30T10:05:00Z"
}
```

## `GET /invoices/{id}` (`getInvoice`)

200: `InvoiceDetail`, every field of `Invoice` and

```json
"lines": [
  {
    "description": "Design review",
    "time_entry_id": "019abe…",
    "minutes": 90,
    "hourly_rate": { "amount_minor": 8000, "currency": "EUR" },
    "amount": { "amount_minor": 12000, "currency": "EUR" }
  }
]
```

## `GET /invoices/{id}/pdf` (`getInvoicePdf`)

200: `application/pdf`, `Content-Disposition: attachment; filename="invoice-<number>.pdf"`, the
web app's PDF, byte-identical while the invoice and the organization's details are unchanged.

## Errors

| Status | `code` | When |
| --- | --- | --- |
| 400 | `bad_request` | a parameter of the wrong type or an unknown status |
| 400 | `invalid_cursor` | a cursor tampered with, expired, or from other filters |
| 401 | `unauthenticated` | no valid token |
| 403 | `forbidden` | the owner may not view all invoices ("Current access is required") |
| 403 | `insufficient_scope` | an OAuth access token |
| 404 | `not_found` | no such invoice in the organization |
| 413 | `too_large` | the PDF's content is over the export's limits |
| 422 | `invalid` | `limit` out of range |
| 429 | `rate_limited` | as every operation |
| 503 | `unavailable` | too many exports at once; try again |
| 504 | `timeout` | the PDF took too long |
