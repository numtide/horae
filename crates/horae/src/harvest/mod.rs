// Harvest-compatible REST API surface, mounted at /harvest/v2.
//
// Tools like harvest-invoicer and harvest-exporter can be pointed at
//   https://horae.example.com/harvest
// and will call /harvest/v2/time_entries etc. as normal.

mod auth;
mod types;

use axum::{Json, Router, extract::Path, extract::Query, http::StatusCode, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use auth::AuthUser;
use types::*;

pub fn router() -> Router {
    Router::new().nest(
        "/harvest/v2",
        Router::new()
            .route("/users/me", get(users_me))
            .route("/time_entries", get(list_time_entries))
            .route("/time_entries/{id}", get(get_time_entry))
            .route("/projects", get(list_projects))
            .route("/projects/{id}", get(get_project))
            .route("/clients", get(list_clients))
            .route("/clients/{id}", get(get_client))
            .route("/tasks", get(list_tasks))
            .route("/tasks/{id}", get(get_task))
            .route("/users", get(list_users)),
    )
}

// ── Error helper ────────────────────────────────────────────────────────────

type ApiResult<T> = Result<Json<T>, (axum::http::StatusCode, String)>;

fn internal(e: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    tracing::error!("Harvest API error: {e}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal error: {e}"),
    )
}

fn not_found() -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::NOT_FOUND, "Not found".to_string())
}

/// The caller is signed in but not senior enough for this collection. Kept
/// distinct from [`not_found`] on purpose: a 404 would tell a Harvest client the
/// org holds no such data, and the login guard deliberately never redirects
/// under `/harvest/`, so 403 is the only honest answer here.
fn forbidden(msg: &str) -> (axum::http::StatusCode, String) {
    (StatusCode::FORBIDDEN, msg.to_string())
}

/// The `user_id` a time-entry query may read, given what the caller asked for.
///
/// Managers and admins see the whole org and may filter to anyone; a member is
/// confined to their own rows, exactly as the SPA's `list_time_entries` scopes
/// its listing to the session user. Asking for a colleague's rows is refused
/// rather than quietly answered with a different row set.
fn scoped_user_filter(
    caller: &AuthUser,
    requested: Option<Uuid>,
) -> Result<Option<Uuid>, (axum::http::StatusCode, String)> {
    if caller.org_role.is_manager_or_above() {
        return Ok(requested);
    }
    match requested {
        Some(id) if id != caller.user_id => Err(forbidden(
            "Reading another user's time entries requires manager access",
        )),
        _ => Ok(Some(caller.user_id)),
    }
}

// ── Pagination ──────────────────────────────────────────────────────────────

/// Harvest v2 pagination window shared by every list endpoint: page defaults
/// to 1 (floored at 1), per_page to 100 (clamped to 1..=100). Returns
/// `(page, per_page, offset)`.
fn page_window(page: Option<i64>, per_page: Option<i64>) -> (i64, i64, i64) {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(100).clamp(1, 100);
    (page, per_page, (page - 1) * per_page)
}

// ── /users/me ───────────────────────────────────────────────────────────────

async fn users_me(user: AuthUser) -> ApiResult<HarvestUser> {
    let state = crate::state::global_state().await;

    let row: UserRow = sqlx::query_as!(
        UserRow,
        r#"SELECT id, name, email, active, org_role::text AS "org_role!: String",
         cost_rate_cents, billable_rate_cents,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1"#,
        user.user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    Ok(Json(user_row_to_harvest(&row)))
}

// ── Time Entries ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TimeEntryFilters {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub is_running: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TimeEntryRow {
    id: Uuid,
    spent_date: NaiveDate,
    minutes: i32,
    start_minute: Option<i32>,
    rounded_minutes: Option<i32>,
    notes: Option<String>,
    billable: bool,
    is_running: bool,
    started_at: Option<DateTime<Utc>>,
    state: String,
    invoice_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // Joined fields
    user_id: Uuid,
    user_name: String,
    project_id: Uuid,
    project_name: String,
    project_code: Option<String>,
    task_id: Uuid,
    task_name: String,
    client_id: Uuid,
    client_name: String,
    // Rates
    user_billable_rate_cents: Option<i64>,
    user_cost_rate_cents: Option<i64>,
    budget_kind: String,
}

