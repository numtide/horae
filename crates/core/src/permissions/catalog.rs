//! Typed permission selections; not an authenticated authorization context.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Version of the grant identifiers, immutable floor and prerequisite semantics.
/// Changes to these persisted semantics require an explicit migration review.
pub const PERMISSION_CATALOG_VERSION: u32 = 1;

/// Known grant identifiers. Ordering only provides deterministic set iteration.
///
/// A catalog entry does not enable a product or define its record/field scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    TimeReadOwn,
    TimeWriteOwn,
    TimeReadManaged,
    TimeWriteManaged,
    TimeApproveManaged,
    TimeReadAll,
    TimeWriteAll,
    TimeApproveAll,
    ExpenseReadOwn,
    ExpenseWriteOwn,
    ExpenseReadManaged,
    ExpenseWriteManaged,
    ExpenseReadAll,
    ExpenseWriteAll,
    ProjectReadManaged,
    ProjectWriteManaged,
    ProjectCreateAll,
    ProjectReadAll,
    ProjectWriteAll,
    ClientReadAll,
    ClientWriteAll,
    TaskReadAll,
    TaskWriteAll,
    PeopleReadManaged,
    PeopleWriteManaged,
    PeopleReadAll,
    PeopleWriteAll,
    BillableRateReadManaged,
    BillableRateWriteManaged,
    BillableRateReadAll,
    BillableRateWriteAll,
    CostRateReadAll,
    CostRateWriteAll,
    InvoiceReadManaged,
    InvoiceDraftWriteManaged,
    InvoiceWriteManaged,
    InvoiceReadAll,
    InvoiceWriteAll,
    EstimateReadAll,
    EstimateWriteAll,
    ReportProfitabilityRead,
    ReportContractorRead,
    ReportInvoicingRead,
    SavedReportReadInactive,
    SavedReportWriteInactive,
    ApprovalWithdrawManaged,
    CompanyRead,
    CompanyWrite,
    BillingRead,
    BillingWrite,
}

impl Permission {
    /// All supported catalog entries; unknown external IDs have no representation.
    pub const ALL: &'static [Self] = &[
        Self::TimeReadOwn,
        Self::TimeWriteOwn,
        Self::TimeReadManaged,
        Self::TimeWriteManaged,
        Self::TimeApproveManaged,
        Self::TimeReadAll,
        Self::TimeWriteAll,
        Self::TimeApproveAll,
        Self::ExpenseReadOwn,
        Self::ExpenseWriteOwn,
        Self::ExpenseReadManaged,
        Self::ExpenseWriteManaged,
        Self::ExpenseReadAll,
        Self::ExpenseWriteAll,
        Self::ProjectReadManaged,
        Self::ProjectWriteManaged,
        Self::ProjectCreateAll,
        Self::ProjectReadAll,
        Self::ProjectWriteAll,
        Self::ClientReadAll,
        Self::ClientWriteAll,
        Self::TaskReadAll,
        Self::TaskWriteAll,
        Self::PeopleReadManaged,
        Self::PeopleWriteManaged,
        Self::PeopleReadAll,
        Self::PeopleWriteAll,
        Self::BillableRateReadManaged,
        Self::BillableRateWriteManaged,
        Self::BillableRateReadAll,
        Self::BillableRateWriteAll,
        Self::CostRateReadAll,
        Self::CostRateWriteAll,
        Self::InvoiceReadManaged,
        Self::InvoiceDraftWriteManaged,
        Self::InvoiceWriteManaged,
        Self::InvoiceReadAll,
        Self::InvoiceWriteAll,
        Self::EstimateReadAll,
        Self::EstimateWriteAll,
        Self::ReportProfitabilityRead,
        Self::ReportContractorRead,
        Self::ReportInvoicingRead,
        Self::SavedReportReadInactive,
        Self::SavedReportWriteInactive,
        Self::ApprovalWithdrawManaged,
        Self::CompanyRead,
        Self::CompanyWrite,
        Self::BillingRead,
        Self::BillingWrite,
    ];

    /// Direct editor prerequisites, not an operation-level authorization decision.
    pub const fn prerequisites(self) -> &'static [Self] {
        use Permission::*;
        match self {
            TimeWriteOwn => &[TimeReadOwn],
            TimeReadManaged => &[TimeReadOwn],
            TimeWriteManaged => &[TimeReadManaged, TimeWriteOwn],
            TimeApproveManaged => &[TimeReadManaged],
            TimeReadAll => &[TimeReadManaged],
            TimeWriteAll => &[TimeReadAll, TimeWriteManaged],
            TimeApproveAll => &[TimeReadAll, TimeApproveManaged],
            ExpenseWriteOwn => &[ExpenseReadOwn],
            ExpenseReadManaged => &[ExpenseReadOwn],
            ExpenseWriteManaged => &[ExpenseReadManaged, ExpenseWriteOwn],
            ExpenseReadAll => &[ExpenseReadManaged],
            ExpenseWriteAll => &[ExpenseReadAll, ExpenseWriteManaged],
            ProjectWriteManaged => &[ProjectReadManaged],
            ProjectCreateAll => &[ProjectReadAll],
            ProjectReadAll => &[ProjectReadManaged],
            ProjectWriteAll => &[ProjectReadAll, ProjectWriteManaged],
            ClientWriteAll => &[ClientReadAll],
            TaskWriteAll => &[TaskReadAll],
            PeopleWriteManaged => &[PeopleReadManaged],
            PeopleReadAll => &[PeopleReadManaged],
            PeopleWriteAll => &[PeopleReadAll, PeopleWriteManaged],
            BillableRateWriteManaged => &[BillableRateReadManaged],
            BillableRateReadAll => &[BillableRateReadManaged],
            BillableRateWriteAll => &[BillableRateReadAll, BillableRateWriteManaged],
            CostRateWriteAll => &[CostRateReadAll],
            InvoiceDraftWriteManaged => &[InvoiceReadManaged],
            InvoiceWriteManaged => &[InvoiceReadManaged, InvoiceDraftWriteManaged],
            InvoiceReadAll => &[InvoiceReadManaged],
            InvoiceWriteAll => &[InvoiceReadAll, InvoiceWriteManaged],
            EstimateWriteAll => &[EstimateReadAll],
            SavedReportWriteInactive => &[SavedReportReadInactive],
            CompanyWrite => &[CompanyRead],
            BillingWrite => &[BillingRead],
            TimeReadOwn
            | ExpenseReadOwn
            | ProjectReadManaged
            | ClientReadAll
            | TaskReadAll
            | PeopleReadManaged
            | BillableRateReadManaged
            | CostRateReadAll
            | InvoiceReadManaged
            | EstimateReadAll
            | ReportProfitabilityRead
            | ReportContractorRead
            | ReportInvoicingRead
            | SavedReportReadInactive
            | ApprovalWithdrawManaged
            | CompanyRead
            | BillingRead => &[],
        }
    }
}

