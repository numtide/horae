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

/// Parsed intent for one rate field, distinct from its current stored value.
/// Amount validation and the field's currency remain the caller's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateEdit {
    /// Retain storage without reading or echoing the protected value.
    Unchanged,
    /// Remove the override, using the field's own inheritance rules.
    Reset,
    /// An explicit amount in the field's currency's minor units, including zero.
    Set(i64),
}

/// An explicit rate mutation lacks current field-write authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Current rate editing authority is required")]
pub struct RateEditDenied;

impl RateEdit {
    /// Check a server-derived field-write decision without comparing stored data.
    ///
    /// Resource authority, currency/amount validation and state locks remain
    /// independent. Keeping a field unchanged does not authorize the operation.
    ///
    /// # Errors
    /// Rejects every explicit edit without write authority, even a reset of an
    /// absent override or a value equal to storage.
    pub fn authorize(self, may_write: bool) -> Result<Self, RateEditDenied> {
        if matches!(self, Self::Unchanged) || may_write {
            Ok(self)
        } else {
            Err(RateEditDenied)
        }
    }
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
