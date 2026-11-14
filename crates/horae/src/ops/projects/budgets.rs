//! Budget progress as project readers see it: the Projects page's batch and
//! the detail page's headline. The progress query itself is shared with the
//! alert check in `notifications::budgets`.

use chrono::NaiveDate;
use horae_core::permissions::catalog::Permission;
use horae_core::types::BudgetKind;
use sqlx::PgConnection;
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use crate::models::permissions::PersonPermissions;
use crate::models::project::ProjectDetailBudget;
use crate::models::{ProjectBudgetOverview, ProjectBudgetProgress};

/// Every readable configured project's progress in the period of `date`.
/// Scope details are kept only for those who may read them, and money only
/// for those who may see it. Empty for an actor with no authority.
pub(crate) async fn progress(
    tx: &mut Tx,
    actor: &Actor,
    date: NaiveDate,
) -> Result<Vec<ProjectBudgetOverview>, OpError> {
    if !actor.is_active() {
        return Ok(Vec::new());
    }
    let viewer_id = actor.id();
    let rows = fetch_progress(
        tx,
        actor.org_id(),
        None,
        Some(viewer_id),
        date,
        actor.permissions(),
        false,
    )
    .await?;
    let mut grouped = std::collections::BTreeMap::<_, Vec<_>>::new();
    for row in rows {
        grouped.entry(row.project_id).or_default().push(row);
    }
    let mut overview = Vec::with_capacity(grouped.len());
    for (project_id, rows) in grouped {
        let Some(first) = rows.first() else { continue };
        let visible = first.kind != BudgetKind::Amount || first.can_view_money;
        let (budget, consumed) = if !visible {
            (None, None)
        } else if first.scope == "project" {
            (first.budget, Some(first.consumed))
        } else {
            horae_core::budget::allocated_totals(rows.iter().map(|row| (row.budget, row.consumed)))
        };
        let mut project = ProjectBudgetOverview {
            project_id,
            scope: first.scope.clone(),
            kind: first.kind,
            currency: first.currency.clone(),
            period_key: first.period_key.clone(),
            budget,
            consumed,
            breakdown: Vec::new(),
        };
        project.breakdown = rows
            .into_iter()
            .filter(|row| {
                visible
                    && (row.can_read_details
                        || row.scope == "project"
                        || (row.scope == "person" && row.user_id == Some(viewer_id)))
            })
            .map(|row| ProjectBudgetProgress {
                project_id: row.project_id,
                task_id: row.task_id,
                user_id: row.user_id,
                scope: row.scope,
                label: row.label,
                kind: row.kind,
                currency: row.currency,
                period_key: row.period_key,
                budget: row.budget,
                consumed: row.consumed,
            })
            .collect();
        overview.push(project);
    }
    Ok(overview)
}

