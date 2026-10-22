# Invoice links, sources and authorization

Checked: 2026-10-20. Refines OP21–OP24/OP33 and T006/T014/T015.
This closes distinctions needed by acceptance fixtures, not the unresolved
whole-invoice managed-access predicate. No account data was changed.

## Reference facts and limits

The [linking guide][linking] distinguishes line-project associations from original
time/expense sources. Relinking does not move source entries. For Time & Materials,
automatically invoiced entries reduce the original project's uninvoiced amount,
not a newly linked project's; manually linking a line does not bill its time.
Fixed Fee balances instead respond to linked line amounts.

A mixed-project invoice's project-analysis row shows that project's pre-tax
portion, not necessarily the invoice total. Linking from project analysis assigns
every line to the selected project. Retainer funding invoices cannot use this
linking mechanism; invoices drawing funds have separate linking restrictions.
These are documented domain effects, not authorization predicates.

The [Invoices API][invoices] separately describes source import project IDs,
per-line project associations and invoice-wide amounts. Its role-level access
description does not establish the newer custom managed/all invoice read policy.
Do not infer whole-invoice visibility from a list's project filter or from API
examples containing only one project.

The [current permission guide][permissions] separates managed invoice read,
managed draft editing and broader invoice management. Draft editing alone does
not establish authority for sending or recording payments. Neither that catalog
nor the linking guide settles access to mixed-project or unlinked manual lines.

### Payment evidence

Read-only navigation opened an existing draft invoice in the connected account.
The Record payment control was disabled with an explicit instruction to send or
mark the invoice as sent first. Snapshot:
`.scratch/playwright-windows/output/page-2026-10-20T09-55-27-729Z.yml`.
This establishes the current owner's UI state, not a server rejection test.
No send, mark-sent, payment, online-payment configuration or save action was used.

The [payment guide][payments] documents partial payments, remaining balances and
delete-then-recreate correction rather than editing an existing payment. It
requires a sent/marked-sent invoice for write-off. Payment records, write-offs,
invoice status and source billing locks must remain separate concepts. Merely
setting Horae's invoice status to paid does not deliver the payment workflow.
Harvest's subscription/integration admission requirements are not additional
Horae product requirements; no new payment integration is introduced here.

## Acceptance fixtures

Use project P managed by actor A, unrelated project Q for the same client, a
foreign-organization project R, and independent time/expense sources. Cross the
invoice grants with separate source/resource/financial grants. Do not infer
those grants from A's ordinary project or client access.

| Case | Required expected result / unresolved discriminator | Trace |
| --- | --- | --- |
| Original time/expense in Q, line relinked to P | Relinking preserves source ownership and historical billing provenance. P's linkage alone must not authorize source reads or mutations in Q | FR-006/008/017/018; OP22/23; T014/T015 |
| T&M source in Q, line linked to T&M P | Source-based uninvoiced accounting remains Q's; a manual line linked to P does not mark P's entries invoiced | Linking domain reference; OP22/23; invoice-domain tests plus T014 |
| Line linked to Fixed Fee P | Link-based balance effects are distinct from source-entry billing; verify financial authority for all affected resources | FR-008/017; OP22/23; T014/T015 |
| Invoice has P and Q lines | Obtain the whole-invoice allow/deny predicate before implementation. Neither first matching project nor client visibility is a sufficient oracle; no unauthorized notes, line amounts or totals may leak | FR-002/006/008/020; OP21/33; T006 then T014/T015 |
| Project P's invoice-analysis row | Project-specific pre-tax projection is not the whole-invoice total or authority to follow the full invoice/export link | FR-008/018; OP21/33; T014/T015 |
| Unlinked manual line, or all lines removed | Resolve invoice ownership/scope explicitly; an empty project set must not accidentally pass an all-project check and expose the invoice | FR-002/008/020; OP21/23/33; T006 then T014/T015 |
| Foreign R or fabricated source/link ID | No foreign data, partial writes or successful audit; trusted same-tenant facts must govern the complete requested operation | FR-006/013/017; OP21–24/33; T014/T015 |
| Scope/grant/link/source changes after preview | Recheck current authority over the actual operation and all affected resources before commit or download; stale preview grants no authority | FR-010/018; OP22/23/33; T014/T015 |
| Draft with otherwise valid invoice-management authority | Payment control remains unavailable until the required sent transition; validate direct requests too before claiming server parity | Observed UI; OP24; T006/T014/T015 and payment-domain acceptance |
| Draft-only actor on sent invoice | Do not infer send/payment/write-off authority from draft editing; prove independent management and business-state checks | FR-002/007/018; OP24; T014/T015 |

These are planned cases, not passing tests. Security expectations derive from
already accepted Horae invariants; they do not claim Harvest denied the same
restricted-user requests. Do not add a generic person-or-project union, require
unverified extra rate grants, or pick a permissive empty-set interpretation to
make the remaining scope cases executable.

## Boundaries for the deferred client reader

An authorized workflow may need a client's ID/name without obtaining client
administration, contacts, address, defaults or invoice access. Reuse
`client-permissions.md`; its minimal identity projection cannot resolve OP21.
Conversely, owning a client catalog entry is not permission to read every
associated invoice. Preserve the global-first gate for the new reader PR.

Retained official guide JSON:
`.scratch/harvest-reference-20261020/invoice-project-links.json`,
article `updated_at` `2026-05-20T06:55:06Z`, SHA-256
`0a93e72eb620eeacf6a2eefff01f0d36be687cb7be2bdf860d9bf450e88ab9ba`.
Raw account snapshots remain outside Git. Only ordinary read navigation and
public documentation retrieval were performed.

[invoices]: https://help.getharvest.com/api-v2/invoices-api/invoices/invoices/
[linking]: https://support.getharvest.com/hc/en-us/articles/360048686631-Linking-invoices-to-projects
[payments]: https://support.getharvest.com/hc/en-us/articles/360048181032-Recording-and-removing-invoice-payments-and-write-offs
[permissions]: https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions
