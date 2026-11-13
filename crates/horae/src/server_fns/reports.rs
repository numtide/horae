//! Report and plugin-widget server functions.

use super::*;
#[cfg(feature = "server")]
use crate::ops::{self, Caller};

#[cfg(all(test, feature = "server"))]
mod tests;

/// Resolve report mode and identity together before mounting consumer resources.
#[server]
pub async fn get_time_report_access(
    expected_requester: Option<crate::models::permission_editor::PermissionRequester>,
) -> Result<crate::models::time_report::TimeReportAccess, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate report access");
            server_err("Report access is unavailable")
        }
    })?;
    let requester = crate::models::permission_editor::PermissionRequester {
        org_id: user.org_id,
        user_id: user.id,
    };
    if expected_requester.is_some_and(|expected| expected != requester) {
        return Err(forbidden("Report requester has changed"));
    }
    let state = crate::state::global_state().await;
    crate::reports::read_time_report_access(&state.db, requester)
        .await
        .map_err(|status| match status {
            axum::http::StatusCode::FORBIDDEN => forbidden("Current report access is required"),
            _ => server_err("Report access is unavailable"),
        })
}

/// Nonfinancial detailed time under current scoped authority, in bounded pages.
#[server]
pub async fn list_visible_time_report_entries(
    query: crate::models::time_report::TimeReportQuery,
) -> Result<crate::models::time_report::TimeReportPage, ServerFnError> {
    use permissions::time_entries::TimeReadError;

    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate time-report read");
            server_err("Time report is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::time_reports::read(&state.db, user.org_id, user.id, &query)
        .await
        .map_err(|error| match error {
            TimeReadError::Forbidden => {
                forbidden("Current time-report access or identity has changed")
            }
            TimeReadError::Conflict(message) => conflict(message),
            TimeReadError::InvalidQuery => {
                err(BAD_REQUEST, "Invalid time-report date range or cursor")
            }
            error => {
                tracing::error!(%error, "Scoped time-report read failed");
                server_err("Time report is unavailable")
            }
        })
}

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped ordinary time facts, without financial or directory authority.
#[server]
pub async fn list_visible_time_report_groups(
    query: crate::models::time_report::TimeReportGroupQuery,
) -> Result<crate::models::time_report::TimeReportGroupPage, ServerFnError> {
    use permissions::time_entries::TimeReadError;

    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate grouped time-report read");
            server_err("Time report is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::time_reports::groups::read(&state.db, user.org_id, user.id, &query)
        .await
        .map_err(|error| match error {
            TimeReadError::Forbidden => {
                forbidden("Current time-report access or identity has changed")
            }
            TimeReadError::Conflict(message) => conflict(message),
            TimeReadError::InvalidQuery => {
                err(BAD_REQUEST, "Invalid time-report date range or cursor")
            }
            error => {
                tracing::error!(%error, "Scoped grouped time-report read failed");
                server_err("Time report is unavailable")
            }
        })
}

/// Grouped time report. Groups by "project", "task", "client", or "person", with
/// optional client/project/teammate/tag filters. Each group carries billable and cost
/// amounts (rates via FR-024), partitioned by entity identity and currency.
/// Covers the time the viewer may read; amounts follow their rate grants.
#[server]
pub async fn report_time(
    from: String,
    to: String,
    group_by: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
    tag_id: Option<String>,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let filters = crate::reports::ReportFilters {
        client_id: parse_opt_uuid(client_id, "client_id")?,
        project_id: parse_opt_uuid(project_id, "project_id")?,
        user_id: parse_opt_uuid(user_id, "user_id")?,
        tag_id: parse_opt_uuid(tag_id, "tag_id")?,
    };

    fetch_report(
        &state.db,
        user.org_id,
        user.id,
        (from_date, to_date),
        &group_by,
        filters,
    )
    .await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_report(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    group_by: &str,
    filters: crate::reports::ReportFilters,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = ops::reports::time(&mut read.tx, &read.actor, period, group_by, filters).await;
    Ok(read.finish(result).await?)
}

/// Detailed (per-entry) report for the range, with the same optional filters
/// and the same entries as `report_time`.
#[server]
pub async fn report_detailed(
    from: String,
    to: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
    tag_id: Option<String>,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let user = require_user().await?;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let filters = crate::reports::ReportFilters {
        client_id: parse_opt_uuid(client_id, "client_id")?,
        project_id: parse_opt_uuid(project_id, "project_id")?,
        user_id: parse_opt_uuid(user_id, "user_id")?,
        tag_id: parse_opt_uuid(tag_id, "tag_id")?,
    };

    let state = crate::state::global_state().await;
    fetch_detailed(
        &state.db,
        user.org_id,
        user.id,
        (from_date, to_date),
        filters,
    )
    .await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_detailed(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    filters: crate::reports::ReportFilters,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = ops::reports::detailed(&mut read.tx, &read.actor, period, filters).await;
    Ok(read.finish(result).await?)
}

// ── Plugins ────────────────────────────────────────────────────────────────

/// Collect dashboard widgets from all loaded plugins (FR-022).
#[server]
pub async fn get_plugin_widgets() -> Result<Vec<PluginWidget>, ServerFnError> {
    let _user = require_user().await?;
    let state = crate::state::global_state().await;
    let widgets = state.plugins.collect_widgets().await;
    Ok(widgets
        .into_iter()
        .map(|w| PluginWidget {
            plugin_name: w.plugin_name,
            title: w.title,
            body: w.body,
        })
        .collect())
}

/// A dashboard widget contributed by a plugin, serializable for the SPA.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub struct PluginWidget {
    pub plugin_name: String,
    pub title: String,
    pub body: String,
}
