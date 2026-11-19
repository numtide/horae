//! The public types of `/api/v1` (research R-8). They belong to the contract,
//! so they change only by additions (FR-050); the conversions from
//! `horae-types` and `ops` are spelled out here, so a change to the web app's
//! shapes cannot reach the API unnoticed.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::own_permissions::OwnPermissions;
use crate::models::scoped_time::TimesheetTrackingOption;
use crate::models::time_entry::TimeEntry as StoredEntry;
use crate::ops::api_tokens::TokenIdentity;

/// One page of a list, in `id` order. `next_cursor` is `null` on the last
/// page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
}

/// An amount of money: an integer in minor units (cents) and its ISO 4217
/// currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Money {
    pub amount_minor: i64,
    #[schema(pattern = "^[A-Z]{3}$")]
    pub currency: String,
}

/// Who the token acts as, and in which organization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Me {
    #[schema(inline)]
    pub person: MePerson,
    #[schema(inline)]
    pub organization: MeOrganization,
    pub administrator: bool,
    /// The person's profile or permission template; `null` for individually
    /// chosen permissions.
    pub profile: Option<String>,
    #[schema(inline)]
    pub token: MeToken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MePerson {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MeOrganization {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MeToken {
    pub id: Uuid,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Me {
    pub fn new(org_id: Uuid, identity: TokenIdentity, permissions: OwnPermissions) -> Self {
        let token = identity.token;
        Self {
            person: MePerson {
                id: token.user_id,
                name: token.owner_name,
                email: identity.person_email,
            },
            organization: MeOrganization {
                id: org_id,
                name: identity.organization_name,
            },
            administrator: permissions.is_administrator,
            profile: permissions.profile,
            token: MeToken {
                id: token.id,
                name: token.name,
                expires_at: token.expires_at,
            },
        }
    }
}

/// A project and task the person may track time on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Trackable {
    pub project_id: Uuid,
    pub project_name: String,
    pub task_id: Uuid,
    pub task_name: String,
    /// Whether time on this pair is billable.
    pub billable: bool,
}

impl From<TimesheetTrackingOption> for Trackable {
    fn from(option: TimesheetTrackingOption) -> Self {
        Self {
            project_id: option.project_id,
            project_name: option.project_name,
            task_id: option.task_id,
            task_name: option.task_name,
            billable: option.billable,
        }
    }
}

/// Where an entry is in its life: editable, submitted for approval, approved,
/// or on an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Open,
    Submitted,
    Approved,
    Invoiced,
}

impl From<horae_core::types::EntryState> for EntryState {
    fn from(state: horae_core::types::EntryState) -> Self {
        use horae_core::types::EntryState as Stored;
        match state {
            Stored::Open => Self::Open,
            Stored::Submitted => Self::Submitted,
            Stored::Approved => Self::Approved,
            Stored::Invoiced => Self::Invoiced,
        }
    }
}

/// Time a person spent on a project's task on one day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TimeEntry {
    pub id: Uuid,
    pub person_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    /// The person's calendar day, as in the timesheet.
    pub spent_date: NaiveDate,
    /// Tracked minutes; a running timer's minutes before it started.
    #[schema(minimum = 0)]
    pub minutes: i32,
    /// The rounded minutes, set when the entry is submitted.
    pub rounded_minutes: Option<i32>,
    pub notes: Option<String>,
    pub billable: bool,
    pub state: EntryState,
    pub is_running: bool,
    /// When the running timer started; `null` when it is not running.
    pub timer_started_at: Option<DateTime<Utc>>,
    /// Start time as minutes after the day's midnight; `null` for an entry
    /// with only a duration.
    #[schema(minimum = 0, maximum = 1439)]
    pub start_minute: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A time entry with a duration. Unknown fields are refused.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewTimeEntry {
    /// Whose time; the token's owner when absent. Another person needs the
    /// owner's write scope over them.
    pub person_id: Option<Uuid>,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    #[schema(minimum = 0)]
    pub minutes: i32,
    #[serde(default)]
    pub notes: Option<String>,
    /// Whether the time is billable; the task's eligibility still applies.
    #[serde(default = "billable_by_default")]
    #[schema(default = true)]
    pub billable: bool,
    #[serde(default)]
    #[schema(minimum = 0, maximum = 1439)]
    pub start_minute: Option<i32>,
}

fn billable_by_default() -> bool {
    true
}

/// Changes to an entry: an absent field keeps its value, and `null` clears
/// `notes` or `start_minute`. Unknown fields are refused.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimeEntryPatch {
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub spent_date: Option<NaiveDate>,
    #[schema(minimum = 0)]
    pub minutes: Option<i32>,
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<String>)]
    pub notes: Option<Option<String>>,
    pub billable: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<i32>, minimum = 0, maximum = 1439)]
    pub start_minute: Option<Option<i32>>,
}

/// A field that is present, even as `null`; an absent one stays `None`.
fn present<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl From<TimeEntryPatch> for crate::ops::time::EntryPatch {
    fn from(patch: TimeEntryPatch) -> Self {
        Self {
            project_id: patch.project_id,
            task_id: patch.task_id,
            spent_date: patch.spent_date,
            minutes: patch.minutes,
            notes: patch.notes,
            billable: patch.billable,
            start_minute: patch.start_minute,
        }
    }
}

/// A timer to start now, on the person's today. Unknown fields are refused.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NewTimer {
    /// Whose timer; the token's owner when absent.
    pub person_id: Option<Uuid>,
    pub project_id: Uuid,
    pub task_id: Uuid,
    #[serde(default)]
    pub notes: Option<String>,
}

impl From<StoredEntry> for TimeEntry {
    fn from(entry: StoredEntry) -> Self {
        Self {
            id: entry.id,
            person_id: entry.user_id,
            project_id: entry.project_id,
            task_id: entry.task_id,
            spent_date: entry.spent_date,
            minutes: entry.minutes,
            rounded_minutes: entry.rounded_minutes,
            notes: entry.notes,
            billable: entry.billable,
            state: entry.state.into(),
            is_running: entry.is_running,
            timer_started_at: entry.started_at,
            start_minute: entry.start_minute,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
        }
    }
}
