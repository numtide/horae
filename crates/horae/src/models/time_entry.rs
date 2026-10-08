use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::EntryState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A project/task pair on which the session user may record new time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeEntryContext {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub billable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct TimeEntry {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    /// Precise tracked minutes.
    pub minutes: i32,
    /// Persisted at submit time; None until the entry is locked.
    pub rounded_minutes: Option<i32>,
    pub notes: Option<String>,
    pub billable: bool,
    pub is_running: bool,
    /// Non-null only while is_running = true.
    pub started_at: Option<DateTime<Utc>>,
    /// Optional start time as minutes since local midnight (0..=1439). `None` =
    /// untimed (duration-only); the calendar stacks those from the top of the day.
    pub start_minute: Option<i32>,
    /// Explicit order among a day's untimed entries (calendar reordering). Timed
    /// entries ignore it (they order by start time). Defaults to 0.
    pub sort_order: i32,
    pub state: EntryState,
    /// Internal billing relation; time-entry responses never grant invoice access.
    #[serde(skip)]
    pub invoice_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn billed_entry() -> TimeEntry {
        let now = Utc::now();
        TimeEntry {
            id: Uuid::now_v7(),
            org_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            task_id: Uuid::now_v7(),
            spent_date: now.date_naive(),
            minutes: 90,
            rounded_minutes: Some(90),
            notes: Some("Work notes".into()),
            billable: true,
            is_running: false,
            started_at: None,
            start_minute: Some(540),
            sort_order: 2,
            state: EntryState::Invoiced,
            invoice_id: Some(Uuid::now_v7()),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn time_entry_wire_omits_invoice_identity_without_mutating_internal_record() {
        let entry = billed_entry();
        let invoice = entry.invoice_id;
        let value = serde_json::to_value(&entry).unwrap();
        assert!(value.get("invoice_id").is_none());
        assert_eq!(entry.invoice_id, invoice);
        let mut decoded: TimeEntry = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.invoice_id, None);
        decoded.invoice_id = invoice;
        assert_eq!(decoded, entry);
    }

    #[test]
    fn time_entry_wire_does_not_accept_invoice_identity_from_input() {
        let entry = billed_entry();
        let mut value = serde_json::to_value(&entry).unwrap();
        value["invoice_id"] = serde_json::json!(Uuid::now_v7());
        let decoded: TimeEntry = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.invoice_id, None);
        assert_eq!(decoded.state, EntryState::Invoiced);
    }
}