const MEMBER_FLOOR: &[Permission] = &[
    Permission::TimeReadOwn,
    Permission::TimeWriteOwn,
    Permission::ExpenseReadOwn,
    Permission::ExpenseWriteOwn,
];

/// Editable grant set with the Member floor and transitive prerequisites included.
///
/// This is not an authenticated policy. In particular, possessing every grant
/// does not establish administrator identity. Editor input uses [`Self::new`];
/// saved grants use [`Self::from_stored`] to prevent silent privilege expansion.
/// Neither construction path replaces server authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PermissionSelection(BTreeSet<Permission>);

/// An edit that would break the immutable own-tracking baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PermissionEditError {
    #[error("Own time and expense permissions cannot be removed")]
    MemberFloor,
}

/// Invalid saved grants must be explicitly repaired, never normalized on read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StoredPermissionError {
    #[error("Unsupported permission catalog version")]
    UnsupportedCatalogVersion,
    #[error("Stored permissions contain duplicate grants")]
    DuplicatePermission,
    #[error("Stored permissions are missing required grants")]
    NonCanonicalSelection,
}

impl PermissionSelection {
    /// Restores a saved grant set without adding defaults or prerequisites.
    ///
    /// Input order is irrelevant. The caller must decode every grant, rejecting
    /// unknown identifiers, and verify tenant, identity, revision and scope.
    /// This result is structurally valid, not an authenticated authority.
    ///
    /// # Errors
    /// Rejects unsupported versions, duplicate grants and incomplete selections.
    pub fn from_stored(
        catalog_version: u32,
        permissions: &[Permission],
    ) -> Result<Self, StoredPermissionError> {
        if catalog_version != PERMISSION_CATALOG_VERSION {
            return Err(StoredPermissionError::UnsupportedCatalogVersion);
        }
        let grants: BTreeSet<_> = permissions.iter().copied().collect();
        if grants.len() != permissions.len() {
            return Err(StoredPermissionError::DuplicatePermission);
        }
        if MEMBER_FLOOR.iter().any(|grant| !grants.contains(grant))
            || grants.iter().any(|grant| {
                grant
                    .prerequisites()
                    .iter()
                    .any(|required| !grants.contains(required))
            })
        {
            return Err(StoredPermissionError::NonCanonicalSelection);
        }
        Ok(Self(grants))
    }

    /// Builds a selection with the Member floor and every required grant.
    #[must_use]
    pub fn new(permissions: &[Permission]) -> Self {
        let mut selection = Self(BTreeSet::new());
        for &permission in MEMBER_FLOOR.iter().chain(permissions) {
            selection.add(permission);
        }
        selection
    }

    /// Tests explicit grant membership, not record access or administrator status.
    #[must_use]
    pub fn contains(&self, permission: Permission) -> bool {
        self.0.contains(&permission)
    }