fn time_entry_row_to_harvest(
    row: &TimeEntryRow,
    org_round_min: u32,
    org_round_dir: horae_core::types::RoundDir,
) -> HarvestTimeEntry {
    let hours = row.minutes as f64 / 60.0;
    let rounded_hours = horae_core::rounding::effective_minutes(
        row.minutes as u32,
        row.rounded_minutes.map(|minutes| minutes as u32),
        org_round_min,
        org_round_dir,
    ) as f64
        / 60.0;
    let is_locked = matches!(row.state.as_str(), "submitted" | "approved" | "invoiced");
    let locked_reason = match row.state.as_str() {
        "submitted" => Some("Pending Approval".to_string()),
        "approved" => Some("Approved".to_string()),
        "invoiced" => Some("Invoiced".to_string()),
        _ => None,
    };
    let approval_status = match row.state.as_str() {
        "open" => "unsubmitted",
        "submitted" => "pending_approval",
        "approved" | "invoiced" => "approved",
        other => other,
    };

    // Wall-clock start/end from the optional start minute (D8); null when untimed.
    let started_time = row
        .start_minute
        .map(|m| horae_core::time_of_day::format_12h(m as u16));
    let ended_time = row
        .start_minute
        .map(|m| horae_core::time_of_day::format_12h((m + row.minutes) as u16));

    HarvestTimeEntry {
        id: row.id.to_string(),
        spent_date: row.spent_date.to_string(),
        hours,
        rounded_hours,
        started_time,
        ended_time,
        notes: row.notes.clone(),
        is_locked,
        locked_reason,
        is_closed: is_locked,
        is_billed: row.invoice_id.is_some(),
        is_running: row.is_running,
        timer_started_at: row.started_at.map(|t| t.to_rfc3339()),
        billable: row.billable,
        budgeted: row.budget_kind != "none",
        billable_rate: row.user_billable_rate_cents.map(|c| c as f64 / 100.0),
        cost_rate: row.user_cost_rate_cents.map(|c| c as f64 / 100.0),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        user: HarvestRef {
            id: row.user_id.to_string(),
            name: row.user_name.clone(),
        },
        client: HarvestRef {
            id: row.client_id.to_string(),
            name: row.client_name.clone(),
        },
        project: HarvestProjectRef {
            id: row.project_id.to_string(),
            name: row.project_name.clone(),
            code: row.project_code.clone(),
        },
        task: HarvestRef {
            id: row.task_id.to_string(),
            name: row.task_name.clone(),
        },
        approval_status: approval_status.to_string(),
    }
}

async fn list_time_entries(
    user: AuthUser,
    Query(filters): Query<TimeEntryFilters>,
) -> ApiResult<HarvestPagination<HarvestTimeEntry>> {
    let state = crate::state::global_state().await;
    time_entries_page(&state.db, &user, filters).await
}

