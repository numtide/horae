use dioxus::prelude::*;
#[cfg(feature = "server")]
pub(crate) use horae_core::types::{EntryState, InvoiceStatus};
// The tests reach these through `use super::*`.
#[cfg(all(test, feature = "server"))]
pub(crate) use horae_core::types::{BudgetKind, ProjectRole, ProjectType};

pub(crate) use crate::models::{
    Approval, ApprovalSummary, Client, DetailedReportRow, Invoice, InvoiceWithLines, OrgBranding,
    Project, ProjectSpend, ReportRow, Task, TimeEntry,
};
#[cfg(feature = "server")]
pub(crate) use crate::ops::people::User;

// HTTP status codes for `ServerFnError::ServerError { code, .. }` — named so
// error paths read at a glance (see AGENTS.md conventions). Server-only: on the
// web target `#[server]` bodies are replaced by client stubs that never build
// these errors.
#[cfg(feature = "server")]
mod status {
    pub const BAD_REQUEST: u16 = 400;
    pub const UNAUTHORIZED: u16 = 401;
    pub const FORBIDDEN: u16 = 403;
    pub const NOT_FOUND: u16 = 404;
    pub const CONFLICT: u16 = 409;
    pub const INTERNAL_ERROR: u16 = 500;
    pub const SERVICE_UNAVAILABLE: u16 = 503;
}
#[cfg(feature = "server")]
pub(crate) use status::*;

/// Extract the session user UUID from the request context.
/// Returns `Err(401)` if the session has no user (not logged in).
#[cfg(feature = "server")]
pub(crate) async fn session_user_id() -> Result<uuid::Uuid, ServerFnError> {
    use tower_sessions::Session;

    // `FullstackContext::extract` pulls the Session from the Axum request extensions
    // injected by the SessionManagerLayer wrapping the server.
    // Session implements FromRequestParts for any S, so M is inferred as the
    // axum via::Parts marker type.
    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    horae_api::session::get_session_user_id(&session)
        .await
        .map_err(session_unavailable)?
        .ok_or_else(|| unauthorized("Not authenticated — please sign in."))
}

/// A session store that cannot be read is no sign-out: a 503, which the app
/// reports as an error, not "Session ended".
#[cfg(feature = "server")]
fn session_unavailable(error: horae_api::session::SessionUnavailable) -> ServerFnError {
    tracing::error!(%error, "Could not read the session");
    err(
        SERVICE_UNAVAILABLE,
        "Horae is temporarily unavailable. Please try again.",
    )
}

/// Build a `ServerFnError::ServerError` with a named status code, so error
/// paths read as `not_found("…")` rather than a five-line struct literal.
#[cfg(feature = "server")]
pub(crate) fn err(code: u16, msg: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: msg.to_string(),
        code,
        details: None,
    }
}

#[cfg(feature = "server")]
pub(crate) fn server_err(msg: impl std::fmt::Display) -> ServerFnError {
    err(INTERNAL_ERROR, msg)
}
#[cfg(feature = "server")]
pub(crate) fn not_found(msg: impl std::fmt::Display) -> ServerFnError {
    err(NOT_FOUND, msg)
}
#[cfg(feature = "server")]
pub(crate) fn unauthorized(msg: impl std::fmt::Display) -> ServerFnError {
    err(UNAUTHORIZED, msg)
}

/// Server functions report an operation's domain error with its status code:
/// `.map_err(op_error)?`. A function rather than a `From` impl, because the
/// operations will live in a crate that does not depend on Dioxus.
#[cfg(feature = "server")]
pub(crate) fn op_error(error: crate::ops::OpError) -> ServerFnError {
    use crate::ops::OpError;
    let code = match error.kind() {
        OpError::NotFound(_) => NOT_FOUND,
        OpError::Forbidden(_) => FORBIDDEN,
        OpError::Unauthenticated(_) => UNAUTHORIZED,
        OpError::Conflict(_) => CONFLICT,
        OpError::Invalid { .. } => BAD_REQUEST,
        OpError::Internal(_) | OpError::Detailed { .. } => INTERNAL_ERROR,
    };
    // Only an error with a detail carries `details`; the rest answer as before.
    let details = error.detail().map(|detail| {
        let mut details = serde_json::Map::new();
        if let Some(field) = &detail.field {
            details.insert("field".into(), field.clone());
        }
        if let Some(reason) = detail.reason {
            details.insert("reason".into(), reason.into());
        }
        serde_json::Value::Object(details)
    });
    ServerFnError::ServerError {
        message: error.to_string(),
        code,
        details,
    }
}

