//! The public types of `/api/v1` (research R-8). They belong to the contract,
//! so they change only by additions (FR-050); the conversions from
//! `horae-types` and `ops` are spelled out here, so a change to the web app's
//! shapes cannot reach the API unnoticed.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::money::{MinorUnitsError, from_iso_minor, to_iso_minor};
use horae_core::permissions::oauth::Scope;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::own_permissions::OwnPermissions;
use crate::models::scoped_time::TimesheetTrackingOption;
use crate::models::time_entry::TimeEntry as StoredEntry;
use crate::ops::api_tokens::TokenIdentity;
use crate::ops::clients::ClientRecord;
use crate::ops::oauth::tokens::OAuthIdentity;
use crate::ops::people::PersonRecord;
use crate::ops::projects::reads::ProjectRecord;
use crate::ops::projects::task_catalog::TaskRecord;
use crate::problem::Problem;

/// One page of a list, in `id` order. `next_cursor` is `null` on the last
/// page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
}

/// An amount of money: an integer in the ISO 4217 minor unit of its currency.
/// EUR 12.10 is `1210`, JPY 8000 (no minor unit) is `8000` and BHD 10.500
/// (three decimals) is `10500`. Every amount in this API is a `Money`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(examples(
    json!({ "amount_minor": 1210, "currency": "EUR" }),
    json!({ "amount_minor": 8000, "currency": "JPY" }),
    json!({ "amount_minor": 10500, "currency": "BHD" }),
))]
pub struct Money {
    amount_minor: i64,
    #[schema(pattern = "^[A-Z]{3}$")]
    currency: String,
}

// Storage keeps hundredths for every currency; these are the API's only
// conversions to and from them.
impl Money {
    /// The `Money` of an amount stored in hundredths. Errors, never rounds,
    /// when the amount has no exact value in the currency's minor unit.
    pub fn from_stored(hundredths: i64, currency: &str) -> Result<Self, MinorUnitsError> {
        Ok(Self {
            amount_minor: to_iso_minor(hundredths, currency)?,
            currency: currency.to_owned(),
        })
    }

    /// The hundredths to store for a received amount, or a 422 naming
    /// `field` when storage cannot hold it exactly.
    pub fn to_stored(&self, field: &str) -> Result<i64, Problem> {
        from_iso_minor(self.amount_minor, &self.currency).map_err(|error| {
            Problem::invalid(
                field,
                format!("{} {}: {error}", self.amount_minor, self.currency),
            )
        })
    }
}

/// A stored amount `/api/v1` cannot serve exactly: a 500, logged with the row
/// it came from, rather than a rounded amount.
fn inexact_stored(row: Uuid, column: &str, error: MinorUnitsError) -> Problem {
    tracing::error!(%row, column, %error, "stored amount has no exact ISO 4217 minor-unit value");
    Problem::internal()
}

/// Who the token acts as, and in which organization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Me {
    #[schema(inline)]
    pub person: MePerson,
    #[schema(inline)]
    pub organization: MeOrganization,
    /// Whether the token acts as an administrator; never for an app's token.
    pub administrator: bool,
    /// The person's profile or permission template; `null` for individually
    /// chosen permissions. For an app's token it is still the person's,
    /// though the token is limited to its scopes.
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
    /// The personal access token, or the OAuth access token.
    pub id: Uuid,
    /// The personal access token's name, or the app's name.
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub kind: TokenKind,
    /// The app and its granted scopes; `null` for a personal access token.
    #[schema(inline, required = true)]
    pub oauth: Option<MeOAuth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Personal,
    #[serde(rename = "oauth")]
    OAuth,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MeOAuth {
    /// The app's public client id.
    pub client_id: String,
    pub app_name: String,
    pub scopes: Vec<OAuthScope>,
}

/// A scope an app may be granted (spec 024 FR-060).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum OAuthScope {
    #[serde(rename = "time:read")]
    TimeRead,
    #[serde(rename = "time:write")]
    TimeWrite,
    #[serde(rename = "catalog:read")]
    CatalogRead,
}

impl From<Scope> for OAuthScope {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::TimeRead => Self::TimeRead,
            Scope::TimeWrite => Self::TimeWrite,
            Scope::CatalogRead => Self::CatalogRead,
        }
    }
}

impl Me {
    /// Who an app's OAuth access token acts as.
    pub fn from_oauth(org_id: Uuid, identity: OAuthIdentity, permissions: OwnPermissions) -> Self {
        Self {
            person: MePerson {
                id: identity.person_id,
                name: identity.person_name,
                email: identity.person_email,
            },
            organization: MeOrganization {
                id: org_id,
                name: identity.organization_name,
            },
            administrator: permissions.is_administrator,
            profile: permissions.profile,
            token: MeToken {
                id: identity.token_id,
                name: identity.app_name.clone(),
                expires_at: Some(identity.expires_at),
                kind: TokenKind::OAuth,
                oauth: Some(MeOAuth {
                    client_id: identity.client_id,
                    app_name: identity.app_name,
                    scopes: identity.scopes.iter().map(OAuthScope::from).collect(),
                }),
            },
        }
    }

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
                kind: TokenKind::Personal,
                oauth: None,
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

/// How a project bills its time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    TimeAndMaterials,
    FixedFee,
    NonBillable,
    Retainer,
}

