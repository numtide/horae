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
| Approve/withdraw expenses | Shared timesheet workflow documented | Exact combination of time approval, expense scope and withdrawal grants; do not collapse these operations |

Legacy lifecycle sources: [editing expenses](https://support.getharvest.com/hc/en-us/articles/4406054281101-How-to-edit-expenses),
[tracking/categories](https://support.getharvest.com/hc/en-us/articles/360048687611-Tracking-expenses),
[detailed reports](https://support.getharvest.com/hc/en-us/articles/360048687171-Detailed-time-and-detailed-expense-reports).

Feature 016 / PR #214 owns expense behavior and discriminating fixtures. These
findings narrow FR-002/020's gaps but do not close the complete operation matrix,
supersede last-administrator protection or expose unimplemented product grants.
An editable non-owner reference account is still needed for enforcement checks;
no seat purchase or invitation is authorized by this document.