/// Whole-project headline for the detail view, read inside the caller's
/// snapshot after its authority locks. Configured projects use
/// `fetch_progress`; projects without settings keep the Projects list's legacy
/// lifetime spend. Scope details never leave this function.
pub(super) async fn headline(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    date: NaiveDate,
) -> Result<Option<ProjectDetailBudget>, OpError> {
    use horae_core::budget::{BudgetHeadline, headline};
    use horae_core::permissions::catalog::Permission;

    let org_id = actor.org_id();
    // Non-arithmetic facts first, so withheld or unallocated budgets never
    // value an entry. The predicates and joins match the consumption queries.
    let meta = sqlx::query!(
        r#"SELECT p.budget_kind AS "kind!: BudgetKind", p.currency AS "currency!",
            p.budget_minutes, p.budget_amount_cents,
            ps.project_id IS NOT NULL AS "configured!",
            COALESCE(ps.monthly_reset, false) AS "monthly!",
            $3 OR ($4 AND management.id IS NOT NULL) AS "can_view_money!",
            CASE
              WHEN ps.project_id IS NULL OR ps.budget_scope = 'project'
                THEN CASE WHEN p.budget_kind = 'hours' THEN p.budget_minutes
                  ELSE p.budget_amount_cents END IS NOT NULL
              WHEN ps.budget_scope = 'task' THEN EXISTS (
                SELECT 1 FROM project_tasks pt
                JOIN tasks t ON t.id = pt.task_id AND t.org_id = $1
                JOIN project_task_settings s ON s.project_id = p.id AND s.org_id = $1
                  AND s.task_id = pt.task_id
                WHERE pt.project_id = p.id AND CASE WHEN p.budget_kind = 'hours'
                  THEN s.budget_minutes ELSE s.budget_cents END IS NOT NULL)
              ELSE p.budget_kind = 'hours' AND EXISTS (
                SELECT 1 FROM assignments a
                JOIN users u ON u.id = a.user_id AND u.org_id = $1
                JOIN project_member_budgets s ON s.project_id = p.id AND s.org_id = $1
                  AND s.user_id = a.user_id
                WHERE a.project_id = p.id)
            END AS "allocated!"
        FROM projects p
        LEFT JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = $5
        WHERE p.org_id = $1 AND p.id = $2"#,
        org_id,
        project_id,
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::BillableRateReadManaged),
        actor.id(),
    )
    .fetch_one(&mut **tx)
    .await?;
    if meta.kind == BudgetKind::None {
        return Ok(None);
    }
    let budget = |headline| ProjectDetailBudget {
        kind: meta.kind,
        currency: meta.currency.clone(),
        period_key: if meta.monthly {
            date.format("%Y-%m").to_string()
        } else {
            "lifetime".to_string()
        },
        headline,
    };
    if meta.kind == BudgetKind::Amount && !meta.can_view_money {
        return Ok(Some(budget(BudgetHeadline::Unavailable)));
    }
    if !meta.allocated {
        return Ok(Some(budget(BudgetHeadline::Unallocated)));
    }

    // Valuation can exceed bigint and aborts the statement. A savepoint keeps
    // the snapshot and authority locks while containing only that failure.
    let mut savepoint = sqlx::Connection::begin(&mut **tx).await?;
    let consumption = if meta.configured {
        fetch_progress(
            &mut savepoint,
            org_id,
            Some(project_id),
            Some(actor.id()),
            date,
            actor.permissions(),
            true,
        )
        .await
        .map(|rows| {
            let first = rows.first()?;
            Some(if first.scope == "project" {
                headline(&[(first.budget, first.consumed)])
            } else {
                let scopes: Vec<_> = rows.iter().map(|row| (row.budget, row.consumed)).collect();
                headline(&scopes)
            })
        })
    } else {
        super::reads::spend_rows(
            &mut savepoint,
            actor,
            Some(project_id),
            meta.kind == BudgetKind::Amount,
        )
        .await
        .map(|mut rows| {
            let spend = rows.pop()?;
            Some(match meta.kind {
                BudgetKind::Hours => headline(&[(meta.budget_minutes, spend.spent_minutes)]),
                _ => headline(&[(meta.budget_amount_cents, spend.spent_cents?)]),
            })
        })
    };
    // `None` means the budget readers withheld this project from the viewer.
    // The detail reader can still admit them while it answers for a
    // policy-zero organization, so the figures stay hidden but the page loads.
    let headline = match consumption {
        Ok(Some(headline)) => {
            savepoint.commit().await?;
            headline
        }
        Ok(None) => {
            // The detail and budget readers should agree, so log a
            // divergence instead of hiding it.
            tracing::warn!(%project_id, "Budget readers withheld a readable project");
            savepoint.commit().await?;
            BudgetHeadline::Unavailable
        }
        Err(error) if numeric_overflow(&error) => {
            savepoint.rollback().await?;
            BudgetHeadline::Unavailable
        }
        Err(error) => return Err(error.into()),
    };
    Ok(Some(budget(headline)))
}

/// Whether PostgreSQL refused a value as out of range for its numeric type.
pub(crate) fn numeric_overflow(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|error| error.code().as_deref() == Some("22003"))
}

pub(crate) struct BudgetRow {
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub label: Option<String>,
    pub scope: String,
    pub kind: BudgetKind,
    pub currency: String,
    pub period_key: String,
    pub budget: Option<i64>,
    pub consumed: i64,
    pub can_view_money: bool,
    pub can_read_details: bool,
}

