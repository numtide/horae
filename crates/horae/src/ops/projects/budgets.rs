//! Budget progress as project readers see it: the Projects page's batch and
//! the detail page's headline. The progress query itself is shared with the
//! alert check in `server_fns::budgets`.

use chrono::NaiveDate;
use horae_core::types::BudgetKind;
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use crate::models::project::ProjectDetailBudget;
use crate::models::{ProjectBudgetOverview, ProjectBudgetProgress};
use crate::server_fns::budgets::{fetch_progress, numeric_overflow};

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
