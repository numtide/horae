# Managed-rate scope: reference evidence and verification

Checked: 2026-10-02. Status: C03 resolved for Horae by user decision; Harvest
enforcement remains unverified and C04 cost visibility remains unresolved.
Owns the rate-scope portion of T006 and FR-002/005/008/015/020/021.

## What the evidence establishes

| Source | Finding | Limitation |
| --- | --- | --- |
| [Permissions](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions) | The managed billable-rate read/write descriptions refer to projects. Cost grants are account-wide read/write; Accounting and Executive Manager have financial reads without rate edits. | This conflicts with the previously observed editor label and does not define each underlying rate field's scope. |
| [Retained editor observation](reference-profiles.md) | The connected account's managed billable-rate labels referred to people. | Read-only configuration observation on 2026-09-30, not a persisted authorization test. |
| [Setting billable rates](https://support.getharvest.com/hc/en-us/articles/360048181492) | Describes assigned people and managed projects using Manager terminology. | Older role terminology does not prove new custom-grant semantics. |
| [Users API](https://help.getharvest.com/api-v2/users-api/users/users/) | The legacy `billable_rates_manager` role description includes people/projects; teammate default-rate visibility has a separate restriction. | It does not specify the new managed grant's field-by-field enforcement. |
| [User Billable Rates API](https://help.getharvest.com/api-v2/users-api/users/billable-rates/) | Even GET access is described in terms of administrator/manager rate-editing authority. | This is not proof that the new finance profiles cannot read rates in the application. |
| [User Cost Rates API](https://help.getharvest.com/api-v2/users-api/users/cost-rates/) | Describes administrator-only endpoint access. | Does not explain newer Accounting/Executive Manager read access in the application. |
| [Project User Assignments API](https://help.getharvest.com/api-v2/projects-api/projects/user-assignments/) | Membership/manager designation and default/custom rate fields are distinct. | A broad endpoint role statement does not prove which custom permission permits each field mutation. |

Initial investigation decision: retain the conflict rather than choose a people-only, project-only or
union predicate from documentation wording. Rationale: each interpretation grants
different sensitive data access. Rejected: treating the most permissive union as
safe, using API legacy role prose as proof of current UI enforcement, or inventing
a managed-cost dimension absent from the observed catalog.

The [2026-10-02 live capture](current-account-investigation.md) now retains exact
keys: `billable_rates:read:managed` (20), `billable_rates:write:managed` (41),
`billable_rates:read:all` (9), `billable_rates:write:all` (10),
`cost_rates:read:all` (11) and `cost_rates:write:all` (12). Managed labels still
refer to people; capturing identifiers does not resolve enforcement scope.
Person Rates help also says costs are administrator-only despite the current
Accounting/Executive defaults containing cost read. The owner has no rate history;
project editing separately exposes billing modes and custom person-project costs.
No rates were changed. Horae internal names need not copy external identifiers.

## Discriminating reference fixture

Use disposable records on an account with the new permissions model and an
editable non-owner person. The current account evidence contains only an immutable
owner and a paid-seat admission gate; no purchase, invitation or role mutation is
authorized by this document.

Set up actor A, separately managed person B, unrelated person C, managed project
P and unmanaged project Q. Include both B and C on both projects so person and
project responsibility vary independently. Record ordinary tracking membership,
manager assignments, exact selected grants and automatic prerequisites separately.

| Probe | Observation needed |
| --- | --- |
| Managed billable read | Compare B/C default rates and their overrides on P/Q; record visible controls and serialized payloads, not just navigation access |
| Managed billable write | Independently test person defaults, person-project overrides, task overrides and project rates; record success/denial and persisted state |
| Own financial access | Inspect A's own billable/cost values with no rate grant, managed read, then managed write; own-time access does not settle this |
| Finance read-only profiles | Accounting and Executive Manager can read the documented values but cannot change rates; check UI and direct requests |
| Independent cost access | Compare no cost grant, account-wide cost read and cost write; billable grants must not be assumed to grant cost access |
| Assignment versus rate editing | Test membership, manager designation, default-rate selection and custom rate fields independently under assignment-only and rate-only grants |
| Relationship revocation | Remove B's people assignment while retaining P, then remove P while retaining B; repeat reads/writes to identify the actual predicate |
| Derived and historical values | Compare profile, project, reports/exports and applicable API payloads for unauthorized rate history or derived financial leakage |

Every result must identify the account permission model, actor grants, exact
resource relation, request surface and outcome. A denied request to an old-model
API endpoint does not automatically settle the new-model web application's rule.
For unresolved reference cases, retain the missing outcomes in the matrix.
The approved C03 decision may supply Horae's contract and local expected results;
it does not turn a local pass into an observed Harvest result.

This is a reference-validation protocol, not executed tests. The approved C03
rule below supplies expected local outcomes without claiming Harvest verification.
Record any later reference difference explicitly; do not silently change the
approved contract or use it to approve existing-data migration mappings.

## 2026-10-02 approved Horae rule

Rechecked the linked permission, rate-setting and Users API documentation.
[Editing billable rates](https://support.getharvest.com/hc/en-us/articles/12522267831181-Editing-billable-rates)
adds a useful resource distinction: person-default changes affect projects that
inherit that rate, whereas person-project overrides affect only that project.
Its access instructions still use legacy Manager terminology, so this does not
establish new custom-grant enforcement or resolve C03 by itself.

User accepted option A after the resource-specific recommendation: with the corresponding
managed billable read/write grant, authorize person-default rates/history by the
managed-person relationship and project-owned rates/history (including person
and task overrides) by the managed-project relationship. Managing a project does
not authorize changing participants' global defaults; managing a person does not
authorize their overrides in unrelated projects. Project access may disclose the
effective inherited rate needed for that project, not unrelated personal rate
history. Global task defaults require organization-wide rate authority; retain
separate operation/resource permissions and independent cost permissions.

This is a field-specific rule, not a generic person-or-project union. A
person-default edit can legitimately affect inheriting projects outside the
actor's project-management scope; show that effect without exposing unauthorized
project identities. Such an edit must not write those projects' local overrides.
The alternative of leaving C03 undecided was not selected. No paid seat or
company-account write is required for local verification. FR-021 defines the
approved contract; application enforcement has not been implemented.

Local tests must cross managed/unmanaged B/C with managed/unmanaged P/Q for
read and write independently, then remove each grant or relationship. Include
inherited-rate display versus unrelated history, global task defaults,
read-only mutation denial, foreign organizations and unchanged project overrides
after a permitted person-default change. Full-feature acceptance remains gated.
