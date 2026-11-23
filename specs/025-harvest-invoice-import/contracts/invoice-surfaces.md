# Contract: invoice surfaces for imported invoices

## Server functions and operations

No new server function. Existing ones change as follows.

| Function / operation | Imported invoice (`source = 'harvest'`) |
| --- | --- |
| list, list items, Uninvoiced, detail | returned like any invoice; same `READ` grants; `Invoice.imported` is set |
| detail | `InvoiceDetailView.imported_lines` holds the lines; `lines` is empty |
| draft editor, save draft | refused: conflict "Invoices imported from Harvest cannot be edited" |
| Duplicate (PR-I1) | refused with the same conflict |
| `set_status` | Sent → Paid, Sent → Closed, Sent → Void; anything else refused. Paid sets due to 0 and the paid date to the organization's today. Void reopens linked time as for Horae invoices |
| `set_status` on a Horae invoice | unchanged; Closed is refused |
| PDF export | renders imported lines and the stored breakdown (below) |

Refusal texts go in the text catalog and the error carries the conflict status code.

Status changes made in Horae stay until a later import finds the Harvest invoice changed
(`updated_at` moved); then Harvest's state wins and the report counts the invoice as updated.

## Breakdown of an imported invoice

Rows, in order, omitting zero discount and taxes: Subtotal; Discount (rate as written, when Harvest
gives one); Tax (rate); Second tax (rate); Total; then Paid (total − due) and Due when part of the
total has been paid. Labels are Horae's own.

## Harvest-compatible API (`/harvest/v2/time_entries`, `/v2/time_entries`)

- `is_billed` is `true` when the entry's state is `invoiced`, with or without an invoice (was: when it
  had an invoice).
- No other field changes; there is no invoices endpoint in this API.

## Reports and exports

The billable expression treats an `invoiced` entry as billable whatever its project now says (was:
an entry with an invoice). Exported columns and report totals are otherwise unchanged.

## Plugin events

None for imported invoices or for import links. Sent → Paid and Sent → Void on an imported invoice
made in Horae raise `InvoicePaid` and `InvoiceVoided` as for any invoice; Sent → Closed raises none.
