//! Nonfinancial report facts; candidate discovery and financial reports are separate.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

/// Narrow ordinary report facts without granting financial or invoice access.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeReportBillability {
    #[default]
    All,
    Billable,
    NonBillable,
}

impl TimeReportBillability {
    #[cfg(feature = "server")]
    pub(crate) fn filter(self) -> Option<bool> {
        match self {
            Self::All => None,
            Self::Billable => Some(true),
            Self::NonBillable => Some(false),
        }
    }
}

/// Exclusive bound in date/project/task/entry order, not an authorization token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeReportCursor {
    pub spent_date: NaiveDate,
    pub project_name: String,
    pub task_name: String,
    pub id: Uuid,
}

/// Empty filter lists leave authorized rows unrestricted along that dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(dead_code, reason = "T203 connects the canonical Reports consumer.")
)]
pub struct TimeReportQuery {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    #[serde(default)]
    pub active_projects_only: bool,
    #[serde(default)]
    pub billability: TimeReportBillability,
    pub client_ids: Vec<Uuid>,
    pub project_ids: Vec<Uuid>,
    pub user_ids: Vec<Uuid>,
    pub task_ids: Vec<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub after: Option<TimeReportCursor>,
    pub expected_requester: Option<PermissionRequester>,
}

/// Effective report rounding and billability, without financial/account fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeReportEntry {
    pub id: Uuid,
    pub spent_date: NaiveDate,
    pub project_name: String,
    pub task_name: String,
    pub user_name: String,
    pub minutes: i32,
    pub rounded_minutes: i32,
    pub billable: bool,
    pub notes: Option<String>,
}

/// Exact time facts for the full authorized period/filter set, before pagination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeReportTotals {
    pub entry_count: i64,
    pub total_minutes: i64,
    pub rounded_minutes: i64,
    /// Effective rounded minutes of billable entries, not a monetary amount.
    pub billable_minutes: i64,
}

/// A bounded page and full-period totals from one snapshot, not filter candidates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(dead_code, reason = "T203 connects the canonical Reports consumer.")
)]
pub struct TimeReportPage {
    pub requester: PermissionRequester,
    pub entries: Vec<TimeReportEntry>,
    pub next_after: Option<TimeReportCursor>,
    pub totals: TimeReportTotals,
}
