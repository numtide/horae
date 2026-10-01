# Phase 0 Research: Project Detail Dashboard

Reconciled 2026-09-29 against `9301112`, the expanded specification and
[Harvest MCP observations and official references](harvest-reference.md).
This replaces the obsolete no-chart/no-period research. Phase 0 is not complete:
PD-001 and the imported-billing conflict below still need resolution before the
full plan passes its gate. No application changes are claimed.

## D1 — Reuse valuation; preserve separate accounting bases

**Decision:** Preserve `server_fns/projects.rs::fetch_project_spend` semantics:
actual tracked minutes, attached gross invoice-line amount where applicable,
otherwise per-entry `effective_minutes`, `resolve_project_rate` and checked
`line_amount_cents`. Keep the batched SQL list aggregate; do not fetch all raw
entries merely to share a dashboard DTO.

**Rationale:** Configured billing modes and currency-safe inherited rates supersede
the historical three-level cascade. `crates/core/src/invoice.rs` contains pure
rules; migrations `0017`, `0018` and `0031` provide SQL counterparts. Billing
rounds each entry, not the summed total. Budgets retain their own scope and reset
period through `server_fns/budgets.rs`.

**Alternative rejected:** Equating live tracked value, budget consumption and net
invoice revenue. Discounts, fixed fees, rate changes and voids separate them.

## D2 — Stored contributions and fee balances are authoritative

**Decision:** Attribute lines through their time-entry or fee-occurrence source.
Sum `invoice_line_items.net_before_tax_cents`, distinguishing draft reservations,
other non-void contributions and void history. Label amounts after discount and
before tax; whole mixed-project invoice totals are not project revenue.

**Rationale:** Migration `0040_invoice_fee_balances.sql` stores allocated discounts
and net contributions. Invoice-level taxes have no per-project allocation rule.
Reuse `server_fns/invoices/fees.rs::preview_fees` and the read-only transaction in
`projects.rs::fetch_project_fee_balances`. Reads must not materialize occurrences.
Preserve negative remaining balances and the special zero-fee availability case.

Monthly fees honor both interval boundaries; single/milestone balances may include
overdue occurrences through the upper boundary. Label this separately from time
charts. Preview's 10,000-entry/occurrence bounds cannot silently truncate analytics.

**Alternatives rejected:** Gross amounts for net invoiced totals, current rates for
historical invoices, or subtracting tracked value from agreed fees.

## D3 — Imported billing conflicts with the expanded specification

**Evidence:** `specs/004-harvest-importer/spec.md` FR-016 explicitly imports entries
as locally open, without a local invoice. `importers/harvest/api_source.rs` parses
`SourceRow.invoiced`, but `apply.rs::upsert_time_entry` does not persist it.
`harvest_import_map` is provenance, not an external-billing ledger.

The expanded dashboard's US3 scenario 3 / FR-011 cannot promise exclusion of
externally billed entries with current data. This follows the approved importer
contract and is not evidence of an importer implementation bug.

**Recommendation pending reconciliation:** Preserve local eligibility and label it
“not invoiced in Horae”; identify imported history whose external status is
unknown. Do not silently change generation through a dashboard feature.
If external-billing exclusion is chosen, amend the importer contract, persist
true/false/unknown provenance and update generation and display together. Never
invent a historical billed status for earlier imports.

Local eligibility lives in `server_fns/invoices/entries.rs::read`: billable
entry/task, not running, no local invoice, open or approved (not submitted),
compatible project billing type and inclusive work-date interval. Missing rates
must remain incomplete, not a confident zero.

## D4 — Permissions are not one financial-access flag

This table records the legacy implementation boundary, not the final Harvest
parity matrix. The confirmed feature 015 capability/scope model supersedes it for
final acceptance; changing this document does not activate that pending policy.

**Decision:** Use active-user checks and `project_read_access`
(`0035_project_read_access.sql`) before loading dashboard/export data.

| Viewer | Progress | Rates/invoices | Recent notes |
|---|---|---|---|
| Organization admin | Same-org projects | Yes | Authorized team entries |
| Organization manager | Same-org projects | Yes | Authorized team entries |
| Assigned member with project lead/admin role | Yes | No | Own entries only |
| Ordinary assigned member | Only project-members visibility | No | Own entries only |
| Unassigned member with own historic time | Identity only, no dashboard | No | Own timesheet outside dashboard |
| Inactive or foreign-org actor | No | No | No |

Budget amounts intentionally follow progress access; this does not grant rates,
invoices or costs. Administrator notes stay administrator-only. Private fields
and peers' notes must be absent from unauthorized payloads.

**Alternatives rejected:** Treating project leads as organization managers, adding
a nonexistent show-rates setting, or fetching private data then hiding cells.