    /// Iterates the complete normalized selection in deterministic catalog order.
    pub fn iter(&self) -> impl Iterator<Item = Permission> + '_ {
        self.0.iter().copied()
    }

    /// Adds a grant and its transitive prerequisites; repeated additions do nothing.
    pub fn add(&mut self, permission: Permission) {
        if self.0.insert(permission) {
            for &required in permission.prerequisites() {
                self.add(required);
            }
        }
    }

    /// Removes a grant and all dependants. An absent grant is a no-op.
    ///
    /// # Errors
    /// Returns [`PermissionEditError::MemberFloor`] without modifying the selection
    /// if the requested grant belongs to the immutable own-tracking baseline.
    pub fn remove(&mut self, permission: Permission) -> Result<(), PermissionEditError> {
        if MEMBER_FLOOR.contains(&permission) {
            return Err(PermissionEditError::MemberFloor);
        }
        self.0.remove(&permission);
        while let Some(dependent) = self.0.iter().copied().find(|candidate| {
            candidate
                .prerequisites()
                .iter()
                .any(|required| !self.0.contains(required))
        }) {
            self.0.remove(&dependent);
        }
        Ok(())
    }
}

/// Built-in profile identity, deliberately without rank ordering or legacy mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInProfile {
    Member,
    ProjectManager,
    PeopleAdmin,
    Accounting,
    ExecutiveManager,
    Administrator,
}

impl BuiltInProfile {
    /// Profiles in reference display order, not privilege precedence.
    pub const ALL: &'static [Self] = &[
        Self::Administrator,
        Self::ExecutiveManager,
        Self::ProjectManager,
        Self::Accounting,
        Self::PeopleAdmin,
        Self::Member,
    ];

    /// Known direct defaults before adding the floor and dependency closure.
    pub const fn direct_permissions(self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Self::Member => MEMBER_FLOOR,
            Self::ProjectManager => &[
                TimeReadOwn,
                TimeWriteOwn,
                TimeReadManaged,
                TimeWriteManaged,
                TimeApproveManaged,
                ExpenseReadOwn,
                ExpenseWriteOwn,
                ExpenseReadManaged,
                ExpenseWriteManaged,
                ProjectReadManaged,
                ProjectWriteManaged,
                ProjectCreateAll,
                ClientReadAll,
                ClientWriteAll,
                TaskReadAll,
                TaskWriteAll,
            ],
            Self::PeopleAdmin => &[
                TimeReadAll,
                TimeWriteAll,
                TimeApproveAll,
                ExpenseReadAll,
                ExpenseWriteAll,
                ProjectReadAll,
                PeopleReadAll,
                PeopleWriteAll,
                ReportContractorRead,
                ApprovalWithdrawManaged,
            ],
            Self::Accounting => &[
                TimeReadAll,
                ExpenseReadAll,
                ExpenseWriteAll,
                ProjectReadAll,
                ClientReadAll,
                ClientWriteAll,
                BillableRateReadAll,
                CostRateReadAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateReadAll,
                EstimateWriteAll,
                ReportProfitabilityRead,
                ReportInvoicingRead,
                SavedReportReadInactive,
                SavedReportWriteInactive,
            ],
            Self::ExecutiveManager => &[
                TimeReadAll,
                TimeWriteAll,
                TimeApproveAll,
                ExpenseReadAll,
                ExpenseWriteAll,
                ProjectReadAll,
                ProjectWriteAll,
                ProjectCreateAll,
                ClientReadAll,
                ClientWriteAll,
                TaskReadAll,
                TaskWriteAll,
                PeopleReadAll,
                PeopleWriteAll,
                BillableRateReadAll,
                CostRateReadAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateReadAll,
                EstimateWriteAll,
                ReportProfitabilityRead,
                ReportContractorRead,
                ReportInvoicingRead,
                SavedReportReadInactive,
                SavedReportWriteInactive,
                ApprovalWithdrawManaged,
            ],
            Self::Administrator => &[
                TimeReadAll,
                TimeWriteAll,
                TimeApproveAll,
                ExpenseReadAll,
                ExpenseWriteAll,
                ProjectReadAll,
                ProjectWriteAll,
                ProjectCreateAll,
                ClientReadAll,
                ClientWriteAll,
                TaskReadAll,
                TaskWriteAll,
                PeopleReadAll,
                PeopleWriteAll,
                BillableRateReadAll,
                BillableRateWriteAll,
                CostRateReadAll,
                CostRateWriteAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateReadAll,
                EstimateWriteAll,
                ReportProfitabilityRead,
                ReportContractorRead,
                ReportInvoicingRead,
                SavedReportReadInactive,
                SavedReportWriteInactive,
                ApprovalWithdrawManaged,
                CompanyRead,
                CompanyWrite,
                BillingRead,
                BillingWrite,
            ],
        }
    }

    /// Creates an editable selection, without establishing the actor's identity.
    #[must_use]
    pub fn selection(self) -> PermissionSelection {
        PermissionSelection::new(self.direct_permissions())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod stored_tests;
