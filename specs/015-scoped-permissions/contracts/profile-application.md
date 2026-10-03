# Profile selection, individual edits and unchanged saves

This refines FR-004/011/013/015, not template renaming or bulk propagation.

## Evidence

- The [permission guide](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
  describes selection loading a custom role's permissions, optional further edits
  and a separate person save. Individual edits can remain personal or become a
  newly saved reusable role.
- The [Users API](https://help.getharvest.com/api-v2/users-api/users/users/#custom-profiles)
  documents unchanged-profile submissions preserving individual grants and
  different-profile submissions replacing them. That is an API contract, not a
  reason to disregard an explicit permission edit in the web editor.
- The retained `permissions-VNAENDKR.js` asset, inspected on 2026-10-03, binds
  custom radios and labels to a click handler invoking `gi`, including an already
  selected radio. `gi` replaces draft grants with template grants plus the Member
  floor; it retains only Forecast grants separately, outside Horae's scope.
  Built-in changes invoke `Rt`; the explicit reset control invokes `bt`.
  Neither selection nor reset submits the person form. See
  [asset provenance](current-account-investigation.md#current-catalog-and-editor-provenance).

These observations establish draft intent, not successful Harvest persistence.
The exact saved classification and template-creation name equivalence remain
open. Horae's compatibility API remains read-only.

## Required behavior

### Classification is not assignment provenance

The retained editor's initialization calls `dt(L)` for non-Administrators,
choosing among contained built-in and Member-normalized custom profiles by
largest permission count. Built-ins win equal-size cross-kind ties; within a kind,
source order is retained. The fallback `Ve`/`DL` excludes Administrator and
selects a contained built-in, or the built-in with the fewest missing grants.
This computation changes the displayed selection, not `L`.

Consequently, adding a more specific matching template can change which label
the next editor load displays without changing the person's grants. Two distinct
templates with equal grants can also have order-dependent displayed selection.
This is a deduction from the inspected client algorithm, not an observed saved
template-assignment transition. Do not assume alphabetical order: the handler
sorts newly created templates, but that does not establish server ordering.

Keep three concepts distinct in acceptance: canonical grants, explicitly applied
source/template revision, and computed presentation. A changed presentation cannot
replace provenance, increment access revisions, emit an access-change audit or
authorize an operation. Conversely, an explicit source change may require audit
even when grants are equal, as specified by the permission-state protocol.
The exact saved classification remains unverified; do not use the client fallback
to repair unknown/malformed stored state or infer an Administrator.

### Commands

1. Loading or saving an unchanged permission editor preserves the canonical
   person grants. A displayed best-fit profile is descriptive: rendering or
   serializing its label cannot replace those grants.
1. Explicitly selecting a built-in/custom profile replaces the proposed baseline
   with that profile's grants, rather than unioning it with the previous profile.
   Clicking the selected custom profile or using reset explicitly reloads its
   baseline, so individual adjustments may disappear from the draft. Show the
   resulting differences before save; merely displaying the selected radio is
   not a selection action.
1. Changes made after selection are part of the final proposed person grants.
   They must not be discarded just because the profile identifier is unchanged.
   A reset can also restore template grants previously removed by the person
   adjustment; it is not always a privilege reduction.
1. Selecting/resetting/cancelling changes no persisted person, template or other
   assignee. Saving a reusable template is a distinct explicit command, not
   silent modification of the template used by other people.
1. Confirm the final normalized grants and relationship effects under
   FR-025/029/030. Recheck Administrator identity, organization and relevant
   person/template revisions. Deleted, foreign or changed templates cannot be
   silently substituted with a new same-name object or latest unreviewed grants.
   Apply the existing atomic audit, stale-edit and last-administrator protocol.
1. After successful save and reload, the canonical grants equal the confirmed
   proposal, including person adjustments. Identity and authorization never come
   from best-fit names. Do not normalize malformed stored grants during loading.

## Planned acceptance

T010/T011 cover the saved state; T016/T017 cover profile operations; T018 covers
both permission screens. Use disposable users and production commands.

| Action | Expected result |
| --- | --- |
| Load and save unchanged person with extra and missing template grants | Preserve exact grants; no profile-name-driven reset |
| Select a different profile | Draft matches its baseline, not old/new union; nothing persists before save |
| Click selected custom profile or reset after individual edits | Draft returns to baseline; display additions and removals before confirmation |
| Select, then customize, then save with the same profile ID | Persist the final confirmed adjusted set, not a profile-ID no-op |
| Cancel replacement/reset | Original stored grants and relationships remain |
| Template deleted/changed or person revised after preview | Conflict without partial mutation, same-name substitution or stale restoration |
| Non-admin, revoked actor, foreign target, last-admin violation or audit failure | No unauthorized or partial permission change |
| Final grants remove management eligibility | Apply confirmed FR-025/029 effects atomically; FR-030 remains an explicit option |
| Add a more specific matching template, then load an unchanged person | Any changed display classification leaves canonical grants and explicit provenance untouched; no access-change event |
| Equal-grant templates change display order | No persisted reassignment or access change from the display tie; explicit same-grant source changes remain separately auditable |

Classification on reload, creation-name equivalence and in-place template
update/rename behavior require their own remaining contracts. This increment
does not mark T006–T009 complete or claim passing runtime tests.
