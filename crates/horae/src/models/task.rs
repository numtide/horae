use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Explicit intent for a task's global default, independent of project overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskRateEdit {
    Preserve {},
    Clear {},
    Set { amount_cents: i64, currency: String },
}

/// An org-level task (global catalog entry). Tasks are enabled per project via
/// `project_tasks`; a task row here does NOT belong to a specific project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
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
                json!({"action":"set","amount_cents":0,"currency":"EUR"}),
                TaskRateEdit::Set {
                    amount_cents: 0,
                    currency: "EUR".into(),
                },
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
            json!({"action":"set","amount_cents":8000}),
            json!({"action":"set","amount_cents":1.5,"currency":"EUR"}),
        ] {
            assert!(
                serde_json::from_value::<TaskRateEdit>(value.clone()).is_err(),
                "{value}"
            );
        }
    }
}
