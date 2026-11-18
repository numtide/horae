//! Harvest-shaped error responses (specs/021-harvest-api-compat/contracts/harvest-api-v2.md,
//! *Error bodies*). Failed authentication answers with `error` and
//! `error_description`, like an OAuth2 resource server; other refusals with
//! `message`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::ops::OpError;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "Not found")
    }

    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    /// Logged in full; the client only learns that the server failed.
    pub fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!("Harvest API error: {error}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
    }
}

/// A refusal from a shared Horae operation keeps its reason (FR-024) and maps
/// its status: validation becomes Harvest's 422, the rest keep their code.
impl From<OpError> for ApiError {
    fn from(error: OpError) -> Self {
        let status = match error.kind() {
            OpError::Invalid { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            OpError::NotFound(_) => StatusCode::NOT_FOUND,
            OpError::Forbidden(_) => StatusCode::FORBIDDEN,
            OpError::Conflict(_) => StatusCode::CONFLICT,
            OpError::Internal(_) | OpError::Detailed { .. } => return Self::internal(error),
        };
        Self::new(status, error.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = match self.status {
            StatusCode::UNAUTHORIZED => {
                json!({ "error": "invalid_token", "error_description": self.message })
            }
            StatusCode::FORBIDDEN => {
                json!({ "error": "forbidden", "error_description": self.message })
            }
            StatusCode::NOT_FOUND => {
                json!({ "error": "not_found", "error_description": self.message })
            }
            _ => json!({ "message": self.message }),
        };
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn body(error: ApiError) -> (StatusCode, serde_json::Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn authentication_failures_use_the_oauth_shape() {
        let (status, body) = body(ApiError::unauthorized("Not authenticated")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            body,
            json!({ "error": "invalid_token", "error_description": "Not authenticated" })
        );
    }

    #[tokio::test]
    async fn validation_failures_carry_a_message() {
        let (status, body) = body(ApiError::unprocessable("bad per_page")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "message": "bad per_page" }));
    }

    #[tokio::test]
    async fn internal_errors_hide_their_cause() {
        let (status, body) = body(ApiError::internal("relation does not exist")).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, json!({ "message": "Internal error" }));
    }

    #[test]
    fn a_horae_refusal_keeps_its_reason_and_maps_bad_request_to_422() {
        let validation = ApiError::from(OpError::bad_input("Daily limit reached"));
        assert_eq!(validation.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(validation.message, "Daily limit reached");
        let field = ApiError::from(OpError::bad_input("Bad").with_field("name"));
        assert_eq!(
            (field.status, field.message.as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, "Bad")
        );
        let refusals = [
            (OpError::not_found("Gone"), StatusCode::NOT_FOUND),
            (OpError::forbidden("No"), StatusCode::FORBIDDEN),
            (OpError::conflict("Changed"), StatusCode::CONFLICT),
        ];
        for (error, status) in refusals {
            let message = error.to_string();
            let answer = ApiError::from(error);
            assert_eq!((answer.status, answer.message), (status, message));
        }
        let internal = ApiError::from(OpError::Internal("relation does not exist".into()));
        assert_eq!(
            (internal.status, internal.message.as_str()),
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
        );
    }
}
