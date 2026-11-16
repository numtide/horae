//! The grant sets each invoice action requires, shared by the server and the
//! interface so the two cannot disagree on what is offered.
//!
//! The managed invoice grants (`InvoiceReadManaged`, `InvoiceDraftWriteManaged`,
//! `InvoiceWriteManaged`) authorize nothing yet: which invoices a person
//! manages is still open (T006,
//! `specs/015-scoped-permissions/contracts/invoice-scope-evidence.md`).

use super::catalog::Permission;

/// Reading issued invoices, including their exports.
pub const READ: &[Permission] = &[Permission::InvoiceReadAll];
/// Changing an invoice's status.
pub const WRITE: &[Permission] = &[Permission::InvoiceWriteAll];
/// Previewing and generating an invoice from uninvoiced work, and editing a
/// draft's values and fee lines: these price work from the current billable
/// rates, beyond what an issued invoice records.
pub const PREPARE: &[Permission] = &[Permission::InvoiceWriteAll, Permission::BillableRateReadAll];

/// Whether `has` grants every permission in `required`.
#[must_use]
pub fn holds(has: impl Fn(Permission) -> bool, required: &[Permission]) -> bool {
    required.iter().all(|&permission| has(permission))
}
