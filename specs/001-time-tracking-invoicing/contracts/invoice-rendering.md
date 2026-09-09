# Invoice Rendering Contract

**Status: Default invoice PDF implemented.** Implements FR-025 using **Typst**
(see [research.md](../research.md)), the embedded invoice template, and embedded
typst-kit fonts. Host font directories are not searched. Planned customization
and review features below are not part of the current export endpoint.

See [export execution and resource limits](exports.md) for admission, dataset
and file limits, and the distinction between a request timeout and termination
of a native renderer.

## Inputs

1. **Invoice data** — the stored invoice and line-item DTOs (see [data-model.md](../data-model.md) and [server-functions.md](server-functions.md)): number, client, issue/due dates, currency, line descriptions, minutes, rates, amounts, and total. This is separate from the read-only Harvest-style REST surface.
1. **Branding / provider settings** — current organization provider identity and bank/payment details. Logo and template selection remain planned.
1. **Editable fields (planned)** — reviewer-adjustable values captured before finalize/send. The current export reads stored invoice notes and current organization payment/provider settings.
1. **Template** — `crates/horae/templates/invoice.typ`, embedded at compilation. Runtime-supplied templates are not implemented.

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

### Monetary limits

Per-line amounts use `(rate_cents * minutes + 30) / 60`, with half-cent ties up
for non-negative stored rates and durations, not banker's rounding. Rust uses
an `i128` intermediate and SQL uses exact `numeric` integer division; both reject
a final result outside `i64` instead of wrapping or clamping it. Migration
`0018_checked_line_amount.sql` replaces only the SQL function implementation;
it does not recalculate stored invoices or change function permissions.

Invoice creation checks both each line and the accumulated total. An
unrepresentable invoice returns a conflict without writing an invoice, lines,
or changed entry states. Reports likewise reject an unrepresentable aggregate rather
than returning a partial or wrapped total.

The Typst money formatter also uses integer division, preserving every digit of
large valid amounts instead of losing precision through a floating-point value.

### PDF

- A single PDF per invoice whose line items and `total_cents` reconcile **exactly** with the invoice (FR-012/FR-023/SC-007).
- **Deterministic**: identical inputs (invoice + branding + template + fonts) MUST produce byte-identical output (FR-025) — no timestamps or nondeterministic ordering baked in.

## Behavior

1. The planned review UI may adjust editable fields before finalization; the current endpoint renders the stored invoice without a separate preview workflow.
1. Rendering does not mutate authoritative data — the invoice/line records are the source of truth; the PDF is a derived artifact and MAY be cached/regenerated.
1. Fonts are resolved from the embedded typst-kit font set, so rendering does not depend on host-installed fonts.

## Notes

- The same rendering path is intended to serve **timesheet/report PDFs** later (`crates/horae/templates/timesheet.typ`); this contract covers invoices for v1.
- Fallback: `printpdf` is retained only as a documented fallback if Typst is unavailable (research.md); it is not part of this contract's guarantees.
