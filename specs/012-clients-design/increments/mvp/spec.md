# Feature Increment: Clients MVP

**Feature Branch**: `feat/clients-mvp`

**Created**: 2026-10-02

**Status**: Implemented and verified; delivered in PR #216 (parent 012 remains incomplete)

**Input**: User-authorized implementation of searchable/filterable Clients, useful
detail, existing-field create/edit and contextual navigation, following `design/`
and comparing behavior with Harvest. Parent: [feature 012](../../spec.md).

## Clarifications

### Session 2026-10-02

- Q: Does the increment require activating the new six-profile model? → A: No. Preserve and test current runtime permissions; do not partially activate feature 015 or widen access to make the interface work.
- Q: Must pending contacts, lifecycle and bulk-action decisions be resolved tonight? → A: No. Do not implement new contacts, changed archive policy, deletion or bulk actions. Preserve existing functionality and record full-feature gaps without silently removing them.
- Q: What reference and verification access is authorized? → A: Current official Harvest documentation and read-only authenticated browser inspection; local mutations/tests only with synthetic data and disposable databases. No real records, messages, permissions or existing databases may be changed.
- Q: Are inactive clients ineligible for every creation flow? → A: Preserve existing destination policies: new projects require an active client; invoice recovery and historical billing retain current organization/manager checks. The active-only invoice picker is not a new server-side prohibition.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Find a client (Priority: P1)

A signed-in user searches the client catalogue, filters its lifecycle/currency
and opens the correct client instead of a placeholder.

**Independent Test**: Use active/inactive clients, duplicate/long/non-Latin names
and multiple currencies in two organizations; combine filters and reload/retry.

**Acceptance Scenarios**:

1. Name search is case-insensitive and combines with active/inactive/all and
   currency filters. Empty query restores the selected filter scope.
1. Counts and currency choices are derived only from authorized records. The
   preferred currency and authorized project currencies participate as specified
   by the parent FR-004; unrelated hidden projects cannot affect a match.
1. No clients, no matching clients, pending loads and failed loads are distinct;
   clearing filters or retrying performs a real recovery.
1. Client links use identifiers, not names, and lead to that client's detail.

### User Story 2 — Review a client's work (Priority: P1)

A user sees the client's saved identity and billing information, its authorized
projects and, where permitted, actual invoices with separate currencies.

**Independent Test**: Open active/inactive/unknown/foreign clients under the
current roles; reconcile every displayed row and amount against fixtures.

**Acceptance Scenarios**:

1. Detail shows the actual name, preferred currency, saved address/tax identifier
   and other existing permitted billing fields; absent values are not fabricated.
1. Project rows obey existing visibility/progress boundaries and retain their
   real identity, currency, lifecycle and destination. Restricted rows and fields
   are absent from responses and derived counts, not merely hidden in the UI.
1. Invoice rows require existing invoice authority, use stored status/amounts,
   identify ISO currency and open the real invoice. No cross-currency total or
   guessed external billing status is presented.
1. Invalid, unknown and foreign identifiers fail without revealing foreign
   client existence. Rapid client navigation never displays stale client data.

### User Story 3 — Maintain existing client information (Priority: P1)

An authorized manager creates or edits a client using the same accessible form
from list and detail, preserving historical work.

**Independent Test**: Create, reload, edit, cancel, inject a failed save and retry;
compare all linked project/invoice values before and after.

**Acceptance Scenarios**:

1. Existing name, preferred currency, address, tax identifier and applicable
   default-rate fields have real persistence and consistent presentation.
1. Invalid names, currencies or rates are rejected at the trusted boundary with
   useful errors. Failed saves preserve all entered values; cancel changes none.
1. Saving cannot silently reinterpret an existing rate in another currency or
   rewrite existing project/invoice amounts, currency or terms.
1. Pending submissions cannot be duplicated or silently dismissed. Successful
   updates refresh both list/detail data and restore meaningful keyboard focus.
1. Existing single-client activation/deactivation remains available to its
   current authorized users with unchanged semantics and truthful failure states;
   it is not presented as a new cascading archive implementation.

### User Story 4 — Continue into projects and invoices (Priority: P1)

An authorized user starts existing project/invoice workflows for the selected
client without losing unrelated work.

**Independent Test**: Start each workflow with and without an unrelated saved
draft/recovery payload; cancel, reload and navigate back.

**Acceptance Scenarios**:

1. New project and New invoice retain the selected client using existing editors
   and independent destination permissions; following a link creates no business record.