/// Parse a UUID argument, mapping a malformed value to a clear error naming the
/// field (e.g. `parse_uuid(&entry_id, "entry_id")`).
#[cfg(feature = "server")]
pub(crate) fn parse_uuid(s: &str, field: &str) -> Result<uuid::Uuid, ServerFnError> {
    s.parse()
        .map_err(|_| err(BAD_REQUEST, format!("Invalid {field}")))
}

/// Parse an optional UUID filter: `None`/empty → `None`, otherwise a validated
/// UUID. Used by the report filters (client/project/teammate).
#[cfg(feature = "server")]
pub(crate) fn parse_opt_uuid(
    s: Option<String>,
    field: &str,
) -> Result<Option<uuid::Uuid>, ServerFnError> {
    match s.as_deref() {
        None | Some("") => Ok(None),
        Some(v) => Ok(Some(parse_uuid(v, field)?)),
    }
}

/// Parse a `YYYY-MM-DD` date argument, mapping a malformed value to a clear
/// error naming the field.
#[cfg(feature = "server")]
pub(crate) fn parse_date(s: &str, field: &str) -> Result<chrono::NaiveDate, ServerFnError> {
    s.parse()
        .map_err(|_| err(BAD_REQUEST, format!("Invalid {field} (use YYYY-MM-DD)")))
}

/// Parse a stringly enum argument (roles, states, kinds), mapping a malformed
/// value to a clear error naming the field.
#[cfg(feature = "server")]
pub(crate) fn parse_enum<T: std::str::FromStr>(s: &str, field: &str) -> Result<T, ServerFnError> {
    s.parse()
        .map_err(|_| err(BAD_REQUEST, format!("Invalid {field}")))
}

/// Load the session's user, requiring the account to still be active. This is
/// the base gate for every endpoint: sessions outlive deactivation, so the
/// `active = true` filter is what actually revokes a deactivated user's access
/// (FR-002).
#[cfg(feature = "server")]
pub(crate) async fn require_user() -> Result<User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| unauthorized("Account is not active"))
}

/// The session's organization and person, for an endpoint that runs through
/// an `ops::begin*` transaction, from the session alone: no query.
///
/// It does not check that the account is still active. The transaction does:
/// loading a session actor whose person is gone or deactivated fails with
/// `OpError::Unauthenticated`, the same 401 as [`require_user`]. An endpoint
/// that reads the database without such a transaction takes
/// [`active_session_caller`] instead, which is why this one is named
/// unchecked.
#[cfg(feature = "server")]
pub(crate) async fn unchecked_session_caller() -> Result<crate::ops::Caller, ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;
    let user_id = horae_api::session::get_session_user_id(&session)
        .await
        .map_err(session_unavailable)?
        .ok_or_else(|| unauthorized("Not authenticated — please sign in."))?;
    match horae_api::session::get_session_org_id(&session)
        .await
        .map_err(session_unavailable)?
    {
        Some(org_id) => Ok(crate::ops::Caller::session(org_id, user_id)),
        // Signed in before sessions stored the organization.
        None => checked_caller(user_id).await,
    }
}

/// The session's caller, refusing a deactivated account before returning, for
/// endpoints that read the database outside an `ops::begin*` transaction.
#[cfg(feature = "server")]
pub(crate) async fn active_session_caller() -> Result<crate::ops::Caller, ServerFnError> {
    checked_caller(session_user_id().await?).await
}

#[cfg(feature = "server")]
async fn checked_caller(user_id: uuid::Uuid) -> Result<crate::ops::Caller, ServerFnError> {
    let state = crate::state::global_state().await;
    crate::ops::active_caller(&state.db, user_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| unauthorized(crate::ops::ACCOUNT_NOT_ACTIVE))
}

