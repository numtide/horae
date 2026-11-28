//! Errors of `/api/v1` as `application/problem+json` (RFC 9457, research R-4).
//! [`Problem::from`] is the one mapping from [`OpError`] to a status and a
//! stable `code`; `detail` is the operation's message, the text the web app
//! shows for the same refusal.

use axum::extract::{FromRequest, FromRequestParts, Path, Query, Request};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ops::OpError;

pub const CONTENT_TYPE: &str = "application/problem+json";

/// An error answer. Clients branch on `code` and treat an unknown one as its
/// `status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    /// Always `about:blank`: the status and `code` say what happened.
    #[serde(rename = "type")]
    pub kind: String,
    /// The status's reason phrase.
    pub title: String,
    pub status: u16,
    /// Why, in the words the web app uses. Absent for internal errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A stable, snake-case code for the kind of refusal.
    pub code: String,
    /// The input to blame, when one is: a name, or a structured identity.
    // Boxed to keep `Result<_, Problem>` small.
    #[serde(default)]
    #[schema(value_type = Option<serde_json::Value>)]
    pub field: Option<Box<serde_json::Value>>,
}

impl Problem {
    pub fn new(status: StatusCode, code: &str, detail: Option<String>) -> Self {
        Self {
            kind: "about:blank".into(),
            title: status.canonical_reason().unwrap_or_default().into(),
            status: status.as_u16(),
            detail,
            code: code.into(),
            field: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<serde_json::Value>) -> Self {
        self.field = Some(Box::new(field.into()));
        self
    }

    /// Missing, unknown, revoked or expired token, or an inactive owner: one
    /// answer for all (FR-004).
    pub fn unauthenticated() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
            Some("A valid API token is required".into()),
        )
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", Some(detail.into()))
    }

    pub fn invalid(field: &str, detail: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
            Some(detail.into()),
        )
        .with_field(field)
    }

    pub fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", None)
    }

    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// For conversions that cannot fail, such as `TryFrom` via `From`.
impl From<std::convert::Infallible> for Problem {
    fn from(never: std::convert::Infallible) -> Self {
        match never {}
    }
}

impl From<OpError> for Problem {
    fn from(error: OpError) -> Self {
        let detail = error.detail().cloned().unwrap_or_default();
        // A conflict or invalid input may carry a finer, stable reason.
        let (status, code, reason_applies) = match error.kind() {
            OpError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", false),
            OpError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden", false),
            OpError::Unauthenticated(_) => (StatusCode::UNAUTHORIZED, "unauthorized", false),
            OpError::Conflict(_) => (StatusCode::CONFLICT, "conflict", true),
            OpError::Invalid { .. } => (StatusCode::UNPROCESSABLE_ENTITY, "invalid", true),
            OpError::Internal(_) | OpError::Detailed { .. } => return Self::internal(),
        };
        let code = detail.reason.filter(|_| reason_applies).unwrap_or(code);
        let field = detail.field.or_else(|| match error.kind() {
            OpError::Invalid {
                field: Some(field), ..
            } => Some((*field).into()),
            _ => None,
        });
        Self {
            field: field.map(Box::new),
            ..Self::new(status, code, Some(error.to_string()))
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut response = (self.status(), axum::Json(self)).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE));
        response
    }
}

/// A JSON body that refuses unknown fields with 422 `unknown_field` and any
/// other body that does not decode with 400 `bad_request`. Its type should
/// carry `#[serde(deny_unknown_fields)]`.
pub struct ApiJson<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for ApiJson<T> {
    type Rejection = Problem;

    async fn from_request(request: Request, state: &S) -> Result<Self, Problem> {
        let json = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(is_json);
        if !json {
            return Err(Problem::bad_request(
                "Send the body as JSON, with Content-Type: application/json",
            ));
        }
        let body = axum::body::Bytes::from_request(request, state)
            .await
            .map_err(|_| Problem::bad_request("The request body could not be read"))?;
        serde_json::from_slice(&body)
            .map(Self)
            .map_err(|error| body_problem(&error))
    }
}

fn body_problem(error: &serde_json::Error) -> Problem {
    let message = error.to_string();
    if let Some(field) = message
        .strip_prefix("unknown field `")
        .and_then(|rest| rest.split_once('`'))
        .map(|(field, _)| field)
    {
        return Problem::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown_field",
            Some(format!("Unknown field `{field}`")),
        )
        .with_field(field);
    }
    Problem::bad_request(format!("The JSON body is malformed: {message}"))
}

/// Query parameters; a value of the wrong type is 400 `bad_request`. Unknown
/// parameters are ignored.
pub struct ApiQuery<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = Problem;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Problem> {
        Query::from_request_parts(parts, state)
            .await
            .map(|Query(query)| Self(query))
            .map_err(|rejection| Problem::bad_request(rejection.body_text()))
    }
}

/// Path parameters; a value of the wrong type is 400 `bad_request`.
pub struct ApiPath<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned + Send> FromRequestParts<S> for ApiPath<T> {
    type Rejection = Problem;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Problem> {
        Path::from_request_parts(parts, state)
            .await
            .map(|Path(path)| Self(path))
            .map_err(|rejection| Problem::bad_request(rejection.body_text()))
    }
}

/// `application/json` and the `+json` media types, parameters ignored.
fn is_json(content_type: &str) -> bool {
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    essence == "application/json" || essence.ends_with("+json")
}

#[cfg(test)]
mod tests;
