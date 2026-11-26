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
    const UNAVAILABLE: &str = "Report access is unavailable";
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    let read =
        authenticated(ops::begin_read(&state.db, caller).await, UNAVAILABLE).map_err(op_error)?;
    let result = ops::time_reads::reports::access(&read.actor, expected_requester);
    read.finish(result).await.map_err(op_error)
}

/// Nonfinancial detailed time under current scoped authority, in bounded pages.
#[server]
pub async fn list_visible_time_report_entries(
    query: crate::models::time_report::TimeReportQuery,
) -> Result<crate::models::time_report::TimeReportPage, ServerFnError> {
    const UNAVAILABLE: &str = "Time report is unavailable";
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    let mut read =
        authenticated(ops::begin_read(&state.db, caller).await, UNAVAILABLE).map_err(op_error)?;
    let result = ops::time_reads::reports::entries(&mut read.tx, &read.actor, &query).await;
    read.finish(result).await.map_err(op_error)
}

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped ordinary time facts, without financial or directory authority.
#[server]
pub async fn list_visible_time_report_groups(
    query: crate::models::time_report::TimeReportGroupQuery,
) -> Result<crate::models::time_report::TimeReportGroupPage, ServerFnError> {
    const UNAVAILABLE: &str = "Time report is unavailable";
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    let mut read =
        authenticated(ops::begin_read(&state.db, caller).await, UNAVAILABLE).map_err(op_error)?;
    let result = ops::time_reads::reports::groups::read(&mut read.tx, &read.actor, &query).await;
    read.finish(result).await.map_err(op_error)
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
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let filters = crate::ops::reports::ReportFilters {
        client_id: parse_opt_uuid(client_id, "client_id")?,
        project_id: parse_opt_uuid(project_id, "project_id")?,
        user_id: parse_opt_uuid(user_id, "user_id")?,
        tag_id: parse_opt_uuid(tag_id, "tag_id")?,
    };

    fetch_report(
        &state.db,
        caller.org_id,
        caller.user_id,
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
    filters: crate::ops::reports::ReportFilters,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id))
        .await
        .map_err(op_error)?;
    let result = ops::reports::time(&mut read.tx, &read.actor, period, group_by, filters).await;
    read.finish(result).await.map_err(op_error)
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
    let caller = session_caller().await?;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let filters = crate::ops::reports::ReportFilters {
        client_id: parse_opt_uuid(client_id, "client_id")?,
        project_id: parse_opt_uuid(project_id, "project_id")?,
        user_id: parse_opt_uuid(user_id, "user_id")?,
        tag_id: parse_opt_uuid(tag_id, "tag_id")?,
    };

    let state = crate::state::global_state().await;
    fetch_detailed(
        &state.db,
        caller.org_id,
        caller.user_id,
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
    filters: crate::ops::reports::ReportFilters,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id))
        .await
        .map_err(op_error)?;
    let result = ops::reports::detailed(&mut read.tx, &read.actor, period, filters).await;
    read.finish(result).await.map_err(op_error)
}

// ── Plugins ────────────────────────────────────────────────────────────────

/// Collect dashboard widgets from all loaded plugins (FR-022).
#[server]
pub async fn get_plugin_widgets() -> Result<Vec<PluginWidget>, ServerFnError> {
    session_caller().await?;
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
