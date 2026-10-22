# Feature Specification: Currency-separated totals in the legacy Time report (R02)

**Feature Branch**: `fix/report-currency-totals`

**Created**: 2026-10-22

**Status**: Draft (clarified; plan reviewed before application edits)

**Parent**: `specs/001-time-tracking-invoicing` — refines FR-016 and FR-023 and
their measures SC-002 and SC-007 for one surface. It does not amend the parent
spec, and does not change `specs/015-scoped-permissions/contracts/time-reports.md`
(the scoped reader has no monetary fields; policy 1 is not activated here).

**Input**: Bounded UI parity increment R02: exact currency-separated monetary
totals in the existing legacy Time report total row.

## Context

The legacy Time report shows EUR and USD rows with an em dash as the total
billable amount. The current total refuses to add different currencies, which
is correct, but then shows nothing at all. Harvest's Time report summaries break
amounts out per currency, with no conversion, and value cost in the account's
default currency. The sources, who opened each of them, and the coordinator's
before-state baseline are in [research.md](research.md#evidence).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See every currency's exact total (Priority: P1)

A manager runs the Time report for a period in which clients bill in different
currencies. The total row shows one exact billable total per currency, each
labelled with its currency code, so nothing is hidden and nothing is converted.

**Why this priority**: This is the defect. The total row is currently blank for
any multi-currency period.

**Independent Test**: Load a period containing EUR and USD rows and read the
Billable amount total cell.

**Acceptance Scenarios**:

1. **Given** rows billing EUR 100.00 and EUR 250.00 and USD 40.00, **When** the
   Time report loads, **Then** the Billable amount total shows `EUR 350.00` and
   `USD 40.00` on separate lines, in that order.
1. **Given** one EUR row and one USD row each holding the largest representable
   amount, **When** the report loads, **Then** both totals are shown exactly.
   Values in different currencies are never added to each other, so the range
   cannot be exceeded across currencies.
1. **Given** every row bills in one currency and the total is in range, **When**
   the report loads, **Then** the total reads exactly as it does today (for
   example `EUR 350.00`), on one line.

______________________________________________________________________

### User Story 2 - Restricted and out-of-range totals stay truthful (Priority: P1)

A manager who cannot see some cost overrides never gets a partial cost total,
and a total that does not fit the supported range says so instead of showing a
wrong number.

**Why this priority**: A wrong or partial monetary figure is worse than the
blank shown today.

**Independent Test**: Load rows where one cost is withheld, and rows whose
same-currency total exceeds the supported range.

**Acceptance Scenarios**:

1. **Given** any loaded row's cost is withheld, **When** the report loads,
   **Then** the Cost total shows only `Restricted`, with no per-currency lines,
   even if the other rows are visible, in other currencies or out of range.
1. **Given** EUR rows whose sum exceeds the supported range and a USD row of
   USD 5.00, **When** the report loads, **Then** the total shows
   `EUR amount exceeds supported range` and `USD 5.00`.
1. **Given** rows whose running sum would temporarily exceed the range but whose
   final sum fits, **When** the report loads, **Then** the exact final sum is
   shown, whatever the row order.

______________________________________________________________________

### User Story 3 - Cost stays in the organization currency (Priority: P2)

Cost is valued in the organization's default currency regardless of each
client's billing currency, so the Cost total shows a single line even when the
billable amounts span several currencies.

**Independent Test**: Load USD and CHF billing rows in an organization whose
default currency is EUR.

**Acceptance Scenarios**:

1. **Given** USD and CHF billing rows with visible costs of 0.50 and 0.75 in an
   EUR organization, **When** the report loads, **Then** the Cost total is
   `EUR 1.25` on one line.

### Edge Cases

- No rows in the period: the existing empty state is shown and there is no
  total row (unchanged).
- A currency whose rows bill nothing still gets a line (for example `USD 0.00`).
  The row is loaded, and its zero total is a fact.
- Changing filters or grouping, or a reload, keeps the existing loading and
  error/retry states. The total row only ever reflects the loaded response, and
  currency order does not depend on grouping or row order.
- Negative amounts, if present, are summed exactly like positive ones.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-R02-001**: The Billable amount and Cost total cells MUST show one exact
  sum for each distinct currency present in the loaded rows. Totals are grouped
  by the exact stored code, so distinct stored codes are distinct currencies.
  Each line is labelled with the existing money format, which trims surrounding
  whitespace from the code. Standard ISO codes display unchanged. Amounts in
  different currencies MUST never be added together or converted.
- **FR-R02-002**: Currency lines MUST be ordered by stored code (byte order),
  independent of row order, grouping and filters.
- **FR-R02-003**: When every loaded row shares one currency, the cell MUST read
  exactly as it does today for an in-range total.
- **FR-R02-004**: If any loaded row's amount for that column is withheld, the
  cell MUST show only `Restricted`. This takes precedence over every other
  outcome, and no partial or per-currency figure may be disclosed.
- **FR-R02-005**: A per-currency total that cannot be represented exactly MUST
  read `<code> amount exceeds supported range` on its own line. Other currencies'
  totals MUST stay exact. Being out of range depends only on the final sum,
  never on summation order.
- **FR-R02-006**: The change MUST NOT alter the row cells, hours totals, filters,
  grouping, the detailed tab, the CSV/XLSX downloads, report queries, schema,
  permissions, valuation rules or shared styles. It MUST NOT add columns (such as
  uninvoiced amount) or controls.
- **FR-R02-007**: The per-currency lines MUST stay readable and in reading order
  in both themes, on desktop and mobile, with text enlarged to 200%, and through assistive
  technology. Each line is plain text in the existing total cell.

### Key Entities

- **Report row (existing)**: billable amount with its billing currency, and an
  optional cost with the organization's currency. Neither changes.
- **Currency total (derived, display-only)**: stored currency code plus either an exact
  minor-unit sum or "out of range". It is not stored or exported.

## Success Criteria *(mandatory)*

- **SC-R02-001**: For every tested multi-currency fixture, each displayed
  currency total equals the exact sum of that currency's row amounts (zero
  discrepancy). This applies SC-002 per currency.