## D5 — Costs retain workspace currency and completeness

**Decision:** Reuse precedence from `server_fns/reports.rs::fetch_report`:
project-member override, then user cost rate, valued on actual tracked minutes.
Cost currency is the organization default, not project billing currency.
Do not calculate a margin across unlike currencies.

Administrators may see private overrides. Managers may see user costs only where
no private override participates; an aggregate containing one must be unavailable,
not partially summed. A private zero remains private. Keep missing rates distinct
from explicit zero.

Those role-specific visibility rules describe the legacy boundary. Final cost
access must follow feature 015's verified contract; cost valuation and currency
exactness remain separate from permission to see the result.

**Rationale:** Existing reports establish privacy and precedence but coalesce
missing defaults to zero. Reuse their policy, not that coercion. Regression anchor:
`manager_report_omits_groups_containing_private_project_costs`.

## D6 — Reuse lifecycle APIs; extra actions need contracts

**Decision:** Reuse `set_project_active` / `set_projects_active`, row locks,
idempotency, revision triggers and post-commit events. Use feature 011's editor
for editing and any approved duplicate flow.

Pin, duplicate, permanent delete and manual invoice linking have no existing
mutation APIs. PD-001 remains open. Source-linked lines, fee occurrences,
completed drafts and import references make deletion more than a cascade.
Manual attribution cannot consume a fee occurrence without an allocation
contract. Preserve original time/fee sources when changing presentation.

**Alternative rejected:** Treating prototype action handlers as purely visual
changes. No fake success or destructive tests against imported data.

## D7 — Existing export infrastructure, new authorized projection

**Decision:** Reuse `reports/limits.rs::configure_transaction` and
`reports/bounded.rs::ExportPermit`: repeatable-read/read-only, SQL timeouts,
bounded concurrency, render/output limits and cancellation handling.
CSV/XLSX/PDF share the authorized dashboard projection and selected period.
Reject bounds explicitly rather than truncating.

The existing `render.rs::render_invoice_pdf` already uses Typst/embedded fonts.
Add a dashboard-specific template; do not relabel an invoice PDF or add another
renderer dependency.

**Alternatives rejected:** Browser-print-only exports, unbounded rendering or
exporting everything before suppressing forbidden columns in the browser.

## Validation implications

- Exact per-entry rounding, discounts, mixed invoices, voids and fee periods.
- Full permission matrix, demotion/deactivation, private zero costs, missing rates
  and actual serialized/exported omissions.
- Imported provenance without asserting unavailable external facts.
- Enabled zero-time rows and disabled historical contributors.
- Chart/breakdown reconciliation, invalid intervals and stale route results.
- Archive idempotency, stale-editor rejection and preserved history.
- Full detail handoff/imported components read before UI implementation, followed
  by utility/token compliance and shared-screen regression checks.

Plan, data model, contracts and tasks remain historical until reconciled.
Their old assumptions are not implementation authority.

## Clarification propagation — 2026-10-01

Ran the checked-in clarification paths helper and reused the user's recorded
six-profile/custom-permission decision from feature 015. Updated the spec's
clarifications, management persona, FR-009/017, SC-005 and dependencies; annotated
the historical plan/research so legacy checks are not mistaken for the target.
No new question was answered and no runtime authority changed. Constitution 1.1.0
is proposed in PR #212, not merged into this worktree.

Revalidated the requirements checklist: 13/16 → 11/16. The earlier checks for
unambiguous requirements and complete acceptance scenarios were too optimistic:
PD-001, imported-billing semantics and the dependent permission matrix remain
unresolved. Only those two markers changed; checklist notes retain their dated
historical snapshot. No item became newly passing. Full clarification/planning
remains open rather than treating this narrow reconciliation as feature readiness.

## Expense budget dependency — 2026-10-01

Feature 016's isolated reference checks verify USD 100 fee budget / USD 1.50
expense → USD 98.50 remaining, including non-billable expenses, with removal
from consumption when inclusion is disabled and work-date monthly boundaries.
A 100-hour budget retains 100 hours; both cases reconcile with Projects. The
detailed expense report reports the USD 1.50 contribution, not a budget balance.
Added US1 acceptance, FR-023 and the dependency on the owning expense package in
PR #214. Editing stays in feature 011; no new detail management panel or global
CSS change is implied. Expense costs must not be silently relabeled into the
organization's time-cost currency or derived from an invoice line.

This is cross-contract propagation, not a new runtime test of Horae or a completed
technical plan. Existing planning/task artifacts remain historical. Rechecked
the current requirements checklist: 11/16 unchanged; pending action scope,
imported billing and operation-level permissions still prevent full readiness.
