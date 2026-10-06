//! Project, task, and assignment server functions.

use super::*;
#[cfg(feature = "server")]
use crate::db::{OrganizationLock, lock_organization};
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::{ProjectFeeBalance, ProjectOverview};
#[cfg(feature = "server")]
use crate::models::project::{ProjectOverviewClient, ProjectOverviewRow};
use crate::models::{ProjectDetails, ProjectTagLink, ProjectTaskRate};

mod detail_view;
pub use detail_view::get_project_detail_view;

#[cfg(feature = "server")]
pub(super) mod read_access;
#[cfg(feature = "server")]
use horae_core::permissions::catalog::Permission;

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(all(test, feature = "server"))]
mod privacy_tests;

#[cfg(all(test, feature = "server"))]
mod canonical_read_tests;

#[cfg(all(test, feature = "server"))]
mod details_tests;

#[cfg(all(test, feature = "server"))]
mod mutation_tests;

#[cfg(all(test, feature = "server"))]
mod bulk_tests;

#[cfg(all(test, feature = "server"))]
mod assignment_tests;

// ── Projects ─────────────────────────────────────────────────────────────────

#[server]
pub async fn get_project_fee_balances(
    project_id: String,
    period_from: String,
    period_to: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectFeeBalance>, ServerFnError> {
    let viewer = require_manager().await?;
    project_requester(&viewer, expected_requester)?;
    let state = crate::state::global_state().await;
    fetch_project_fee_balances(
        &state.db,
        viewer.org_id,
        viewer.id,
        parse_uuid(&project_id, "project_id")?,
        (
            parse_date(&period_from, "period_from")?,
            parse_date(&period_to, "period_to")?,
        ),
    )
    .await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_project_fee_balances(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
    (from, to): (chrono::NaiveDate, chrono::NaiveDate),
) -> Result<Vec<ProjectFeeBalance>, ServerFnError> {
    if from > to {
        return Err(err(BAD_REQUEST, "Fee period ends before it starts"));
    }
    let mut tx = super::snapshot::manager(pool, org_id, viewer_id).await?;
    let client_id = sqlx::query_scalar!(
        "SELECT p.client_id FROM projects p
         JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id
         WHERE p.org_id = $1 AND p.id = $2 AND a.user_id = $3
           AND a.can_view_rates AND a.can_view_progress",
        org_id,
        project_id,
        viewer_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))?;
    let lines = super::invoices::fees::preview_fees(
        &mut tx,
        org_id,
        client_id,
        from,
        to,
        Some(&[project_id]),
    )
    .await?;
    let balances = lines
        .into_iter()
        .map(|line| {
            let crate::models::invoice::InvoiceSource::Fee { period_key, .. } = line.source else {
                return Err(server_err("Unexpected time source in project fee balances"));
            };
            Ok(ProjectFeeBalance {
                period_key,
                description: line.description,
                currency: line.currency,
                balance: line
                    .fee_balance
                    .ok_or_else(|| server_err("Missing fee balance"))?,
            })
        })
        .collect::<Result<Vec<_>, ServerFnError>>()?;
    tx.commit().await.map_err(server_err)?;
    Ok(balances)
}

#[server]
pub async fn get_project_details(
    project_id: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectDetails, ServerFnError> {
    let viewer = require_user().await?;
    project_requester(&viewer, expected_requester)?;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let state = crate::state::global_state().await;
    fetch_project_details(&state.db, viewer.org_id, viewer.id, project_id)
        .await?
        .ok_or_else(|| not_found("Project not found"))
}

#[cfg(feature = "server")]
async fn fetch_project_details(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<Option<ProjectDetails>, ServerFnError> {
    let Some(mut access) = read_access::ReadAccess::begin(pool, org_id, viewer_id).await? else {
        return Ok(None);
    };
    let details =
        project_details_in_transaction(&mut access, org_id, viewer_id, project_id).await?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(details)
}

#[cfg(feature = "server")]
async fn project_details_in_transaction(
    access: &mut read_access::ReadAccess<'_>,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<Option<ProjectDetails>, ServerFnError> {
    sqlx::query_as!(
        ProjectDetails,
        r#"SELECT p.id, p.name, p.code, c.name AS client_name, p.currency,
            CASE WHEN (CASE WHEN $4 THEN u.org_role IN ('admin', 'manager')
              ELSE ($8 OR ($9 AND management.id IS NOT NULL))
                AND ($10 OR ($11 AND management.id IS NOT NULL)) END)
              AND p.project_type <> 'non_billable'
              AND (settings.project_id IS NULL
                OR (p.project_type = 'time_and_materials' AND settings.rate_mode = 'task'))
              THEN p.currency END AS task_rate_currency,
            p.starts_on as "starts_on: chrono::NaiveDate",
            p.ends_on as "ends_on: chrono::NaiveDate",
            ARRAY(SELECT t.name FROM project_tag_links l
                JOIN project_tags t ON t.id = l.tag_id AND t.org_id = l.org_id
                WHERE l.project_id = p.id AND l.org_id = p.org_id
                ORDER BY lower(t.name), t.id) as "tags!",
            CASE WHEN (CASE WHEN $4 THEN u.org_role = 'admin' ELSE $7 END)
              THEN private.admin_notes END AS admin_notes
        FROM projects p
        JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
        JOIN users u ON u.id = $2 AND u.org_id = p.org_id AND u.active
        LEFT JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id AND a.user_id = u.id
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = u.id
        LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = u.id
        LEFT JOIN project_private_settings private ON private.project_id = p.id AND private.org_id = p.org_id
        LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
        WHERE p.org_id = $1 AND p.id = $3
          AND CASE WHEN $4 THEN COALESCE(a.can_view_progress, false)
            ELSE $5 OR ($6 AND management.id IS NOT NULL)
              OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members') END"#,
        org_id,
        viewer_id,
        project_id,
        access.legacy(),
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
        access.administrator(),
        access.has(Permission::ProjectWriteAll),
        access.has(Permission::ProjectWriteManaged),
        access.has(Permission::BillableRateWriteAll),
        access.has(Permission::BillableRateWriteManaged),
    )
    .fetch_optional(&mut *access.tx)
    .await
    .map_err(server_err)
}

#[server]
pub async fn list_project_tags(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectTagLink>, ServerFnError> {
    let viewer = require_user().await?;
    project_requester(&viewer, expected_requester)?;
    let state = crate::state::global_state().await;
    fetch_project_tags(&state.db, viewer.org_id, viewer.id).await
}

#[cfg(feature = "server")]
async fn fetch_project_tags(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
) -> Result<Vec<ProjectTagLink>, ServerFnError> {
    let Some(mut access) = read_access::ReadAccess::begin(pool, org_id, viewer_id).await? else {
        return Ok(Vec::new());
    };
    let tags = sqlx::query_as!(
        ProjectTagLink,
        r#"SELECT l.project_id, l.tag_id, t.name
        FROM project_tag_links l
        JOIN project_tags t ON t.id = l.tag_id AND t.org_id = l.org_id
        JOIN projects p ON p.id = l.project_id AND p.org_id = l.org_id
        LEFT JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
        LEFT JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id AND a.user_id = $2
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = $2
        LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
        LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
        WHERE l.org_id = $1 AND ($3 OR c.id IS NOT NULL)
          AND CASE WHEN $3 THEN COALESCE(a.can_view_progress, false)
          ELSE $4 OR ($5 AND management.id IS NOT NULL)
            OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members') END
        ORDER BY lower(t.name), t.id, l.project_id"#,
        org_id,
        viewer_id,
        access.legacy(),
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(tags)
}

#[cfg(feature = "server")]
fn parse_project_rate(value: &str) -> Result<Option<i64>, ServerFnError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let cents = horae_core::money::parse_cents(value)
        .map_err(|_| conflict("Rate must be an amount, e.g. 120 or 120.50"))?;
    if cents < 0 {
        return Err(conflict("Rate cannot be negative"));
    }
    Ok(Some(cents))
}

/// Load overview rows and labels without requiring an unrelated client directory grant.
#[server]
pub async fn get_project_overview(
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    fetch_project_overview(&state.db, &user, expected_requester).await
}

#[cfg(feature = "server")]
async fn fetch_project_overview(
    pool: &sqlx::PgPool,
    viewer: &User,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let requester = project_requester(viewer, expected_requester)?;
    let mut access = read_access::ReadAccess::begin(pool, viewer.org_id, viewer.id)
        .await?
        .ok_or_else(|| forbidden("Current project access is required"))?;
    let canonical_permissions = !access.legacy();
    let role = sqlx::query_scalar!(
        r#"SELECT org_role as "org_role: OrgRole" FROM users WHERE org_id=$1 AND id=$2"#,
        viewer.org_id,
        viewer.id,
    )
    .fetch_one(&mut *access.tx)
    .await
    .map_err(server_err)?;
    let can_create = if canonical_permissions {
        access.has(Permission::ProjectCreateAll)
    } else {
        role.is_manager_or_above()
    };
    let can_import = if canonical_permissions {
        access.administrator()
    } else {
        role == OrgRole::Admin
    };
    let can_change_legacy_status = !canonical_permissions && role.is_manager_or_above();
    let projects = project_rows(&mut access, viewer, None, true, ProjectRead::Overview).await?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(ProjectOverview {
        requester,
        canonical_permissions,
        can_create,
        can_import,
        can_change_legacy_status,
        projects,
    })
}

#[cfg(feature = "server")]
fn project_requester(
    viewer: &User,
    expected_requester: Option<PermissionRequester>,
) -> Result<PermissionRequester, ServerFnError> {
    let requester = PermissionRequester {
        org_id: viewer.org_id,
        user_id: viewer.id,
    };
    if expected_requester.is_some_and(|expected| expected != requester) {
        return Err(forbidden("Project requester has changed"));
    }
    Ok(requester)
}

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    include_inactive: bool,
) -> Result<Vec<Project>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_opt_uuid(client_id, "client_id")?;
    projects_for_viewer(
        &state.db,
        &user,
        client_id,
        include_inactive,
        ProjectRead::Overview,
    )
    .await
}

/// Minimal project identities for starting time and resolving an own history.
/// Reporting visibility must not prevent a teammate from using their timesheet.
#[server]
pub async fn list_tracking_projects() -> Result<Vec<Project>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    projects_for_viewer(&state.db, &user, None, true, ProjectRead::Tracking).await
}

#[cfg(feature = "server")]
enum ProjectRead {
    Overview,
    Tracking,
}

#[cfg(feature = "server")]
async fn projects_for_viewer(
    pool: &sqlx::PgPool,
    viewer: &User,
    client_id: Option<uuid::Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<Project>, ServerFnError> {
    let Some(mut access) = read_access::ReadAccess::begin(pool, viewer.org_id, viewer.id).await?
    else {
        return Ok(Vec::new());
    };
    let rows = project_rows(&mut access, viewer, client_id, include_inactive, purpose).await?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(rows.into_iter().map(|row| row.project).collect())
}

#[cfg(feature = "server")]
async fn project_rows(
    access: &mut read_access::ReadAccess<'_>,
    viewer: &User,
    client_id: Option<uuid::Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<ProjectOverviewRow>, ServerFnError> {
    let overview = matches!(purpose, ProjectRead::Overview);
    let projects = sqlx::query!(
        r#"SELECT p.id, p.org_id, p.client_id, p.code, p.name,
                p.project_type as "project_type: ProjectType", p.currency,
                CASE WHEN $4 AND (CASE WHEN $6 THEN a.can_view_rates
                    ELSE $9 OR ($10 AND management.id IS NOT NULL) END)
                  THEN p.rate_cents END AS rate_cents,
                p.starts_on as "starts_on: chrono::NaiveDate",
                p.ends_on as "ends_on: chrono::NaiveDate",
                CASE WHEN $4 THEN p.budget_kind ELSE 'none'::budget_kind END as "budget_kind!: BudgetKind",
                CASE WHEN $4 AND (CASE WHEN $6 THEN a.can_view_progress
                    ELSE $9 OR ($10 AND management.id IS NOT NULL) END)
                  THEN p.budget_amount_cents END AS budget_amount_cents,
                CASE WHEN $4 AND ($6 OR p.budget_kind = 'hours') THEN p.budget_minutes END AS budget_minutes,
                p.active, p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                CASE WHEN $4 THEN c.name END AS client_name,
                CASE WHEN $4 THEN c.active END AS client_active,
                ($4 AND CASE WHEN $6 THEN u.org_role IN ('admin', 'manager')
                  ELSE $12 OR ($13 AND management.id IS NOT NULL) END) AS "can_edit!"
         FROM projects p
         JOIN users u ON u.org_id = p.org_id AND u.id = $3 AND u.active
         LEFT JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         LEFT JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id AND a.user_id = $3
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $3
         LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $3
         LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
         WHERE p.org_id = $2 AND ($6 OR c.id IS NOT NULL)
           AND ($1::bool OR p.active) AND ($5::uuid IS NULL OR p.client_id = $5)
           AND CASE WHEN $6 THEN a.user_id IS NOT NULL AND (NOT $4 OR a.can_view_progress)
             WHEN $4 THEN $7 OR ($8 AND management.id IS NOT NULL)
               OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members')
             ELSE member.id IS NOT NULL OR $11 OR EXISTS (
               SELECT 1 FROM time_entries te WHERE te.org_id = p.org_id
                 AND te.project_id = p.id AND te.user_id = $3) END
         ORDER BY p.name, p.id"#,
        include_inactive,
        viewer.org_id,
        viewer.id,
        overview,
        client_id,
        access.legacy(),
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
        access.has(Permission::BillableRateReadAll),
        access.has(Permission::BillableRateReadManaged),
        access.administrator(),
        access.has(Permission::ProjectWriteAll),
        access.has(Permission::ProjectWriteManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;

    Ok(projects
        .into_iter()
        .map(|row| ProjectOverviewRow {
            client: match (row.client_name, row.client_active) {
                (Some(name), Some(active)) => Some(ProjectOverviewClient {
                    id: row.client_id,
                    name,
                    active,
                }),
                _ => None,
            },
            can_edit: row.can_edit,
            project: Project {
                id: row.id,
                org_id: row.org_id,
                client_id: row.client_id,
                code: row.code,
                name: row.name,
                project_type: row.project_type,
                currency: row.currency,
                rate_cents: row.rate_cents,
                starts_on: row.starts_on,
                ends_on: row.ends_on,
                budget_kind: row.budget_kind,
                budget_amount_cents: row.budget_amount_cents,
                budget_minutes: row.budget_minutes,
                active: row.active,
                created_at: row.created_at,
            },
        })
        .collect())
}

/// Per-project tracked totals for the overview's Spent column: every project's
/// total logged minutes plus its billable amount, with each entry's rate resolved
/// through the FR-024 cascade (task → assignment → project → user default) and summed.
/// Only projects whose progress the viewer may read; never per-user time or rates.
#[server]
pub async fn list_project_spend(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let user = require_user().await?;
    project_requester(&user, expected_requester)?;
    let state = crate::state::global_state().await;

    fetch_project_spend(&state.db, user.org_id, user.id).await
}

/// Configured budgets use their own period and scope; tracked totals remain
/// available separately through `list_project_spend`.
#[server]
pub async fn list_project_budget_progress(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<crate::models::ProjectBudgetOverview>, ServerFnError> {
    let user = require_user().await?;
    project_requester(&user, expected_requester)?;
    let state = crate::state::global_state().await;
    super::budgets::progress_for_viewer(
        &state.db,
        user.org_id,
        user.id,
        chrono::Utc::now().date_naive(),
    )
    .await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_project_spend(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let Some(mut access) = read_access::ReadAccess::begin(pool, org_id, viewer_id).await? else {
        return Ok(Vec::new());
    };
    // Grouped in Postgres, not folded here: the overview needs one number per
    // project, and folding in Rust meant fetching one row per time entry to get
    // there. SQL rate resolution preserves legacy precedence for projects
    // without settings. Attached invoice amounts take priority over live rates.
    // The joins cannot multiply rows: project
    // tasks, assignments, and invoice lines each have a unique pair key.
    let spend = sqlx::query_as!(
        ProjectSpend,
        r#"WITH visible AS (
             SELECT p.*,
               CASE WHEN $3 THEN COALESCE(legacy.can_view_progress, false)
                 ELSE $6 OR ($7 AND management.id IS NOT NULL) END AS can_view_money
             FROM projects p
             JOIN clients parent ON parent.id=p.client_id AND parent.org_id=p.org_id
             LEFT JOIN project_read_access legacy ON legacy.org_id=p.org_id
               AND legacy.project_id=p.id AND legacy.user_id=$2
             LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
               AND management.project_id=p.id AND management.manager_id=$2
             LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
             LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
             WHERE p.org_id=$1 AND CASE WHEN $3 THEN COALESCE(legacy.can_view_progress, false)
               ELSE $4 OR ($5 AND management.id IS NOT NULL)
                 OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members')='project_members') END
           ), entries AS (
             SELECT p.id AS project_id, te.minutes,
               CASE WHEN p.can_view_money AND te.billable
                 AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                   AND COALESCE(pt.billable, t.billable_default)))
               THEN COALESCE(line.amount_cents, line_amount_cents(
                 COALESCE(CASE WHEN ps.project_id IS NULL OR p.project_type = 'time_and_materials'
                   THEN resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                   CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                   CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
                 ) END, 0),
                 effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)
               )) END AS amount_cents
           FROM visible p
           JOIN time_entries te ON te.project_id=p.id AND te.org_id=p.org_id
           JOIN clients c ON c.id = p.client_id AND c.org_id=te.org_id
           LEFT JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id=te.org_id
           JOIN tasks t ON t.id = te.task_id AND t.org_id=te.org_id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           LEFT JOIN invoices invoice ON invoice.id=te.invoice_id AND invoice.org_id=te.org_id
           LEFT JOIN invoice_line_items line ON line.invoice_id = invoice.id AND line.time_entry_id = te.id
           JOIN users u ON u.id = te.user_id AND u.org_id=te.org_id
           JOIN organizations o ON o.id = te.org_id
           WHERE te.invoice_id IS NULL OR invoice.id IS NOT NULL
           )
           SELECT p.id AS "project_id!",
             COALESCE(SUM(e.minutes),0)::bigint AS "spent_minutes!",
             CASE WHEN p.can_view_money THEN COALESCE(SUM(e.amount_cents),0)::bigint END AS "spent_cents?"
           FROM visible p LEFT JOIN entries e ON e.project_id=p.id
           GROUP BY p.id,p.can_view_money
           ORDER BY p.id"#,
        org_id,
        viewer_id,
        access.legacy(),
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
        access.has(Permission::BillableRateReadAll),
        access.has(Permission::BillableRateReadManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;

    access.tx.commit().await.map_err(server_err)?;
    Ok(spend)
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    project_requester(&manager, expected_requester)?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let (project, transition) =
        set_project_active_record(&state.db, manager.org_id, project_id, active).await?;
    dispatch_project_status(state, manager.org_id, &project, transition);
    Ok(project)
}

/// Set up to 100 projects to the requested status in one organization-scoped transaction.
#[server]
pub async fn set_projects_active(
    project_ids: Vec<String>,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<Project>, ServerFnError> {
    let manager = require_manager().await?;
    project_requester(&manager, expected_requester)?;
    let ids = parse_bulk_project_ids(&project_ids)?;
    let state = crate::state::global_state().await;
    let results = set_projects_active_records(&state.db, manager.org_id, &ids, active).await?;
    Ok(results
        .into_iter()
        .map(|(project, transition)| {
            dispatch_project_status(state, manager.org_id, &project, transition);
            project
        })
        .collect())
}

#[cfg(feature = "server")]
fn parse_bulk_project_ids(values: &[String]) -> Result<Vec<uuid::Uuid>, ServerFnError> {
    if values.is_empty() || values.len() > 100 {
        return Err(err(BAD_REQUEST, "Select between 1 and 100 projects"));
    }
    let mut ids = values
        .iter()
        .map(|value| {
            uuid::Uuid::parse_str(value).map_err(|_| err(BAD_REQUEST, "Invalid project ID"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Overlapping batches acquire their row locks in one global order.
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

#[cfg(feature = "server")]
fn dispatch_project_status(
    state: &crate::state::AppState,
    org_id: uuid::Uuid,
    project: &Project,
    transition: Option<crate::plugin::event::ActiveTransition>,
) {
    if let Some(t) = transition {
        let occurred_at = chrono::Utc::now();
        let project = project_payload(project);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::ProjectReactivated {
                    occurred_at,
                    org_id,
                    project,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::ProjectDeactivated {
                    occurred_at,
                    org_id,
                    project,
                }
            }
        });
    }
}

#[cfg(feature = "server")]
async fn set_projects_active_records(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    sorted_ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<(Project, Option<crate::plugin::event::ActiveTransition>)>, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let mut results = Vec::with_capacity(sorted_ids.len());
    for &id in sorted_ids {
        results.push(set_project_active_in_transaction(&mut tx, org_id, id, active).await?);
    }
    tx.commit().await.map_err(server_err)?;
    Ok(results)
}

#[cfg(feature = "server")]
async fn set_project_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    active: bool,
) -> Result<(Project, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let result = set_project_active_in_transaction(&mut tx, org_id, project_id, active).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

#[cfg(feature = "server")]
async fn set_project_active_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    active: bool,
) -> Result<(Project, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let before = lock_project(tx, org_id, project_id).await?;

    let project = sqlx::query_as!(
        Project,
        r#"UPDATE projects SET active = $3
          WHERE id = $1 AND org_id = $2
            AND active IS DISTINCT FROM $3
         RETURNING id, org_id, client_id, code, name,
                   project_type as "project_type: ProjectType", currency, rate_cents,
                   starts_on as "starts_on: chrono::NaiveDate",
                   ends_on as "ends_on: chrono::NaiveDate",
                   budget_kind as "budget_kind: BudgetKind",
                   budget_amount_cents, budget_minutes, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        project_id,
        org_id,
        active,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?;

    let transition = project.as_ref().and_then(|updated| {
        crate::plugin::event::active_transition(Some(before.active), updated.active)
    });
    Ok((project.unwrap_or(before), transition))
}

#[cfg(feature = "server")]
async fn lock_project(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<Project, ServerFnError> {
    // Lock before comparing so competing edits and activation changes use
    // the latest committed row even when the request initially looked unchanged.
    sqlx::query_as!(
        Project,
        r#"SELECT id, org_id, client_id, code, name,
                  project_type as "project_type: ProjectType", currency, rate_cents,
                  starts_on as "starts_on: chrono::NaiveDate",
                  ends_on as "ends_on: chrono::NaiveDate",
                  budget_kind as "budget_kind: BudgetKind",
                  budget_amount_cents, budget_minutes, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM projects WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        project_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))
}

// ── Tasks ────────────────────────────────────────────────────────────────────

/// Lists all active org-level tasks.
#[server]
pub async fn list_tasks() -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    tasks_for_viewer(&state.db, &user, None, TaskRead::Catalog).await
}

/// Rate-free task identities, including archived tasks in the viewer's history.
#[server]
pub async fn list_tracking_tasks() -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    tasks_for_viewer(&state.db, &user, None, TaskRead::Tracking).await
}

/// Lists tasks linked to a specific project via the `project_tasks` join table.
#[server]
pub async fn list_project_tasks(project_id: String) -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    tasks_for_viewer(&state.db, &user, Some(project_id), TaskRead::Catalog).await
}

#[cfg(feature = "server")]
enum TaskRead {
    Catalog,
    Tracking,
}

#[cfg(feature = "server")]
async fn tasks_for_viewer(
    pool: &sqlx::PgPool,
    viewer: &User,
    project_id: Option<uuid::Uuid>,
    purpose: TaskRead,
) -> Result<Vec<Task>, ServerFnError> {
    let tracking = matches!(purpose, TaskRead::Tracking);
    let Some(mut access) = read_access::ReadAccess::begin(pool, viewer.org_id, viewer.id).await?
    else {
        return Ok(Vec::new());
    };
    let rows = if access.legacy() {
        sqlx::query_as!(
        Task,
        "SELECT t.id, t.org_id, t.name, t.billable_default, t.active,
                CASE WHEN NOT $4 AND access.can_view_rates THEN t.default_rate_cents END AS default_rate_cents
         FROM tasks t JOIN task_read_access access ON access.task_id = t.id AND access.org_id = t.org_id
         WHERE t.org_id = $2 AND access.user_id = $1
           AND (t.active OR ($4 AND access.has_own_history))
           AND ($3::uuid IS NULL OR EXISTS (
             SELECT 1 FROM project_tasks pt JOIN project_read_access a ON a.project_id = pt.project_id
             WHERE pt.task_id = t.id AND pt.project_id = $3 AND a.org_id = $2 AND a.user_id = $1 AND a.can_view_team))
         ORDER BY t.name, t.id",
        viewer.id,
        viewer.org_id,
        project_id,
        tracking,
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?
    } else {
        sqlx::query_as!(
            Task,
            "SELECT t.id,t.org_id,t.name,t.billable_default,t.active,
               CASE WHEN NOT $4 AND $6 THEN t.default_rate_cents END AS default_rate_cents
             FROM tasks t
             CROSS JOIN LATERAL (
               SELECT EXISTS (
                 SELECT 1 FROM time_entries te
                 JOIN projects p ON p.id=te.project_id AND p.org_id=te.org_id
                 JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
                 WHERE te.org_id=t.org_id AND te.task_id=t.id AND te.user_id=$1
               ) AS own_history,
               EXISTS (
                 SELECT 1 FROM project_tasks pt
                 JOIN projects p ON p.id=pt.project_id AND p.org_id=t.org_id
                 JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
                 JOIN assignments member ON member.project_id=p.id AND member.user_id=$1
                 WHERE pt.task_id=t.id
               ) AS member_task
             ) context
             WHERE t.org_id=$2 AND (t.active OR ($4 AND context.own_history))
               AND CASE WHEN $4 THEN $5 OR context.own_history OR context.member_task
                 WHEN $3::uuid IS NULL THEN $5 ELSE true END
               AND ($3::uuid IS NULL OR EXISTS (
                 SELECT 1 FROM project_tasks pt
                 JOIN projects p ON p.id=pt.project_id AND p.org_id=t.org_id
                 JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
                 LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
                   AND management.project_id=p.id AND management.manager_id=$1
                 LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$1
                 WHERE pt.task_id=t.id AND p.id=$3
                   AND ($7 OR ($8 AND management.id IS NOT NULL) OR member.id IS NOT NULL)
               ))
             ORDER BY t.name,t.id",
            viewer.id,
            viewer.org_id,
            project_id,
            tracking,
            access.has(Permission::TaskReadAll),
            access.has(Permission::BillableRateReadAll),
            access.has(Permission::ProjectReadAll),
            access.has(Permission::ProjectReadManaged),
        )
        .fetch_all(&mut *access.tx)
        .await
        .map_err(server_err)?
    };
    access.tx.commit().await.map_err(server_err)?;
    Ok(rows)
}

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
    project_id: Option<String>,
) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_opt_uuid(project_id, "project_id")?;
    let task = create_task_for_project(
        &state.db,
        manager.org_id,
        manager.id,
        &name,
        billable_default,
        project_id,
    )
    .await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TaskCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            task: task_payload(&task),
        });
    Ok(task)
}

#[cfg(feature = "server")]
async fn create_task_for_project(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    project_id: Option<uuid::Uuid>,
) -> Result<Task, ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let mut tx = db.begin().await.map_err(server_err)?;
    let gate = if project_id.is_some() {
        OrganizationLock::AccessChange
    } else {
        OrganizationLock::Shared
    };
    project_creation::lock_creation_actor(&mut tx, actor_id, org_id, gate).await?;
    let id = uuid::Uuid::now_v7();
    let task = sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default)
         VALUES ($1, $2, $3, $4)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        id,
        org_id,
        name,
        billable_default,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;

    if let Some(project_id) = project_id {
        enable_project_task(&mut tx, org_id, project_id, task.id, None).await?;
    }
    tx.commit().await.map_err(server_err)?;
    Ok(task)
}

