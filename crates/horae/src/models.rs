pub mod approval;
pub mod assignment;
pub mod client;
pub mod invoice;
mod jobs;
pub mod organization;
pub mod project;
pub mod project_creation;
pub mod task;
pub mod time_entry;
pub mod user;

pub use approval::{Approval, ApprovalSummary};
pub use assignment::Assignment;
pub use client::Client;
#[cfg(feature = "server")]
pub use invoice::InvoiceLine;
pub use invoice::{Invoice, InvoiceWithLines};
pub use jobs::{JobStatus, RetryAvailability};
pub use organization::OrgBranding;
pub use project::Project;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use user::User;

/// Per-project tracked totals: all logged minutes, and the billable amount in
/// cents (rates resolved via FR-024). Powers the Projects overview's Spent column.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSpend {
    pub project_id: uuid::Uuid,
    pub spent_minutes: i64,
    pub spent_cents: i64,
}

// ── Report DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportRow {
    /// ID of the project, task, client or person selected as the dimension.
    /// Together with `currency`, this identifies the row independently of its label.
    pub group_id: uuid::Uuid,
    pub label: String,
    pub total_minutes: i64,
    pub rounded_minutes: i64,
    pub billable_minutes: i64,
    /// Billing is partitioned by currency; cost has its own organization currency.
    pub billable_cents: i64,
    /// Absent if any entry in the group has an administrator-only cost override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_cents: Option<i64>,
    pub currency: String,
    pub cost_currency: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct DetailedReportRow {
    pub spent_date: chrono::NaiveDate,
    pub project_name: String,
    pub task_name: String,
    pub user_name: String,
    pub minutes: i32,
    pub rounded_minutes: Option<i32>,
    pub billable: bool,
    pub notes: Option<String>,
}
