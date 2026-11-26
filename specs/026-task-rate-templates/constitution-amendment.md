# Constitution amendment for 026 Task rate templates

**Status**: Approved by the maintainer on 2026-11-23, with the last three sentences of the
Principle I paragraph added the same day (review finding H3). Applied verbatim to
`.specify/memory/constitution.md` as version 7.2.0 by the templates PR (delivery 2 in plan.md, #484),
with a Sync Impact Report. The rounding PR (delivery 1) does not need it: it only rounds
money.

**Version change**: MINOR, 7.1.0 → 7.2.0, as the maintainer chose (materially expanded guidance on
Principle I). The OAuth amendment of spec 024 takes 7.1.0 and lands first. If it has not landed when
the templates PR starts, that PR waits for it rather than taking 7.1.0.

## Why

Spec 026 makes a task's default rate a number with no currency, which becomes a rate in a project's
currency when the task is added to the project. Principle I says that money is integer minor units
"with an explicit ISO 4217 currency code". Without a note, a currency-free amount reads as a breach
of that rule, and a later change could start summing or invoicing templates as if they were money.
The note says what a template is and where its use stops.

## Proposed text

### Principle I: add a paragraph after "Decimal wire formats (not an exception)"

> **Rate templates (not money)**: a rate template, such as a task's default hourly rate, is an
> integer at a fixed scale of two decimals with no currency. It is not money until it is applied to
> something that has a currency, such as a project, where it becomes a rate of the same number in
> that currency. A template MUST NOT be summed, reported, invoiced, compared with money or given a
> currency by any other route, and a currency MUST NOT be guessed for it. Amounts computed from an
> applied rate are money and follow the rules above, rounded once, half up, to the currency's
> precision. A rate, whether applied from a template or typed, keeps two decimals in every currency,
> even where the currency has fewer (JPY 80.50 per hour); only amounts computed from a rate round to
> the currency's precision. A currency's precision is one whole unit for a currency with no decimals
> and one hundredth for every other currency, the finest unit Horae stores. An interface that writes
> such a rate as a decimal, such as the Harvest-compatible API, writes its two decimals; this does not
> breach the decimal wire formats rule above.

### Rationale: add one sentence at the end

> A task is shared by projects billed in different currencies, so its default rate cannot carry one
> currency without being wrong for the others; keeping it apart from money until it is applied
> keeps both exact.

Nothing else changes.

## Follow-up, not part of this amendment

Principle I says "integer minor units (cents)". Horae stores hundredths of the major unit for every
currency, including zero-decimal ones (JPY 121 is 12100) and three-decimal ones (BHD amounts have no
thousandths); `horae_core::money::format_major_units` documents this. The rule as written reads as
ISO minor units. The maintainer decided on 2026-11-23 to fix that wording in a separate, later PATCH
outside spec 026. This amendment leaves it as it is.