/// Entries carry both a teammate's notes and their rates, so the row set is
/// scoped by [`scoped_user_filter`]: org-wide for a manager, own rows for a
/// member.
async fn time_entries_page(
    db: &PgPool,
    caller: &AuthUser,
    filters: TimeEntryFilters,
) -> ApiResult<HarvestPagination<HarvestTimeEntry>> {
    let (round_min, round_dir) = crate::db::org_rounding(db, caller.org_id)
        .await
        .map_err(internal)?;

    let (page, per_page, offset) = page_window(filters.page, filters.per_page);

    // Parse filter strings to properly typed values
    let user_id_filter: Option<Uuid> = filters
        .user_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid user_id filter")))
        .transpose()?;
    let user_id_filter = scoped_user_filter(caller, user_id_filter)?;
    let project_id_filter: Option<Uuid> = filters
        .project_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid project_id filter")))
        .transpose()?;
    let total_entries = sqlx::query_scalar!(
        // The count filters on `te` alone. Joining `projects` (as the page query
        // below has to) would cost a heap lookup per counted row, which Postgres
        // cannot elide even though the FK is NOT NULL.
        "SELECT COUNT(*) FROM time_entries te
         WHERE te.org_id = $1
           AND ($2::uuid IS NULL OR te.user_id = $2)
           AND ($3::uuid IS NULL OR te.project_id = $3)
           AND ($4::date IS NULL OR te.spent_date >= $4::date)
           AND ($5::date IS NULL OR te.spent_date <= $5::date)
           AND ($6::bool IS NULL OR te.is_running = $6)
           AND ($7::timestamptz IS NULL OR te.updated_at >= $7::timestamptz)",
        caller.org_id,
        user_id_filter,
        project_id_filter,
        filters.from.as_deref() as Option<&str>,
        filters.to.as_deref() as Option<&str>,
        filters.is_running,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        TimeEntryRow,
        r#"SELECT te.id,
               te.spent_date as "spent_date: chrono::NaiveDate",
               te.minutes, te.start_minute, te.rounded_minutes, te.notes,
               (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) as "billable!", te.is_running,
               te.started_at as "started_at: chrono::DateTime<chrono::Utc>",
               te.state::text AS "state!: String", te.invoice_id,
               te.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               te.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
               te.user_id, u.name AS user_name,
               te.project_id, p.name AS project_name, p.code AS project_code,
               te.task_id, t.name AS task_name,
               p.client_id, c.name AS client_name,
               u.billable_rate_cents AS user_billable_rate_cents,
               u.cost_rate_cents AS user_cost_rate_cents,
               p.budget_kind::text AS "budget_kind!: String"
         FROM time_entries te
         JOIN users u ON u.id = te.user_id
         JOIN projects p ON p.id = te.project_id
         JOIN tasks t ON t.id = te.task_id
         LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
         JOIN clients c ON c.id = p.client_id
         WHERE te.org_id = $1
           AND ($2::uuid IS NULL OR te.user_id = $2)
           AND ($3::uuid IS NULL OR te.project_id = $3)
           AND ($4::date IS NULL OR te.spent_date >= $4::date)
           AND ($5::date IS NULL OR te.spent_date <= $5::date)
           AND ($6::bool IS NULL OR te.is_running = $6)
           AND ($7::timestamptz IS NULL OR te.updated_at >= $7::timestamptz)
         ORDER BY te.spent_date DESC, te.created_at DESC
         LIMIT $8 OFFSET $9"#,
        caller.org_id,
        user_id_filter,
        project_id_filter,
        filters.from.as_deref() as Option<&str>,
        filters.to.as_deref() as Option<&str>,
        filters.is_running,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(db)
    .await
    .map_err(internal)?;

    let entries: Vec<HarvestTimeEntry> = rows
        .iter()
        .map(|r| time_entry_row_to_harvest(r, round_min, round_dir))
        .collect();

    Ok(Json(HarvestPagination::new(
        "time_entries",
        entries,
        page,
        per_page,
        total_entries,
        "/harvest/v2/time_entries",
    )))
}

async fn get_time_entry(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestTimeEntry> {
    let state = crate::state::global_state().await;
    time_entry_by_id(&state.db, &user, id).await
}

/// A single entry out of the same scoped row set as the listing. To a member a
/// colleague's entry is simply not in their collection, so it reads as missing
/// rather than forbidden — the answer an id from another org already gets.
async fn time_entry_by_id(db: &PgPool, caller: &AuthUser, id: Uuid) -> ApiResult<HarvestTimeEntry> {
    let user_scope = scoped_user_filter(caller, None)?;

    let (round_min, round_dir) = crate::db::org_rounding(db, caller.org_id)
        .await
        .map_err(internal)?;

    let row = sqlx::query_as!(
        TimeEntryRow,
        r#"SELECT te.id,
               te.spent_date as "spent_date: chrono::NaiveDate",
               te.minutes, te.start_minute, te.rounded_minutes, te.notes,
               (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) as "billable!", te.is_running,
               te.started_at as "started_at: chrono::DateTime<chrono::Utc>",
               te.state::text AS "state!: String", te.invoice_id,
               te.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               te.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
               te.user_id, u.name AS user_name,
               te.project_id, p.name AS project_name, p.code AS project_code,
               te.task_id, t.name AS task_name,
               p.client_id, c.name AS client_name,
               u.billable_rate_cents AS user_billable_rate_cents,
               u.cost_rate_cents AS user_cost_rate_cents,
               p.budget_kind::text AS "budget_kind!: String"
         FROM time_entries te
         JOIN users u ON u.id = te.user_id
         JOIN projects p ON p.id = te.project_id
         JOIN tasks t ON t.id = te.task_id
         JOIN clients c ON c.id = p.client_id
         LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
         WHERE te.id = $1 AND te.org_id = $2
           AND ($3::uuid IS NULL OR te.user_id = $3)"#,
        id,
        caller.org_id,
        user_scope,
    )
    .fetch_optional(db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(time_entry_row_to_harvest(&row, round_min, round_dir)))
}

// ── Projects ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProjectFilters {
    pub is_active: Option<bool>,
    pub client_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    name: String,
    code: Option<String>,
    project_type: String,
    active: bool,
    budget_kind: String,
    budget_amount_cents: Option<i64>,
    budget_minutes: Option<i64>,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    created_at: DateTime<Utc>,
    client_id: Uuid,
    client_name: String,
}

fn project_row_to_harvest(row: &ProjectRow) -> HarvestProject {
    let is_billable = row.project_type != "non_billable";
    let bill_by = match row.project_type.as_str() {
        "time_and_materials" => "Tasks",
        "fixed_fee" => "Project",
        "retainer" => "Project",
        _ => "none",
    };
    let budget_by = match row.budget_kind.as_str() {
        "hours" => "person",
        "amount" => "project_cost",
        _ => "none",
    };
    let budget = match row.budget_kind.as_str() {
        "hours" => row.budget_minutes.map(|m| m as f64 / 60.0),
        "amount" => row.budget_amount_cents.map(|c| c as f64 / 100.0),
        _ => None,
    };

    HarvestProject {
        id: row.id.to_string(),
        name: row.name.clone(),
        code: row.code.clone(),
        is_active: row.active,
        is_billable,
        bill_by: bill_by.to_string(),
        budget_by: budget_by.to_string(),
        budget,
        starts_on: row.starts_on.map(|d| d.to_string()),
        ends_on: row.ends_on.map(|d| d.to_string()),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // projects table has no updated_at
        client: HarvestRef {
            id: row.client_id.to_string(),
            name: row.client_name.clone(),
        },
    }
}

async fn list_projects(
    user: AuthUser,
    Query(filters): Query<ProjectFilters>,
) -> ApiResult<HarvestPagination<HarvestProject>> {
    let state = crate::state::global_state().await;
    let (page, per_page, offset) = page_window(filters.page, filters.per_page);

    let client_id_filter: Option<Uuid> = filters
        .client_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid client_id")))
        .transpose()?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM projects p
         WHERE p.org_id = $1
           AND ($2::bool IS NULL OR p.active = $2)
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::timestamptz IS NULL OR p.created_at >= $4::timestamptz)",
        user.org_id,
        filters.is_active,
        client_id_filter,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        ProjectRow,
        r#"SELECT p.id, p.name, p.code, p.project_type::text AS "project_type!: String", p.active,
         p.budget_kind::text AS "budget_kind!: String", p.budget_amount_cents, p.budget_minutes,
         p.starts_on as "starts_on: chrono::NaiveDate",
         p.ends_on as "ends_on: chrono::NaiveDate",
         p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
         p.client_id, c.name AS client_name
         FROM projects p
         JOIN clients c ON c.id = p.client_id
         WHERE p.org_id = $1
           AND ($2::bool IS NULL OR p.active = $2)
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::timestamptz IS NULL OR p.created_at >= $4::timestamptz)
         ORDER BY p.name
         LIMIT $5 OFFSET $6"#,
        user.org_id,
        filters.is_active,
        client_id_filter,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestProject> = rows.iter().map(project_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "projects",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/projects",
    )))
}