#[server]
pub async fn update_task(
    task_id: String,
    name: String,
    billable_default: bool,
    default_rate_cents: Option<i64>,
) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, changed) = update_task_record(
        &state.db,
        manager.org_id,
        task_id,
        &name,
        billable_default,
        default_rate_cents,
    )
    .await?;
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TaskUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                task: task_payload(&task),
            });
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn update_task_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    default_rate_cents: Option<i64>,
) -> Result<(Task, bool), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_task(&mut tx, org_id, task_id).await?;

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks
            SET name = $3, billable_default = $4, default_rate_cents = $5,
                default_rate_currency = CASE
                  WHEN default_rate_cents IS NOT DISTINCT FROM $5 THEN default_rate_currency
                  WHEN $5::bigint IS NULL THEN NULL
                  ELSE (SELECT upper(btrim(default_currency)) FROM organizations WHERE id = $2) END
          WHERE id = $1 AND org_id = $2
            AND (name, billable_default, default_rate_cents) IS DISTINCT FROM ($3, $4, $5)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        name,
        billable_default,
        default_rate_cents,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let changed = task.is_some();
    tx.commit().await.map_err(server_err)?;
    Ok((task.unwrap_or(before), changed))
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(task_id: String, active: bool) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, transition) =
        set_task_active_record(&state.db, manager.org_id, task_id, active).await?;
    if let Some(t) = transition {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::TaskReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::TaskDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
        });
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn set_task_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<(Task, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_task(&mut tx, org_id, task_id).await?;

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks SET active = $3
          WHERE id = $1 AND org_id = $2
            AND active IS DISTINCT FROM $3
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        active,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let transition = task.as_ref().and_then(|updated| {
        crate::plugin::event::active_transition(Some(before.active), updated.active)
    });
    tx.commit().await.map_err(server_err)?;
    Ok((task.unwrap_or(before), transition))
}

