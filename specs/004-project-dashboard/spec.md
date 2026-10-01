# Feature Specification: Project Detail Dashboard

**Feature Branch**: `feat/project-dashboard`

**Created**: 2026-09-01 | **Reconciled**: 2026-09-29

**Status**: Draft — expanded design coverage; implementation not accepted

**Input**: Implement Project Detail, Clients list/detail, and Settings/Workspace against the current handoff. This specification owns Project Detail; [delivery.md](delivery.md) tracks the whole requested delivery. Completing this first feature does not complete that request.

## Scope and authority

The visual reference is [11 Project Detail](../../design/project/app/11_Project%20Detail.dc.html), including progress/hours charts, summary tiles and Tasks/Team/Invoices tabs. Feature 011's shared editor remains the only creation/editing form. The September 1 specification's chart, period, cost and fee-schedule deferrals are superseded by this revision.

Existing accounting and access rules take precedence over sample values and simulated actions in the handoff. Amounts must come from real authorized data and distinguish tracked value, invoice value, costs and agreed fees. No sample records or fake-success controls may ship.

## Clarifications

### Session 2026-10-01

- Q: Does dashboard access retain the three-role boundary or use the confirmed Harvest permission model? → A: Reuse the user's 2026-09-30 decision recorded in feature 015: six built-in profiles, custom profiles, individual adjustments and applicable scopes. This propagates an existing answer; it does not approve the pending action or imported-billing decisions.

### Reconciliation 2026-09-29

These are conclusions from approved feature 011 and current application code, not newly answered user questions:

- Saved identity, tags, private notes, tasks, assignments and fee calculations must survive the redesign. Fee balances remain in invoice preparation, not in an extra dashboard panel.
- Hours use tracked minutes. Monetary valuation follows current effective-minute, configured-rate and attached-invoice rules, not the obsolete three-level cascade.
- Budget periods/scopes are independent of lifetime tracked totals. Monthly allowances cannot be compared with lifetime consumption.
- Partial fixed-fee billing, milestones, monthly fees, discounts and voids already exist. Reuse their balances, including negative remaining amounts.
- Project-progress, rates, costs, invoices and administrator-note permissions are distinct. Aggregate progress access does not grant private time-note access.
- There is now a dedicated detail handoff. Charts, period filtering, costs and dashboard exports are required by this revision; the old no-chart MVP is not the final outcome.

## User Scenarios & Testing

### User Story 1 — Understand progress (Priority: P1)

A permitted viewer opens a project and sees identity, client, status and real project metrics in the handoff layout, with working back/client/editor navigation.

**Independent Test**: Open configured and imported projects covering every billing type, budget scope and empty state; compare equivalent periods and permissions with Projects.

**Acceptance Scenarios**:

1. An hours-budgeted project shows tracked hours, billable/non-billable split, consumed allowance and signed remaining allowance. An overrun remains visible when the bar is full.
1. No budget produces real totals and an authorized Set up a budget link, not a synthetic zero allowance.
1. Monthly and task/person-scoped budgets identify period/scope. Unallocated allowances are not zero allowances or a fabricated project cap.
1. Archived projects remain readable to authorized viewers; inaccessible projects reveal no identity.

### User Story 2 — Explore time and costs (Priority: P1)

A viewer switches between Project progress and Hours per week, selects a reporting interval and expands task/person breakdowns.

**Independent Test**: Multi-week data across people/tasks, disabled historical records, missing rates and unlike cost currencies reconciles charts, reciprocal breakdowns and period totals.

**Acceptance Scenarios**:

1. All time, this/last month, this quarter, this year and custom intervals select the same records for period-labelled breakdowns and exports.
1. Tasks expand into people and people into tasks. Child minutes and complete monetary subtotals sum exactly to parents.
1. Weekly buckets sum to selected tracked minutes. Cumulative progress has a labelled origin and never compares an hours axis with a monetary allowance.
1. Missing rates are incomplete, distinct from zero. Unlike currencies are not combined into one number.
1. Enabled zero-time tasks/team members remain discoverable; disabled historical contributors remain included.

### User Story 3 — Understand and act on billing (Priority: P1)

A financially authorized viewer sees actual invoices and remaining time/fee balances and continues into the real invoice workflow with project context.

**Independent Test**: Exercise time billing, single/milestone/monthly fees, partial drafts, discounts, over-invoicing, paid/void invoices and mixed-project invoices.

**Acceptance Scenarios**:

