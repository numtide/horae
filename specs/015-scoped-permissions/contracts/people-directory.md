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

Current Horae consumers of `server_fns::list_users`:

| Consumer | Used fields |
| --- | --- |
| `pages::admin::AdminUsers` | ID, name, email, legacy role, active |
| `pages::projects::ProjectDetail` | ID/name lookup; ID/name/email assignment choices |
| `pages::reports::Reports` | ID/name filter choices |
| `pages::approvals::Approvals` | ID/name lookup |

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

- Directory rows require `PeopleReadManaged` or `PeopleReadAll`, with trusted
  current person-management relationships; project membership or management alone
  is not a people-directory grant. Do not infer access from profile ordering.
- Ordinary people reads must not expose role/grant configuration. Administrative
  editing stays on OP51; own access stays on OP01/49.
- Reports and approval name resolution must follow the authorized result set;
  an independent operation's identity needs do not grant whole-directory access.
  Project assignment choices need the reviewed OP12/16 eligibility contract.
- Rates follow their independent FR-021/022 field rules. Email/profile fields,
  inactive visibility, bounded pagination/search and compatibility API projection
  must be settled and tested across consumers before replacing legacy guards.
- Directory scope and field predicates, mutation/admission authority, migration
  differences and revocation tests remain mandatory. This repair closes none of
  T006–T009/T020 by itself and is not proof of Harvest parity.