/// [`unchecked_session_caller`] for endpoints that answer any failure other
/// than a 401, or the 503 of a session store that cannot be read, as
/// `unavailable`, without exposing the cause.
#[cfg(feature = "server")]
pub(crate) async fn unchecked_session_caller_or(
    unavailable: &'static str,
) -> Result<crate::ops::Caller, ServerFnError> {
    unchecked_session_caller()
        .await
        .map_err(|error| unavailable_unless_unauthorized(error, unavailable))
}

#[cfg(feature = "server")]
fn unavailable_unless_unauthorized(error: ServerFnError, unavailable: &str) -> ServerFnError {
    match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED | SERVICE_UNAVAILABLE,
            ..
        } => error,
        error => {
            tracing::error!(%error, unavailable, "Unable to authenticate request");
            server_err(unavailable)
        }
    }
}

/// The `ops::begin*` of an endpoint that took [`unchecked_session_caller_or`]: the
/// account check runs in that transaction, so a storage failure there answers
/// `unavailable` as a failed session lookup does. Its refusals are unchanged.
#[cfg(feature = "server")]
pub(crate) fn authenticated<T>(
    begun: Result<T, crate::ops::OpError>,
    unavailable: &str,
) -> Result<T, crate::ops::OpError> {
    use crate::ops::OpError;
    begun.map_err(|error| match error.kind() {
        OpError::Internal(_) => {
            tracing::error!(%error, unavailable, "Unable to authenticate request");
            OpError::Internal(unavailable.to_owned())
        }
        _ => error,
    })
}

/// For tests: whether `result` failed as a session whose person is gone,
/// foreign or deactivated does, signed out with a 401.
#[cfg(all(test, feature = "server"))]
pub(crate) fn signed_out<T>(result: &Result<T, ServerFnError>) -> bool {
    matches!(
        result,
        Err(ServerFnError::ServerError { code: UNAUTHORIZED, message, .. })
            if message == crate::ops::ACCOUNT_NOT_ACTIVE
    )
}

#[cfg(all(test, feature = "server"))]
mod session_caller_tests {
    use super::*;

    #[test]
    fn a_sign_in_refusal_keeps_its_status_and_message() {
        let error = unavailable_unless_unauthorized(
            unauthorized("Account is not active"),
            "Timesheet is unavailable",
        );
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: UNAUTHORIZED, ref message, .. }
                if message == "Account is not active"
        ));
    }

    #[test]
    fn any_other_failure_reads_as_unavailable() {
        let error = unavailable_unless_unauthorized(
            server_err("pool timed out"),
            "Timesheet is unavailable",
        );
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: INTERNAL_ERROR, ref message, .. }
                if message == "Timesheet is unavailable"
        ));
    }
}

// ── Feature modules ──────────────────────────────────────────────────────────
// The #[server] endpoints grouped by feature; re-exported so call sites keep
// using `server_fns::<fn>` regardless of which submodule a function lives in.
#[cfg(all(test, feature = "server"))]
pub(crate) mod budget_tests;

mod api_tokens;
mod approvals;
mod auth;
mod clients;
mod importers;
mod invoices;
mod oauth;
mod oauth_apps;
#[cfg(all(test, feature = "server"))]
mod op_error_tests;
mod organization;
#[cfg(all(test, feature = "server"))]
mod parse_tests;
mod permission_editor;
#[cfg(all(test, feature = "server"))]
pub(crate) mod permissions;
// Temporary: the server_fns tests still provision through this path.
#[cfg(all(test, feature = "server"))]
pub(crate) use crate::ops::permissions::provisioning;
mod plugins;
mod profile;
mod project_creation;
pub(crate) mod project_managers;
mod projects;
mod reports;
mod time_entries;
mod users;

pub use api_tokens::*;
pub use approvals::*;
pub use auth::*;
pub use clients::*;
pub use importers::*;
pub use invoices::*;
pub use oauth::*;
pub use oauth_apps::*;
pub use organization::*;
pub use permission_editor::*;
pub use plugins::*;
pub use profile::*;
pub use project_creation::*;
pub use projects::*;
pub use reports::*;
pub use time_entries::*;
pub use users::*;
