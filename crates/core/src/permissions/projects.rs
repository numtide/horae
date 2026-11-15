//! Project grant rules shared by every place that decides them.
//!
//! `has` answers whether the person's current, trusted grants include a
//! permission; `managed` whether they are a designated manager of the project.
//! Callers still check that the person is active and that the project exists
//! in their organization. SQL that filters many projects passes the two grants
//! as parameters and applies the same rule there.

use super::catalog::Permission;

/// Whether someone may edit projects at all: every project, or the ones they
/// manage. Offered before a project is chosen.
#[must_use]
pub fn writes_any(has: impl Fn(Permission) -> bool) -> bool {
    has(Permission::ProjectWriteAll) || has(Permission::ProjectWriteManaged)
}

/// Whether someone may edit a project: with `ProjectWriteAll`, or with
/// `ProjectWriteManaged` on a project they manage.
#[must_use]
pub fn writes(has: impl Fn(Permission) -> bool, managed: bool) -> bool {
    has(Permission::ProjectWriteAll) || (managed && has(Permission::ProjectWriteManaged))
}

/// Whether someone sees a project's rate and money budget: with
/// `BillableRateReadAll`, or with `BillableRateReadManaged` on a project they
/// manage.
#[must_use]
pub fn reads_money(has: impl Fn(Permission) -> bool, managed: bool) -> bool {
    has(Permission::BillableRateReadAll) || (managed && has(Permission::BillableRateReadManaged))
}
