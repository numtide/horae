//! Axum handlers in front of the backend. The backend modules (`jobs`,
//! `importers`, …) take ids and byte streams; reading the session, the
//! request and building the response happens here. The export routes in
//! [`exports`] keep their own queries and streaming, since they answer with
//! HTTP statuses throughout.

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{any, get};

pub mod auth;
pub mod cursor;
pub mod exports;
pub mod harvest_oauth;
mod health;
pub mod import_errors;
pub mod problem;
pub mod rate_limit;
pub mod token;
pub mod v1;

pub use auth::{LoginAssets, login_redirect_guard, make_session_layer, session};
pub use token::ApiCaller;

// The backend lives in `horae-server`; keep the `crate::ops` (…) paths.
use horae_server::{config, importers, notifications, ops, plugin, render, state};
#[cfg(test)]
use horae_server::{db, jobs, testing};
use horae_types as models;

/// What the binary decides for [`router`].
pub struct RouterOptions {
    /// `DEV_LOGIN=1`: the one-click admin sign-in instead of OIDC.
    pub dev_login: bool,
    pub login_assets: LoginAssets,
    pub v1: v1::V1State,
}

/// Every route this crate serves: `/health`, the exports, the import error
/// download, the Harvest OAuth callback, `/auth/*` and `/api/v1`. The caller
/// adds the session layer and [`login_redirect_guard`].
///
/// Any other `/api/…` path answers 404 with an empty body, so it does not
/// fall through to the web app's HTML.
pub fn router(options: RouterOptions) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/{*rest}", any(|| async { StatusCode::NOT_FOUND }))
        .merge(v1::router(options.v1))
        .route("/api/reports/export/csv", get(exports::export_csv))
        .route("/api/reports/export/xlsx", get(exports::export_xlsx))
        .route(
            "/api/reports/time/grouped/csv",
            get(exports::export_time_groups_csv),
        )
        .route(
            "/api/reports/time/grouped/xlsx",
            get(exports::export_time_groups_xlsx),
        )
        .route(
            "/api/projects/export/csv",
            get(exports::export_projects_csv),
        )
        .route(
            "/api/projects/export/xlsx",
            get(exports::export_projects_xlsx),
        )
        .route(
            "/api/invoices/{id}/export/csv",
            get(exports::export_invoice_csv),
        )
        .route(
            "/api/invoices/{id}/export/xlsx",
            get(exports::export_invoice_xlsx),
        )
        .route(
            "/api/invoices/{id}/export/pdf",
            get(exports::export_invoice_pdf),
        )
        .merge(auth::router(options.dev_login, options.login_assets))
        .merge(harvest_oauth::callback_router())
        .route(
            "/api/import/harvest/jobs/{job_id}/errors",
            get(import_errors::download),
        )
}

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
