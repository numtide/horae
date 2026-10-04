//! Nonfinancial report facts; candidate discovery and financial reports are separate.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

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

/// A bounded result page, not a complete-period summary or filter candidate list.
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
}