#[cfg(feature = "server")]
async fn lock_task(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
) -> Result<Task, ServerFnError> {
    // Share the row lock between detail edits and activation changes, including no-ops.
    sqlx::query_as!(
        Task,
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks WHERE id = $1 AND org_id = $2 FOR UPDATE",
        task_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Task not found"))
}

/// Enable an org-level task on a project so it becomes loggable there. The
/// project-task link inherits the task's default billable flag; idempotent.
/// Both the project and the task must belong to the manager's organization.
#[server]
pub async fn link_project_task(
    project_id: String,
    task_id: String,
    rate: Option<ProjectTaskRate>,
) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;

    link_project_task_record(
        &state.db,
        manager.org_id,
        manager.id,
        project_id,
        task_id,
        rate.as_ref(),
    )
    .await
}

#[cfg(feature = "server")]
async fn link_project_task_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    rate: Option<&ProjectTaskRate>,
) -> Result<(), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    project_creation::lock_creation_actor(
        &mut tx,
        actor_id,
        org_id,
        OrganizationLock::AccessChange,
    )
    .await?;
    enable_project_task(&mut tx, org_id, project_id, task_id, rate).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(())
}

#[cfg(feature = "server")]
async fn enable_project_task(
    db: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    explicit_rate: Option<&ProjectTaskRate>,
) -> Result<(), ServerFnError> {
    if !lock_project_revision(db, org_id, project_id).await? {
        return Err(not_found(
            "Active project and task not found in this organization",
        ));
    }
    // Validate before the idempotent insert, including already-linked pairs.
    // Hold these rows until commit so archiving cannot race task enablement.
    // Preserve legacy catalog inheritance, including non-billable projects,
    // while accepting explicit overrides only where billing uses them.
    let task = sqlx::query!(
        r#"SELECT t.billable_default, p.currency, t.default_rate_currency,
                (ps.project_id IS NOT NULL) AS "configured!",
                (p.project_type <> 'non_billable' AND (ps.project_id IS NULL
                  OR (p.project_type = 'time_and_materials' AND ps.rate_mode = 'task'))) AS "uses_task_rates!",
                EXISTS(SELECT 1 FROM project_tasks pt
                       WHERE pt.project_id = p.id AND pt.task_id = t.id) AS "linked!",
                CASE WHEN ps.project_id IS NULL
                       OR (p.project_type = 'time_and_materials' AND ps.rate_mode = 'task')
                     THEN t.default_rate_cents ELSE NULL END AS default_rate_cents
         FROM projects p JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         JOIN tasks t ON t.org_id = p.org_id
         LEFT JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
         WHERE p.id = $1 AND t.id = $2 AND p.org_id = $3
           AND p.active AND c.active AND t.active
         FOR SHARE OF c, t"#,
        project_id,
        task_id,
        org_id,
    )
    .fetch_optional(&mut *db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Active project and task not found in this organization"))?;
    if task.linked {
        return Ok(());
    }
    let rate_cents = if let Some(rate) = explicit_rate {
        if !task.uses_task_rates {
            return Err(conflict(
                "This project's billing mode does not use task rates",
            ));
        }
        if !rate
            .currency
            .trim()
            .eq_ignore_ascii_case(task.currency.trim())
        {
            return Err(conflict(
                "Project currency changed. Reload the project and enter its task rate again",
            ));
        }
        Some(
            parse_project_rate(&rate.amount)?
                .ok_or_else(|| conflict("Enter an explicit rate in the project currency"))?,
        )
    } else {
        if task.configured
            && task.default_rate_cents.is_some()
            && !task
                .default_rate_currency
                .as_deref()
                .is_some_and(|source| source.eq_ignore_ascii_case(task.currency.trim()))
        {
            return Err(conflict(
                "Task rate: currency is unknown or incompatible; enter an explicit rate in the project currency",
            ));
        }
        task.default_rate_cents
    };
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
         VALUES ($1, $2, $3, $4) ON CONFLICT (project_id, task_id) DO NOTHING",
        project_id,
        task_id,
        task.billable_default,
        rate_cents,
    )
    .execute(db)
    .await
    .map_err(server_err)?;
    Ok(())
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_assignments(
    project_id: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<Assignment>, ServerFnError> {
    let viewer = require_user().await?;
    project_requester(&viewer, expected_requester)?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    assignments_for_viewer(&state.db, &viewer, project_id).await
}

#[cfg(feature = "server")]
async fn assignments_for_viewer(
    db: &sqlx::PgPool,
    viewer: &User,
    project_id: uuid::Uuid,
) -> Result<Vec<Assignment>, ServerFnError> {
    let Some(mut access) = read_access::ReadAccess::begin(db, viewer.org_id, viewer.id).await?
    else {
        return Ok(Vec::new());
    };
    let team = assignment_rows(&mut access, viewer, project_id).await?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(team)
}

#[cfg(feature = "server")]
async fn assignment_rows(
    access: &mut read_access::ReadAccess<'_>,
    viewer: &User,
    project_id: uuid::Uuid,
) -> Result<Vec<Assignment>, ServerFnError> {
    sqlx::query_as!(
        Assignment,
        r#"SELECT a.id, a.project_id, a.user_id, a.role as "role: ProjectRole",
                CASE WHEN (CASE WHEN $4 THEN legacy.can_view_rates
                    ELSE $7 OR ($8 AND management.id IS NOT NULL) END)
                  THEN a.rate_cents END AS rate_cents,
                a.created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM assignments a JOIN projects p ON p.id = a.project_id
         JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         JOIN users member ON member.id = a.user_id AND member.org_id = p.org_id
         LEFT JOIN project_read_access legacy ON legacy.project_id = p.id
           AND legacy.org_id = p.org_id AND legacy.user_id = $3
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $3
         WHERE a.project_id = $1 AND p.org_id = $2
           AND CASE WHEN $4 THEN COALESCE(legacy.can_view_team, false)
             ELSE $5 OR ($6 AND management.id IS NOT NULL) OR a.user_id = $3 END
         ORDER BY a.created_at, a.id"#,
        project_id,
        viewer.org_id,
        viewer.id,
        access.legacy(),
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
        access.has(Permission::BillableRateReadAll),
        access.has(Permission::BillableRateReadManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)
}

#[server]
pub async fn create_assignment(
    project_id: String,
    user_id: String,
    role: String,
) -> Result<Assignment, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let pr: ProjectRole = parse_enum(&role, "role")?;
    let assignment = insert_assignment(&state.db, &admin, project_id, user_id, pr).await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserAssignedToProject {
            occurred_at: chrono::Utc::now(),
            org_id: admin.org_id,
            assignment: assignment_payload(&assignment),
        });
    Ok(assignment)
}

#[cfg(feature = "server")]
pub(super) async fn insert_assignment(
    db: &sqlx::PgPool,
    admin: &User,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
    role: ProjectRole,
) -> Result<Assignment, ServerFnError> {
    let mut tx = begin_assignment_change(db, admin).await?;
    if !lock_project_revision(&mut tx, admin.org_id, project_id).await? {
        return Err(not_found("Project or person not found"));
    }
    sqlx::query_scalar!(
        "SELECT p.id FROM projects p JOIN users u ON u.org_id = p.org_id
         WHERE p.id = $1 AND u.id = $2 AND p.org_id = $3 FOR SHARE OF u",
        project_id,
        user_id,
        admin.org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project or person not found"))?;
    let id = uuid::Uuid::now_v7();
    let assignment = sqlx::query_as!(
        Assignment,
        r#"INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        project_id,
        user_id,
        role as ProjectRole,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(assignment)
}

#[server]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&assignment_id, "assignment_id")?;
    let removed = remove_assignment(&state.db, &admin, id).await?;

    if let Some(a) = removed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::AssignmentRemoved {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                assignment: assignment_payload(&a),
            });
    }
    Ok(())
}

