//! The Projects page and the pickers: project rows, tags, tracked totals and
//! tasks, each limited to what the actor may read. [`page`] and [`get`] read
//! the same projects as the Projects page, by id, for `/api/v1`.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::projects::writes_any;
use horae_core::types::{BudgetKind, ProjectType};
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use super::REQUESTER_CHANGED;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::{ProjectOverview, ProjectOverviewClient, ProjectOverviewRow};
use crate::models::{Project, ProjectSpend, ProjectTagLink, Task};

/// Which project rows a read is for.
#[derive(Clone, Copy)]
pub enum ProjectRead {
    /// The Projects page: projects the actor may read, with their client and
    /// the money fields their rate grants allow.
    Overview,
    /// Starting time and resolving an own history: projects the actor works on
    /// or has tracked time to, as identities only. Reporting visibility must
    /// not keep a teammate from their timesheet.
    Tracking,
}

/// Which tasks a read is for.
#[derive(Clone, Copy)]
pub enum TaskRead {
    /// Active catalog tasks, for task readers or within one readable project.
    Catalog,
    /// Rate-free identities, including archived tasks in the actor's history.
    Tracking,
}

/// The Projects page: its rows and what the actor may do there.
pub async fn overview(
    tx: &mut Tx,
    actor: &Actor,
    expected: Option<PermissionRequester>,
) -> Result<ProjectOverview, OpError> {
    let requester = actor.expect_requester(expected, REQUESTER_CHANGED)?;
    if !actor.is_active() {
        return Err(OpError::forbidden("Current project access is required"));
    }
    Ok(ProjectOverview {
        requester,
        can_create: actor.has(Permission::ProjectCreateAll),
        can_import: actor.administrator(),
        can_change_status: writes_any(|permission| actor.has(permission)),
        can_open_clients: crate::ops::clients::reads_catalog(actor),
        projects: project_rows(tx, actor, None, true, ProjectRead::Overview).await?,
    })
}

/// Project rows for `purpose`, optionally of one client. Empty for an actor
/// with no authority.
pub async fn list(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Option<Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<Project>, OpError> {
    if !actor.is_active() {
        return Ok(Vec::new());
    }
    let rows = project_rows(tx, actor, client_id, include_inactive, purpose).await?;
    Ok(rows.into_iter().map(|row| row.project).collect())
}

async fn project_rows(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Option<Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<ProjectOverviewRow>, OpError> {
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
        actor.org_id(),
        actor.id(),
        overview,
        client_id,
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::BillableRateReadManaged),
        actor.administrator(),
        actor.has(Permission::ProjectWriteAll),
        actor.has(Permission::ProjectWriteManaged),
    )
    .fetch_all(&mut **tx)
    .await?;

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

/// A project the actor may read, with the Projects page's fields and money
/// rule, and when it last changed.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRecord {
    pub id: Uuid,
    pub client_id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub project_type: ProjectType,
    pub currency: String,
    /// `None` when unset or hidden from the actor.
    pub rate_cents: Option<i64>,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub budget_kind: BudgetKind,
    /// `None` when unset or hidden from the actor.
    pub budget_amount_cents: Option<i64>,
    /// Set for an hours budget only.
    pub budget_minutes: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which projects [`page`] lists, and where its page starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectQuery {
    pub client_id: Option<Uuid>,
    /// Active projects, or archived ones.
    pub active: bool,
    /// Only projects whose `updated_at` is at or after it.
    pub updated_since: Option<DateTime<Utc>>,
    /// The last id of the previous page.
    pub after: Option<Uuid>,
    pub limit: u32,
}

/// The projects of the Projects page in `id` order: up to `limit + 1` rows,
/// the extra one saying that another page follows.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    query: &ProjectQuery,
) -> Result<Vec<ProjectRecord>, OpError> {
    records(tx, actor, None, Some(query)).await
}

/// One project of [`page`], whether active or archived. A project the actor
/// may not read is not found.
pub async fn get(tx: &mut Tx, actor: &Actor, project_id: Uuid) -> Result<ProjectRecord, OpError> {
    records(tx, actor, Some(project_id), None)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| OpError::not_found(super::NOT_FOUND))
}

