//! Borrowed parent fields shared by catalog imports and denormalized time rows.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::importers::harvest::types::SourceRow;

pub struct ClientFields<'a> {
    pub harvest_client_id: Option<i64>,
    pub client_name: &'a str,
    pub client_address: Option<&'a str>,
    pub client_active: bool,
    pub currency: Option<&'a str>,
    pub harvest_updated_at: Option<DateTime<Utc>>,
}

pub struct ProjectFields<'a> {
    pub harvest_project_id: Option<i64>,
    pub client_name: &'a str,
    pub project_name: &'a str,
    pub project_code: Option<&'a str>,
    pub project_active: bool,
    pub project_starts_on: Option<NaiveDate>,
    pub project_ends_on: Option<NaiveDate>,
    pub harvest_updated_at: Option<DateTime<Utc>>,
}

pub struct TaskFields<'a> {
    pub harvest_task_id: Option<i64>,
    pub task_name: &'a str,
    pub task_billable_default: bool,
    pub task_active: bool,
    pub billable_rate: Option<&'a str>,
    pub harvest_updated_at: Option<DateTime<Utc>>,
}

impl<'a> From<&'a SourceRow> for ClientFields<'a> {
    fn from(row: &'a SourceRow) -> Self {
        Self {
            harvest_client_id: row.harvest_client_id,
            client_name: &row.client_name,
            client_address: row.client_address.as_deref(),
            client_active: row.client_active,
            currency: row.currency.as_deref(),
            harvest_updated_at: row.harvest_updated_at,
        }
    }
}

impl<'a> From<&'a SourceRow> for ProjectFields<'a> {
    fn from(row: &'a SourceRow) -> Self {
        Self {
            harvest_project_id: row.harvest_project_id,
            client_name: &row.client_name,
            project_name: &row.project_name,
            project_code: row.project_code.as_deref(),
            project_active: row.project_active,
            project_starts_on: row.project_starts_on,
            project_ends_on: row.project_ends_on,
            harvest_updated_at: row.harvest_updated_at,
        }
    }
}

impl<'a> From<&'a SourceRow> for TaskFields<'a> {
    fn from(row: &'a SourceRow) -> Self {
        Self {
            harvest_task_id: row.harvest_task_id,
            task_name: &row.task_name,
            task_billable_default: row.task_billable_default,
            task_active: true,
            billable_rate: row.billable_rate.as_deref(),
            harvest_updated_at: row.harvest_updated_at,
        }
    }
}
