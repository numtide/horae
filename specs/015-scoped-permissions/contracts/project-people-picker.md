# Project team identity selection

Owner: OP12/16/19, FR-002/005/006/007/008/010/016/018/026.
Reviewed baseline: `981d0e3`, 2026-10-04. This closes the identity-only read
boundary for project creation/editing, not project persistence or policy activation.

## Evidence and limits

The [project creation guide](https://support.getharvest.com/hc/en-us/articles/360048686831-Create-and-duplicate-projects),
reopened 2026-10-04, allows any active account person to be assigned to a project.
Its old automatic Manager promotion is superseded for Horae by FR-026; do not
copy that side effect. The current
[permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
separates project editing from people management. Requiring `PeopleRead*` for
project-team selection would therefore prevent the intended Project Manager
workflow. The capability mapping below is an explicit inference from these
sources and the approved catalog, not observed restricted-user enforcement.

Read-only Chrome inspection reached an existing project editor. The sole
available person was already on the team, so there was no additional candidate
to inspect. No checkbox, rate or assignment was changed; Cancel left the editor.
Raw evidence stays under the main checkout's ignored
`.scratch/playwright-windows/output/page-2026-10-04T09-01-57-704Z.yml`.
Owner visibility does not establish custom-grant behavior.

`design/project/app/13_New Project.dc.html` uses a searchable teammate selector,
separate manager checkbox and separate rate controls. Its sample role labels
are not authority to disclose another person's permission profile. Existing
`project_creation::options` searches/resolves active same-organization people
but includes rates and relies on legacy Manager authorization without a project
context. `ProjectDetailContent` separately fetches the general user directory
for names and name/email choices. Neither response is the canonical picker.

## Authorized read

Use an explicit operation context, never a caller-supplied authority boolean:

| Context | Current required authority |
| --- | --- |
| New project | `ProjectCreateAll` |
| Existing project | Same-organization project plus `ProjectWriteAll`, or `ProjectWriteManaged` with a current designation on that exact project |
| Read-only project, tracking membership, people management, rates, or legacy role alone | Deny |

Every search and selected-ID resolution derives actor/organization from the
session, requires an active actor and exactly canonical policy 1, and strictly
loads stored grants. No legacy fallback or Administrator-identity bypass.
Creating a project is not authority to use the existing-project context; editing
one project is not authority to select under another project. An archived project
retains the existing editor's availability; this read never restores it.

Once that operation is authorized, eligible identities are all active people
in the same organization. Do not require them to be managed by the caller,
already on the project, or already have project-management grants. Their names
are needed for assignment, not evidence that their profiles or work are readable.
Selecting a tracking teammate never makes that person a project manager.
FR-026 independently governs designation eligibility and changes at commit.

Return only UUID/name choices plus the session requester and continuation bound.
No email, activity/role/profile/grant fields, rates, costs, provider identifiers,
timestamps or global count. This is deliberately not `PersonSummary` or
`CreationPerson`: finance and existing-member display remain separate projections.
Name is a label, UUID is identity; duplicate and Unicode names must remain distinct.

Follow the current bounded catalogs: trim search text, limit it to 100 Unicode
characters, reject NUL, perform literal case-insensitive substring matching,
return at most 50 choices plus a continuation cursor. Use deterministic name/UUID
keyset bounds; no invented wildcard language or full-directory download. Apply
tenant/activity/search bounds before continuation calculation. A supplied cursor
is only a bound, never an identity lookup or evidence of access.

Selected-ID resolution uses the same operation authorization and active-tenant
predicate, independently of search or pagination. Accept at most 500 IDs, return
each eligible identity once and omit unavailable choices without explaining
foreign versus inactive versus missing. Omitting an unavailable display choice
must not cause a later save to silently drop that person: finalization/editing
must validate the complete requested assignment set atomically.

Existing archived teammate labels must come from authorized saved-project or
assignment reads, not this new-assignment picker. No existing assignment is
removed, restored or converted by reading choices. Do not resolve the separate
pending archived person-management relationship question through this contract.

## Transaction and integration

Reuse bounded READ COMMITTED transaction settings and organization SHARE then
active actor SHARE, as the canonical directory reader does. Hold current
authority through the materialized read and commit. Reload after lock waits;
reauthorize every page and selected-ID request. Sanitize internal failures.
No receipt, audit event, schema, dependency or permission write is needed for
these reads. The choices are not a reservation: assignment/manager/rate saves
must independently recheck current actor, target eligibility and revisions.

Keep the financial form contract explicit during later integration: substituting
name-only choices must not clear stored rates, treat restricted values as zero,
or grant access to default rates. FR-021/022 require resource-specific financial
projection. Current editor writes and the complete guard replacement gate remain
separate; do not activate a partly enforced policy to demonstrate the picker.

## Requirement-to-test acceptance

- FR-002/006/007: create-only actor can search/resolve without people grants;
  managed editor can use only the designated project; all-project editor can
  use a local project; own/read-only/people-only/rate-only/legacy-only deny.
- FR-005/026: a selectable teammate needs no management eligibility; no permission,
  designation, tracking membership or business-record change occurs on reads.
- FR-006/008: active same-tenant choices only, exact minimal fields with non-null
  sensitive fixtures, duplicate names/distinct UUIDs, foreign/inactive/missing
  selected IDs omitted, unrelated project context denied without private details.
- FR-010: same-session revocation, both organization-gate orders, actor
  deactivation, missing/malformed state, cancellation and pool-default restoration.
- FR-018: 50/51 boundaries after filtering, continuation/search interaction,
  missing/foreign cursor IDs, selected identities outside the search/page, literal
  wildcard characters, Unicode/blank/oversize/NUL input and bounded ID resolution.
- Registered-route tests must prove session-derived requester fields and exact
  payload. UI wiring later must retain selections across pages without giving
  stale labels authority; full browser and cross-surface gates remain mandatory.

Adversarial contract review rejects a shared general-directory response, rate
fields hidden only by CSS, an edit context without project ID, promotion through
selection, pagination as mutation scope and silently losing unavailable form
selections. Reader implementation and verification evidence is recorded in
`../quickstart.md` and `../progress.md` (T154–T157). No restricted-account browser,
form integration or full-policy acceptance is claimed.
