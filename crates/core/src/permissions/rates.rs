//! Financial field gates, not complete authorization for a resource operation.
//!
//! Callers supply current authenticated facts and independently enforce resource
//! access, state locks and revocation. Report-only projections have their own
//! policy and must not use these checks to grant ordinary rate access.

use uuid::Uuid;

use super::{
    AccessScope, Actor, ManagementAssignments, ScopedResource,
    catalog::{Permission, PermissionSelection},
};

/// The financial action, independent of ordinary resource read/write authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateAction {
    Read,
    Write,
}

/// The owner of the requested billable field, established by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillableRateOwner {
    /// General person defaults and history, not project overrides.
    Person(Uuid),
    /// Project rates/overrides and effective inherited project values only.
    Project(Uuid),
    /// Global task defaults, requiring organization-wide rate authority.
    GlobalTask,
}

/// Trusted tenant and field ownership; not a client-selectable scope.
#[derive(Debug, Clone, Copy)]
pub struct BillableRateResource {
    pub org_id: Uuid,
    pub owner: BillableRateOwner,
}

/// Checks only the financial grant and its exact owner scope.
///
/// Project-effective inherited values do not authorize general person history
/// or default edits. The owner must describe the actual field being accessed.
#[must_use]
pub fn billable_rate_access(
    selection: &PermissionSelection,
    action: RateAction,
    actor: &Actor,
    resource: &BillableRateResource,
    assignments: &ManagementAssignments<'_>,
) -> bool {
    let (managed_grant, all_grant) = match action {
        RateAction::Read => (
            Permission::BillableRateReadManaged,
            Permission::BillableRateReadAll,
        ),
        RateAction::Write => (
            Permission::BillableRateWriteManaged,
            Permission::BillableRateWriteAll,
        ),
    };
    let (person_id, project_id, managed_scope) = match resource.owner {
        BillableRateOwner::Person(id) => (Some(id), None, AccessScope::MANAGED_PEOPLE),
        BillableRateOwner::Project(id) => (None, Some(id), AccessScope::MANAGED_PROJECTS),
        BillableRateOwner::GlobalTask => (None, None, AccessScope::NONE),
    };
    let scope = if selection.contains(all_grant) {
        AccessScope::ORGANIZATION
    } else if selection.contains(managed_grant) {
        managed_scope
    } else {
        AccessScope::NONE
    };
    scope.covers(
        actor,
        &ScopedResource {
            org_id: resource.org_id,
            person_id,
            project_id,
        },
        assignments,
    )
}

/// Checks only the explicit cost grant and active, same-organization identity.
///
/// There is no managed-cost dimension or Administrator-identity shortcut.
/// Resource authority and business-state constraints remain independent.
#[must_use]
pub fn cost_rate_access(
    selection: &PermissionSelection,
    action: RateAction,
    actor: &Actor,
    org_id: Uuid,
) -> bool {
    let grant = match action {
        RateAction::Read => Permission::CostRateReadAll,
        RateAction::Write => Permission::CostRateWriteAll,
    };
    actor.active && actor.org_id == org_id && selection.contains(grant)
}

#[cfg(test)]
mod tests;