async fn get_project(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestProject> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        ProjectRow,
        r#"SELECT p.id, p.name, p.code, p.project_type::text AS "project_type!: String", p.active,
         p.budget_kind::text AS "budget_kind!: String", p.budget_amount_cents, p.budget_minutes,
         p.starts_on as "starts_on: chrono::NaiveDate",
         p.ends_on as "ends_on: chrono::NaiveDate",
         p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
         p.client_id, c.name AS client_name
         FROM projects p
         JOIN clients c ON c.id = p.client_id
         WHERE p.id = $1 AND p.org_id = $2"#,
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(project_row_to_harvest(&row)))
}

// ── Clients ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ClientFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ClientRow {
    id: Uuid,
    name: String,
    active: bool,
    address: Option<String>,
    currency: String,
    created_at: DateTime<Utc>,
}

fn client_row_to_harvest(row: &ClientRow) -> HarvestClient {
    HarvestClient {
        id: row.id.to_string(),
        name: row.name.clone(),
        is_active: row.active,
        address: row.address.clone(),
        currency: row.currency.clone(),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // no updated_at column
    }
}

async fn list_clients(
    user: AuthUser,
    Query(filters): Query<ClientFilters>,
) -> ApiResult<HarvestPagination<HarvestClient>> {
    let state = crate::state::global_state().await;
    let (page, per_page, offset) = page_window(filters.page, filters.per_page);

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM clients
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
           AND ($3::timestamptz IS NULL OR created_at >= $3::timestamptz)",
        user.org_id,
        filters.is_active,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        ClientRow,
        r#"SELECT id, name, active, address, currency,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
           AND ($3::timestamptz IS NULL OR created_at >= $3::timestamptz)
         ORDER BY name
         LIMIT $4 OFFSET $5"#,
        user.org_id,
        filters.is_active,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestClient> = rows.iter().map(client_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "clients",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/clients",
    )))
}

