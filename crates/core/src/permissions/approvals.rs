//! Record grant/scope checks, not a complete approval operation.
//!
//! Callers must load the complete intended set under current authorization,
//! independently check submission/lock coverage and self-approval, and commit
//! all effects atomically. Never obtain these inputs by filtering out unreadable
//! records or reuse this result after releasing the authorization transaction.

use super::{
    AccessScope, Actor, ManagementAssignments, ScopedResource,
    catalog::{Permission, PermissionSelection},
};

const MANAGED: AccessScope = AccessScope::MANAGED_PEOPLE.union(AccessScope::MANAGED_PROJECTS);

/// Whether every supplied record has both approval and domain-read coverage.
///
/// Denies inactive/mismatched identity and missing approval grants, including
/// empty inputs. An empty domain needs no read grant. Success with two empty
/// slices establishes no date/project lock coverage; own-record success does
/// not override the independent self-approval policy.
///
/// Returns no allowed subset or identifying denial detail. This pure check does
/// not prove that the caller supplied the complete current selection.
#[must_use]
pub fn approval_records_covered(
    selection: &PermissionSelection,
    actor: &Actor,
    assignments: &ManagementAssignments<'_>,
    time: &[ScopedResource],
    expenses: &[ScopedResource],
) -> bool {
    if !actor.active || actor.id != assignments.actor_id || actor.org_id != assignments.org_id {
        return false;
    }
    let approval = if selection.contains(Permission::TimeApproveAll) {
        AccessScope::ORGANIZATION
    } else if selection.contains(Permission::TimeApproveManaged) {
        MANAGED
    } else {
        return false;
    };
    let time_read = read_scope(
        selection,
        Permission::TimeReadOwn,
        Permission::TimeReadManaged,
        Permission::TimeReadAll,
    );
    let expense_read = read_scope(
        selection,
        Permission::ExpenseReadOwn,
        Permission::ExpenseReadManaged,
        Permission::ExpenseReadAll,
    );
    time.iter().all(|record| {
        approval.covers(actor, record, assignments) && time_read.covers(actor, record, assignments)
    }) && expenses.iter().all(|record| {
        approval.covers(actor, record, assignments)
            && expense_read.covers(actor, record, assignments)
    })
}

fn read_scope(
    selection: &PermissionSelection,
    own: Permission,
    managed: Permission,
    all: Permission,
) -> AccessScope {
    if selection.contains(all) {
        return AccessScope::ORGANIZATION;
    }
    let mut scope = AccessScope::NONE;
    if selection.contains(own) {
        scope = scope.union(AccessScope::OWN);
    }
    if selection.contains(managed) {
        scope = scope.union(MANAGED);
    }
    scope
}

#[cfg(test)]
mod tests;
