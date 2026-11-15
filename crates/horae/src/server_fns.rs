use dioxus::prelude::*;
#[cfg(feature = "server")]
pub(crate) use horae_core::types::{
    BudgetKind, EntryState, InvoiceStatus, ProjectRole, ProjectType,
};

#[cfg(feature = "server")]
pub(crate) use crate::models::User;
pub(crate) use crate::models::{
    Approval, ApprovalSummary, Client, DetailedReportRow, Invoice, InvoiceWithLines, OrgBranding,
    Project, ProjectSpend, ReportRow, Task, TimeEntry,
};

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

    crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or_else(|| unauthorized("Not authenticated — please sign in."))
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
pub(crate) fn forbidden(msg: impl std::fmt::Display) -> ServerFnError {
    err(FORBIDDEN, msg)
}
#[cfg(feature = "server")]
pub(crate) fn conflict(msg: impl std::fmt::Display) -> ServerFnError {
    err(CONFLICT, msg)
}
#[cfg(feature = "server")]
pub(crate) fn unauthorized(msg: impl std::fmt::Display) -> ServerFnError {
    err(UNAUTHORIZED, msg)
}

/// Server functions report an operation's domain error with its status code.
#[cfg(feature = "server")]
impl From<crate::ops::OpError> for ServerFnError {
    fn from(error: crate::ops::OpError) -> Self {
        use crate::ops::OpError;
        let code = match &error {
            OpError::NotFound(_) => NOT_FOUND,
            OpError::Forbidden(_) => FORBIDDEN,
            OpError::Conflict(_) => CONFLICT,
            OpError::Invalid { .. } => BAD_REQUEST,
            OpError::Internal(_) => INTERNAL_ERROR,
        };
        err(code, error)
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
pub(crate) async fn require_user() -> Result<crate::models::User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    sqlx::query_as!(
        crate::models::User,
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

// ── Plugin event dispatch helpers ────────────────────────────────────────────
// Fire-and-forget: spawn dispatch so the core action is never blocked (FR-021).

/// Compare-and-swap the last announced legacy plugin band.
#[cfg(feature = "server")]
async fn claim_budget_band(
    pool: &sqlx::PgPool,
    project_id: uuid::Uuid,
    last: i32,
    current: i32,
) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar!(
        r#"UPDATE projects
           SET last_budget_alert_pct = $2
           WHERE id = $1
             AND COALESCE(last_budget_alert_pct, 0) = $3
           RETURNING id"#,
        project_id,
        current,
        last,
    )
    .fetch_optional(pool)
    .await?
    .is_some())
}

#[cfg(feature = "server")]
pub(crate) async fn check_project_budget(
    state: &'static crate::state::AppState,
    project_id: uuid::Uuid,
) {
    let settings_org = sqlx::query_scalar!(
        "SELECT org_id FROM project_settings WHERE project_id = $1",
        project_id,
    )
    .fetch_optional(&state.db)
    .await;
    match settings_org {
        Ok(Some(org_id)) => {
            let enqueued = match crate::timezone::today_for_org(&state.db, org_id).await {
                Ok(today) => {
                    crate::notifications::budgets::enqueue_alerts(
                        &state.db, org_id, project_id, today,
                    )
                    .await
                }
                Err(error) => Err(error.into()),
            };
            if enqueued.is_err() {
                tracing::warn!(%project_id, "configured budget alert check failed");
            }
            return;
        }
        Ok(None) => {}
        Err(_) => {
            tracing::warn!(%project_id, "budget settings lookup failed");
            return;
        }
    }
    let row = match sqlx::query!(
        r#"SELECT p.org_id, p.client_id, p.name, p.active, p.last_budget_alert_pct,
                  p.project_type as "project_type: horae_core::types::ProjectType",
                  p.budget_kind as "budget_kind: horae_core::types::BudgetKind",
                  p.budget_minutes,
                  o.budget_alert_pcts
           FROM projects p
           JOIN organizations o ON o.id = p.org_id
           WHERE p.id = $1"#,
        project_id,
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!("budget check: load project {project_id} failed: {e}");
            return;
        }
    };

