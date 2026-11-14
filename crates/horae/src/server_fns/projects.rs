//! Project, task, and assignment server functions.

use super::*;
#[cfg(feature = "server")]
use crate::db::OrganizationLock;
#[cfg(all(test, feature = "server"))]
use crate::db::lock_organization;
#[cfg(feature = "server")]
use crate::models::ProjectDetails;
use crate::models::ProjectTagLink;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::ProjectOverview;
#[cfg(feature = "server")]
use crate::models::project::{ProjectOverviewClient, ProjectOverviewRow};
use crate::models::task::TaskRateEdit;

mod detail_view;
pub use detail_view::get_project_detail_view;

mod task_catalog;
pub use task_catalog::load_task_catalog;

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
mod client_lifecycle_tests;

#[cfg(all(test, feature = "server"))]
mod task_write_tests;

#[cfg(all(test, feature = "server"))]
mod task_edit_tests;

#[cfg(all(test, feature = "server"))]
mod task_activity_tests;

#[cfg(all(test, feature = "server"))]
mod common_task_tests;

// ── Projects ─────────────────────────────────────────────────────────────────

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
            CASE WHEN ($6 OR ($7 AND management.id IS NOT NULL))
                AND ($8 OR ($9 AND management.id IS NOT NULL))
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
            CASE WHEN $6 THEN private.admin_notes END AS admin_notes
        FROM projects p
        JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = $2
        LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
        LEFT JOIN project_private_settings private ON private.project_id = p.id AND private.org_id = p.org_id
        LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
        WHERE p.org_id = $1 AND p.id = $3
          AND ($4 OR ($5 AND management.id IS NOT NULL)
            OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members'))"#,
        org_id,
        viewer_id,
        project_id,
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
        // Private notes belong to whoever may edit every project.
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
        JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = $2
        LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
        LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
        WHERE l.org_id = $1
          AND ($3 OR ($4 AND management.id IS NOT NULL)
            OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members'))
        ORDER BY lower(t.name), t.id, l.project_id"#,
        org_id,
        viewer_id,
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(tags)
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
    let overview = ProjectOverview {
        requester,
        can_create: access.has(Permission::ProjectCreateAll),
        can_import: access.administrator(),
        can_change_status: access.has(Permission::ProjectWriteAll)
            || access.has(Permission::ProjectWriteManaged),
        // The Clients page's `ClientReadScope::can_read`.
        can_open_clients: access.has(Permission::ClientReadAll),
        projects: project_rows(&mut access, viewer, None, true, ProjectRead::Overview).await?,
    };
    access.tx.commit().await.map_err(server_err)?;
    Ok(overview)
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
                CASE WHEN $4 AND ($8 OR ($9 AND management.id IS NOT NULL))
                  THEN p.rate_cents END AS rate_cents,
                p.starts_on as "starts_on: chrono::NaiveDate",
                p.ends_on as "ends_on: chrono::NaiveDate",
                CASE WHEN $4 THEN p.budget_kind ELSE 'none'::budget_kind END as "budget_kind!: BudgetKind",
                CASE WHEN $4 AND ($8 OR ($9 AND management.id IS NOT NULL))
                  THEN p.budget_amount_cents END AS budget_amount_cents,
                CASE WHEN $4 AND p.budget_kind = 'hours' THEN p.budget_minutes END AS budget_minutes,
                p.active, p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                CASE WHEN $4 THEN c.name END AS client_name,
                CASE WHEN $4 THEN c.active END AS client_active,
                ($4 AND ($11 OR ($12 AND management.id IS NOT NULL))) AS "can_edit!"
         FROM projects p
         JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $3
         LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $3
         LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
         WHERE p.org_id = $2
           AND ($1::bool OR p.active) AND ($5::uuid IS NULL OR p.client_id = $5)
           AND CASE WHEN $4 THEN $6 OR ($7 AND management.id IS NOT NULL)
               OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members')
             ELSE member.id IS NOT NULL OR $10 OR EXISTS (
               SELECT 1 FROM time_entries te WHERE te.org_id = p.org_id
                 AND te.project_id = p.id AND te.user_id = $3) END
         ORDER BY p.name, p.id"#,
        include_inactive,
        viewer.org_id,
        viewer.id,
        overview,
        client_id,
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
    // Budget months are the workspace's, so every viewer sees the same period.
    let today = crate::timezone::today_for_org(&state.db, user.org_id)
        .await
        .map_err(server_err)?;
    super::budgets::progress_for_viewer(&state.db, user.org_id, user.id, today).await
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
    let spend = spend_in_transaction(
        &mut access.tx,
        org_id,
        viewer_id,
        &access.permissions,
        None,
        true,
    )
    .await
    .map_err(server_err)?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(spend)
}

/// Tracked totals for every readable project, or only `project_id`, inside the
/// caller's authority transaction. Without `include_money` no amount is valued
/// and `spent_cents` is absent.
#[cfg(feature = "server")]
pub(super) async fn spend_in_transaction(
    connection: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    permissions: &crate::models::permissions::PersonPermissions,
    project_id: Option<uuid::Uuid>,
    include_money: bool,
) -> Result<Vec<ProjectSpend>, sqlx::Error> {
    let has = |permission| permissions.grants.contains(permission);
    // Grouped in Postgres, not folded here: the overview needs one number per
    // project, and folding in Rust meant fetching one row per time entry to get
    // there. SQL rate resolution preserves legacy precedence for projects
    // without settings. Attached invoice amounts take priority over live rates.
    // The joins cannot multiply rows: project
    // tasks, assignments, and invoice lines each have a unique pair key.
    //
    // Entries are summed per project inside a MATERIALIZED CTE so the scan runs
    // once. Left inlined, a low estimate for `visible` (stale or missing
    // statistics) put the whole entry join on the inner side of a nested loop,
    // re-running it once per visible project: seconds instead of milliseconds.
    sqlx::query_as!(
        ProjectSpend,
        r#"WITH visible AS (
             SELECT p.*,
               ($8 AND ($5 OR ($6 AND management.id IS NOT NULL))) AS can_view_money
             FROM projects p
             JOIN clients parent ON parent.id=p.client_id AND parent.org_id=p.org_id
             LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
               AND management.project_id=p.id AND management.manager_id=$2
             LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
             LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
             WHERE p.org_id=$1 AND ($7::uuid IS NULL OR p.id=$7)
               AND ($3 OR ($4 AND management.id IS NOT NULL)
                 OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members')='project_members'))
           ), spent AS MATERIALIZED (
             SELECT p.id AS project_id, SUM(te.minutes) AS minutes,
               SUM(CASE WHEN p.can_view_money AND te.billable
                 AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                   AND COALESCE(pt.billable, t.billable_default)))
               THEN COALESCE(line.amount_cents, line_amount_cents(
                 COALESCE(CASE WHEN ps.project_id IS NULL OR p.project_type = 'time_and_materials'
                   THEN resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                   CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                   CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
                 ) END, 0),
                 effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)
               )) END) AS amount_cents
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
           GROUP BY p.id
           )
           SELECT p.id AS "project_id!",
             COALESCE(s.minutes,0)::bigint AS "spent_minutes!",
             CASE WHEN p.can_view_money THEN COALESCE(s.amount_cents,0)::bigint END AS "spent_cents?"
           FROM visible p LEFT JOIN spent s ON s.project_id=p.id
           ORDER BY p.id"#,
        org_id,
        viewer_id,
        has(Permission::ProjectReadAll),
        has(Permission::ProjectReadManaged),
        has(Permission::BillableRateReadAll),
        has(Permission::BillableRateReadManaged),
        project_id,
        include_money,
    )
    .fetch_all(connection)
    .await
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Project, ServerFnError> {
    let user = require_user().await?;
    project_requester(&user, expected_requester)?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let mut projects = set_projects_status(
        &state.db,
        &*state.plugins,
        user.org_id,
        user.id,
        &[project_id],
        active,
    )
    .await?;
    projects
        .pop()
        .ok_or_else(|| server_err("Project status change returned no project"))
}

