//! Management time reports: the grouped report with billable amounts and
//! costs, and the detailed per-entry report behind it.
//!
//! Report access and rate access are separate grants (contract C02): `TimeRead*`
//! decides which entries a report covers, `BillableRateRead*` whether their
//! billable amounts are shown, and only `CostRateReadAll` shows costs. That
//! includes the per-project cost overrides (`project_member_costs`): the
//! catalog has no separate grant for them. No report shows a margin yet; one
//! that sets costs against amounts also needs `ReportProfitabilityRead`.

use chrono::NaiveDate;
use horae_core::permissions::catalog::Permission;

use super::{Actor, OpError, Tx};
use crate::models::{DetailedReportRow, ReportRow};
use crate::reports::{ReportFilters, TimeScope};

const ACCESS_REQUIRED: &str = "Reports need permission to view managed or all time";

/// The entries a management report may cover. Own time alone is the
/// timesheet, not a management report.
fn scope(actor: &Actor) -> Result<TimeScope, OpError> {
    if !actor.has(Permission::TimeReadAll) && !actor.has(Permission::TimeReadManaged) {
        return Err(OpError::forbidden(ACCESS_REQUIRED));
    }
    Ok(TimeScope {
        viewer_id: actor.id(),
        own: actor.has(Permission::TimeReadOwn),
        managed: actor.has(Permission::TimeReadManaged),
        all: actor.has(Permission::TimeReadAll),
    })
}