/// [`project_rows`]' rules for [`ProjectRead::Overview`], for one project or
/// a page of them.
async fn records(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Option<Uuid>,
    query: Option<&ProjectQuery>,
) -> Result<Vec<ProjectRecord>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden("Current project access is required"));
    }
    Ok(sqlx::query_as!(
        ProjectRecord,
        r#"SELECT p.id, p.client_id, p.code, p.name,
                p.project_type as "project_type: ProjectType", p.currency,
                CASE WHEN $6 OR ($7 AND management.id IS NOT NULL) THEN p.rate_cents END AS rate_cents,
                p.starts_on as "starts_on: NaiveDate", p.ends_on as "ends_on: NaiveDate",
                p.budget_kind as "budget_kind: BudgetKind",
                CASE WHEN $6 OR ($7 AND management.id IS NOT NULL)
                  THEN p.budget_amount_cents END AS budget_amount_cents,
                CASE WHEN p.budget_kind = 'hours' THEN p.budget_minutes END AS budget_minutes,
                p.active, p.created_at as "created_at: DateTime<Utc>",
                p.updated_at as "updated_at: DateTime<Utc>"
         FROM projects p
         JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $2
         LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
         LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
         WHERE p.org_id = $1
           AND ($4 OR ($5 AND management.id IS NOT NULL)
             OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members'))
           AND ($3::uuid IS NULL OR p.id = $3) AND ($8::uuid IS NULL OR p.client_id = $8)
           AND ($9::bool IS NULL OR p.active = $9)
           AND ($10::timestamptz IS NULL OR p.updated_at >= $10)
           AND p.id > $11
         ORDER BY p.id LIMIT $12"#,
        actor.org_id(),
        actor.id(),
        project_id,
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::BillableRateReadManaged),
        query.and_then(|query| query.client_id),
        query.map(|query| query.active),
        query.and_then(|query| query.updated_since) as Option<DateTime<Utc>>,
        query.and_then(|query| query.after).unwrap_or(Uuid::nil()),
        query.map_or(1, |query| i64::from(query.limit) + 1),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// The tags of every project the actor may read. Empty for an actor with no
/// authority.
pub async fn tags(
    tx: &mut Tx,
    actor: &Actor,
    expected: Option<PermissionRequester>,
) -> Result<Vec<ProjectTagLink>, OpError> {
    actor.expect_requester(expected, REQUESTER_CHANGED)?;
    if !actor.is_active() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as!(
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
        actor.org_id(),
        actor.id(),
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Per-project tracked totals for the overview's Spent column: every readable
/// project's logged minutes plus its billable amount, with each entry's rate
/// resolved through the FR-024 cascade (task → assignment → project → user
/// default). Never per-person time or rates. Empty for an actor with no
/// authority.
pub async fn spend(
    tx: &mut Tx,
    actor: &Actor,
    expected: Option<PermissionRequester>,
) -> Result<Vec<ProjectSpend>, OpError> {
    actor.expect_requester(expected, REQUESTER_CHANGED)?;
    if !actor.is_active() {
        return Ok(Vec::new());
    }
    Ok(spend_rows(tx, actor, None, true).await?)
}

/// Tracked totals for every readable project, or only `project_id`. Without
/// `include_money` no amount is valued and `spent_cents` is absent.
pub(super) async fn spend_rows(
    connection: &mut sqlx::PgConnection,
    actor: &Actor,
    project_id: Option<Uuid>,
    include_money: bool,
) -> Result<Vec<ProjectSpend>, sqlx::Error> {
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
                 AND (te.state = 'invoiced' OR (p.project_type <> 'non_billable'
                   AND COALESCE(pt.billable, t.billable_default)))
               THEN COALESCE(line.amount_cents, line_amount_cents(
                 COALESCE(CASE WHEN ps.project_id IS NULL OR p.project_type = 'time_and_materials'
                   THEN resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                   CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                   CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
                 ) END, 0),
                 effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir),
                 COALESCE(invoice.currency, CASE WHEN ps.project_id IS NULL THEN c.currency ELSE p.currency END)
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
        actor.org_id(),
        actor.id(),
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::BillableRateReadManaged),
        project_id,
        include_money,
    )
    .fetch_all(connection)
    .await
}

/// Tasks for `purpose`, optionally only those of one project the actor may
/// read. Empty for an actor with no authority.
pub async fn tasks(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Option<Uuid>,
    purpose: TaskRead,
) -> Result<Vec<Task>, OpError> {
    if !actor.is_active() {
        return Ok(Vec::new());
    }
    let tracking = matches!(purpose, TaskRead::Tracking);
    Ok(sqlx::query_as!(
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
        actor.id(),
        actor.org_id(),
        project_id,
        tracking,
        actor.has(Permission::TaskReadAll),
        actor.has(Permission::BillableRateReadAll),
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut **tx)
    .await?)
}
