//! Time-read projections, independent of financial and mutation authority.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::EntryState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

/// An exclusive descending ordering bound, not a permission snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeEntryCursor {
    pub spent_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

/// Inclusive dates and optional narrowing filters, applied after scope checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Timesheet integration waits for reviewed policy cutover."
    )
)]
pub struct TimeEntryQuery {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub user_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub after: Option<TimeEntryCursor>,
}

/// Authorized entry facts and contextual labels; no rates or invoice metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleTimeEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub project_id: Uuid,
    pub project_name: String,
    pub task_id: Uuid,
    pub task_name: String,
    pub client_id: Uuid,
    pub client_name: String,
    pub spent_date: NaiveDate,
    pub minutes: i32,
    pub rounded_minutes: Option<i32>,
    pub notes: Option<String>,
    pub billable: bool,
    pub is_running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub start_minute: Option<i32>,
    pub sort_order: i32,
    pub state: EntryState,
    pub created_at: DateTime<Utc>,
}

/// Reauthorized on every page. Exhaust pages before presenting period totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Timesheet integration waits for reviewed policy cutover."
    )
)]
pub struct TimeEntryPage {
    pub requester: PermissionRequester,
    pub entries: Vec<VisibleTimeEntry>,
    pub next_after: Option<TimeEntryCursor>,
}