1. Existing drafts/recovery are preserved. A conflicting saved draft is never
   silently replaced by the client supplied through navigation.
1. Project/invoice links and View in Projects retain client context. New projects
   require an active client. Invoice navigation preserves the current active-only
   picker without introducing an active-client restriction on historical invoice
   recovery or server-side billing.

### Edge Cases

- Null versus zero rates; stored currency codes; mixed project currencies.
- Missing imported billing provenance; draft/sent/paid/void invoice states.
- Names containing HTML-like text, whitespace, non-Latin characters or duplicates.
- Deactivation/revocation between rendering and a request; cross-organization IDs.
- Concurrent edits, duplicate submission and network failure after submission.
- Keyboard-only use, 320/390/768/1440px, short screens, 200% text and both themes.

## Requirements *(mandatory)*

### Functional Requirements

- **MVP-001**: Deliver US1 name search and combined lifecycle/currency filters,
  preserving catalogue authorization and truthful counts/empty/error states.
- **MVP-002**: Deliver US2 persisted client identity/billing detail and authorized
  project/invoice rows with real links, separate currencies and no fabricated data.
- **MVP-003**: Enforce current session, role, organization and project/financial
  boundaries at the server for every read and mutation. No feature 015 cutover.
- **MVP-004**: Deliver US3 shared existing-field create/edit, trusted validation,
  reload persistence, cancelled/failed-input preservation and duplicate protection.
- **MVP-005**: Preserve historical project/invoice money, currency, terms and
  records when editing defaults. Reject silent currency/rate reinterpretation.
- **MVP-006**: Preserve existing activation/deactivation and import navigation;
  add no contacts, deletion, bulk actions or changed archive semantics.
- **MVP-007**: Deliver US4 contextual project/invoice creation and existing-record
  navigation without automatic business-record creation or unrelated draft loss.
- **MVP-008**: Match the applicable handoff composition, typography and controls
  through existing components/tokens. Omit no in-scope journey; record excluded
  full-feature panels/actions explicitly, without fake controls or sample totals.
- **MVP-009**: Provide reachable keyboard/focus behavior, both themes, narrow/short
  viewport and enlarged-text layouts without shared-style regressions.
- **MVP-010**: Compare behavior with current official Harvest sources and read-only
  browser evidence where available; record conflicts and evidence limitations.
- **MVP-011**: Verify permissions, persistence, validation and cross-screen
  regressions using synthetic/disposable fixtures; preserve real data and mail.
- **MVP-012**: Publish isolated unsigned commits and scoped PRs with green CI and
  no unresolved critical/high review findings. Do not merge or alter queued PRs.

### Key Entities

- **Client**: Existing organization-owned identity, preferred currency, billing
  metadata, default rate and lifecycle; no new contact or payment-term entity.
- **Client work view**: Authorized project/invoice projections; not a new money
  ledger or authority source.
- **Creation context**: A client identity proposed to an existing editor; not a
  replacement draft or permission grant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **MVP-SC-001**: Every displayed client/project/invoice resolves to the fixture
  record; all filter combinations, counts and displayed money reconcile exactly.
- **MVP-SC-002**: Unauthorized direct reads/mutations, search/count probes and
  foreign identifiers disclose no new restricted records or financial fields.
- **MVP-SC-003**: Valid saves survive reload; invalid/cancelled saves produce zero
  partial writes; linked historical project/invoice values remain unchanged.
- **MVP-SC-004**: Contextual navigation preserves client identity and unrelated
  drafts and creates zero project/invoice records before explicit submission.
- **MVP-SC-005**: Browser acceptance passes the stated viewport/theme/keyboard
  matrix and existing projects/import/invoice regressions, with recorded evidence.
- **MVP-SC-006**: Published PR checks pass and review has no critical/high findings;
  the delivery report distinguishes this increment from full feature 012 parity.

## Assumptions

- Full feature 012 remains authoritative for final parity. This authorized
  increment stages delivery; it does not resolve its contacts/lifecycle/015 gates.
- Existing runtime and constitution 1.0.0 remain in effect. Access omissions or
  excesses discovered during research need explicit safe handling, not new grants.
- Handoff summary panels need a metric contract and permission evidence before
  inclusion. No unknown external billing is relabeled as certainly uninvoiced.
- New contacts, terms, destructive/bulk actions and the full six-profile policy
  remain separate; none is inferred from an inert prototype control.
