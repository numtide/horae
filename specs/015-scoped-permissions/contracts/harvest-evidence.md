# Harvest permissions evidence

Checked 2026-09-30. Status labels distinguish documentation from actual browser observations. This is not yet the complete allow/deny matrix required for runtime cutover.

The [2026-10-02 current-account investigation](current-account-investigation.md)
records fresh browser outcomes, the current 50-grant catalog/public editor,
documentation conflicts and company-cutoff evidence. It revalidates the owner
and seat restrictions below without establishing non-owner enforcement.

## Current account observations

The connected account exposes the new six-profile editor: Member, Project Manager, People Admin, Accounting, Executive Manager and Administrator. The only person is its owner; the editor disables every profile and save action, explicitly explaining that owner permissions cannot change. There is no individual-permission checkbox panel for this immutable Administrator view.

The Team invite link leads to a subscription/plans page requesting a second seat. No seat was purchased, invitation sent, owner permission changed or control bypassed. A non-owner test person is needed for custom-profile interaction checks.

Read-only inspection subsequently recovered the editor's delivered profile/catalog configuration and public client-side dependency logic. [Reference profiles](reference-profiles.md) records these findings and documentation conflicts. This narrows the unknowns without bypassing disabled controls; persistence checks still need an editable test person.

Local evidence, intentionally excluded from Git because it contains account identity:

- `.scratch/playwright-windows/output/page-2026-09-30T13-28-20-855Z.yml`: single-person Team and invite link.
- `.scratch/playwright-windows/output/page-2026-09-30T13-29-59-116Z.yml`: six profiles, owner warning, disabled controls.

Both files are under the primary checkout, not this worktree. DOM inspection was read-only. Some normal clicks timed out waiting for visibility/stability; navigation and snapshots succeeded. This is evidence of rendered account state, not successful interactive custom-profile validation.

## Reference-to-requirement register

| Behavior | Evidence status | Requirement / acceptance |
| --- | --- | --- |
| Six built-in choices | Documented and observed | FR-001; test all operations, not only selector labels |
| Owner cannot change own permissions | Observed | Preserve last-administrator safety; ownership is not a custom grant |
| Only administrators assign/customize profiles | Documented | FR-011; deny direct forged requests |
| Reusable templates and per-person differences | Documented, interaction unverified | FR-004/015; save/apply/delete and isolated adjustments |
| Template deletion preserves current grants | Reference conflict remains: documentation versus editor warning; Harvest persistence unverified | C01 resolved for Horae by user decision on 2026-10-02: preserve existing grants/scope as person-specific configurations; local acceptance required, not observed Harvest parity |
| Prerequisite closure/dependent removal | Documented; client-side rule traversal observed, persistence unverified | FR-015; verify every customizable grant dependency |
| Built-in classification | Client-side selection rule observed; saved classification unverified | FR-015; classification must not grant additional access |
| Template rename/update/propagation | Unverified; observed editor supports save-as-new | FR-015; do not invent propagation |
| Profile reapplication and replacement | API-documented; UI persistence unverified | FR-015; [application evidence](reference-profiles.md#api-application-evidence) |
| Profile lookup and lossy role projection | API-documented | FR-014/017; keep import identity resolution separate from local authorization |
| Management assignment authority and promotion | Partially documented; current-profile interaction unverified | FR-005; no unintended privilege escalation |
| Person/project approval and date-filtered coverage | Documented | FR-009/019; mixed-project and date-range tests |
| Empty-cell locks and submitted-work editing | Documented | FR-019; entry creation/edit tests before/after approval |
| Scoped vs Day/Week withdrawal | Documented | FR-019; unrelated coverage and independent locks preserved |
| Approval from arbitrary custom combinations | Explicit managed/all approval grants observed; enforcement unverified | FR-009/015; explicit eligibility matrix |
| New projects after whole-submission approval, overlap splitting, post-withdrawal submission state | Unverified | FR-019; browser fixtures before final storage contract |

Sources: [new permission framework](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions), [flexible approval](https://support.getharvest.com/hc/en-us/articles/39974542812429-Flexible-timesheet-approval), [independent locks](https://support.getharvest.com/hc/en-us/articles/360048687491-Unlocking-time-and-expenses). This register retains full parity as the target; the account limitation does not authorize a simpler product.
