//! Axum handlers in front of the backend. The backend modules (`jobs`,
//! `importers`, …) take ids and byte streams; reading the session, the
//! request and building the response happens here. The export routes in
//! [`reports`] keep their own queries and streaming, since they answer with
//! HTTP statuses throughout.

use axum::http::StatusCode;

pub mod harvest_oauth;
pub mod import_errors;
pub mod reports;

/// The status every Axum route (exports, reports, job downloads) answers an
/// [`OpError`](crate::ops::OpError) with. A hidden record is refused as
/// forbidden rather than not found, so a download never confirms it exists.
pub(crate) fn op_status(error: crate::ops::OpError) -> StatusCode {
    use crate::ops::OpError;
    match error.kind() {
        OpError::Forbidden(_) | OpError::NotFound(_) => StatusCode::FORBIDDEN,
        OpError::Conflict(_) => StatusCode::CONFLICT,
        OpError::Invalid { .. } => StatusCode::BAD_REQUEST,
        OpError::Internal(_) | OpError::Detailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod op_status_tests {
    use super::op_status;
    use crate::ops::OpError;
    use axum::http::StatusCode;

    #[test]
    fn invalid_input_is_a_bad_request() {
        assert_eq!(
            op_status(OpError::invalid("from", "Invalid date")),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            op_status(OpError::bad_input("Invalid filters")),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn other_errors_keep_their_status() {
        assert_eq!(
            op_status(OpError::not_found("hidden")),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            op_status(OpError::forbidden("refused")),
            StatusCode::FORBIDDEN
        );
        assert_eq!(op_status(OpError::conflict("raced")), StatusCode::CONFLICT);
        assert_eq!(
            op_status(OpError::Internal("failed".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
