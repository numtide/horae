use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::{BudgetKind, ProjectType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An explicit task rate in the currency shown to the project manager.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "The task-link endpoint retains this wire DTO; the project editor uses ProjectForm."
    )
)]
pub struct ProjectTaskRate {
    pub amount: String,
    pub currency: String,
}

/// Read-only fee context for authorized billing managers, one row per occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFeeBalance {
    pub period_key: String,
    pub description: String,
    pub currency: String,
    pub balance: super::invoice::InvoiceFeeBalance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub client_id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub project_type: ProjectType,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_cents: Option<i64>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub budget_kind: BudgetKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_amount_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_minutes: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

/// Minimal client context for a visible project, not access to its directory record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectOverviewClient {
    pub id: Uuid,
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectOverviewRow {
    pub project: Project,
    pub client: Option<ProjectOverviewClient>,
    /// Display affordance only; every edit reloads current server authority.
    pub can_edit: bool,
}

/// Rows and workflow labels bound to one authenticated requester and policy mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectOverview {
    pub requester: super::permission_editor::PermissionRequester,
    pub canonical_permissions: bool,
    pub can_create: bool,
    pub can_import: bool,
    pub can_change_legacy_status: bool,
    pub projects: Vec<ProjectOverviewRow>,
}

/// Authorized budget consumption, without entry details or billing/cost rates.
/// A missing budget is unallocated, not a zero allowance. Legacy projects have
/// no configured progress rows and retain their lifetime overview totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectBudgetProgress {
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub scope: String,
    pub label: Option<String>,
    pub kind: BudgetKind,
    pub currency: String,
    pub period_key: String,
    pub budget: Option<i64>,
    pub consumed: i64,
}

/// Whole-project summary computed before filtering private task/person scopes.
/// Withheld money is absent, not zero; breakdown contains only allowed scopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectBudgetOverview {
    pub project_id: Uuid,
    pub scope: String,
    pub kind: BudgetKind,
    pub currency: String,
    pub period_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed: Option<i64>,
    pub breakdown: Vec<ProjectBudgetProgress>,
}

/// Saved basic project details; private notes are absent for non-administrators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetails {
    pub id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub client_name: String,
    pub currency: String,
    /// Present only when this manager can supply rates for new task links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_rate_currency: Option<String>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_notes: Option<String>,
}

/// A label within a readable project, not a directory record or financial value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetailIdentity {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetailView {
    pub requester: super::permission_editor::PermissionRequester,
    pub canonical_permissions: bool,
    pub project: ProjectDetails,
    /// Display affordance; the editor independently reloads its current authority.
    pub can_edit: bool,
    pub team: Vec<ProjectDetailIdentity>,
    pub tasks: Vec<ProjectDetailIdentity>,
}

/// A tag associated with a project whose progress the caller may read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTagLink {
    pub project_id: Uuid,
    pub tag_id: Uuid,
    pub name: String,
}