1. Invoice rows link to real records and show status, dates, reference, subject and the project's attributable pre-tax value, not an entire mixed-project invoice amount.
1. Fee balances match invoice preparation. Discounts leave the reduced contribution available; void releases only that invoice's contribution.
1. Imported billed time without a local invoice does not become eligible merely because a local reference is absent.
1. New invoice opens preparation with client/project context; cancelling creates no billing mutation.
1. Historical invoiced value plus live uninvoiced value need not equal live tracked value after rate changes, fixed fees, discounts or taxes.

### User Story 4 — Manage from the dashboard (Priority: P2)

A person with the applicable project-management capability uses contextual actions and the shared editor without losing history or navigating into a disconnected form.

**Independent Test**: Edit/save/cancel, archive/reactivate, switch projects and return from reports/invoices using keyboard and browser history.

**Acceptance Scenarios**:

1. Edit and Set up a budget use feature 011's populated editor. Cancel preserves the project; Save updates the same identity.
1. Archive confirms intent, preserves time/invoices and blocks new tracking according to current rules. Reactivation is a real authorized action.
1. Reports/exports preserve project and applicable reporting interval without widening access.
1. Switching projects clears old expanded rows and cannot display another project's pending data or private amounts.

## Functional Requirements

- **FR-001**: Show client link, code/name, type, status, currency, optional dates/tags and administrator-only notes, with Back to Projects and active Projects navigation.
- **FR-002**: Provide the handoff summary layout for tracked hours, budget remaining, internal costs, invoiced and uninvoiced context. Forbidden, unavailable and incomplete values are not zeros.
- **FR-003**: Separate tracked minutes from billable valuation. Equivalent list/detail valuations match current configured/legacy rates, rounding and invoice snapshots.
- **FR-004**: Show budgets by configured scope/unit/period with absent/zero allowances and signed overruns. Reuse existing semantics.
- **FR-005**: Provide real Project progress and Hours per week charts, period navigation and accessible textual/tabular equivalents. Follow the handoff's compact legend; do not add technical explanatory paragraphs, duplicated totals or a visible weekly-data panel. Keep exact data available to assistive technology. Do not convert money to hours without a valid rule.
- **FR-006**: Support all-time, month, quarter, year and custom reporting intervals, inclusive by work date. Reject invalid intervals; label selected-period, lifetime and budget-period metrics distinctly.
- **FR-007**: Provide Tasks, Team and Invoices tabs with meaningful counts and keyboard interaction. Preserve the interval across tabs.
- **FR-008**: Provide reciprocal expandable task/person groups, stable hours sorting, enabled zero-time rows and disabled historical contributors. Complete subtotals reconcile exactly.
- **FR-009**: Apply feature 015's distinct cost capability and scope contract, keeping currencies distinct and identifying missing rates. Preserve existing cost valuation and private-override protection until a reviewed policy migration. Cost access is not inferred from progress or billing-rate access.
- **FR-010**: Invoice totals use stored attributable contributions, not current-rate recomputation or whole mixed-project invoices. Distinguish draft reservations, non-void invoiced contributions and void history through row statuses and exact totals. Follow the handoff's compact Invoiced tile and table; do not add permanent accounting paragraphs, an extra history heading or per-state summary panels.
- **FR-011**: Uninvoiced time uses actual invoice eligibility; fixed-fee remaining uses approved per-occurrence partial-fee accounting. Do not subtract lifetime tracked value from an unrelated agreed fee.
- **FR-012**: Invoice navigation/creation works with project context. A reference alone cannot change financial attribution, balances or locks.
- **FR-013**: Recent entries are newest-first with deterministic ties and respect current person-level reporting and note permissions.
- **FR-014**: Edit reuses the shared editor. Task/team management belongs in Edit project; the detail page keeps the reporting Tasks/Team tabs without separate management forms or accordions.
- **FR-015**: Archive/reactivate provides confirmation, pending protection, visible errors and recovery, preserving history and billing.
- **FR-016**: CSV, spreadsheet and PDF summary exports match the selected authorized scope. Explain bounded limits before download; never silently truncate or widen access.
- **FR-017**: Recheck feature 015's effective capabilities and current person/project/organization scopes on every read, export and contextual mutation. Enforce project progress, detailed time/notes, rates, costs, invoices and administrative private notes independently. Inaccessible identities are non-disclosing; unauthorized fields and records are absent from payloads and derived totals, not merely hidden visually. Project membership or a Manager-like label MUST NOT substitute for the operation's required authority.
- **FR-018**: Distinguish loading, empty, incomplete, forbidden and error states. Retry cannot expose stale data from another project/period.
- **FR-019**: Support 320/390/768/1440px, short viewports, 200% text zoom and keyboard-only operation. Tables may scroll inside containers; preserve mobile navigation and avoid shell overflow.
- **FR-020**: Reuse current design tokens/utilities/components and preserve shared defaults. Do not copy prototype handlers, inline styles or sample data.
- **FR-021**: Use exact integer quantities and explicit currency; overflow is an error rather than wrapping, saturation or partial success.
- **FR-022**: Dashboard reads do not mutate business records or send notifications. Verification uses disposable data, preserving imported development data and existing previews.

