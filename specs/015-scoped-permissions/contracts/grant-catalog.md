# Grant catalog and profile selection

Implementation increment authorized by the user's 2026-10-02 request to implement
permissions using local fixtures without paid or company Harvest access. This
contract covers FR-001/003/004/015's confirmed catalog and editor dependencies;
it does not settle operation authorization, persisted template lifecycle or C01–C07.

## Pure model

- `Permission` is a closed, typed catalog of the 50 observed web grants. Stable
  snake-case names are Horae wire identifiers; unknown names must fail decoding.
  Reference IDs 59/60 are not assigned meanings. Billing/estimates/expense grants
  in the catalog are not evidence that those product surfaces are implemented.
- `BuiltInProfile` has six distinct variants, not an ordered rank. Direct defaults
  follow the captured configuration in `current-account-investigation.md` and
  `reference-profiles.md`; Administrator's two unknown IDs are excluded.
- `PermissionSelection` is an editable, normalized grant selection, not a trusted
  authorization context. Construction includes the four immutable Member grants
  (own time/expense read/write) and transitive prerequisites. Adding a permission
  adds prerequisites. Removing one removes its dependants transitively; attempting
  to remove a floor grant fails without changing the selection.
- Normalization is deterministic and idempotent. Duplicates disappear. The selected
  set can be inspected to display every included/removed grant before a future save.
  Decoding raw grants does not deserialize directly into a trusted selection.
- Profile identity is separate from grant membership. Selecting all grants does
  not establish Administrator status or authority to assign permissions. There is
  no conversion from the legacy `OrgRole`, auto-promotion or database migration.

## Prerequisites

Use explicit typed edges matching the observed editor, not English-label parsing:
write/create/approve includes corresponding read; organization grants include
managed variants where present; managed reads include own reads; managed writes
include own writes. Managed invoice management additionally includes draft editing.
Report reads add no ordinary rate/time grant; approval adds reads, not expense or
time-write authority; withdrawal adds no implicit approval grant. These editor
edges are not yet the complete server operation contract.

## Verification and boundary

Tests cover all catalog entries, exact direct defaults, all six normalized sets,
every dependency edge and removal, immutable floor, normalization laws, unknown
wire names, financial/report separation and approval/withdrawal separation.
No capability-to-record-scope mapping is inferred for unresolved managed rates.
No runtime guard consumes this increment before reviewed integration and migration.
Full feature acceptance remains all five user stories and SC-001–009, not this module.