/// Configured progress, project by project and scope by scope. With a
/// `viewer_id`, only the projects the viewer's `permissions` may read, and
/// money only where they may see it; without one (the alert check), every
/// configured project of the organization.
///
/// `allocated_only` skips valuing entries no non-blank scope can count, so an
/// excluded entry's value cannot overflow the allocated total.
pub(crate) async fn fetch_progress(
    connection: &mut PgConnection,
    org_id: Uuid,
    project_id: Option<Uuid>,
    viewer_id: Option<Uuid>,
    date: NaiveDate,
    permissions: Option<&PersonPermissions>,
    allocated_only: bool,
) -> Result<Vec<BudgetRow>, sqlx::Error> {
    let has = |permission| permissions.is_some_and(|state| state.grants.contains(permission));

    sqlx::query_as!(
        BudgetRow,
        r#"WITH config AS (
             SELECT p.id, p.budget_kind, p.budget_minutes, p.budget_amount_cents,
                    p.project_type, p.currency, p.rate_cents, p.client_id,
                    ps.budget_scope, ps.monthly_reset, ps.include_nonbillable, ps.rate_mode,
                    ($4::uuid IS NULL OR $5 OR ($6 AND management.id IS NOT NULL)) AS can_read_details,
                    ($4::uuid IS NULL OR $7 OR ($8 AND management.id IS NOT NULL)) AS can_view_money
             FROM projects p JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
             JOIN clients parent ON parent.id=p.client_id AND parent.org_id=p.org_id
             LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
               AND management.project_id=p.id AND management.manager_id=$4
             LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$4
             WHERE p.org_id = $1 AND ($2::uuid IS NULL OR p.id = $2)
               AND ($4::uuid IS NULL OR $5 OR ($6 AND management.id IS NOT NULL)
                 OR (member.id IS NOT NULL AND ps.report_visibility='project_members'))
           ), scopes AS (
             SELECT c.id AS project_id, NULL::uuid AS task_id, NULL::uuid AS user_id,
                    NULL::text AS label,
                    CASE WHEN c.budget_kind = 'hours' THEN c.budget_minutes ELSE c.budget_amount_cents END AS budget
             FROM config c WHERE c.budget_scope = 'project'
             UNION ALL
             SELECT c.id, pt.task_id, NULL::uuid, t.name,
                    CASE WHEN c.budget_kind = 'hours' THEN s.budget_minutes ELSE s.budget_cents END
             FROM config c JOIN project_tasks pt ON pt.project_id = c.id
             JOIN tasks t ON t.id = pt.task_id AND t.org_id = $1
             LEFT JOIN project_task_settings s ON s.project_id = c.id AND s.org_id = $1 AND s.task_id = pt.task_id
             WHERE c.budget_scope = 'task'
             UNION ALL
             SELECT c.id, NULL::uuid, a.user_id, u.name, s.budget_minutes
             FROM config c JOIN assignments a ON a.project_id = c.id
             JOIN users u ON u.id = a.user_id AND u.org_id = $1
             LEFT JOIN project_member_budgets s ON s.project_id = c.id AND s.org_id = $1 AND s.user_id = a.user_id
             WHERE c.budget_scope = 'person' AND c.budget_kind = 'hours'
           ), entries AS (
             SELECT c.id AS project_id, te.task_id, te.user_id,
                    CASE WHEN $9 AND NOT EXISTS (
                        SELECT 1 FROM scopes allocated
                        WHERE allocated.project_id = c.id AND allocated.budget IS NOT NULL
                          AND (allocated.task_id IS NULL OR allocated.task_id = te.task_id)
                          AND (allocated.user_id IS NULL OR allocated.user_id = te.user_id))
                      THEN 0
                      WHEN c.budget_kind = 'hours'
                      THEN effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)::bigint
                      WHEN c.can_view_money THEN COALESCE(line.amount_cents, line_amount_cents(
                        COALESCE(resolve_project_rate(c.rate_mode, pt.rate_cents, a.rate_cents, c.rate_cents,
                          CASE WHEN c.currency = o.default_currency THEN u.billable_rate_cents END,
                          CASE WHEN c.currency = cl.currency THEN cl.default_rate_cents END), 0),
                        effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)))
                      ELSE 0
                    END AS consumed
             FROM config c JOIN time_entries te ON te.project_id = c.id AND te.org_id = $1
             JOIN organizations o ON o.id = te.org_id
             JOIN users u ON u.id = te.user_id AND u.org_id = te.org_id
             JOIN clients cl ON cl.id = c.client_id AND cl.org_id = te.org_id
             JOIN tasks t ON t.id = te.task_id AND t.org_id = te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id = c.id AND pt.task_id = te.task_id
             LEFT JOIN assignments a ON a.project_id = c.id AND a.user_id = te.user_id
             LEFT JOIN invoices invoice ON invoice.id=te.invoice_id AND invoice.org_id=te.org_id
             LEFT JOIN invoice_line_items line ON line.invoice_id = invoice.id AND line.time_entry_id = te.id
             WHERE c.budget_kind <> 'none' AND (NOT c.monthly_reset OR (
                      te.spent_date >= date_trunc('month', $3::date)::date
                      AND te.spent_date < (date_trunc('month', $3::date) + interval '1 month')::date))
               AND (te.invoice_id IS NULL OR invoice.id IS NOT NULL)
               AND (c.include_nonbillable OR c.project_type = 'non_billable'
                    OR (te.billable AND (te.invoice_id IS NOT NULL OR COALESCE(pt.billable, t.billable_default))))
           )
           SELECT c.id AS "project_id!", s.task_id AS "task_id?", s.user_id AS "user_id?",
                  s.label AS "label?", c.budget_scope AS "scope!", c.currency AS "currency!",
                  c.budget_kind AS "kind!: BudgetKind",
                  CASE WHEN c.budget_kind <> 'amount' OR c.can_view_money THEN s.budget END AS "budget?",
                  c.can_view_money AS "can_view_money!", c.can_read_details AS "can_read_details!",
                  CASE WHEN c.monthly_reset THEN to_char($3::date, 'YYYY-MM') ELSE 'lifetime' END AS "period_key!",
                  COALESCE(SUM(e.consumed), 0)::bigint AS "consumed!"
           FROM config c LEFT JOIN scopes s ON s.project_id = c.id
           LEFT JOIN entries e ON e.project_id = c.id
                             AND (s.task_id IS NULL OR e.task_id = s.task_id)
                             AND (s.user_id IS NULL OR e.user_id = s.user_id)
           GROUP BY c.id, c.budget_kind, c.budget_scope, c.currency, c.monthly_reset,
                    s.task_id, s.user_id, s.label, s.budget, c.can_view_money, c.can_read_details
           ORDER BY c.id, s.label, s.task_id, s.user_id"#,
        org_id,
        project_id,
        date as NaiveDate,
        viewer_id,
        has(Permission::ProjectReadAll),
        has(Permission::ProjectReadManaged),
        has(Permission::BillableRateReadAll),
        has(Permission::BillableRateReadManaged),
        allocated_only,
    )
    .fetch_all(connection)
    .await
}