/// Set up to 100 projects to the requested status in one organization-scoped transaction.
#[server]
pub async fn set_projects_active(
    project_ids: Vec<String>,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<Project>, ServerFnError> {
    let user = require_user().await?;
    project_requester(&user, expected_requester)?;
    let ids = parse_bulk_project_ids(&project_ids)?;
    let state = crate::state::global_state().await;
    set_projects_status(
        &state.db,
        &*state.plugins,
        user.org_id,
        user.id,
        &ids,
        active,
    )
    .await
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

/// Archives or restores `sorted_ids` in one transaction through
/// `ops::projects::set_active`, dispatching its events after the commit.
#[cfg(feature = "server")]
async fn set_projects_status(
    db: &sqlx::PgPool,
    events: &dyn crate::ops::EventSink,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    sorted_ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<Project>, ServerFnError> {
    use crate::ops::{self, Caller};
    let mut write = ops::begin(db, Caller::session(org_id, actor_id)).await?;
    let result = ops::projects::set_active(&mut write.tx, &write.actor, sorted_ids, active).await;
    Ok(write.commit(result, events).await?)
}

/// The status write as the tests drive it: each project with the transition
/// its committed event reported.
#[cfg(all(test, feature = "server"))]
async fn set_projects_active_records(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    sorted_ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<(Project, Option<crate::plugin::event::ActiveTransition>)>, ServerFnError> {
    use crate::plugin::{AppEvent, event::ActiveTransition};
    let recorder = crate::ops::testing::Recorder::default();
    let projects = set_projects_status(db, &recorder, org_id, actor_id, sorted_ids, active).await?;
    let events = recorder.take();
    Ok(projects
        .into_iter()
        .map(|project| {
            let transition = events.iter().find_map(|event| match event {
                AppEvent::ProjectReactivated { project: p, .. } if p.id == project.id => {
                    Some(ActiveTransition::Reactivated)
                }
                AppEvent::ProjectDeactivated { project: p, .. } if p.id == project.id => {
                    Some(ActiveTransition::Deactivated)
                }
                _ => None,
            });
            (project, transition)
        })
        .collect())
}

#[cfg(all(test, feature = "server"))]
async fn set_project_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    project_id: uuid::Uuid,
    active: bool,
) -> Result<(Project, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut results =
        set_projects_active_records(db, org_id, actor_id, &[project_id], active).await?;
    results
        .pop()
        .ok_or_else(|| server_err("Project status change returned no project"))
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
    let rows = sqlx::query_as!(
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
    .map_err(server_err)?;
    access.tx.commit().await.map_err(server_err)?;
    Ok(rows)
}

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
    is_default: bool,
    rate: TaskRateEdit,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let actor = require_user().await?;
    project_requester(&actor, Some(expected_requester))
        .map_err(|_| forbidden("Task requester has changed"))?;
    let state = crate::state::global_state().await;
    let (task, event) = create_catalog_task(
        &state.db,
        actor.org_id,
        actor.id,
        &name,
        billable_default,
        is_default,
        &rate,
    )
    .await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TaskCreated {
            occurred_at: chrono::Utc::now(),
            org_id: actor.org_id,
            task: event,
        });
    Ok(task)
}

/// Begins a task write holding the organization `gate` and requires
/// `TaskWriteAll`.
#[cfg(feature = "server")]
async fn begin_task_write(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    gate: OrganizationLock,
) -> Result<(crate::ops::Tx, crate::ops::Actor), ServerFnError> {
    let crate::ops::WriteTx { tx, actor } = crate::ops::begin_gated(
        db,
        crate::ops::Caller::session(org_id, actor_id),
        gate,
        crate::ops::WriteIsolation::ReadCommitted,
    )
    .await?;
    if !actor.has(Permission::TaskWriteAll) {
        return Err(forbidden("Current task editing authority is required"));
    }
    Ok((tx, actor))
}

/// Adds a task to the organization catalog. Projects link tasks through the
/// project editor, as Harvest links them through task assignments.
#[cfg(feature = "server")]
async fn create_catalog_task(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    is_default: bool,
    rate: &TaskRateEdit,
) -> Result<(Task, crate::plugin::event::TaskPayload), ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let (mut tx, actor) = begin_task_write(db, org_id, actor_id, OrganizationLock::Shared).await?;
    let (amount, currency) = authorize_task_rate(&mut tx, org_id, &actor, rate).await?;
    let id = uuid::Uuid::now_v7();
    let mut task = sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, default_rate_currency, is_default)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        id,
        org_id,
        name,
        billable_default,
        amount,
        currency,
        is_default,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    let event = task_payload(&task);
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    tx.commit().await.map_err(server_err)?;
    Ok((task, event))
}

