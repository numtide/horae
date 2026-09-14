#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JobStatus {
    pub id: uuid::Uuid,
    pub kind: String,
    pub status: String,
    pub phase: Option<String>,
    pub processed_count: i64,
    pub total_count: Option<i64>,
    pub report: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl JobStatus {
    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running")
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.status.as_str(), "failed" | "cancelled")
    }
}
