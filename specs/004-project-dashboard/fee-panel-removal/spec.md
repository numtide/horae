# Feature specification: remove the Project Detail fee balances panel

**Branch**: `fix/project-detail-fee-panel` | **Created**: 2026-10-26 | **Base**: `8975b881` (#305)

This bounded child increment of feature 004 ports #208 task F034 (fee-panel
part only) from `feat/project-dashboard` at `48a41560`. It does not regenerate
the dashboard specification, complete any other dashboard slice, or answer,
waive or close G01, O1 or any open feature 015 gate. It activates no policy.
Input: remove the extra Fee balances panel from Project Detail, as the user
requested, while fee accounting, invoice preparation and server authorization
stay unchanged.

## User scenarios & testing

### US1 — Project Detail without the extra panel (P1)

A billing manager opens a project and sees the designed detail page: header,
project information, tasks and team. The page has no extra fee-balance
form or accounting prose.

Independent test: open a fixed-fee project as an administrator under the legacy
policy, where the panel used to appear; it is absent at every width and no
fee-balance read is issued, while header actions, Edit project and the lists remain.

1. Given an authorized manager, the detail page has no "Fee balances" region,
   period fields or Refresh balances action.
1. Header identity, archive/reactivate/Undo, archived-client refusal recovery,
   tasks/team names and Edit project behave as before.

### US2 — Fee balances stay in invoice preparation (P1)

Independent test: prepare an invoice for the same project; "Fees to invoice"
shows per-occurrence agreed, invoiced (including drafts) and signed remaining
amounts, recovers from a failed review and refuses a revoked manager.

1. Partial and over-invoiced fees show exact amounts, including negatives.
1. Monthly fees show one balance per occurrence; preview materializes nothing.
1. A failed preparation read shows an error and recovers on retry.
1. A replayed preparation request from a demoted account is refused (403) and
   discloses no amount.

### Edge cases

- Losing access is still detected by the remaining Project Detail reads and the
  status write (401 and 403), hiding the loaded project behind the bound Retry
  without showing server diagnostics.
- A 320–1440px viewport has no horizontal page scroll after the removal.

## Requirements

- PFR-001: Project Detail renders no fee-balance panel, date range, refresh
  action or fee accounting text, and no longer requests fee balances.
- PFR-002: Fee calculation, invoice preparation balances and the authorized
  fee-balance server function are unchanged. No SQL, schema, migration,
  dependency, shared component or global CSS changes.
- PFR-003: Header binding, access-loss handling, status changes with Undo and
  409 archived-client recovery, task/team lists and the shared editor link are
  unchanged.
- PFR-004: Removed tests are replaced by equivalent assertions where the
  behavior still exists: balances, recovery and authorization in invoice
  preparation; access loss through the status write.
- PFR-005: No replacement prose, invented UI, policy activation, real-account
  write or persistent data change.

### Key entities

Existing fee occurrences, invoice line contributions and the acting account stay
authoritative. Nothing new is stored.

## Success criteria

- PSC-001: The panel is absent for every role and policy on Project Detail.
- PSC-002: Each removed financial, recovery or authorization assertion has an
  equivalent passing assertion in invoice preparation or the status flow.
- PSC-003: Focused SSR and browser suites, server/WASM lint and formatting pass.
  Two independent reviews are done before publication.

## References, assumptions and clarification

The visual authority is `design/project/app/11_Project Detail.dc.html`; it has
no fee-balance panel (its only fee references are the Invoiced tile's "Total
project fees" line and the editor's billing controls). #208 spec at `48a41560`
records "Fee balances remain in invoice preparation, not in an extra dashboard
panel" and US/SC-004 "Fee balances match invoice preparation". The user
explicitly requested this removal. The saved Harvest evidence for #208 is reused; no fresh
account read or login was needed.

Clarification scan: scope, authority, recovery, data and completion follow from
F034 and the existing contracts. No product question is open for this increment.
The chart-diagnostic half of F034 does not apply: master has no activity chart.
