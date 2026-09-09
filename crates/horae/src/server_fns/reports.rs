//! Report and plugin-widget server functions.

use super::*;

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped time report. Groups by "project", "task", "client", or "person", with
/// optional client/project/teammate filters. Each group carries billable and cost
/// amounts (rates via FR-024); its `currency` is `None` when it mixes currencies.
/// Manager-only: reports span every user's time and money (SPEC §6).
#[server]
pub async fn report_time(
    from: String,
    to: String,
    group_by: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let client_filter = parse_opt_uuid(client_id, "client_id")?;
    let project_filter = parse_opt_uuid(project_id, "project_id")?;
    let user_filter = parse_opt_uuid(user_id, "user_id")?;

    fetch_report(
        &state.db,
        manager.org_id,
        (from_date, to_date),
        &group_by,
        client_filter,
        project_filter,
        user_filter,
    )
    .await
    .map_err(server_err)
}

#[cfg(feature = "server")]
pub(super) async fn fetch_report(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    group_by: &str,
    client_filter: Option<uuid::Uuid>,
    project_filter: Option<uuid::Uuid>,
    user_filter: Option<uuid::Uuid>,
) -> Result<Vec<ReportRow>, sqlx::Error> {
    // Grouped in Postgres: a year of entries is a six-figure row count folded
    // down to a few hundred report lines, and none of the per-entry detail
    // survives the fold. The group key is a runtime choice but the query macro
    // takes a string literal, so the dimension rides in as a parameter and the
    // CASE picks the column — an unknown value falls through to the project
    // name, as the Rust match did.
    //
    // The CTE names each entry's derived values once so the aggregates below
    // read as the sums they are; Postgres inlines a CTE referenced once.
    // `COALESCE(pt, a, p, u)` is `horae_core::invoice::resolve_rate` written out,
    // and `line_amount_cents` is the SQL twin of the Rust function invoicing
    // uses (migration 0016). The rounding term inside it is per row, so these
    // sums cannot be taken over pre-aggregated minutes.
    let rows = sqlx::query_as!(
        ReportRow,
        r#"WITH entry AS (
             SELECT
               CASE $7::text
                 WHEN 'task' THEN t.name
                 WHEN 'client' THEN c.name
                 WHEN 'person' THEN u.name
                 ELSE p.name
               END AS label,
               c.currency AS currency,
               te.minutes AS minutes,
               -- Billable hours and amount use the rounded minutes that get
               -- invoiced; cost is what the worked time costs, so it stays on
               -- the actual ones.
               effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) AS rounded_minutes,
               (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) AS billable,
               COALESCE(pt.rate_cents, a.rate_cents, p.rate_cents, u.billable_rate_cents, 0)
                 AS billable_rate_cents,
               line.amount_cents AS frozen_amount_cents,
               COALESCE(u.cost_rate_cents, 0) AS cost_rate_cents
             FROM time_entries te
             JOIN projects p ON te.project_id = p.id
             JOIN clients c ON p.client_id = c.id
             JOIN tasks t ON te.task_id = t.id
             JOIN users u ON te.user_id = u.id
             JOIN organizations o ON o.id = te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
             LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
             LEFT JOIN invoice_line_items line ON line.invoice_id = te.invoice_id AND line.time_entry_id = te.id
             WHERE te.org_id = $6
               AND te.spent_date BETWEEN $1 AND $2
               AND ($3::uuid IS NULL OR p.client_id = $3)
               AND ($4::uuid IS NULL OR te.project_id = $4)
               AND ($5::uuid IS NULL OR te.user_id = $5)
           )
           SELECT
             label as "label!",
             SUM(minutes)::bigint as "total_minutes!",
             SUM(rounded_minutes)::bigint as "rounded_minutes!",
             COALESCE(SUM(rounded_minutes) FILTER (WHERE billable), 0)::bigint
               as "billable_minutes!",
             COALESCE(
               SUM(COALESCE(frozen_amount_cents, line_amount_cents(billable_rate_cents, rounded_minutes)))
                 FILTER (WHERE billable),
               0)::bigint as "billable_cents!",
             SUM(line_amount_cents(cost_rate_cents, minutes))::bigint as "cost_cents!",
             -- A group that spans two currencies has no summable total, so it
             -- reports none rather than a meaningless number.
             CASE WHEN COUNT(DISTINCT currency) = 1 THEN MIN(currency) END as "currency?"
           FROM entry
           GROUP BY label
           -- Byte order, so the row order matches the BTreeMap this replaced
           -- rather than the database's default collation.
           ORDER BY label COLLATE "C"
        "#,
        period.0 as chrono::NaiveDate,
        period.1 as chrono::NaiveDate,
        client_filter,
        project_filter,
        user_filter,
        org_id,
        group_by,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Detailed (per-entry) report for the range, with the same optional filters.
/// Manager-only, like `report_time`: rows cover every user's entries and notes.
#[server]
pub async fn report_detailed(
    from: String,
    to: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let manager = require_manager().await?;

    let from_date = parse_date(&from, "from")?;
    let to_date = parse_date(&to, "to")?;
    let client_filter = parse_opt_uuid(client_id, "client_id")?;
    let project_filter = parse_opt_uuid(project_id, "project_id")?;
    let user_filter = parse_opt_uuid(user_id, "user_id")?;

    // The CSV/XLSX exports must return exactly these rows, so the query lives
    // once in `crate::reports` and both surfaces call it.
    let state = crate::state::global_state().await;
    crate::reports::fetch_entries(
        &state.db,
        manager.org_id,
        from_date,
        to_date,
        client_filter,
        project_filter,
        user_filter,
    )
    .await
    .map_err(server_err)
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
pub struct PluginWidget {
    pub plugin_name: String,
    pub title: String,
    pub body: String,
}
