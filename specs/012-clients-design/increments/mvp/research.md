# Clients MVP research

Reference date: 2026-10-02. Baseline: `f5bacfbd` (PR #209 still open).
The parent [research](../../research.md) remains relevant; this records new
evidence and the authorized increment, not full parity.

## Harvest evidence

| Subject | Evidence and status | Increment consequence |
| --- | --- | --- |
| Identity and management | Official [client guide](https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts): client name, optional address/currency, administrator and specially permitted manager management. Documented. | Preserve existing Horae roles until the separate permissions work ships. |
| Search | Authenticated `/clients` shows client/contact search. Capture contains no rows and console errors; observed control, not verified matching/filter behavior or proof of an empty account. | Name-only search is authorized. Currency/lifecycle controls follow the clarified Horae spec/design, not unobserved Harvest behavior. |
| Form | Authenticated `/clients/new`: name, address, currency, due date, tax, second tax, discount; Back/Cancel links. DOM confirms required name; no submission. Observed. | Existing-field editor only; new terms/tax-percentage/discount fields excluded. Harvest server validation/persistence not verified. |
| Currency | Official [currency guide](https://support.getharvest.com/hc/en-us/articles/360055384512-Does-Harvest-support-multiple-currencies): project override, separate report currencies, no FX. Documented. Older FAQ on the same page also claims client-derived project currency. | Preserve persisted project currencies and explicit user instruction against retroactive changes. Do not implement the contradictory FAQ as a new rule. |
| Lifecycle | Client guide requires archived projects before client archival. Documented, not exercised. | Goal expressly excludes policy changes; preserve Horae activation/deactivation and record the difference. |
| Historical presentation | Client guide says name/address edits affect historical invoice presentation. Documented, not tested. | Protect requested historical amounts/currency/terms; do not add invoice snapshot semantics. |
| Context and roles | No populated client detail or alternate-role session available in this inspection. | Verify Horae context/negative permissions locally; Harvest equivalence here remains unverified. |

Private snapshots stay untracked in root `.scratch/playwright-windows/output/`:
`page-2026-10-02T00-30-56-373Z.yml` and
`page-2026-10-02T00-37-30-474Z.yml`. Existing MCP connection revalidated; no record
submission, new connection, permission change or mail.

## Authorization decisions

Decision: reuse current runtime authorities, not pending feature 015.
Rationale: explicit scope. Alternative rejected: widening access for new UI.

- `list_clients` requires an active session and session organization; its DTO
  includes address/tax ID but not default rate.
- Client mutations and `CreationClient` rates require manager/admin.
- `project_read_access` (migration 0035) governs project progress visibility.
  Assigned leads/admins and members admitted by project visibility can see
  progress. Tracking history alone is insufficient. Rates remain manager-only;
  aggregate spent/budgets already follow progress authority.
- Invoice reads/actions remain manager/admin; never serialize invoices under
  catalog authority alone.
- Lifecycle writes change only the client flag. Project finalization requires
  active client; invoice preview/generation do not impose that gate.

Independent read-only contract research confirmed these against client/project
server functions, project privacy tests, project creation, invoice recovery and
migration 0035. This is research, not an independent implemented-code review.

## Editor decisions

Decision: reuse existing editors and recovery owners. Project drafts carry
revision/idempotency state; invoices preserve exact pending payload/request IDs.
Alternative rejected: separate creation flow or overwriting restored draft values.

- Prefill project context once only without an existing draft. Resolve IDs beyond
  initial picker pages through the authorized client read. Keep inherited project
  currency unset. Any existing draft wins, including one without a client.
- Invoice `RecoveryGate` stays outermost; pending Generate/Save wins over route
  context, preserving payload, request ID, storage acknowledgement and navigation.
- Invalid/foreign/ineligible context gets a real error/notice, not first-client
  fallback. Context does not create business records.

Decision: no schema/dependency change. Keep `Client` catalog compatible; use
manager-only billing data and authorized project currency/count projections.
Reuse project/invoice financial reads instead of adding a new aggregation ledger.

Decision: profile saves carry explicit keep/replace/clear rate intent. Lock and
compare current denomination/rate before writing. Currency change cannot silently
relabel an existing amount. Legacy editors retain their currency guard.

## Deferred full-feature work

Contacts/search, bulk actions, export, new lifecycle policies, six-profile
permissions, YTD/open/unbilled cards and imported billing provenance remain parent
012 work. This increment has real work rows/billing fields, not fabricated totals
or inert controls. Its readiness checklist does not waive parent checklist gates.
