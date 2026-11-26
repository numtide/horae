use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskActivity {
    Active,
    Archived,
    All,
}

/// Exclusive ordering bound, not a saved authorization decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCursor {
    /// The catalog lists common tasks before the others.
    pub is_default: bool,
    pub name: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCatalogEntry {
    pub id: Uuid,
    pub name: String,
    pub billable_default: bool,
    /// A common task: the new-project form adds it to the task list.
    pub is_default: bool,
    pub active: bool,
    /// The rate template: hundredths with no currency, which a project applies
    /// in its own currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_rate_cents: Option<i64>,
}

/// Rows and affordances share the same current authorization transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCatalogPage {
    pub requester: PermissionRequester,
    pub can_edit: bool,
    pub can_read_rates: bool,
    pub can_edit_rates: bool,
    pub tasks: Vec<TaskCatalogEntry>,
    pub next_after: Option<TaskCursor>,
}

/// Explicit intent for a task's global default, independent of project overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskRateEdit {
    Preserve {},
    Clear {},
    /// A rate template in hundredths, with no currency.
    Set {
        amount_cents: i64,
    },
}

/// An org-level task (global catalog entry). Tasks are enabled per project via
/// `project_tasks`; a task row here does NOT belong to a specific project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub billable_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_rate_cents: Option<i64>,
    pub active: bool,
}

#[cfg(test)]
mod tests {
    use super::TaskRateEdit;
    use serde_json::json;

    #[test]
    fn rate_transport_distinguishes_preserve_clear_and_zero() {
        for (value, expected) in [
            (json!({"action":"preserve"}), TaskRateEdit::Preserve {}),
            (json!({"action":"clear"}), TaskRateEdit::Clear {}),
            (
                json!({"action":"set","amount_cents":8000}),
                TaskRateEdit::Set { amount_cents: 8000 },
            ),
        ] {
            assert_eq!(
                serde_json::from_value::<TaskRateEdit>(value.clone()).unwrap(),
                expected
            );
            assert_eq!(serde_json::to_value(expected).unwrap(), value);
        }
    }

    #[test]
    fn rate_transport_rejects_ambiguous_or_imprecise_intent() {
        for value in [
            json!(null),
            json!({}),
            json!({"action":"unknown"}),
            json!({"action":"preserve","amount_cents":8000}),
            json!({"action":"set","amount_cents":8000,"currency":"EUR"}),
            json!({"action":"set"}),
            json!({"action":"set","amount_cents":1.5}),
        ] {
            assert!(
                serde_json::from_value::<TaskRateEdit>(value.clone()).is_err(),
                "{value}"
            );
        }
    }
}
