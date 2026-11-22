# Constitution amendment for 026 Task rate templates

**Status**: Draft, for the maintainer's approval. Not applied. After approval, the implementation's
first PR applies the text below verbatim to `.specify/memory/constitution.md`, with a Sync Impact
Report.

**Version change**: 7.0.0 → 7.0.1 (PATCH: a clarification). Principle I's rule does not change:
money is still integer minor units with an explicit ISO currency, and a template is defined as
something that is not money. If the maintainer reads it as materially expanded guidance, it is
7.1.0 instead; the text is the same.

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
> precision.

### Rationale: add one sentence at the end

> A task is shared by projects billed in different currencies, so its default rate cannot carry one
> currency without being wrong for the others; keeping it apart from money until it is applied
> keeps both exact.

Nothing else changes.

## Not proposed, for the maintainer to decide

Principle I says "integer minor units (cents)". Horae stores hundredths of the major unit for every
currency, including zero-decimal ones (JPY 121 is 12100) and three-decimal ones (BHD amounts have no
thousandths); `horae_core::money::format_major_units` documents this. The rule as written reads as
ISO minor units. This amendment does not change it. A later PATCH could say "integer hundredths of
the major unit (cents)", or a later feature could store ISO minor units; either is outside 026.