## Open product decision

The current [source audit](research.md) also identifies a conflict between US3
scenario 3 / FR-011 and the approved importer's FR-016: external billed status is
not persisted, and imported entries deliberately remain locally open. Resolve
this before implementation; do not infer a historical billing fact or silently
change invoice eligibility. The recommended compatible approach is to label the
metric as locally uninvoiced and explicitly mark unknown external billing.

The [Harvest comparison](harvest-reference.md) now records actual MCP observations
and official action semantics, including permanent deletion and the different
financial effects of invoice linking for time-based and fixed-fee projects.

The handoff lists Pin, Duplicate, Delete, Link invoice and Unlink, but its handlers do not implement their full business semantics. Placeholder controls cannot count as delivered functionality.

- **PD-001 — NEEDS CLARIFICATION**: Implement all five additional actions with new persistence/destructive rules, or retain existing Edit/Archive/Reactivate and actual invoice creation/navigation? Charts, periods, costs, tabs and exports remain required either way.

## Edge Cases

- Empty projects; zero/unallocated budgets; signed overruns; missing versus zero rates.
- Archived projects/clients, inactive people and disabled historical tasks.
- Legacy imports without settings; imported billed flags without local invoices.
- Rounded/locked minutes, changed rates, non-billable tasks and incompatible inherited-rate currencies.
- Mixed-project invoices; partial fees, discounts, taxes, paid/void history and independent occurrences.
- Revoked/deactivated accounts, changed visibility and foreign-organization identifiers.
- Month/year boundaries, leap days, empty weeks, invalid intervals and stable ordering.
- Concurrent entries/invoices during reads, rapid route/filter changes and export limits/overflow.

## Key Entities

- **Overview**: authorized identity, configuration, permissions and lifecycle state.
- **Reporting interval**: inclusive work dates and reproducible navigation/export context.
- **Progress series**: exact minute buckets/cumulative values and applicable budget metadata.
- **Breakdown**: task/person totals and reciprocal children with completeness/currency.
- **Billing summary**: attributable invoice contributions, eligible time and per-occurrence balances.

## Success Criteria

- **SC-001**: All four journeys pass for configured, imported, empty and archived projects without fabricated data or dead-end primary actions.
- **SC-002**: Equivalent list/detail budget and spend figures match exactly for every fixture; different periods/bases are visibly identified.
- **SC-003**: Charts and breakdown subdivisions reconcile to their relevant parents/totals down to the minute and minor currency unit.
- **SC-004**: Billing summaries agree with preparation, stored invoice contributions and partial-fee balances through every listed lifecycle case.
- **SC-005**: All six built-in profiles, custom grants, individual adjustments and revoked/overlapping person/project scopes pass feature 015's applicable operation matrix. No unauthorized private amounts, notes, identities, aggregate contributions or exports appear. Verify direct reads/actions and downloads as well as visible controls.
- **SC-006**: Primary flows pass keyboard/viewport/zoom checks and preserve the eight-screen shared-style baseline.
- **SC-007**: Full application/browser, domain, permission, export and deployment verification passes on the final implementation before acceptance.

## Assumptions and Dependencies

- Retain single-organization OIDC. The final permission boundary is feature 015's user-confirmed six-profile/custom model, not the old administrator/manager/member assumption. Its [pending PR #212](https://github.com/numtide/horae/pull/212) owns the constitution amendment, verified matrix and reviewed migration; none is silently activated by this dashboard spec. Current implementation evidence remains valid only for the legacy policy it actually tested.
- Reuse feature 011's editor, budgets, privacy and fee accounting plus current report/export infrastructure.
- Charts visualize recorded data, not forecasts. Currency conversion and historical financial-snapshot editing are not introduced.
- Client navigation is an end-to-end acceptance dependency on the Clients slice, not complete while its destination is a placeholder.
- Preserve the imported handoff; document actual differences and any explicit user-approved exclusions in acceptance evidence.
