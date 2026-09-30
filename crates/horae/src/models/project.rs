use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::{BudgetKind, ProjectType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Inclusive reporting dates; an absent interval requests all recorded work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectActivityInterval {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Authorized tracked-time series, without people, notes, rates or costs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectActivity {
    /// Absent only for an all-time request with no recorded entries.
    pub interval: Option<ProjectActivityInterval>,
    pub week_start: chrono::Weekday,
    pub weeks: Vec<ProjectActivityWeek>,
}

/// A week clipped to the selected reporting interval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectActivityWeek {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub billable_minutes: i64,
    pub non_billable_minutes: i64,
    /// Cumulative tracked minutes since the selected interval began.
    pub cumulative_minutes: i64,
}

/// Lifetime tracked work and independently scoped budget/cost summaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub total_minutes: i64,
    pub billable_minutes: i64,
    pub non_billable_minutes: i64,
    pub configured_budget: bool,
    pub budgets: Vec<ProjectBudgetProgress>,
    pub budget_totals: horae_core::budget::BudgetTotals,
    /// Entirely absent when the viewer cannot read every contributing cost.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_costs: Option<ProjectCostSummary>,
}

/// Current cost rates applied per entry to actual minutes, in workspace currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectCostSummary {
    pub currency: String,
    /// Missing rates make the total unknown, not zero or a partial sum.
    pub total_cents: Option<i64>,
    pub missing_rate_minutes: i64,
}

/// Identity-only reporting dimension, including zero-time and historical rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct ProjectWorkEntity {
    pub id: Uuid,
    pub name: String,
    pub active: bool,
    /// Currently enabled task or assigned person, independent of past work.
    pub current: bool,
    pub manager: bool,
}

/// One authorized reporting snapshot used by both reciprocal breakdown views.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectBreakdown {
    pub interval: Option<ProjectActivityInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_currency: Option<String>,
    pub tasks: Vec<ProjectWorkEntity>,
    pub people: Vec<ProjectWorkEntity>,
    pub cells: Vec<horae_core::project_breakdown::WorkCell>,
    pub totals: horae_core::project_breakdown::WorkSummary,
}

/// One invoice's stored net contribution to this project, never its full total.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct ProjectInvoice {
    pub id: Uuid,
    pub number: String,
    pub status: horae_core::types::InvoiceStatus,
    pub issued_on: NaiveDate,
    pub currency: String,
    pub net_before_tax_cents: i64,
}

/// All invoice history, independent of the chart's work-date interval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInvoices {
    pub invoices: Vec<ProjectInvoice>,
    pub totals: std::collections::BTreeMap<String, horae_core::invoice::InvoiceLedgerTotals>,
}

/// An explicit task rate in the currency shown to the project manager.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Retained for the task-linking server-function API"
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

/// Saved basic project details; private notes are absent for non-administrators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDetails {
    pub id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub client_id: Uuid,
    pub client_name: String,
    pub project_type: ProjectType,
    pub active: bool,
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

/// A tag associated with a project whose progress the caller may read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTagLink {
    pub project_id: Uuid,
    pub tag_id: Uuid,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_wire_format_preserves_dates_weekday_and_exact_minutes() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
        let activity = ProjectActivity {
            interval: Some(ProjectActivityInterval {
                from: date,
                to: date,
            }),
            week_start: chrono::Weekday::Sun,
            weeks: vec![ProjectActivityWeek {
                from: date,
                to: date,
                billable_minutes: i64::MAX - 1,
                non_billable_minutes: 1,
                cumulative_minutes: i64::MAX,
            }],
        };
        let json = serde_json::to_string(&activity).unwrap();
        assert_eq!(
            serde_json::from_str::<ProjectActivity>(&json).unwrap(),
            activity
        );
    }

    #[test]
    fn activity_wire_format_preserves_empty_all_time() {
        let activity = ProjectActivity {
            interval: None,
            week_start: chrono::Weekday::Mon,
            weeks: Vec::new(),
        };
        let json = serde_json::to_string(&activity).unwrap();
        assert_eq!(
            serde_json::from_str::<ProjectActivity>(&json).unwrap(),
            activity
        );
    }
}
