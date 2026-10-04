# People directory and workflow identities

Owner: OP19, FR-002/006/008/010/018; integration dependencies OP01/06/12/34/51.
Reviewed source: `b735b3a`, 2026-10-04. This contract does not authorize policy
activation or substitute the Administrator-only permission picker for a directory.

## Reference and current consumers

The current [Harvest permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions),
opened 2026-10-04, distinguishes managed/all people reads from billable/cost rate
reads. Its People Admin profile can read/manage people without rates. Only
Administrators may view/change another person's role. The older
[Team overview](https://support.getharvest.com/hc/en-us/articles/360048687431-Team-overview)
includes archived-person navigation, but its three-role guidance is not evidence
for every new custom permission. No authenticated reference browser was available
in this turn; no Harvest account or data was changed.

Horae consumers of `server_fns::list_users` at the inspected baseline:

| Consumer | Used fields |
| --- | --- |
| `pages::admin::AdminUsers` | ID, name, email, legacy role, active |
| `pages::projects::ProjectDetail` | ID/name lookup; ID/name/email assignment choices |
| `pages::reports::Reports` | ID/name filter choices |
| `pages::approvals::Approvals` | ID/name lookup |

The approval-label increment removes the last consumer from this table; the
other three continue using the legacy directory until their reviewed cutover.

None uses rates, organization ID, identity-provider subject or account creation
time. The inspected baseline query nevertheless loads and serializes those fields; the
Member-only rate scrub does not remove provider metadata. Returning database
`User` records couples unrelated consumers to authentication and financial data.

## Closed payload repair before cutover

`list_users` returns a dedicated `UserListItem` with exactly `id`, `name`, `email`,
`org_role`, `active`. SQL selects only those fields. This is a **legacy consumer
projection**, not the final six-profile directory DTO. Keep existing active-user
access, Admin-only inactive inclusion, same-organization filtering and name order.
No financial or authentication fields are present even for an Administrator.
No database values, identity linking, mutation response, own-user response or
Harvest-compatible `/users` response are changed by this repair.

Retaining email/role here preserves current consumers, not future permission to
disclose them. New-model workflow choices must not reuse this response blindly.
Financial profile/report reads remain separate operations; unused directory rate
fields are not a supported consumer dependency in the inspected application.

Acceptance: invoke the registered server function with real session cookies for
all three legacy roles. Assert exact response keys using fixtures with non-null
rates/provider subjects; same-organization IDs; active/inactive filtering;
unauthenticated and deactivated denial; same-cookie demotion of inactive access.
Compile server and WASM consumers and run the detail-navigation regressions.

## Required full-policy integration

### Approval row labels, independent of the directory

At the baseline, `Approvals` fetches `list_users(false)` solely to map approval user IDs
to names. An archived submitter disappears from that lookup even though the
approval remains visible, and an unrelated directory failure replaces names with
UUIDs. This also fetches email/role/activity for every active person unnecessarily.
The handoff `design/project/app/06_Approvals.dc.html` displays teammate names and
avatars; no general directory lookup is required for those labels.

Return `ApprovalSummary.user_name` from the same query as its approval, joining
the submitter by both user ID and organization. Do not require an active submitter
or canonical permission state to label retained business records. Invalid
cross-organization references must not resolve a foreign identity or appear as
valid review rows. Keep the existing manager gate, state filter, period order,
minute aggregation and mutation contracts; no grants or approval eligibility
are inferred from a display label. Remove the page's directory resource and map,
render the supplied name as escaped text and retain existing components/styles.

Acceptance: real registered-session requests for manager/admin, member/anonymous/
inactive denial, state filters, active and archived submitters, duplicate labels,
renames, foreign approvals and malformed cross-organization references, exact
summary fields and unchanged total/billable minutes. Render the actual page with
controlled approval responses and no directory service; verify escaped names,
loading/error/empty/access-denied behavior and unchanged mutation IDs.
This closes only the label dependency, not the scoped approval lifecycle.

For reports, distinguish result labels from filter choices: the
[archiving guide](https://support.getharvest.com/hc/en-us/articles/360048687311-Archiving-deleting-and-restoring-people),
reopened 2026-10-04, says archived people's time remains in all-person results,
while selecting archived people in detailed report filters requires the archived
items option. Do not narrow report totals by reusing an active-only directory or
derive the complete choice set from the currently displayed rows. Custom report
scope, zero-record candidates and project assignment eligibility remain separate
contracts. This source does not prove new-model approval enforcement.

### Scoped directory reader

The [person-profile guide](https://support.getharvest.com/hc/en-us/articles/360048687291-Person-profiles)
documents name and work email as basic person information, separately from rates,
permissions and account security. The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/)
documents active/inactive filtering under its ordinary person-read boundary,
without requiring an archive/restore action grant to read an archived record.
Both were opened on 2026-10-04. Their legacy role descriptions are not new-model
enforcement observations. Mapping those read dimensions to the confirmed
`PeopleReadManaged`/`PeopleReadAll` catalog is an explicit implementation inference;
new-model reference acceptance remains required before activation.

For the policy-1 directory read (not a mutation or workflow picker):

| Condition | Result |
| --- | --- |
| `PeopleReadAll` | All same-organization people matching the requested activity filter |
| Only `PeopleReadManaged` | Matching people with a current direct person-management edge from the actor |
| Neither read grant | Deny even when the requested page would be empty |
| Own identity, project membership/management, time/rate/report grants alone | No directory authority; own-profile/workflow reads remain separate |
| Inactive or foreign actor, absent/unsupported policy, missing/invalid canonical state | No directory data and no legacy-role fallback |

The response carries only ID/name/email/activity and requester identity. It
contains no role label, grants, provider subject, rates, timestamps or total count.
No target permission state is needed to display an otherwise visible person.
Activity is a filter (`active`, `archived`, `all`), not a grant to restore, assign,
invite or edit anyone. Reading retained archived relationships does not resolve
the separate pending decision about creating new relationships with archived
endpoints.

Use fixed pages of 50 with name/UUID ordering and an exclusive name/UUID cursor.
Scope and activity predicates run before limit/continuation calculation. A cursor
is a caller-supplied bound, not a lookup, capability or snapshot; deleted/foreign
cursor identities reveal nothing and each page reauthorizes. Renames and activity
changes can move rows between requests; do not promise a frozen traversal.
Reject NUL-containing cursor names as invalid input after authorization rather
than exposing PostgreSQL's text-encoding error.

Hold the organization SHARE gate and active actor SHARE lock until the read
commits, using the existing bounded READ COMMITTED transaction setup. Canonical
grant/relationship writers serialize through the organization gate; direct actor
deactivation must also be observed after waits. HTTP authentication and storage
failures are sanitized. Legacy `list_users` and its remaining consumers stay
in place until the coordinated shell/workflow cutover; this reader must never
be a fallback to broaden a denied report/approval/project picker.

Acceptance requires the six default profiles and custom managed/all selections,
non-directory grants with relationships, active/archived/all filters, pagination
with hidden records around boundaries, exact response fields, missing target
state, foreign/empty cursors, grant/relationship revocation, direct deactivation,
cancellation/pool reuse and registered-session requests. The new endpoint does
not activate policy, complete the directory UI or pass the complete OP19 row.

- Directory rows require `PeopleReadManaged` or `PeopleReadAll`, with trusted
  current person-management relationships; project membership or management alone
  is not a people-directory grant. Do not infer access from profile ordering.
- Ordinary people reads must not expose role/grant configuration. Administrative
  editing stays on OP51; own access stays on OP01/49.
- Reports and approval name resolution must follow the authorized result set;
  an independent operation's identity needs do not grant whole-directory access.
  Project assignment choices need the reviewed OP12/16 eligibility contract.
- Rates follow their independent FR-021/022 field rules. The directory's bounded
  basic-identity read does not settle workflow-picker fields, search behavior,
  compatibility API projection or new-model reference acceptance for inactive
  visibility. Resolve and test those across consumers before replacing guards.
- Directory scope and field predicates, mutation/admission authority, migration
  differences and revocation tests remain mandatory. This repair closes none of
  T006–T009/T020 by itself and is not proof of Harvest parity.
