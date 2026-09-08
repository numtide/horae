# Invoice Rendering Contract

**Status: Planned.** Implements FR-025. Renders an invoice to a print-ready, reproducible PDF using **Typst** (see [research.md](../research.md)); fonts are supplied from nixpkgs for reproducible typography.

## Inputs

1. **Invoice data** — the invoice and its line items (see [data-model.md](../data-model.md) and [contracts/server-functions.md](./server-functions.md)): number, client, issue/due dates, currency, line items (`description`, `minutes`, `rate_cents`, `amount_cents`), and `total_cents`. This mirrors the Harvest-shaped export data, so the same JSON that drives [contracts/harvest-api.md](./harvest-api.md) can drive rendering (as in `eureka-cpu/nvoice`).
1. **Branding / provider settings** — from the organization: provider identity, bank/payment details, logo, and the default template selection.
1. **Editable fields** — reviewer-adjustable values (notes, payment terms, provider identity overrides) captured before finalize/send.
1. **Template** — a Typst `.typ` template (default `crates/horae/templates/invoice.typ`); operators MAY customize or supply their own.

## Output

### Billing minutes and historical quantities

Invoice line `minutes` are effective billing minutes, not raw worked time.
`horae_core::rounding::effective_minutes` and its PostgreSQL equivalent use a
persisted `rounded_minutes` value when present (including zero); otherwise they
apply the organization's current rule to each entry. Invoice creation freezes
the selected quantity on the entry in the same transaction as the line snapshot.
Changing organization rounding afterwards does not change those quantities.

Grouped reports, detailed reports and their CSV/XLSX exports, Harvest's
`rounded_hours`, and the monetary project-spend calculation use this same rule.
Raw hours and labor cost remain based on worked minutes. The invoice PDF reads
the line snapshot and does not recompute rounding at render time.

Migration `0017_effective_minutes.sql` fills missing frozen quantities for
already-invoiced entries from their own invoice line, never from today's rule.
It does not overwrite existing frozen values, line quantities, rates, or amounts.
Historical discrepancies already recorded in invoice lines require a separate
review; the migration does not silently correct issued documents. Rate and
currency resolution are unchanged by this rounding correction.

### PDF

- A single PDF per invoice whose line items and `total_cents` reconcile **exactly** with the invoice (FR-012/FR-023/SC-007).
- **Deterministic**: identical inputs (invoice + branding + template + fonts) MUST produce byte-identical output (FR-025) — no timestamps or nondeterministic ordering baked in.

## Behavior

1. The manager may review and adjust the invoice's editable fields; the preview reflects the same template that produces the final PDF.
1. Rendering does not mutate authoritative data — the invoice/line records are the source of truth; the PDF is a derived artifact and MAY be cached/regenerated.
1. Fonts referenced by the template MUST be resolvable from the packaged font set (nixpkgs), so rendering never depends on host-installed fonts.

## Notes

- The same rendering path is intended to serve **timesheet/report PDFs** later (`crates/horae/templates/timesheet.typ`); this contract covers invoices for v1.
- Fallback: `printpdf` is retained only as a documented fallback if Typst is unavailable (research.md); it is not part of this contract's guarantees.
