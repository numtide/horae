//! Time-read projections, independent of financial and mutation authority.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::EntryState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::people::PeopleCursor;
use super::permission_editor::PermissionRequester;

/// Explicit server policy; a denied canonical read never selects legacy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimesheetPolicy {
    LegacyOwn,
    Scoped,
}

/// The selected subject is distinct from the authenticated requester.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesheetQuery {
    pub subject_id: Option<Uuid>,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub after: Option<TimeEntryCursor>,
    pub expected_requester: Option<PermissionRequester>,
    pub expected_policy: Option<TimesheetPolicy>,
}

/// A subject, its labels and entries admitted under one current authority fence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesheetPage {
    pub requester: PermissionRequester,
    pub subject: TimesheetPerson,
    pub policy: TimesheetPolicy,
    pub entries: Vec<VisibleTimeEntry>,
    pub next_after: Option<TimeEntryCursor>,
}

/// Narrow authorized Timesheet identities, independently of the displayed dates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Timesheet selected-person integration is pending."
    )
)]
pub struct TimesheetPeopleQuery {
    pub search: String,
    pub user_id: Option<Uuid>,
    pub after: Option<PeopleCursor>,
}

/// A navigation label does not grant access to this person's other records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesheetPerson {
    pub id: Uuid,
    pub name: String,
}

/// Each page is reauthorized; these identities are not write capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Timesheet selected-person integration is pending."
    )
)]
pub struct TimesheetPeoplePage {
    pub requester: PermissionRequester,
    pub people: Vec<TimesheetPerson>,
    pub next_after: Option<PeopleCursor>,
}

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
