//! Harvest v2 JSON shapes (only the fields the importer consumes).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiClient {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRef {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProject {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub client: ApiRef,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTask {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub billable_by_default: bool,
    #[serde(default = "yes")]
    pub is_active: bool,
    /// Harvest's common task flag.
    #[serde(default)]
    pub is_default: bool,
    pub default_hourly_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUser {
    pub id: i64,
    /// Missing on some records; such a person is reported, never guessed.
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
}

impl ApiUser {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiTimeEntry {
    pub id: i64,
    pub spent_date: NaiveDate,
    pub hours: serde_json::Number,
    pub notes: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub is_billed: bool,
    #[serde(default)]
    pub is_running: bool,
    pub client: Option<ApiRef>,
    pub project: ApiRef,
    pub task: ApiRef,
    pub user: ApiRef,
    pub billable_rate: Option<serde_json::Number>,
    pub cost_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// serde default for Harvest's `is_active` / `billable_by_default` flags, which
/// are `true` unless the record says otherwise.
fn yes() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_flags_default_to_harvests_values() {
        let task: ApiTask = serde_json::from_str(r#"{"id":1,"name":"Design"}"#).unwrap();
        assert!(task.billable_by_default && task.is_active);
        let user: ApiUser = serde_json::from_str(r#"{"id":2,"first_name":"Dana"}"#).unwrap();
        assert!(user.is_active);
        assert_eq!(user.email, None);
        assert_eq!(user.full_name(), "Dana");
        let entry: ApiTimeEntry = serde_json::from_str(
            r#"{"id":3,"spent_date":"2026-01-15","hours":1.5,"notes":null,
                "project":{"id":10},"task":{"id":100},"user":{"id":1000}}"#,
        )
        .unwrap();
        assert!(!entry.billable && !entry.is_billed && !entry.is_running);
        assert_eq!(entry.hours.to_string(), "1.5");
    }
}