- **SC-R02-002**: No tested state shows an em dash, a converted figure or a
  partial total in either monetary total cell while rows are loaded.
- **SC-R02-003**: Single-currency reports whose totals are in range render
  character-for-character as before this change. Out-of-range wording is
  governed by FR-R02-005 alone.
- **SC-R02-004**: CSV/XLSX downloads are unchanged (SC-007 unaffected). No
  export code or query changes, and the existing suites' export assertions pass
  unmodified.

## Clarifications

### Session 2026-10-22

- Q: When a cost is withheld and another currency overflows, which wins? →
  A: `Restricted` for the whole cell. Restriction takes precedence, with no
  partial disclosure.
- Q: Same-currency overflow alongside other currencies? → A: Only that
  currency's line reports the range problem. The others stay exact.
- Q: Order of currency lines? → A: Stored code in byte order. This is deterministic and
  matches the stable report ordering.
- Q: Should a currency whose rows total zero be listed? → A: Yes. It has loaded
  rows, and omitting it would hide a fact the row cells already show.
- Q: Does a withheld amount still win if an earlier row already pushed a
  currency out of range? → A: Yes. Withheld is decided over the whole set
  before any total is reported.
- Q: Is overflow judged on running or final sums? → A: On the final sum, so the
  result does not depend on row order.

## Assumptions

- The cost currency is always the organization default (it comes from the
  organization record in the report query). The rule is still applied
  generically per currency, so a second cost currency would render correctly.
- A currency is identified by its exact stored code. The schema guarantees only
  length three for organization, client, project and invoice codes, not
  uppercase. For grouping and ordering, codes are never case-folded, trimmed or
  merged, and this increment adds no validation. Display uses the existing shared
  money formatter unchanged, so a code with surrounding whitespace gets its own
  line but displays trimmed.
- Out of scope: uninvoiced amounts, Team-row withholding, the report builder,
  scoped (policy 1) reports, saved reports and other Reports parity work.
- This increment does not complete Reports. The following remain pending under
  their own requirements:
  - uninvoiced and other financial-family summaries;
  - billability breakdowns;
  - complete filter candidates and multi-selection;
  - grouped exports carrying the current grouping;
  - period navigation;
  - the report builder and saved reports;
  - scoped financial reporting and policy 1 activation;
  - the open T203 work in `015`.
