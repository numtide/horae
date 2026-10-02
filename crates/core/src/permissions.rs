//! Record coverage for one explicitly granted capability.
//!
//! Callers must supply current, trusted facts and separately enforce capability
//! prerequisites, request filters and business-state locks. This module neither
//! authenticates these facts nor protects against stale authorization snapshots.

use uuid::Uuid;

pub mod catalog;

/// Explicitly granted record scopes; management assignments alone grant nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessScope(u8);

/// Current identity loaded by the authorization service, not supplied by a client.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub id: Uuid,
    pub org_id: Uuid,
    pub active: bool,
}

/// Resource dimensions relevant to a particular capability check.
///
/// A missing dimension never matches own or managed scope for that dimension.
#[derive(Debug, Clone, Copy)]
pub struct ScopedResource {
    pub org_id: Uuid,
    pub person_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
}

/// Trusted management assignments for one actor in one organization.
///
/// Callers must verify that every listed person/project belongs to `org_id`.
/// Ordinary project membership is not a management assignment.
#[derive(Debug, Clone, Copy)]
pub struct ManagementAssignments<'a> {
    pub actor_id: Uuid,
    pub org_id: Uuid,
    pub people: &'a [Uuid],
    pub projects: &'a [Uuid],
}

impl AccessScope {
    pub const NONE: Self = Self(0);
    pub const OWN: Self = Self(1);
    pub const MANAGED_PEOPLE: Self = Self(2);
    pub const MANAGED_PROJECTS: Self = Self(4);
    pub const ORGANIZATION: Self = Self(8);

    /// Combines explicit scopes without inferring additional capabilities.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether this scope covers a resource, subject to identity/org isolation.
    ///
    /// A `true` result is only record coverage, not authorization to perform an
    /// action. Provenance mismatches deny even organization-wide or own scope.
    #[must_use]
    pub fn covers(
        self,
        actor: &Actor,
        resource: &ScopedResource,
        assignments: &ManagementAssignments<'_>,
    ) -> bool {
        if !actor.active
            || actor.org_id != resource.org_id
            || actor.org_id != assignments.org_id
            || actor.id != assignments.actor_id
        {
            return false;
        }

        self.includes(Self::ORGANIZATION)
            || (self.includes(Self::OWN) && resource.person_id == Some(actor.id))
            || (self.includes(Self::MANAGED_PEOPLE)
                && resource
                    .person_id
                    .is_some_and(|id| assignments.people.contains(&id)))
            || (self.includes(Self::MANAGED_PROJECTS)
                && resource
                    .project_id
                    .is_some_and(|id| assignments.projects.contains(&id)))
    }

    const fn includes(self, scope: Self) -> bool {
        self.0 & scope.0 != 0
    }
}

#[cfg(test)]
mod tests;