/// Grouped time by "project", "task", "client" or "person"; an unknown
/// dimension groups by project. A group's billable amount is withheld when any
/// of its entries has a rate the actor may not read, and its cost unless the
/// actor reads every cost rate: a partial sum would reveal what is withheld.
pub(crate) async fn time(
    tx: &mut Tx,
    actor: &Actor,
    period: (NaiveDate, NaiveDate),
    group_by: &str,
    filters: ReportFilters,
) -> Result<Vec<ReportRow>, OpError> {
    let scope = scope(actor)?;
    // Grouped in Postgres: a year of entries is a six-figure row count folded
    // down to a few hundred report lines, and none of the per-entry detail
    // survives the fold. The group key is a runtime choice but the query macro
    // takes a string literal, so the dimension rides in as a parameter and the
    // CASE picks the column — an unknown value falls through to the project
    // identity and name.
    //
    // The CTE names each entry's derived values once so the aggregates below
    // read as the sums they are; Postgres inlines a CTE referenced once.
    // `resolve_project_rate` is the SQL twin of the pure core rate selection,
    // and `line_amount_cents` is the SQL twin of the Rust function invoicing
    // uses (migration 0016). The rounding term inside it is per row, so these
    // sums cannot be taken over pre-aggregated minutes.
    let rows = sqlx::query_as!(
        ReportRow,
        r#"WITH entry AS (
             SELECT
               CASE $7::text
                 WHEN 'task' THEN t.id
                 WHEN 'client' THEN c.id
                 WHEN 'person' THEN u.id
                 ELSE p.id
               END AS group_id,
               CASE $7::text
                 WHEN 'task' THEN t.name
                 WHEN 'client' THEN c.name
                 WHEN 'person' THEN u.name
                 ELSE p.name
               END AS label,
               COALESCE(invoice.currency,
                 CASE WHEN ps.project_id IS NULL THEN c.currency ELSE p.currency END) AS currency,
               o.default_currency AS cost_currency,
               te.minutes AS minutes,
               -- Billable hours and amount use the rounded minutes that get
               -- invoiced; cost is what the worked time costs, so it stays on
               -- the actual ones.
               effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) AS rounded_minutes,
               (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) AS billable,
               ($13 OR ($14 AND EXISTS (SELECT 1 FROM project_management_assignments m
                 WHERE m.org_id = te.org_id AND m.manager_id = $6 AND m.project_id = te.project_id)))
                 AS rate_visible,
               COALESCE(CASE WHEN ps.project_id IS NULL OR p.project_type = 'time_and_materials'
                 THEN resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                 CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                 CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
               ) END, 0)
                 AS billable_rate_cents,
               line.amount_cents AS frozen_amount_cents,
               CASE WHEN $15 THEN COALESCE(mc.cost_rate_cents, u.cost_rate_cents, 0) END AS cost_rate_cents
             FROM time_entries te
             JOIN projects p ON te.project_id = p.id
             LEFT JOIN project_settings ps ON ps.project_id = p.id
             JOIN clients c ON p.client_id = c.id
             JOIN tasks t ON te.task_id = t.id
             JOIN users u ON te.user_id = u.id
             JOIN organizations o ON o.id = te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
             LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
             LEFT JOIN invoice_line_items line ON line.invoice_id = te.invoice_id AND line.time_entry_id = te.id
             LEFT JOIN invoices invoice ON invoice.id = te.invoice_id
             LEFT JOIN project_member_costs mc ON mc.project_id = te.project_id AND mc.user_id = te.user_id
             WHERE te.org_id = $9
               AND ($12 OR ($10 AND te.user_id = $6) OR ($11 AND (
                 EXISTS (SELECT 1 FROM person_management_assignments m
                   WHERE m.org_id = te.org_id AND m.manager_id = $6 AND m.managed_user_id = te.user_id)
                 OR EXISTS (SELECT 1 FROM project_management_assignments m
                   WHERE m.org_id = te.org_id AND m.manager_id = $6 AND m.project_id = te.project_id))))
               AND te.spent_date BETWEEN $1 AND $2
               AND ($3::uuid IS NULL OR p.client_id = $3)
               AND ($4::uuid IS NULL OR te.project_id = $4)
               AND ($5::uuid IS NULL OR te.user_id = $5)
               AND ($8::uuid IS NULL OR EXISTS (
                 SELECT 1 FROM project_tag_links l
                 WHERE l.org_id = te.org_id AND l.project_id = te.project_id AND l.tag_id = $8
               ))
           )
           SELECT
             group_id as "group_id!",
             label as "label!",
             SUM(minutes)::bigint as "total_minutes!",
             SUM(rounded_minutes)::bigint as "rounded_minutes!",
             COALESCE(SUM(rounded_minutes) FILTER (WHERE billable), 0)::bigint
               as "billable_minutes!",
             CASE WHEN bool_and(rate_visible) THEN COALESCE(
               SUM(COALESCE(frozen_amount_cents, line_amount_cents(billable_rate_cents, rounded_minutes)))
                 FILTER (WHERE billable),
               0)::bigint END as "billable_cents?",
             CASE WHEN bool_and(cost_rate_cents IS NOT NULL)
               THEN SUM(line_amount_cents(cost_rate_cents, minutes))::bigint END as "cost_cents?",
             currency as "currency!",
             cost_currency as "cost_currency!"
           FROM entry
           GROUP BY group_id, label, currency, cost_currency
           -- Stable ties for duplicate labels and multi-currency entities.
           ORDER BY label COLLATE "C", group_id, currency COLLATE "C"
        "#,
        period.0 as NaiveDate,
        period.1 as NaiveDate,
        filters.client_id,
        filters.project_id,
        filters.user_id,
        scope.viewer_id,
        group_by,
        filters.tag_id,
        actor.org_id(),
        scope.own,
        scope.managed,
        scope.all,
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::BillableRateReadManaged),
        actor.has(Permission::CostRateReadAll),
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

/// The entries behind [`time`], one row each, without amounts.
pub(crate) async fn detailed(
    tx: &mut Tx,
    actor: &Actor,
    period: (NaiveDate, NaiveDate),
    filters: ReportFilters,
) -> Result<Vec<DetailedReportRow>, OpError> {
    let scope = scope(actor)?;
    Ok(crate::reports::fetch_entries(&mut **tx, actor.org_id(), scope, period, filters).await?)
}

#[cfg(test)]
mod tests;
