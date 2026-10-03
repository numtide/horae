# Pure person-management prerequisites

Status: implemented and unit-tested under T050–T052, not runtime authorization or
complete US2. It refines user-approved FR-028/029/031 using the existing catalog.
FR-027 remains the independent Administrator-only writer boundary.

## Inputs and boundary

Use the existing validated `PermissionSelection`, responsible person's UUID and
proposed managed-person UUIDs. Validate additions against existing effective
grants without requiring prior relationships. Validate self-links against the
complete proposed replacement set, not merely its additions (FR-031). Keep the
two checks distinct so removing relationships does not require compatible grants.

Implement only a compatible-grant predicate and a fallible self-link validation
over the proposed set in `crates/core/src/permissions/person_management.rs`.
Expose them through `permissions.rs`; use the sibling-file module layout and no
new dependency, persistence entity, policy engine or assignment planner.
Reuse the compatibility predicate for FR-029's grant-loss decision; do not remove
anything or invent a confirmation/revision representation in this pure module.

Inputs must eventually come from trusted current server state. Passing these
prerequisite checks is **not** permission to write: the module neither establishes
Administrator identity nor validates tenancy, activation, current revisions,
confirmation, audit or concurrent state. No application consumer is added in
this increment. T012/T013 must enforce those boundaries in production transactions
before using the helpers for actual writes. Archive/deactivation and self-approval
semantics are not decided here.

## Closed compatibility mapping

At least one grant in the following table is sufficient for the FR-028 prerequisite.
These are receiving-person grants, never the acting user's assignment authority.
No profile-name comparison or inferred Administrator identity is permitted.

| Family | Existing `Permission` variants |
| --- | --- |
| Time | `TimeReadManaged`, `TimeWriteManaged`, `TimeApproveManaged`, `TimeReadAll`, `TimeWriteAll`, `TimeApproveAll` |
| Expenses | `ExpenseReadManaged`, `ExpenseWriteManaged`, `ExpenseReadAll`, `ExpenseWriteAll` |
| People | `PeopleReadManaged`, `PeopleWriteManaged`, `PeopleReadAll`, `PeopleWriteAll` |
| Person billable rates (FR-021) | `BillableRateReadManaged`, `BillableRateWriteManaged`, `BillableRateReadAll`, `BillableRateWriteAll` |
| Withdrawal | `ApprovalWithdrawManaged` |

All other current catalog grants are incompatible on their own. The Member floor
does not qualify. Use canonical selections, not unchecked wire values, and do not
add grants during validation. New catalog grants require explicit classification
review and tests rather than permission-name substring matching.

## Independent acceptance

| Case | Expected result | Requirement / task |
| --- | --- | --- |
| Every compatible grant, with its existing prerequisite closure | Eligible without a previous assignment or extra people-directory permission | FR-028 / T050–T051 |
| Each other catalog grant plus Member floor; combinations containing none of the compatible grants | Ineligible; no grants added | FR-028 / T050–T051 |
| At least one compatible grant remains after editing | Compatible, including read-only and organization-wide grants | FR-029 / T050–T051 |
| Last compatible grant removed, then later restored | Predicate changes false then true; no relationship is removed or recreated by either call | FR-029 / T050–T051 |
| Proposed set contains responsible person's UUID, alone or among valid IDs, in either order | Validation returns an error for the entire proposal, not a filtered set | FR-031 / T050–T051 |
| Proposed set contains only distinct other people, or is empty | No self-link error; empty removal does not need compatible grants | FR-028/031 / T050–T051 |
| Actor would equal responsible person but managed IDs are different | No false self-link rejection; actor is deliberately not an input to this check | FR-027/031 / T050–T051 |
| Any validation call | Input selection/IDs unchanged; no I/O, schema, runtime consumers or authority inferred from grants | FR-011/017 / T052 |

Test the full `Permission::ALL` catalog using an independently enumerated expected
set, not the implementation's own allow-list. Cover mixed grant sets and removal
through existing selection methods; verify equality of inputs before/after.
This is a local test oracle for approved Horae decisions, not evidence of Harvest
server enforcement or atomic database commits. Transactional mixed-batch denial,
revocation and audit rollback remain T012/T013 acceptance obligations.
