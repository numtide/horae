use chrono::{DateTime, Utc};
use horae_core::types::ProjectRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Assignment endpoints retain this wire DTO; the project editor uses ProjectForm."
    )
)]
pub struct Assignment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: ProjectRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_cents: Option<i64>,
    pub created_at: DateTime<Utc>,
}