impl From<horae_core::types::ProjectType> for ProjectType {
    fn from(kind: horae_core::types::ProjectType) -> Self {
        use horae_core::types::ProjectType as Stored;
        match kind {
            Stored::TimeAndMaterials => Self::TimeAndMaterials,
            Stored::FixedFee => Self::FixedFee,
            Stored::NonBillable => Self::NonBillable,
            Stored::Retainer => Self::Retainer,
        }
    }
}

/// What a project's budget counts: hours, money, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Hours,
    Amount,
    None,
}

impl From<horae_core::types::BudgetKind> for BudgetKind {
    fn from(kind: horae_core::types::BudgetKind) -> Self {
        use horae_core::types::BudgetKind as Stored;
        match kind {
            Stored::Hours => Self::Hours,
            Stored::Amount => Self::Amount,
            Stored::None => Self::None,
        }
    }
}

/// A project the token's owner may see on the Projects page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Project {
    pub id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    #[schema(pattern = "^[A-Z]{3}$")]
    pub currency: String,
    pub project_type: ProjectType,
    pub budget_kind: BudgetKind,
    /// An hours budget in minutes; absent for other budgets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub budget_minutes: Option<i64>,
    /// A money budget; absent when unset or when the owner may not read
    /// this project's rates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub budget_amount: Option<Money>,
    /// The project's hourly rate; absent when unset or when the owner may not
    /// read this project's rates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub hourly_rate: Option<Money>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRecord> for Project {
    type Error = Problem;

    fn try_from(project: ProjectRecord) -> Result<Self, Problem> {
        let money = |hundredths: Option<i64>, column| {
            hundredths
                .map(|hundredths| Money::from_stored(hundredths, &project.currency))
                .transpose()
                .map_err(|error| inexact_stored(project.id, column, error))
        };
        Ok(Self {
            id: project.id,
            client_id: project.client_id,
            budget_amount: money(project.budget_amount_cents, "budget_amount_cents")?,
            hourly_rate: money(project.rate_cents, "rate_cents")?,
            name: project.name,
            code: project.code,
            project_type: project.project_type.into(),
            budget_kind: project.budget_kind.into(),
            budget_minutes: project.budget_minutes,
            starts_on: project.starts_on,
            ends_on: project.ends_on,
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
            currency: project.currency,
        })
    }
}

/// A task of the catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    /// Whether time on the task is billable unless a project says otherwise.
    pub billable_default: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TaskRecord> for Task {
    fn from(task: TaskRecord) -> Self {
        Self {
            id: task.id,
            name: task.name,
            billable_default: task.billable_default,
            active: task.active,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

/// A client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    #[schema(pattern = "^[A-Z]{3}$")]
    pub currency: String,
    /// Absent when unset or when the owner may not read the client catalogue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub address: Option<String>,
    /// Absent when unset or when the owner may not read the client catalogue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub tax_id: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ClientRecord> for Client {
    fn from(client: ClientRecord) -> Self {
        Self {
            id: client.id,
            name: client.name,
            currency: client.currency,
            address: client.address,
            tax_id: client.tax_id,
            active: client.active,
            created_at: client.created_at,
            updated_at: client.updated_at,
        }
    }
}

/// A person of the organization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Person {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PersonRecord> for Person {
    fn from(person: PersonRecord) -> Self {
        Self {
            id: person.id,
            name: person.name,
            email: person.email,
            active: person.active,
            created_at: person.created_at,
            updated_at: person.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::json;

    use super::Money;

    #[test]
    fn a_received_amount_is_stored_in_hundredths_or_refused_naming_its_field() {
        let money = |value| serde_json::from_value::<Money>(value).unwrap();
        let bhd = money(json!({ "amount_minor": 10_500, "currency": "BHD" }));
        assert_eq!(bhd.to_stored("rate"), Ok(1_050));
        let jpy = money(json!({ "amount_minor": 8_000, "currency": "JPY" }));
        assert_eq!(jpy.to_stored("rate"), Ok(800_000));
        let problem = money(json!({ "amount_minor": 10_505, "currency": "BHD" }))
            .to_stored("hourly_rate")
            .unwrap_err();
        assert_eq!(problem.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(problem.field.as_deref(), Some(&json!("hourly_rate")));
    }
}