async fn get_client(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestClient> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        ClientRow,
        r#"SELECT id, name, active, address, currency,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients WHERE id = $1 AND org_id = $2"#,
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(client_row_to_harvest(&row)))
}

// ── Tasks ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TaskFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[allow(dead_code)]
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    name: String,
    active: bool,
    billable_default: bool,
    default_rate_cents: Option<i64>,
}

fn task_row_to_harvest(row: &TaskRow) -> HarvestTask {
    HarvestTask {
        id: row.id.to_string(),
        name: row.name.clone(),
        is_active: row.active,
        billable_by_default: row.billable_default,
        default_hourly_rate: row.default_rate_cents.map(|c| c as f64 / 100.0),
        created_at: String::new(), // tasks table has no created_at
        updated_at: String::new(),
    }
}

async fn list_tasks(
    user: AuthUser,
    Query(filters): Query<TaskFilters>,
) -> ApiResult<HarvestPagination<HarvestTask>> {
    let state = crate::state::global_state().await;
    let (page, per_page, offset) = page_window(filters.page, filters.per_page);

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM tasks
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)",
        user.org_id,
        filters.is_active,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        TaskRow,
        "SELECT id, name, active, billable_default, default_rate_cents
         FROM tasks
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
         ORDER BY name
         LIMIT $3 OFFSET $4",
        user.org_id,
        filters.is_active,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestTask> = rows.iter().map(task_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "tasks",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/tasks",
    )))
}

async fn get_task(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestTask> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        TaskRow,
        "SELECT id, name, active, billable_default, default_rate_cents
         FROM tasks WHERE id = $1 AND org_id = $2",
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(task_row_to_harvest(&row)))
}

// ── Users ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UserFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[allow(dead_code)]
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    name: String,
    email: String,
    active: bool,
    org_role: String,
    cost_rate_cents: Option<i64>,
    billable_rate_cents: Option<i64>,
    created_at: DateTime<Utc>,
}

fn user_row_to_harvest(row: &UserRow) -> HarvestUser {
    // Split "First Last" into first_name / last_name
    let (first, last) = match row.name.split_once(' ') {
        Some((f, l)) => (f.to_string(), l.to_string()),
        None => (row.name.clone(), String::new()),
    };

    HarvestUser {
        id: row.id.to_string(),
        first_name: first,
        last_name: last,
        email: row.email.clone(),
        is_active: row.active,
        is_admin: row.org_role == "admin",
        cost_rate: row.cost_rate_cents.map(|c| c as f64 / 100.0),
        default_hourly_rate: row.billable_rate_cents.map(|c| c as f64 / 100.0),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // no updated_at column
    }
}

async fn list_users(
    user: AuthUser,
    Query(filters): Query<UserFilters>,
) -> ApiResult<HarvestPagination<HarvestUser>> {
    let state = crate::state::global_state().await;
    users_page(&state.db, &user, filters).await
}