    // Preserve the legacy hours-only plugin contract for unconfigured projects.
    let budget = match (row.budget_kind, row.budget_minutes) {
        (horae_core::types::BudgetKind::Hours, Some(b)) if b > 0 => b,
        _ => return,
    };

    let consumed: i64 = match sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(COALESCE(rounded_minutes, minutes)), 0)::bigint as "c!"
           FROM time_entries WHERE project_id = $1"#,
        project_id,
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("budget check: sum for {project_id} failed: {e}");
            return;
        }
    };

    let thresholds = row.budget_alert_pcts;
    let last = row.last_budget_alert_pct.unwrap_or(0);
    let current = horae_core::budget::current_band(consumed, budget, &thresholds);

    // Claim the new band before dispatching. The budget check is spawned after
    // several entry mutations, so two checks can read the same old band. The
    // conditional update makes only one of them responsible for the events.
    if current != last {
        let claimed = match claim_budget_band(&state.db, project_id, last, current).await {
            Ok(claimed) => claimed,
            Err(e) => {
                tracing::warn!("budget check: claim band for {project_id} failed: {e}");
                false
            }
        };
        if !claimed {
            return;
        }
    }

    // Announce every band newly crossed since last. 100 is always a band, so
    // exceeding budget fires project_over_budget regardless of the configured
    // warning thresholds, and a single large jump reports each band it passed.
    for band in horae_core::budget::newly_crossed_bands(consumed, budget, &thresholds, last) {
        let payload = crate::plugin::event::BudgetThresholdPayload {
            project: crate::plugin::event::ProjectPayload {
                id: project_id,
                client_id: row.client_id,
                name: row.name.clone(),
                project_type: row.project_type.to_string(),
                budget_kind: row.budget_kind.to_string(),
                active: row.active,
            },
            threshold_pct: band,
            consumed_minutes: Some(consumed as i32),
            budget_minutes: Some(budget),
            consumed_cents: None,
            budget_amount_cents: None,
        };
        let occurred_at = chrono::Utc::now();
        let event = if band >= horae_core::budget::OVER_BUDGET_BAND {
            crate::plugin::AppEvent::ProjectOverBudget {
                occurred_at,
                org_id: row.org_id,
                budget: payload,
            }
        } else {
            crate::plugin::AppEvent::ProjectBudgetThresholdReached {
                occurred_at,
                org_id: row.org_id,
                budget: payload,
            }
        };
        state.plugins.dispatch(event);
    }
}

// ── Feature modules ──────────────────────────────────────────────────────────
// The #[server] endpoints grouped by feature; re-exported so call sites keep
// using `server_fns::<fn>` regardless of which submodule a function lives in.
#[cfg(all(test, feature = "server"))]
mod budget_tests;
#[cfg(feature = "server")]
pub(crate) mod budgets;
#[cfg(all(test, feature = "server"))]
pub(crate) mod test_seed;

mod approvals;
mod auth;
mod clients;
mod importers;
mod invoices;
mod organization;
#[cfg(all(test, feature = "server"))]
mod parse_tests;
mod permission_editor;
#[cfg(feature = "server")]
pub(crate) mod permissions;
// Temporary: the server_fns tests still provision through this path.
#[cfg(all(test, feature = "server"))]
pub(crate) use crate::ops::permissions::provisioning;
mod plugins;
mod profile;
mod project_creation;
mod project_managers;
mod projects;
mod reports;
#[cfg(all(test, feature = "server"))]
mod snapshot;
mod time_entries;
mod users;

pub use approvals::*;
pub use auth::*;
pub use clients::*;
pub use importers::*;
pub use invoices::*;
// Org-branding endpoints exist but no page consumes them yet.
#[allow(unused_imports)]
pub use organization::*;
pub use permission_editor::*;
pub use plugins::*;
pub use profile::*;
pub use project_creation::*;
pub use projects::*;
pub use reports::*;
pub use time_entries::*;
pub use users::*;
