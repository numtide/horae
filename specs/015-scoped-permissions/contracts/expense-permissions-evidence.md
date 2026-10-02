# Expense permission reference

Observed 2026-10-01, read-only, through the connected Harvest account's
`permissions-config-data-island`. The owner editor remains disabled. No role,
template or person was changed. This is delivered configuration, not proof of
server enforcement or permission to bypass the disabled editor.

## Expense catalog

| Reference ID | Reference capability | Declared scope |
| --- | --- | --- |
| 24 | `expenses:read:own` | Own entries |
| 25 | `expenses:write:own` | Log/edit own entries |
| 15 | `expenses:read:managed` | Managed people and projects |
| 16 | `expenses:write:managed` | Log/edit managed people and projects |
| 3 | `expenses:read:all` | Organization |
| 4 | `expenses:write:all` | Log/edit organization entries |

Reference IDs are evidence labels, not Horae identifiers or trusted imported
grants. No separate expense-category, receipt, expense-approval or locked-edit
capability was present in the displayed catalog. Absence does not prove denial
or establish which other grant governs those operations.

## Built-in defaults

| Profile | Direct expense grants |
| --- | --- |
| Member | Own read + write |
| Project Manager | Own and managed read + write |
| People Admin | All read + write |
| Accounting | All read + write |
| Executive Manager | All read + write |
| Administrator | All read + write |

The [current permission article](https://support.getharvest.com/hc/en-us/articles/44171549176077-Permissions)
corroborates those scopes and customizable expense dimensions. Retain the Member
floor and the editor's prerequisite rules from [reference-profiles.md](reference-profiles.md).
Do not infer scope from a role-name hierarchy or from time editing.

## Unresolved lifecycle mapping

| Operation | Established | Still required |
| --- | --- | --- |
| Ordinary expense read/log/edit | Six grants and profile defaults above | Managed-person versus managed-project overlap; authoritative allow/deny tests |
| Expense deletion | Own unlocked deletion is documented | Other-owner/custom-grant and locked deletion mapping |
| Locked correction | Legacy web documentation permits Administrator notes/amount changes | New-model custom-grant boundary; receipt/date/billability changes are not established by broader API wording |
| Categories | Legacy documentation identifies Administrator management | Whether any custom capability permits administration, separately from expense writing |
| Receipt view/download/change | Expense relationship documented | Current-grant mapping; direct download, report inclusion and revocation enforcement |
| Mark billed/unbilled | Billable-only expense marking documented | New-model authority and independent lock effects |
| Approve/withdraw expenses | Shared timesheet workflow documented; Horae FR-024 now requires approval authority plus visibility of all selected time/expenses, with atomic denial if any are inaccessible | Harvest restricted-user enforcement, remaining approval predicates and withdrawal mapping; do not extend the approved visibility rule to withdrawal or infer ordinary expense-write/receipt authority |

Legacy lifecycle sources: [editing expenses](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses),
[tracking/categories](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses),
[detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).

Feature 016 / PR #214 owns expense behavior and discriminating fixtures. These
findings narrow FR-002/020's gaps but do not close the complete operation matrix,
supersede last-administrator protection or expose unimplemented product grants.
An editable non-owner reference account is still needed for enforcement checks;
no seat purchase or invitation is authorized by this document.

## Owner-only action and lock evidence from feature 016

The authorized disposable experiments in [PR #214](https://github.com/numtide/horae/pull/214)
now distinguish execution scope from actor authority. With 1,000 rows loaded,
an expense moved into the report filter afterwards was also marked by the
no-selection action: all 1,001 current matching entries were marked, while an
out-of-date-filter control stayed unchanged. Explicit selection subsequently
cleared exactly two IDs and left 999 billed. Authorization must be rechecked for
the actual execution target set; a previously loaded report is not authority to
mutate later matching records outside the actor's current scope.

The cleared expense had no invoice association and `is_billed=false`, but still
reported a project-period lock while the other 999 expenses remained billed.
Clearing the remaining filtered set left all 1,001 unbilled and unlocked. This
establishes the named case, not the complete interval derivation algorithm.
The separate archive experiment proves clearing manual billing on an archived
project retains its archive lock until restoration.

Keep billed status, invoice association, project-period protection, archive and
approval/company locks distinct. These are owner reference results, not proof
that any expense-write grant permits bulk billing, interval unlocking or
privileged correction. The non-owner lifecycle matrix above remains open.
