# Client catalog and workflow authority

Owner: OP17/18/34; FR-006/007/008/010/018. Baseline `db3935d`.

## Evidence and scope

Checked 2026-10-06: Harvest's [permission reference](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
separates client read from client create/edit and from rates. The
[client API reference](https://help.getharvest.com/api-v2/clients-api/clients/clients/)
still describes an Administrator/Manager create/edit prerequisite. As already
resolved for tasks, Horae's read-only compatibility surface follows the approved
operation matrix's read capability, not a newly inferred write requirement.
This does not claim exact Harvest public-API authorization/status-code parity
or a restricted-user browser observation.

The `feat/clients-mvp` worktree remains at `5a459c4`; its MVP-003 explicitly
preserves legacy behavior until feature 015 cutover. Its no-cutover assumption
is not permission acceptance. Preserve that independent branch; do not merge
it or implement its deferred contacts, deletion or bulk actions here.

## Compatibility reader contract

- Policy one requires current `ClientReadAll`. Neither a legacy Administrator
  role, an invoice/report/rate grant nor a related project/time identity supplies
  catalog authority. Read-only custom grants must work without client write.
- Preserve the compatibility surface's record-filtering convention: denied
  catalog scope returns no rows/counts; a denied, missing or foreign direct ID
  returns not found. Missing/malformed stored permissions, unknown policy and
  inactive actors fail closed, without falling back to legacy roles.
- Authorized client fields retain the existing compatibility projection: ID,
  name, activity, address, currency and timestamps. Do not add tax identifiers,
  contacts, defaults, project details, invoices or financial totals.
- Keep policy zero's existing same-organization behavior. New policy is tested
  only in disposable databases, not activated on an existing organization.
- Hold the existing organization/active-actor authorization fence through
  materialization. Count and page must share one source statement, including
  exhausted pages; active/date filters and name/ID ordering remain intact.
- Actual-session tests must cover read-only grants, unrelated grants, legacy
  role independence, foreign/direct IDs, unknown/missing/malformed state,
  deactivation, revocation after an authority wait and policy-zero regressions.

## Remaining connected surface

At the original baseline, `list_clients` returned the full client DTO to Clients,
legacy Reports and two invoice consumers. Rechecked at `b675d960` on 2026-10-13:
Clients now uses the canonical page readers described below; legacy Reports and
invoice list/detail still call `list_clients`. It cannot become a general workflow identity API:
addresses and tax IDs need catalog or explicitly authorized billing context.
Canonical Reports already uses its scoped source; invoice/project discovery
requires its own verified workflow predicate and minimal projection.

Standalone create/update/activity still use legacy manager guards; update and
activity helpers do not carry actor identity into their transaction. Inline
creation has an organization/actor fence but still uses the legacy role and can
write a default rate. T238 must reconcile all these paths and caller-bound UI,
not merely hide buttons. Default-rate and lifecycle contracts remain explicit
prerequisites, now resolved at the product level by FR-035/036; this reader does
not implement them. Full activation, migration,
cross-surface review and Nix gates remain required.

## Field and lifecycle investigation

Checked 2026-10-06: Harvest's [rate guide](https://support.getharvest.com/hc/en-us/articles/360048181492)
documents person/task defaults and project rates, not Horae's global client
default. The user confirmed the Spec Kit clarification on 2026-10-06, recorded
in FR-035: require global billable-rate Read/Write, plus the corresponding Client
permission, for this default; managed-project authority does not suffice.
This settles the ownership explicitly excluded from FR-034, but is not proven
Harvest behavior. Omitted/zero/preserved values still need distinct command
intent. Apply the rule to both standalone and inline creation/editing; ordinary
field changes must preserve a withheld unchanged rate. Existing project
overrides and historical financial values are not rewritten by this policy.

The [client guide](https://support.getharvest.com/hc/en-us/articles/360048181312-Create-and-edit-clients-and-client-contacts)
requires all projects archived before archiving a client and provides a separate
client restore action. The user selected option A on 2026-10-06: follow Harvest,
not the handoff's cascade. FR-036 resolves feature 012 FR-016's product choice;
the separate worktree is not edited here. Require `ClientWriteAll` to change the
client lifecycle, reject archive if any linked project remains active, and
restore only the client. Never mutate projects through this operation or require
project-write authority for a client-only change. Independently authorized
project actions remain necessary before archive and after client restoration.
Denials must not reveal hidden project identities/counts. Recheck concurrent
project creation/reactivation and current client authority under the existing
organization-first gate; preserve history and independent state constraints.
Policy-zero compatibility and the reviewed activation delta remain explicit:
under the legacy manager role, client archive now enforces this prerequisite and
restores only the client, implemented and tested in the 012 lifecycle increment.

## Clients page read slice (partial T238)

Scope: OP17 for the canonical Clients list and detail pages only; FR-006/008/
010/018 and FR-035's read half. This is **not** catalog-wide enforcement and is
not sufficient for policy activation. T238 and OP17/18 remain open.

- `client_overview(expected_requester)` returns `ClientOverview` and
  `client_detail_view(client_id, expected_requester)` returns
  `ClientDetailView`. Both carry the bound `PermissionRequester`; a mismatched
  expected requester fails with forbidden. Each holds `ProjectReadAccess`
  through materialization.
- Policy zero keeps the legacy catalog, project-progress counts and
  administrator/manager billing. Policy one requires `ClientReadAll`; without
  it the overview reports `can_read: false` (withheld, not an empty catalog)
  and direct IDs are not found. Counts use the `list_projects` reader. The
  default rate requires `BillableRateReadAll` as well; managed rate grants never
  apply. Unknown policy and missing/malformed state are forbidden; inactive or
  foreign actors fail closed.
- `can_manage` is the legacy manager role under policy zero and always false
  under policy one. This is temporary UI state while the client writes still
  check only the legacy role, not a product rule. Invoice controls must not be
  inferred from billing visibility.

Registered legacy readers left unchanged, which still expose catalog fields or
rates under policy one without `ClientReadAll`:

| Server function | Exposure under policy one | Consumers |
| --- | --- | --- |
| `list_clients` | All organization clients with address and tax ID, for any active user | Legacy Reports filter, invoice list and invoice detail |
| `get_client_details` | Address and tax ID for any active user; default rate for the legacy administrator/manager role | `ProjectsForClient` context, client editor |
| `list_client_invoices` | Gated by the legacy manager role, not invoice or client grants | Client detail invoices |

Each needs a scoped workflow-identity replacement (OP17/21/25) before
activation. The former `list_client_summaries` endpoint is removed; the Clients
page reads `client_overview`.