#[cfg(feature = "server")]
pub(super) async fn remove_assignment(
    db: &sqlx::PgPool,
    admin: &User,
    id: uuid::Uuid,
) -> Result<Option<Assignment>, ServerFnError> {
    let mut tx = begin_assignment_change(db, admin).await?;
    let project_id = sqlx::query_scalar!(
        "SELECT a.project_id FROM assignments a
         JOIN projects p ON p.id=a.project_id JOIN users u ON u.id=a.user_id
         WHERE a.id=$1 AND p.org_id=$2 AND u.org_id=$2",
        id,
        admin.org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    let Some(project_id) = project_id else {
        tx.commit().await.map_err(server_err)?;
        return Ok(None);
    };
    if !lock_project_revision(&mut tx, admin.org_id, project_id).await? {
        tx.commit().await.map_err(server_err)?;
        return Ok(None);
    }
    // Delete and capture the row atomically so the event carries its details
    // and a concurrent delete cannot double-notify.
    let removed = sqlx::query_as!(
        Assignment,
        r#"DELETE FROM assignments a USING projects p, users u
         WHERE a.id = $1 AND a.project_id = p.id AND a.user_id = u.id
           AND p.org_id = $2 AND u.org_id = $2 AND a.project_id = $3
         RETURNING a.id, a.project_id, a.user_id, a.role as "role: ProjectRole", a.rate_cents,
                   a.created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        admin.org_id,
        project_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(removed)
}

#[cfg(feature = "server")]
async fn begin_assignment_change<'a>(
    db: &'a sqlx::PgPool,
    admin: &User,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    lock_organization(&mut tx, admin.org_id, OrganizationLock::AccessChange)
        .await
        .map_err(server_err)?;
    // Session admission can predate a demotion. Hold the current actor row
    // until commit so a completed revocation cannot leave a stale writer.
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND org_id = $2
           AND active AND org_role = $3 FOR SHARE",
        admin.id,
        admin.org_id,
        OrgRole::Admin as OrgRole,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| forbidden("Active administrator access required"))?;
    Ok(tx)
}

#[cfg(feature = "server")]
async fn lock_project_revision(
    db: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<bool, ServerFnError> {
    // Child revision triggers write this parent. Permit entry/invoice FK checks
    // to finish before any dependent membership cascade waits for those writers.
    sqlx::query_scalar!(
        "SELECT id FROM projects WHERE id=$1 AND org_id=$2 FOR NO KEY UPDATE",
        project_id,
        org_id,
    )
    .fetch_optional(db)
    .await
    .map(|row| row.is_some())
    .map_err(server_err)
}