#[server]
pub async fn update_task(
    task_id: String,
    name: String,
    billable_default: bool,
    is_default: bool,
    rate: TaskRateEdit,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let actor = require_user().await?;
    project_requester(&actor, Some(expected_requester))
        .map_err(|_| forbidden("Task requester has changed"))?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, event) = update_task_record(
        &state.db,
        actor.org_id,
        actor.id,
        task_id,
        &name,
        billable_default,
        is_default,
        &rate,
    )
    .await?;
    if let Some(payload) = event {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TaskUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: actor.org_id,
                task: payload,
            });
    }
    Ok(task)
}

#[cfg(feature = "server")]
#[allow(clippy::too_many_arguments)]
async fn update_task_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    task_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    is_default: bool,
    rate: &TaskRateEdit,
) -> Result<(Task, Option<crate::plugin::event::TaskPayload>), ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let (mut tx, actor) = begin_task_write(db, org_id, actor_id, OrganizationLock::Shared).await?;
    let (amount, currency) = authorize_task_rate(&mut tx, org_id, &actor, rate).await?;
    let preserve = matches!(rate, TaskRateEdit::Preserve {});
    let before = lock_task(&mut tx, org_id, task_id).await?;

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks
            SET name = $3, billable_default = $4, is_default = $8,
                default_rate_cents = CASE WHEN $5 THEN default_rate_cents ELSE $6 END,
                default_rate_currency = CASE WHEN $5 THEN default_rate_currency ELSE $7 END
          WHERE id = $1 AND org_id = $2
            AND (name, billable_default, is_default, default_rate_cents, default_rate_currency)
                IS DISTINCT FROM ($3, $4, $8,
                    CASE WHEN $5 THEN default_rate_cents ELSE $6 END,
                    CASE WHEN $5 THEN default_rate_currency ELSE $7 END)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        name,
        billable_default,
        preserve,
        amount,
        currency,
        is_default,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    // Service events retain their committed payload; session responses must not
    // inherit that service authority over a hidden global rate.
    let event = task.as_ref().map(task_payload);
    let mut task = task.unwrap_or(before);
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    tx.commit().await.map_err(server_err)?;
    Ok((task, event))
}

