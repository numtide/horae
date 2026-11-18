//! `GET /health`: the liveness probe. It answers without touching the database.

pub(crate) async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok"}))
}