/// The org's people, pay rates included. Harvest's user object has no rate-free
/// shape — a null `cost_rate` there means "no rate is configured" — so blanking
/// the fields for a member, the way the SPA's `list_users` can, would misreport
/// the org rather than protect it. The collection is gated whole instead, which
/// keeps the rule the SPA states: rates are manager material (SPEC §6). A member
/// still reads their own record from `/users/me`.
async fn users_page(
    db: &PgPool,
    caller: &AuthUser,
    filters: UserFilters,
) -> ApiResult<HarvestPagination<HarvestUser>> {
    if !caller.org_role.is_manager_or_above() {
        return Err(forbidden("Listing users requires manager access"));
    }

    let (page, per_page, offset) = page_window(filters.page, filters.per_page);

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)",
        caller.org_id,
        filters.is_active,
    )
    .fetch_one(db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        UserRow,
        r#"SELECT id, name, email, active, org_role::text AS "org_role!: String",
         cost_rate_cents, billable_rate_cents,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
         ORDER BY name
         LIMIT $3 OFFSET $4"#,
        caller.org_id,
        filters.is_active,
        per_page,
        offset,
    )
    .fetch_all(db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestUser> = rows.iter().map(user_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "users",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/users",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_fns::test_seed::{SeedIds, seed};
    use horae_core::types::{EntryState, OrgRole};

    #[test]
    fn page_window_defaults_clamps_and_offsets() {
        // Defaults: first page of 100.
        assert_eq!(page_window(None, None), (1, 100, 0));
        // Clamps: page floors at 1, per_page stays within 1..=100.
        assert_eq!(page_window(Some(0), None), (1, 100, 0));
        assert_eq!(page_window(None, Some(0)), (1, 1, 0));
        assert_eq!(page_window(None, Some(500)), (1, 100, 0));
        // A later page offsets by the preceding pages.
        assert_eq!(page_window(Some(3), Some(25)), (3, 25, 50));
    }

    // ── Authorization ───────────────────────────────────────────────────────
    // This API rides the same session cookie as the SPA, so these assert that it
    // applies the same policy: rates and other people's rows need a manager.

    fn caller(ids: &SeedIds, org_role: OrgRole) -> AuthUser {
        AuthUser {
            user_id: ids.user_id,
            org_id: ids.org_id,
            org_role,
        }
    }

    fn no_time_entry_filters() -> TimeEntryFilters {
        TimeEntryFilters {
            user_id: None,
            project_id: None,
            from: None,
            to: None,
            is_running: None,
            page: None,
            per_page: None,
            updated_since: None,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn harvest_marks_unbilled_non_billable_contexts_consistently(pool: PgPool) {
        for project_billable in [false, true] {
            let ids = seed(&pool, OrgRole::Manager).await;
            let id = insert_entry(&pool, &ids, ids.user_id, "Non-billable context").await;
            sqlx::query!("UPDATE projects SET project_type = CASE WHEN $2 THEN 'time_and_materials'::project_type ELSE 'non_billable'::project_type END WHERE id = $1", ids.project_id, project_billable).execute(&pool).await.unwrap();
            sqlx::query!(
                "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, $3)",
                ids.project_id,
                ids.task_id,
                !project_billable
            )
            .execute(&pool)
            .await
            .unwrap();
            let caller = caller(&ids, OrgRole::Manager);
            let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters())
                .await
                .unwrap();
            assert!(!entry.billable);
            assert!(!page.data["time_entries"][0].billable);
            assert_eq!(entry.hours, 1.0);
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn harvest_effective_minutes_match_sql_for_open_and_frozen_time(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let id = insert_entry(&pool, &ids, ids.user_id, "rounding").await;
        sqlx::query!(
            "UPDATE organizations SET round_minutes = 15, round_dir = 'nearest' WHERE id = $1",
            ids.org_id,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("UPDATE time_entries SET minutes = 8 WHERE id = $1", id)
            .execute(&pool)
            .await
            .unwrap();
        let caller = caller(&ids, OrgRole::Manager);
        for frozen in [None, Some(10), Some(0)] {
            sqlx::query!(
                "UPDATE time_entries SET rounded_minutes = $2 WHERE id = $1",
                id,
                frozen
            )
            .execute(&pool)
            .await
            .unwrap();
            let expected = sqlx::query_scalar!(
                r#"SELECT effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) as "minutes!"
                   FROM time_entries te JOIN organizations o ON o.id = te.org_id WHERE te.id = $1"#,
                id,
            ).fetch_one(&pool).await.unwrap();
            let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters())
                .await
                .unwrap();
            assert_eq!(entry.hours, 8.0 / 60.0);
            assert_eq!(entry.rounded_hours, f64::from(expected) / 60.0);
            assert_eq!(
                page.data["time_entries"][0].rounded_hours,
                entry.rounded_hours
            );
        }
    }

    fn no_user_filters() -> UserFilters {
        UserFilters {
            is_active: None,
            page: None,
            per_page: None,
            updated_since: None,
        }
    }

    /// The status a refused call answered with. `expect_err` would demand
    /// `Debug` on every DTO just to print a body the test never looks at.
    fn refused<T>(result: ApiResult<T>, leak: &str) -> StatusCode {
        match result {
            Ok(_) => panic!("{leak}"),
            Err((status, _)) => status,
        }
    }

    /// A second person in the org, carrying the cost rate this API used to hand
    /// to anyone with a session.
    async fn colleague(pool: &PgPool, ids: &SeedIds) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO users (id, org_id, email, name, org_role, cost_rate_cents) \
             VALUES ($1, $2, $3, 'Coworker', $4, 12345)",
            id,
            ids.org_id,
            format!("{id}@test.com"),
            OrgRole::Member as OrgRole,
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn insert_entry(pool: &PgPool, ids: &SeedIds, user_id: Uuid, notes: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO time_entries \
               (id, org_id, user_id, project_id, task_id, spent_date, \
                minutes, notes, billable, is_running, state) \
             VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, 60, $6, true, false, $7)",
            id,
            ids.org_id,
            user_id,
            ids.project_id,
            ids.task_id,
            notes,
            EntryState::Open as EntryState,
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_member_cannot_list_users(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        colleague(&pool, &ids).await;

        let status = refused(
            users_page(&pool, &caller(&ids, OrgRole::Member), no_user_filters()).await,
            "a member read the org's pay rates",
        );

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_manager_lists_users_with_their_rates(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        colleague(&pool, &ids).await;

        let Json(page) = users_page(&pool, &caller(&ids, OrgRole::Manager), no_user_filters())
            .await
            .expect("a manager was refused the user list");

        let users = &page.data["users"];
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.cost_rate == Some(123.45)));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_member_sees_only_their_own_time_entries(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let Json(page) = time_entries_page(
            &pool,
            &caller(&ids, OrgRole::Member),
            no_time_entry_filters(),
        )
        .await
        .expect("a member was refused their own entries");

        let entries = &page.data["time_entries"];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user.id, ids.user_id.to_string());
        assert_eq!(entries[0].notes.as_deref(), Some("mine"));
        // The envelope has to agree with the rows, or a client pages into
        // entries it is never shown.
        assert_eq!(page.total_entries, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_manager_sees_the_whole_org(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let Json(page) = time_entries_page(
            &pool,
            &caller(&ids, OrgRole::Manager),
            no_time_entry_filters(),
        )
        .await
        .expect("a manager was refused the org's entries");

        assert_eq!(page.data["time_entries"].len(), 2);
        assert_eq!(page.total_entries, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_member_cannot_filter_to_a_colleague(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let filters = TimeEntryFilters {
            user_id: Some(other.to_string()),
            ..no_time_entry_filters()
        };
        let status = refused(
            time_entries_page(&pool, &caller(&ids, OrgRole::Member), filters).await,
            "a member read a colleague's entries by filter",
        );

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_member_cannot_fetch_a_colleagues_entry(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let other = colleague(&pool, &ids).await;
        let theirs = insert_entry(&pool, &ids, other, "theirs").await;

        // Outside the member's row set, so it reads as missing rather than
        // forbidden — the same answer an id from another org gets.
        let status = refused(
            time_entry_by_id(&pool, &caller(&ids, OrgRole::Member), theirs).await,
            "a member fetched a colleague's entry by id",
        );
        assert_eq!(status, StatusCode::NOT_FOUND);

        let Json(entry) = time_entry_by_id(&pool, &caller(&ids, OrgRole::Manager), theirs)
            .await
            .expect("a manager was refused an entry in their own org");
        assert_eq!(entry.id, theirs.to_string());
    }
}
