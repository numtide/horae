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

The current `list_clients` returns the full client DTO to Clients, legacy Reports
and two invoice selectors. It cannot become a general workflow identity API:
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
the existing legacy operation changes only the client without this prerequisite.
