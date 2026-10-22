# Feature specification: project budget remaining tile

**Branch**: `feat/project-budget-summary` | **Created**: 2026-10-22 | **Base**: `8ae8458e`
**Status**: This branch delivers foundation and server only at head
`0d041745`. Local gates, nine ordered browser suites and a native smoke pass
on package `/nix/store/ykf5kgyyymx5cvhk0rw1gf0d7knnpa5c-horae-0.1.0` (see
[plan.md](plan.md#implementation-record)). Still pending:

- required remote CI on the final head;
- the tile UI;
- its percentage (O1, user-deferred);
- the disclosure (O2).

The feature is not complete.

Input: deliver the Project Detail "Budget remaining" tile from the approved
handoff (`design/project/app/11_Project Detail.dc.html`, stat tiles), backed by
the budgets projects already persist: hour and fee budgets, configured monthly,
task and person scopes, and the legacy lifetime fallback.

This bounded increment partly advances the reconciled #208 specification at
`48a41560`: FR-002 (the budget-remaining tile only), FR-004 (the whole-project
headline, not the scope breakdown) and FR-020–FR-022. Those identifiers differ
from the older `../spec.md` on master. The Total hours tile, its
billable/non-billable split (open question G01 in the unpublished
`feat/project-summary-tiles` draft at `f5784b47`),
charts, costs, invoiced/uninvoiced tiles, tabs and expense budgets (#208 FR-023,
feature 016) are later deliveries. This increment does not answer G01, waive it
or close it.

## Clarifications

### Session 2026-10-22

Answered from repository and official Harvest evidence (research R6). The
handoff draws only the no-budget tile. Budgeted contents follow Harvest's
official Project analysis screenshots, styled with Horae's design. No live
configured Harvest project was observed: the five pages read so far have no
budget.

- P1 (**resolved by evidence**) Q: What does a budgeted tile contain? → A: The
  label "Budget remaining (NN%)" with a signed remaining percentage; the signed
  remaining value, in danger colour when negative; one allowance row; and a
  progress bar of spent share, full when overrun. The allowance row reads
  "Monthly budget" for monthly-reset budgets and "Total budget" otherwise.
  "Monthly budget" is the evidence. "Total budget" comes from Harvest's
  breakdown popover, so using it on the card for lifetime budgets is an
  inference. The superseded no-bar/no-percent proposal was unsupported.
- P2 (**approved by the coordinator**) Q: How are hour quantities written? →
  A: Two decimals with a visible `h` unit on every hour value (`-0.50 h`), and
  each value's accessible name includes "hours". This adapts the existing
  format for accessibility. Harvest's screenshots show only fee budgets.
- P3 (**approved by the coordinator**) Q: What does each non-exact state show?
  → A: Approved 004's unavailable convention: a visible "—" with an accessible
  "unavailable" label, and no number, percent or bar. Withheld money and
  overflow are one wire state and show no reason. `Unallocated` (every
  applicable allowance blank) also shows "—", with no percent or bar. Harvest
  shows neither state.
- P4 (**resolved by evidence**) Q: How is a monthly period labelled? → A: A
  human month and year from the period key (for example "October 2026") above
  the card's content, styled as Horae caption text. Lifetime budgets show no
  caption, as in the handoff. Harvest's "All time" band is not adopted: the
  handoff tiles have no band (inference). "Budget period: YYYY-MM" is existing
  Horae list text, not this presentation.

Open and decided items:

- O1 (**open, deferred by the user**): percent rounding at an exact .5.
  Harvest's 28.75 % → "29%" and −11.53 % → "−12%" contain no tie, and a
  bounded public-bundle and documentation search found no rule. The user
  declined to choose one; it stays pending verification in Harvest. No
  remaining-percentage helper or percentage UI exists, so the percentage part
  of P1 is not deliverable yet.
- O2 (**proposal pending UI review**): a minimal keyboard-operable
  disclosure, built from the smallest existing component, showing only the
  actual allowance ("Total budget") and spent ("Budget spent") totals that the
  official screenshot supports. No Expenses row, no invented zero and no extra
  valuation. The expense breakdown stays an explicit feature-016 dependency.
- O3 (**decided by the coordinator**): an explicit zero allowance shows no
  percentage and no bar. This matches approved 004 and `used_percent`, which
  return `None` for zero budgets. The exact signed remaining value and the
  zero allowance are still shown. Harvest's budget guide confirms explicit-zero
  consumption is over budget; Harvest's zero-allowance card itself has not
  been observed.

Settled, not proposals: budget cardinality and redaction, and the server UTC
date.

## User scenarios & testing

### US1 — See how much budget is left (P1)

An authorized viewer opens a budgeted project and sees how much allowance
remains in the budget's own unit, without opening the Projects list.

Independent test: a lifetime project-scope hour budget of 120 minutes
consumed by 150 qualifying minutes shows "Budget remaining (-25%)",
`-0.50 h` in danger colour, "Total budget 2.00 h" and a full bar (P1, P2). A
monthly fee budget of `EUR 8,000.00` with `EUR 5,700.00` spent shows
"October 2026", "Budget remaining (29%)", `EUR 2,300.00` and "Monthly budget
EUR 8,000.00".

1. Hour budgets show remaining and allowance hours with two decimals and an
   explicit, accessibly named hour unit. Fee budgets show money in the
   project's currency, with no conversion.
1. Remaining is allowance minus consumption. An overrun is negative, shown in
   danger colour and never clamped to zero. Its percentage is negative too.
   The bar shows spent share and is full on overrun.
1. A monthly-reset budget covers the current calendar month and shows it as a
   human month and year. Other budgets cover all recorded time and show no
   period caption.
1. Configured budgets keep their saved unit, scope, rounding and
   non-billable-inclusion settings. Projects without configured settings use
   the existing lifetime legacy budget rules. For the same date and data,
   exact values equal those shown on the Projects list.
1. A budget where every applicable allowance is blank is unallocated: it
   shows no remaining value, and a blank allowance is not zero. An explicit
   zero allowance still counts its consumption. An allowance total that
   overflows is unavailable, not unallocated, even when consumption fits.

### US2 — Understand an absent or unavailable budget (P1)

Independent test: a project with no budget shows the handoff copy; only an
editor sees the setup link, which returns to this project after the edit.

1. With no budget, the tile reads "No budget is set on this project."
1. "Set up a budget" appears only for a viewer who can already edit the
   project, and opens the existing project editor with return to this detail
   page. Other viewers see the sentence alone.
1. A viewer without financial authority sees a fee budget as unavailable; no
   amount, percentage, bar, zero or partial value is shown or sent. This holds
   even when the fee budget is unallocated. A viewer with that authority sees
   an unallocated fee budget as unallocated.
1. A value that cannot be computed exactly (arithmetic overflow) is shown as
   unavailable, never as zero or a wrapped number. On the wire, withheld and
   overflow are the same `Unavailable` state; authorization tests tell them
   apart by reading the same project as both viewers.

### US3 — Trust the tile under change (P1)

Independent test: revoke access or change budget/time data while a detail
read is in flight; the response is either entirely before or entirely after
the change, or a retryable error.

1. Project identity, current authority, budget settings and consumption come
   from one consistent read. Concurrent changes cannot combine values from
   different moments.
1. Current actor, organization, permission policy and financial authority are
   enforced on every read, including after account or policy changes.
   Projects in another organization are not found.
1. Loading, errors, denied access and stale responses keep the detail page's
   existing bound recovery. Reading changes no business data and sends no
   notification.
1. Task and person budget labels, allowances and per-scope consumption are
   not sent to the detail page at all.

### Edge cases

- Archived projects stay readable under existing authority; the setup link
  follows the existing edit affordance (H103 stays undecided).
- Month boundaries and 29 February; entries on the first and last day of the
  month count, adjacent months do not. The month is the server's current UTC
  date, as for the Projects list.
- Rounded consumption differs from raw tracked minutes for configured hour
  budgets; invoiced entries keep their invoice line amounts.
- A project whose configured budget unit is "none" has no budget even if older
  legacy columns hold values.
- Very large and negative values, short screens, both themes and 200% text.

## Requirements

- BFR-001: Show one Budget remaining tile on the project detail page, placed
  as in the handoff, using its label and no-budget copy exactly. A budgeted
  tile follows P1/P4: signed remaining percentage and value, an allowance row
  and a capped spent-share bar. The non-exact states follow P3 and zero
  allowances follow O3. The percentage stays unimplemented while O1 is
  user-deferred.
- BFR-002: Compute the headline with the existing configured budget rules
  (unit, scope, period, rounding, non-billable inclusion, invoice amounts),
  falling back to the existing legacy lifetime rules exactly as the Projects
  list does. No new valuation rule, schema or saved setting. The only new
  arithmetic is the exact headline state (BFR-003), built on the existing
  allocation rule.
- BFR-003: Preserve blank versus zero allowances, signed remaining values,
  exact integer minutes and minor currency units, and the project currency.
  The detail data has four unambiguous states: no budget, unallocated,
  unavailable (withheld money or overflow) and exact. None of the first three
  is zero, and none shares a representation with another.
- BFR-004: Use the current calendar budget month for monthly-reset budgets and
  lifetime otherwise. No later period control may change this headline.
- BFR-005: Read identity, authority, budget settings and consumption in one
  consistent snapshot under the canonical project-read authority (permission
  policies 0 and 1), with existing time limits and a bounded retry when a
  concurrent change invalidates the snapshot. Exhausted retries are a
  retryable error, not partial data.
- BFR-006: Withhold fee values without financial authority and never send
  private task/person scope details. No permission is granted or activated.
- BFR-007: Offer "Set up a budget" only under the existing edit affordance,
  opening the existing editor with this project as its return target.
- BFR-008: Use existing design tokens, utilities and components. Every value
  and the link stay readable and reachable at 320, 390, 768 and 1440 CSS
  pixels, short 200%-zoom screens, both themes and keyboard-only use. Shared
  component defaults stay unchanged.
- BFR-009: Display two-decimal hours from integer arithmetic only, with an
  explicit hour unit visible and in each value's accessible name, without
  changing the existing trimmed-decimal formatter's contract.

### Key entities

Projections of existing records, not new stored data: project identity and
acting account; budget settings (unit, scope, period, rounding, inclusion) or
legacy budget; a whole-project budget headline for one period, with
currency, in exactly one of the states in BFR-003; current read, financial
and edit authority.

## Success criteria

- BSC-001: Seeded legacy, configured, monthly, task/person, blank/zero,
  overrun, overflow, withheld, no-budget and archived cases produce their
  expected state and exact values. Exact values match the Projects list for
  the same date and data. The list's own pair cannot separate unallocated
  from allowance overflow, so those states are compared by state, not to the
  list.
- BSC-002: Unauthorized, foreign-organization and stale requests disclose no
  prohibited value; concurrent changes never produce a mixed response.
- BSC-003: Viewport, theme, zoom and keyboard checks pass, and the existing
  detail header, actions, Undo, editor return, Projects list and reports keep
  passing. Two independent reviews and required checks pass before merge.

## Assumptions and dependencies

The handoff draws only the no-budget state. The budgeted composition follows
the official Harvest screenshots (P1, P4; research R6), in Horae's design
language. Expense-inclusive budgets depend on
feature 016; the tile must not claim to include expenses. Reference208 and
reference212 are untouched. No Harvest account writes.

Concurrent access changes can make a project-editor save return its existing
reload conflict more often. The authority gate now advances the organization
row version without changing its stored values, so a save cannot continue on
a snapshot taken before an access change committed.
