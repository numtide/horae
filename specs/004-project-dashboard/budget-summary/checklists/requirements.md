# Specification quality: project budget remaining tile

Created: 2026-10-22. Feature: [spec.md](../spec.md).

- [x] User value, bounded scope and the #208 requirement mapping are explicit.
- [x] Later deliveries (Total hours/G01, charts, costs, invoices, expenses) are
  named as out of scope, not claimed or waived.
- [x] User scenarios have independent tests with exact expected values.
- [ ] Every product decision is resolved: O1 (exact-half percentage rounding)
  is user-deferred and O2 (the budget disclosure) is a pending proposal.
- [x] Requirements are testable; no `[NEEDS CLARIFICATION]` marker remains in
  the server/foundation scope.
- [x] Success criteria are measurable and technology-agnostic.
- [x] Blank, zero, unavailable (withheld or overflowed) and signed values
  are distinguished.
- [x] Field authority is complete for this tile: fee redaction and private
  scope details follow existing contracts; no shared-member minute split is used.
- [x] Consistency, account binding and organization isolation are required.
- [x] Loading, denial, keyboard, theme, zoom and viewport cases are included.
- [x] Assumptions and dependencies (handoff gap, feature 016) are recorded.

Ten of eleven pass. The foundation and server work is specified well enough
to build; the UI is not fully clarified while O1 and O2 are open. This is not
evidence of delivery.