#[cfg(feature = "server")]
async fn authorize_task_rate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    actor: &crate::ops::Actor,
    rate: &TaskRateEdit,
) -> Result<(Option<i64>, Option<String>), ServerFnError> {
    use horae_core::permissions::rates::RateEdit;

    let intent = match rate {
        TaskRateEdit::Preserve {} => RateEdit::Unchanged,
        TaskRateEdit::Clear {} => RateEdit::Reset,
        TaskRateEdit::Set { amount_cents, .. } => RateEdit::Set(*amount_cents),
    };
    intent
        .authorize(actor.has(Permission::BillableRateWriteAll))
        .map_err(|_| forbidden("Current global billable-rate editing authority is required"))?;
    if let TaskRateEdit::Set {
        amount_cents,
        currency,
    } = rate
    {
        if *amount_cents < 0 {
            return Err(err(BAD_REQUEST, "Task rate must not be negative"));
        }
        let current = sqlx::query_scalar!(
            "SELECT upper(btrim(default_currency)) AS \"currency!\" FROM organizations WHERE id=$1",
            org_id
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(server_err)?;
        if current.len() != 3
            || !current.bytes().all(|byte| byte.is_ascii_uppercase())
            || !currency.trim().eq_ignore_ascii_case(&current)
        {
            return Err(conflict(
                "Task rate currency changed. Reload the task and enter its rate again",
            ));
        }
        Ok((Some(*amount_cents), Some(current)))
    } else {
        Ok((None, None))
    }
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(
    task_id: String,
    active: bool,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let actor = require_user().await?;
    project_requester(&actor, Some(expected_requester))
        .map_err(|_| forbidden("Task requester has changed"))?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, event) =
        set_task_active_record(&state.db, actor.org_id, actor.id, task_id, active).await?;
    if let Some(event) = event {
        state.plugins.dispatch(event);
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn set_task_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<(Task, Option<crate::plugin::AppEvent>), ServerFnError> {
    // Tracking writers hold the shared gate through entry insertion.
    let (mut tx, actor) =
        begin_task_write(db, org_id, actor_id, OrganizationLock::AccessChange).await?;
    if !active {
        // Link updates invalidate editors through their parent project trigger.
        // Take every parent before the task/link locks, in stable order.
        sqlx::query_scalar!(
            "SELECT p.id FROM projects p JOIN project_tasks pt ON pt.project_id=p.id
             JOIN tasks t ON t.id=pt.task_id AND t.org_id=p.org_id
             WHERE p.org_id=$1 AND t.id=$2 ORDER BY p.id FOR NO KEY UPDATE OF p",
            org_id,
            task_id,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(server_err)?;
    }
    let before = lock_task(&mut tx, org_id, task_id).await?;

    if !active {
        let running = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM time_entries
                WHERE org_id=$1 AND task_id=$2 AND is_running) AS "running!""#,
            org_id,
            task_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(server_err)?;
        if running {
            // Do not reveal an entry or its owner to a task-only administrator.
            return Err(conflict("A running timer prevents archiving this task"));
        }
    }

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
    if !active {
        sqlx::query!(
            "UPDATE project_tasks pt SET active=false FROM projects p
             WHERE pt.project_id=p.id AND p.org_id=$1 AND pt.task_id=$2 AND pt.active",
            org_id,
            task_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    }
    let mut task = task.unwrap_or(before);
    let event = transition.map(|transition| {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        match transition {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::TaskReactivated {
                    occurred_at,
                    org_id,
                    task,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::TaskDeactivated {
                    occurred_at,
                    org_id,
                    task,
                }
            }
        }
    });
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    tx.commit().await.map_err(server_err)?;
    Ok((task, event))
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

// ── Assignments ─────────────────────────────────────────────────────────────

/// The project members `viewer` may see, in assignment order.
#[cfg(feature = "server")]
async fn team_member_ids(
    access: &mut read_access::ReadAccess<'_>,
    viewer: &User,
    project_id: uuid::Uuid,
) -> Result<Vec<uuid::Uuid>, ServerFnError> {
    sqlx::query_scalar!(
        r#"SELECT a.user_id
         FROM assignments a JOIN projects p ON p.id = a.project_id
         JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         JOIN users member ON member.id = a.user_id AND member.org_id = p.org_id
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $3
         WHERE a.project_id = $1 AND p.org_id = $2
           AND ($4 OR ($5 AND management.id IS NOT NULL) OR a.user_id = $3)
         ORDER BY a.created_at, a.id"#,
        project_id,
        viewer.org_id,
        viewer.id,
        access.has(Permission::ProjectReadAll),
        access.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)
}

#[cfg(all(test, feature = "server"))]
use crate::models::Assignment;

/// Adds a project member the way a team change does (organization gate, then
/// the project revision), for tests of what a membership change waits for.
/// Production membership changes go through the project editor.
#[cfg(all(test, feature = "server"))]
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

/// Removes a project member like [`insert_assignment`] adds one.
#[cfg(all(test, feature = "server"))]
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

#[cfg(all(test, feature = "server"))]
async fn begin_assignment_change<'a>(
    db: &'a sqlx::PgPool,
    admin: &User,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    lock_organization(&mut tx, admin.org_id, OrganizationLock::AccessChange)
        .await
        .map_err(server_err)?;
    Ok(tx)
}

#[cfg(all(test, feature = "server"))]
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
