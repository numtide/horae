pub mod approval;
pub mod assignment;
pub mod client;
pub mod invoice;
pub mod organization;
pub mod project;
pub mod task;
pub mod time_entry;
pub mod user;

pub use approval::{Approval, ApprovalSummary};
pub use assignment::Assignment;
pub use client::Client;
pub use invoice::{Invoice, InvoiceLine, InvoiceWithLines};
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobStatus {
    pub id: uuid::Uuid,
    pub kind: String,
    pub status: String,
    pub phase: Option<String>,
    pub processed_count: i64,
    pub total_count: Option<i64>,
    pub report: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
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
    /// Billable amount in cents (rates resolved via FR-024) and cost in cents
    /// (`users.cost_rate_cents`). Each row contains only one currency; an entity
    /// with time for clients in different currencies appears in separate rows.
    pub billable_cents: i64,
    pub cost_cents: i64,
    pub currency: String,
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
