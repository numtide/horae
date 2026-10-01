# Clients Design and Harvest Comparison

Date: 2026-09-30. Status: evidence collected; product decisions pending.

## Sources and scope

- Handoffs: `design/project/app/12_Clients.dc.html` and `12_Client Detail.dc.html`, read in full.
- Baseline: `9301112c6a02ae3c92716273534f38889db241d1`, independent `feat/clients-design` worktree; pending Project Detail PR #208 is not included.
- Authenticated Windows Chrome through the existing Playwright MCP connection. Reused the Harvest tab and preserved the Horae tab. No account records were created, edited, archived or deleted.
- Browser observations and published Harvest behavior are distinguished below. No lifecycle mutation is claimed tested.

## Live browser observations

Visited `/clients` and its observed `/clients/new` destination. The list exposed New client, Actions, Import/Export and client/contact search, but no populated rows. The new-client form exposed name, address, preferred currency, invoice due date, tax, optional second tax and discount. Due-date choices included upon receipt, several net-day values and Custom. No default hourly-rate control appeared in that form.

Clicks on New client and Cancel timed out waiting for element stability. Read-only inspection showed `document.visibilityState = hidden` despite a visible bounding rectangle. This suggests a background/minimized-window interaction limitation, but does not establish the cause of the missing rows. Direct navigation to observed links and snapshots worked. No forced clicks, page-script changes or hidden submissions were used.

Local evidence, relative to the main worktree (gitignored because it contains account context):

- `.scratch/playwright-windows/harvest-clients-current-20260930.yml`
- `.scratch/playwright-windows/harvest-clients-dom-20260930.json`
- `.scratch/playwright-windows/harvest-new-client-dom-20260930.json`
- `.scratch/playwright-windows/output/page-2026-09-30T04-01-44-670Z.yml`

The connection remained live. The Harvest tab was returned to `/clients`; the other tab was retained.

## Published Harvest behavior

Harvest documents separate client contacts and CSV/Excel exports. Individual archive requires all client projects already archived; deletion requires removing linked projects and invoices. These are documented rules, not mutations exercised in this session. [Client management](https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts).

Bulk archive likewise skips clients with active projects. The workflow documents preview before Apply, fresh state checks and per-record outcomes. This is useful comparison for Horae's selection menu, not a requirement to copy the entire bulk-actions product. [Bulk actions](https://support.getharvest.com/hc/en-us/articles/46650626118285-Bulk-Actions-make-the-same-change-across-many-records-at-once).

The currency guide documents no exchange conversion and project-specific currency overrides. Its FAQ retains client-level-only wording, so it is not a consistent inheritance specification. Horae's explicit project currencies and exact-money rules take precedence. [Multiple currencies](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies).

## Horae gaps and integration constraints

| Area | Handoff | Existing behavior | Follow-up |
| --- | --- | --- | --- |
| List | Search, filters, cards, selection and actions | Basic table with inline editor/activation | Replace in place; preserve working permissions/mutations |
| Detail | Metrics, projects, contact/billing, invoices | `ClientDetail` placeholder | Authorized real data, not example cards |
| Contact | Name, title, email, phone; searchable | No client contact storage found | Confirm new data scope |
| Rate | Create/edit and billing panel | `clients.default_rate_cents` exists but legacy read/editor omits it | Extend real flow; preserve denomination guard |
| Terms | Billing panel | Organization branding text and project/invoice terms, no client terms | Confirm new data; prefill future projects without rewriting terms |
| Archive | Archives client's projects too | `set_client_active_record` changes only client flag; tracking also checks client | Resolve design/Harvest/current behavior conflict |
| Financial data | Unbilled, billed YTD, open invoices, hours | Catalog accessible to signed-in users; invoices require manager | Do not reuse catalog authorization for money |
| Creation links | Client-specific new project/invoice | Routes have no explicit client parameter | Extend existing flows without overwriting saved drafts |
| Delete/Duplicate | Client actions | No corresponding mutation | Preserve history; duplicate configuration into an unsaved form |

Relevant symbols: `list_clients`, `update_client_record`, `set_client_active_record`, `ClientDetail`, invoice `defaults::resolve`, and tracking eligibility in `projects.rs`/migration `0034_task_tracking_access.sql`.

`update_client_record` rejects currency changes that silently reinterpret a stored rate. Invoice `defaults::resolve` snapshots project defaults, requires explicit overrides for incompatible selections, and retains Net 30 for legacy projects. Client redesign must not bypass these rules.

## Decisions pending

1. Confirm persisted contact/payment-term scope: one primary contact matching the panel, or multiple contacts with a primary selection. This is new business data, not CSS-only work.
1. Choose archive/reactivation semantics. The handoff cascades, Harvest requires archived projects, and Horae currently gates tracking through the client flag alone.

The shared Project Detail action policy, including Pin, already awaits a separate decision and is not assumed approved. Unknown external billed status remains explicit; missing import data cannot be relabeled as certain.

## SpecKit execution status

The installed `speckit-specify` instructions were followed using `create-new-feature.sh --json --number 012 --short-name clients-design` in this worktree. `.specify/feature.json` resolves to `specs/012-clients-design`. The template was replaced with a feature specification and reviewed against `checklists/requirements.md`.

No `.specify/extensions.yml` exists, so no before/after specification hooks apply. Validation remains incomplete pending FR-015/FR-016 answers. `speckit-clarify`, planning, tasks and implementation are not claimed complete; the workflow engine has not been run.

### Clarification propagation — 2026-10-01

Ran the checked-in clarification paths helper and propagated the user's recorded
six-profile/custom-permission decision from feature 015. Updated the spec's
clarifications, personas, FR-003, SC-005 and dependencies. The old catalog and
Manager guards above are current-code evidence, not the final acceptance policy.
Client management does not itself authorize rates, invoices, projects or imports;
the exact operation/contact boundaries remain owned by feature 015's matrix.

No new product answer was inferred. Contact cardinality, archive/reactivation
and the shared Pin policy remain open. Revalidated the checklist at 11/16, with
no marker changes or newly passing items. The pending constitution amendment in
PR #212 is not merged into this branch; full clarification, plan/tasks and
implementation are not complete. No application or database changes were made.
