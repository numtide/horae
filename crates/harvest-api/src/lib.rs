// A Harvest API v2 subset, mounted at /harvest/v2 with the alias /v2.
// `/api/` belongs to Horae's own API (spec 022 FR-052), so Harvest has no alias
// there. Integer ids and exact decimals; personal access tokens, OAuth access
// tokens within their scopes or, for reads only, the session cookie (`auth`). Time entries are also written
// (`time_entry_writes`). The contract is
// specs/021-harvest-api-compat/contracts/harvest-api-v2.md.

mod absent;
mod auth;
mod client_reads;
mod company;
mod errors;
pub(crate) mod ids;
mod pagination;
mod project_assignments;
mod project_reads;
mod task_reads;
mod time_entry_writes;
mod user_access;

#[cfg(test)]
mod api_tests;
#[cfg(test)]
mod oauth_tests;
#[cfg(test)]
mod pagination_tests;
#[cfg(test)]
mod switch_tests;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRef, Path, Query, State},
    http::StatusCode,
    routing::{MethodRouter, any, get, patch},
};
use chrono::{DateTime, NaiveDate, Utc};
use horae_api::rate_limit::RateLimiter;
use horae_api::v1::V1State;
use horae_core::permissions::catalog::Permission;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use auth::{AuthUser, RouteScope};
use errors::ApiError;
use horae_harvest::v2::compat::*;
use ids::Kind;
use pagination::{Key, ListRequest, Listed, Window};

// The backend lives in `horae-server`; keep the `crate::ops` (…) paths.
#[cfg(test)]
use horae_server::testing;
use horae_server::{db, importers, ops};

use crate::ops::{Actor, OpError};

/// What the handlers share: `/api/v1`'s state, so a token's requests to both
/// APIs count against one limit (spec 022 R-7), and writes reach the same
/// plugins and budget checks.
#[derive(Clone)]
struct HarvestState(V1State);

impl FromRef<HarvestState> for PgPool {
    fn from_ref(state: &HarvestState) -> Self {
        state.0.pool.clone()
    }
}

impl FromRef<HarvestState> for Arc<RateLimiter> {
    fn from_ref(state: &HarvestState) -> Self {
        state.0.limiter.clone()
    }
}

/// The Harvest-compatible API when `enabled` (`HORAE_HARVEST_API`), otherwise
/// an empty 404 for its paths, so they do not fall through to the web app's
/// HTML shell. `v1` is the state `/api/v1` serves with.
pub fn mount(enabled: bool, v1: V1State) -> Router {
    if enabled {
        return serve(v1);
    }
    let off = any(|| async { StatusCode::NOT_FOUND });
    Router::new()
        .route("/harvest/v2/{*rest}", off.clone())
        .route("/v2/{*rest}", off)
}

/// A state with no plugins and no budget checks behind its writes.
#[cfg(test)]
fn test_state(db: PgPool, limiter: Arc<RateLimiter>) -> V1State {
    V1State {
        pool: db,
        limiter,
        cursor_key: Default::default(),
        events: Arc::new(ops::NoEvents),
        budgets: None,
    }
}

#[cfg(test)]
fn router(db: PgPool, limiter: Arc<RateLimiter>) -> Router {
    serve(test_state(db, limiter))
}

/// Every route, with the scope an OAuth token needs to read it ([`RouteScope`];
/// writes need `time:write`).
fn routes() -> impl Iterator<Item = (&'static str, RouteScope, MethodRouter<HarvestState>)> {
    use RouteScope::{Any, Catalog, Time};
    use time_entry_writes as writes;
    let routes: [(&str, RouteScope, MethodRouter<HarvestState>); 14] = [
        ("/company", Any, get(company::company)),
        ("/users/me", Any, get(users_me)),
        (
            "/users/me/project_assignments",
            Catalog,
            get(project_assignments::mine),
        ),
        (
            "/time_entries",
            Time,
            get(list_time_entries).post(writes::create),
        ),
        (
            "/time_entries/{id}",
            Time,
            get(get_time_entry)
                .patch(writes::update)
                .delete(writes::delete),
        ),
        ("/time_entries/{id}/stop", Time, patch(writes::stop)),
        ("/time_entries/{id}/restart", Time, patch(writes::restart)),
        ("/projects", Catalog, get(list_projects)),
        ("/projects/{id}", Catalog, get(get_project)),
        ("/clients", Catalog, get(list_clients)),
        ("/clients/{id}", Catalog, get(get_client)),
        ("/tasks", Catalog, get(list_tasks)),
        ("/tasks/{id}", Catalog, get(get_task)),
        ("/users", Catalog, get(list_users)),
    ];
    // The kinds Horae lacks answer an empty list to any caller with catalog
    // access, the closest of the three scopes.
    let absent = absent::KINDS.map(|(path, kind)| (path, Catalog, absent::list(kind)));
    routes.into_iter().chain(absent)
}

fn serve(v1: V1State) -> Router {
    let mut api = Router::new();
    for (path, scope, handler) in routes() {
        let handler = handler.layer(axum::Extension(scope));
        // A `.json` suffix is served like the bare path (FR-026). Ids drop it
        // themselves (`ids::resolve`), so only paths ending in a fixed segment
        // need a twin.
        if !path.ends_with('}') {
            api = api.route(&format!("{path}.json"), handler.clone());
        }
        api = api.route(path, handler);
    }
    Router::new()
        .nest("/harvest/v2", api.clone())
        .nest("/v2", api)
        .layer(axum::middleware::map_response(no_store))
        .with_state(HarvestState(v1))
}

/// Responses are per caller, and list links echo the request's `Host` and
/// `X-Forwarded-Proto`, so no shared cache may keep them; a session-cookie
/// read is not covered by the caching rules for `Authorization`.
async fn no_store(mut response: axum::response::Response) -> axum::response::Response {
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    response
}

// ── Error helper ────────────────────────────────────────────────────────────

type ApiResult<T> = Result<Json<T>, ApiError>;

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError::internal(e)
}

fn not_found() -> ApiError {
    ApiError::not_found()
}

/// The caller is signed in but not senior enough for this collection. Kept
/// distinct from [`not_found`] on purpose: a 404 would tell a Harvest client the
/// org holds no such data, and the login guard deliberately never redirects
/// under `/harvest/`, so 403 is the only honest answer here.
fn forbidden(msg: &str) -> ApiError {
    ApiError::forbidden(msg)
}

/// Opens a read for `caller` and loads their stored permission state, held
/// until the read finishes. `subject` names the collection in refusals: an
/// actor without valid stored state, or no longer an active person of the
/// organization, is refused rather than shown an empty collection.
async fn begin_read(
    db: &PgPool,
    caller: &AuthUser,
    subject: &str,
) -> Result<ops::ReadTx, ApiError> {
    let read = ops::begin_read(db, caller.caller())
        .await
        .map_err(|error| match error {
            // `Actor::load` refuses only a missing or invalid permission state.
            // Revisit this text if `ops::begin_read` gains other refusals.
            OpError::Forbidden(_) => {
                forbidden(&format!("{subject} permission state is unavailable"))
            }
            error => ApiError::from(error),
        })?;
    if !read.actor.is_active() {
        let refusal = forbidden(&format!("{subject} access is unavailable"));
        return Err(read.refuse(refusal).await);
    }
    Ok(read)
}

/// Ends `read` with the handler's result. A refusal rolls back before it is
/// answered, so the read's authority locks do not outlive the response.
async fn finish_read<T>(read: ops::ReadTx, result: Result<T, ApiError>) -> Result<T, ApiError> {
    match result {
        Ok(value) => {
            read.finish(Ok(())).await.map_err(internal)?;
            Ok(value)
        }
        Err(error) => Err(read.refuse(error).await),
    }
}

/// The update-time horizon a list serves below ([`pagination::horizon`]).
/// A read of one record by `id` takes none: it is never a sync watermark.
async fn list_horizon(
    read: &mut ops::ReadTx,
    id: Option<Uuid>,
) -> Result<Option<DateTime<Utc>>, ApiError> {
    match id {
        Some(_) => Ok(None),
        None => pagination::horizon(&mut read.tx).await.map(Some),
    }
}

/// What the caller's stored grants let them read from time entries. Managed
/// grants cover entries of the people they manage and of the projects they
/// manage, as in the timesheet.
#[derive(Clone, Copy)]
struct TimeView {
    own: bool,
    managed: bool,
    all: bool,
    /// An entry's billable rate is an effective project value, so the managed
    /// scope is the managed projects only, never the managed people
    /// (`horae_core::permissions::rates::billable_rate_access`).
    billable_rates_managed: bool,
    billable_rates_all: bool,
    /// Cost rates have no managed scope.
    cost_rates: bool,
    projects_managed: bool,
    projects_all: bool,
}

impl TimeView {
    fn of(actor: &Actor) -> Self {
        Self {
            own: actor.has(Permission::TimeReadOwn),
            managed: actor.has(Permission::TimeReadManaged),
            all: actor.has(Permission::TimeReadAll),
            billable_rates_managed: actor.has(Permission::BillableRateReadManaged),
            billable_rates_all: actor.has(Permission::BillableRateReadAll),
            cost_rates: actor.has(Permission::CostRateReadAll),
            projects_managed: actor.has(Permission::ProjectReadManaged),
            projects_all: actor.has(Permission::ProjectReadAll),
        }
    }
}

/// The `user_id` a time-entry query may read, given what the caller asked for.
///
/// Someone who reads only their own time is confined to their own rows, and
/// asking for a colleague's rows is refused rather than quietly answered with
/// a different row set. Managed and organization-wide readers may filter to
/// anyone; the row scope still decides what they see.
fn scoped_user_filter(
    caller: &AuthUser,
    view: TimeView,
    requested: Option<Uuid>,
) -> Result<Option<Uuid>, ApiError> {
    if view.all || view.managed {
        return Ok(requested);
    }
    if !view.own {
        return Err(forbidden("Reading time entries requires time access"));
    }
    match requested {
        Some(id) if id != caller.user_id => Err(forbidden(
            "Reading another user's time entries requires access to their time",
        )),
        _ => Ok(Some(caller.user_id)),
    }
}

// ── /users/me ───────────────────────────────────────────────────────────────

async fn users_me(user: AuthUser, State(db): State<PgPool>) -> ApiResult<HarvestUser> {
    // Everyone reads their own identity; rates follow the same rule as /users.
    let mut access = user_access::begin(&db, &user).await?;
    let filters = UserFilters::default();
    let listed = user_rows(
        &mut access.read,
        &user,
        &filters,
        Some(user.user_id),
        &Window::one(),
    )
    .await;
    let listed = finish_read(access.read, listed).await?;
    let row = listed.rows.first().ok_or_else(not_found)?;

    Ok(Json(user_row_to_harvest(row, access.rates)))
}

// ── Time Entries ────────────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
pub(crate) struct TimeEntryFilters {
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    /// Horae keeps no external references, so any value matches nothing.
    pub external_reference_id: Option<String>,
    pub is_billed: Option<bool>,
    pub is_running: Option<bool>,
    /// `unsubmitted`, `submitted` or `approved` (which covers invoiced).
    pub approval_status: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub cursor: Option<String>,
    pub updated_since: Option<DateTime<Utc>>,
}

/// The entry states an `approval_status` value covers.
fn approval_states(status: &str) -> Result<&'static [&'static str], ApiError> {
    match status {
        "unsubmitted" => Ok(&["open"]),
        "submitted" => Ok(&["submitted"]),
        "approved" => Ok(&["approved", "invoiced"]),
        _ => Err(ApiError::unprocessable(
            "approval_status must be unsubmitted, submitted or approved",
        )),
    }
}

#[derive(sqlx::FromRow)]
struct TimeEntryRow {
    uuid: Uuid,
    id: i64,
    spent_date: NaiveDate,
    minutes: i32,
    start_minute: Option<i32>,
    rounded_minutes: Option<i32>,
    notes: Option<String>,
    billable: bool,
    is_running: bool,
    started_at: Option<DateTime<Utc>>,
    state: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // Joined fields
    user_id: i64,
    user_name: String,
    project_id: i64,
    project_name: String,
    project_code: Option<String>,
    task_id: i64,
    task_name: String,
    client_id: i64,
    client_name: String,
    // Rates: billing in the project's currency, cost in the organization's.
    billable_rate_cents: Option<i64>,
    user_cost_rate_cents: Option<i64>,
    project_currency: String,
    org_currency: String,
    budget_kind: Option<String>,
}

/// `now` measures a running timer: its `hours` include the time since it
/// last started, and `hours_without_timer` are the stored minutes before that.
fn time_entry_row_to_harvest(
    row: &TimeEntryRow,
    org_round_min: u32,
    org_round_dir: horae_core::types::RoundDir,
    now: DateTime<Utc>,
) -> HarvestTimeEntry {
    let running = match row.started_at {
        Some(started_at) if row.is_running => {
            horae_core::duration::minutes_between(started_at, now)
        }
        _ => 0,
    };
    let stored = i64::from(row.minutes);
    let hours = Hours(stored + i64::from(running));
    let rounded_hours = Hours(
        horae_core::rounding::effective_minutes(
            row.minutes as u32 + running,
            row.rounded_minutes.map(|minutes| minutes as u32),
            org_round_min,
            org_round_dir,
        )
        .into(),
    );
    let is_locked = matches!(row.state.as_str(), "submitted" | "approved" | "invoiced");
    let locked_reason = match row.state.as_str() {
        "submitted" => Some("Pending Approval".to_string()),
        "approved" => Some("Approved".to_string()),
        "invoiced" => Some("Invoiced".to_string()),
        _ => None,
    };
    let approval_status = match row.state.as_str() {
        "open" => "unsubmitted",
        "submitted" => "submitted",
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
        id: row.id,
        spent_date: row.spent_date.to_string(),
        hours,
        hours_without_timer: Hours(stored),
        rounded_hours,
        started_time,
        ended_time,
        notes: row.notes.clone(),
        is_locked,
        locked_reason,
        is_closed: is_locked,
        // With or without an invoice: time billed in Harvest on an invoice
        // Horae could not import is invoiced with none.
        is_billed: row.state == "invoiced",
        is_running: row.is_running,
        timer_started_at: row.started_at.map(|t| t.to_rfc3339()),
        billable: row.billable,
        budgeted: row.budget_kind.as_deref().map(|kind| kind != "none"),
        // The query already withheld the rates the caller may not read.
        billable_rate: row
            .billable_rate_cents
            .map(|cents| Amount::new(cents, &row.project_currency)),
        cost_rate: row
            .user_cost_rate_cents
            .map(|cents| Amount::new(cents, &row.org_currency)),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        user: HarvestRef {
            id: row.user_id,
            name: row.user_name.clone(),
        },
        client: HarvestRef {
            id: row.client_id,
            name: row.client_name.clone(),
        },
        project: HarvestProjectRef {
            id: row.project_id,
            name: row.project_name.clone(),
            code: row.project_code.clone(),
        },
        task: HarvestRef {
            id: row.task_id,
            name: row.task_name.clone(),
        },
        approval_status: approval_status.to_string(),
        user_assignment: None,
        task_assignment: None,
        invoice: None,
        external_reference: None,
    }
}

async fn list_time_entries(
    user: AuthUser,
    State(db): State<PgPool>,
    list: ListRequest,
    Query(filters): Query<TimeEntryFilters>,
) -> ApiResult<HarvestPagination<HarvestTimeEntry>> {
    time_entries_page(&db, &user, filters, &list).await
}

/// The entries the caller's `TimeRead*` grants cover. Billable rates need
/// `BillableRateReadAll`, or `BillableRateReadManaged` for managed people and
/// projects; cost rates need `CostRateReadAll`. Withheld rates are null.
async fn time_entries_page(
    db: &PgPool,
    caller: &AuthUser,
    filters: TimeEntryFilters,
    list: &ListRequest,
) -> ApiResult<HarvestPagination<HarvestTimeEntry>> {
    let window = list.window(filters.page, filters.per_page, filters.cursor.as_deref())?;
    let org = caller.org_id;
    let records = RecordFilters {
        user: ids::filter(db, org, Kind::User, filters.user_id.as_deref()).await?,
        client: ids::filter(db, org, Kind::Client, filters.client_id.as_deref()).await?,
        project: ids::filter(db, org, Kind::Project, filters.project_id.as_deref()).await?,
        task: ids::filter(db, org, Kind::Task, filters.task_id.as_deref()).await?,
    };
    let (round_min, round_dir) = crate::db::org_rounding(db, caller.org_id)
        .await
        .map_err(internal)?;
    // Count and rows are read under the grants held for the whole read.
    let mut read = begin_read(db, caller, "Time entry").await?;
    let result = time_entry_rows(&mut read, caller, records, &filters, &window).await;
    let listed = finish_read(read, result).await?;

    let now = Utc::now();
    let entries: Vec<HarvestTimeEntry> = listed
        .rows
        .iter()
        .map(|r| time_entry_row_to_harvest(r, round_min, round_dir, now))
        .collect();

    Ok(Json(list.keyed(
        "time_entries",
        &listed,
        |row| Key {
            date: Some(row.spent_date),
            id: row.uuid,
        },
        entries,
        &window,
    )))
}

/// The records a time-entry list is filtered to, resolved from their ids.
struct RecordFilters {
    user: Option<Uuid>,
    client: Option<Uuid>,
    project: Option<Uuid>,
    task: Option<Uuid>,
}

/// The entry count and page in the caller's time scope, read under `read`.
/// Entries run latest first by `(spent_date, id)`. One statement picks the
/// page's ids and counts the list, so the two agree; a second reads the
/// page's entries.
async fn time_entry_rows(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    records: RecordFilters,
    filters: &TimeEntryFilters,
    window: &Window,
) -> Result<Listed<TimeEntryRow>, ApiError> {
    let view = TimeView::of(&read.actor);
    let user_id_filter = scoped_user_filter(caller, view, records.user)?;
    let states = filters
        .approval_status
        .as_deref()
        .map(approval_states)
        .transpose()?;
    let horizon = pagination::horizon(&mut read.tx).await?;
    let position = window.position();

    let page = sqlx::query!(
        r#"WITH scoped AS MATERIALIZED (
             SELECT te.id, te.spent_date FROM time_entries te
             JOIN projects p ON p.id = te.project_id
             CROSS JOIN LATERAL (SELECT
               EXISTS (SELECT 1 FROM person_management_assignments m WHERE m.org_id = te.org_id
                 AND m.manager_id = $8 AND m.managed_user_id = te.user_id) AS person,
               EXISTS (SELECT 1 FROM project_management_assignments m WHERE m.org_id = te.org_id
                 AND m.manager_id = $8 AND m.project_id = te.project_id) AS project) managed
             WHERE te.org_id = $1
               AND ($11 OR ($9 AND te.user_id = $8) OR ($10 AND (managed.person OR managed.project)))
               AND ($2::uuid IS NULL OR te.user_id = $2)
               AND ($3::uuid IS NULL OR te.project_id = $3)
               AND ($4::date IS NULL OR te.spent_date >= $4::date)
               AND ($5::date IS NULL OR te.spent_date <= $5::date)
               AND ($6::bool IS NULL OR te.is_running = $6)
               AND ($7::timestamptz IS NULL OR te.updated_at >= $7::timestamptz)
               AND ($12::uuid IS NULL OR p.client_id = $12)
               AND ($13::uuid IS NULL OR te.task_id = $13)
               AND ($14::bool IS NULL OR (te.state = 'invoiced') = $14)
               AND ($15::text[] IS NULL OR te.state::text = ANY($15))
               AND $16::text IS NULL
               AND te.updated_at < $17
           ), totals AS (
             SELECT COUNT(*) total,
               CASE WHEN $19::uuid IS NULL THEN $21
                 WHEN $20 THEN GREATEST(COUNT(*) FILTER (WHERE (spent_date, id) > ($18, $19)) - $22, 0)
                 ELSE COUNT(*) FILTER (WHERE (spent_date, id) >= ($18, $19)) END start,
               CASE WHEN $19::uuid IS NOT NULL AND $20
                 THEN LEAST(COUNT(*) FILTER (WHERE (spent_date, id) > ($18, $19)), $22)
                 ELSE $22 END take
             FROM scoped
           )
           SELECT total AS "total!", start AS "start!",
             ARRAY(SELECT id FROM scoped ORDER BY spent_date DESC, id DESC
                   LIMIT (SELECT take FROM totals) OFFSET (SELECT start FROM totals)) AS "ids!"
           FROM totals"#,
        caller.org_id,
        user_id_filter,
        records.project,
        filters.from as Option<NaiveDate>,
        filters.to as Option<NaiveDate>,
        filters.is_running,
        filters.updated_since as Option<DateTime<Utc>>,
        caller.user_id,
        view.own,
        view.managed,
        view.all,
        records.client,
        records.task,
        filters.is_billed,
        states as Option<&[&str]>,
        filters.external_reference_id,
        horizon as DateTime<Utc>,
        position.date as Option<NaiveDate>,
        position.id,
        position.before,
        window.offset,
        window.per_page,
    )
    .fetch_one(&mut *read.tx)
    .await
    .map_err(internal)?;
    let rows = entries_by_id(read, caller, view, &page.ids).await?;
    Ok(Listed {
        total: page.total,
        start: page.start,
        rows,
    })
}

/// The entries `ids` names in the caller's time scope, latest first, with the
/// rates and budget the caller may read. An id outside the scope is left out.
async fn entries_by_id(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    view: TimeView,
    ids: &[Uuid],
) -> Result<Vec<TimeEntryRow>, ApiError> {
    let rows = sqlx::query_as!(
        TimeEntryRow,
        r#"SELECT te.id AS uuid, te.harvest_id AS id,
               te.spent_date as "spent_date: chrono::NaiveDate",
               te.minutes, te.start_minute, te.rounded_minutes, te.notes,
               (te.billable AND (te.state = 'invoiced' OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) as "billable!", te.is_running,
               te.started_at as "started_at: chrono::DateTime<chrono::Utc>",
               te.state::text AS "state!: String",
               te.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               te.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
               u.harvest_id AS user_id, u.name AS user_name,
               p.harvest_id AS project_id,
               p.name AS project_name, p.code AS project_code,
               t.harvest_id AS task_id, t.name AS task_name,
               c.harvest_id AS client_id, c.name AS client_name,
               CASE WHEN $8 OR ($7 AND managed.project) THEN COALESCE(line.rate_cents,
                 CASE WHEN ps.project_id IS NULL OR p.project_type = 'time_and_materials'
                 THEN resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                 CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                 CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
               ) END) END AS "billable_rate_cents?",
               CASE WHEN $9 THEN COALESCE(mc.cost_rate_cents, u.cost_rate_cents) END AS user_cost_rate_cents,
               p.currency AS project_currency, o.default_currency AS org_currency,
               CASE WHEN $11 OR ($10 AND managed.project)
                 OR (member.id IS NOT NULL AND COALESCE(ps.report_visibility, 'project_members') = 'project_members')
                 THEN p.budget_kind::text END AS "budget_kind?: String"
         FROM time_entries te
         JOIN users u ON u.id = te.user_id
         JOIN projects p ON p.id = te.project_id
         JOIN tasks t ON t.id = te.task_id
         LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
         LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
         LEFT JOIN assignments member ON member.project_id = te.project_id AND member.user_id = $3
         LEFT JOIN invoice_line_items line ON line.invoice_id = te.invoice_id AND line.time_entry_id = te.id
         JOIN clients c ON c.id = p.client_id
         JOIN organizations o ON o.id = te.org_id
         LEFT JOIN project_settings ps ON ps.project_id = p.id
         LEFT JOIN project_member_costs mc ON mc.project_id = te.project_id AND mc.user_id = te.user_id
         CROSS JOIN LATERAL (SELECT
           EXISTS (SELECT 1 FROM person_management_assignments m WHERE m.org_id = te.org_id
             AND m.manager_id = $3 AND m.managed_user_id = te.user_id) AS person,
           EXISTS (SELECT 1 FROM project_management_assignments m WHERE m.org_id = te.org_id
             AND m.manager_id = $3 AND m.project_id = te.project_id) AS project) managed
         WHERE te.org_id = $1 AND te.id = ANY($2)
           AND ($6 OR ($4 AND te.user_id = $3) OR ($5 AND (managed.person OR managed.project)))
         ORDER BY te.spent_date DESC, te.id DESC"#,
        caller.org_id,
        ids,
        caller.user_id,
        view.own,
        view.managed,
        view.all,
        view.billable_rates_managed,
        view.billable_rates_all,
        view.cost_rates,
        view.projects_managed,
        view.projects_all,
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    Ok(rows)
}

async fn get_time_entry(
    user: AuthUser,
    State(db): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<HarvestTimeEntry> {
    let id = ids::path(&db, user.org_id, Kind::TimeEntry, &id).await?;
    time_entry_by_id(&db, &user, id).await
}

/// A single entry out of the same scoped row set as the listing, with the same
/// rate redaction. An entry outside the caller's time scope is simply not in
/// their collection, so it reads as missing rather than forbidden — the answer
/// an id from another org already gets.
async fn time_entry_by_id(db: &PgPool, caller: &AuthUser, id: Uuid) -> ApiResult<HarvestTimeEntry> {
    let (round_min, round_dir) = crate::db::org_rounding(db, caller.org_id)
        .await
        .map_err(internal)?;
    let mut read = begin_read(db, caller, "Time entry").await?;
    let result = scoped_time_entry(&mut read, caller, id).await;
    let row = finish_read(read, result).await?;

    Ok(Json(time_entry_row_to_harvest(
        &row,
        round_min,
        round_dir,
        Utc::now(),
    )))
}

async fn scoped_time_entry(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    id: Uuid,
) -> Result<TimeEntryRow, ApiError> {
    let view = TimeView::of(&read.actor);
    // Refuses a caller without any time-read grant.
    scoped_user_filter(caller, view, None)?;
    entries_by_id(read, caller, view, &[id])
        .await?
        .pop()
        .ok_or_else(not_found)
}

// ── Projects ────────────────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
pub(crate) struct ProjectFilters {
    pub is_active: Option<bool>,
    pub client_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub cursor: Option<String>,
    pub updated_since: Option<DateTime<Utc>>,
}

struct ProjectRow {
    uuid: Uuid,
    updated_at: DateTime<Utc>,
    id: i64,
    name: String,
    code: Option<String>,
    project_type: String,
    active: bool,
    budget_kind: String,
    budget_amount_cents: Option<i64>,
    budget_minutes: Option<i64>,
    currency: String,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    created_at: DateTime<Utc>,
    client_id: i64,
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
        "hours" => row.budget_minutes.map(|m| Budget::Hours(Hours(m))),
        "amount" => row
            .budget_amount_cents
            .map(|c| Budget::Amount(Amount::new(c, &row.currency))),
        _ => None,
    };

    HarvestProject {
        id: row.id,
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
        updated_at: row.updated_at.to_rfc3339(),
        client: HarvestRef {
            id: row.client_id,
            name: row.client_name.clone(),
        },
    }
}

async fn list_projects(
    user: AuthUser,
    State(db): State<PgPool>,
    list: ListRequest,
    Query(filters): Query<ProjectFilters>,
) -> ApiResult<HarvestPagination<HarvestProject>> {
    let window = list.window(filters.page, filters.per_page, filters.cursor.as_deref())?;
    let listed = project_reads::read(&db, &user, &filters, None, &window).await?;
    let items = listed.rows.iter().map(project_row_to_harvest).collect();
    Ok(Json(list.keyed(
        "projects",
        &listed,
        |row| Key::id(row.uuid),
        items,
        &window,
    )))
}

async fn get_project(
    user: AuthUser,
    State(db): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<HarvestProject> {
    let id = ids::path(&db, user.org_id, Kind::Project, &id).await?;
    let listed = project_reads::read(
        &db,
        &user,
        &ProjectFilters::default(),
        Some(id),
        &Window::one(),
    )
    .await?;
    let row = listed.rows.first().ok_or_else(not_found)?;
    Ok(Json(project_row_to_harvest(row)))
}

// ── Clients ─────────────────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
pub(crate) struct ClientFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub cursor: Option<String>,
    pub updated_since: Option<DateTime<Utc>>,
}

struct ClientRow {
    uuid: Uuid,
    updated_at: DateTime<Utc>,
    id: i64,
    name: String,
    active: bool,
    address: Option<String>,
    currency: String,
    created_at: DateTime<Utc>,
}

fn client_row_to_harvest(row: &ClientRow) -> HarvestClient {
    HarvestClient {
        id: row.id,
        name: row.name.clone(),
        is_active: row.active,
        address: row.address.clone(),
        currency: row.currency.clone(),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

async fn list_clients(
    user: AuthUser,
    State(db): State<PgPool>,
    list: ListRequest,
    Query(filters): Query<ClientFilters>,
) -> ApiResult<HarvestPagination<HarvestClient>> {
    let window = list.window(filters.page, filters.per_page, filters.cursor.as_deref())?;
    let listed = client_reads::read(&db, &user, &filters, None, &window).await?;
    let items = listed.rows.iter().map(client_row_to_harvest).collect();
    Ok(Json(list.keyed(
        "clients",
        &listed,
        |row| Key::id(row.uuid),
        items,
        &window,
    )))
}

async fn get_client(
    user: AuthUser,
    State(db): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<HarvestClient> {
    let id = ids::path(&db, user.org_id, Kind::Client, &id).await?;
    let listed = client_reads::read(
        &db,
        &user,
        &ClientFilters::default(),
        Some(id),
        &Window::one(),
    )
    .await?;
    let row = listed.rows.first().ok_or_else(not_found)?;

    Ok(Json(client_row_to_harvest(row)))
}

// ── Tasks ───────────────────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
pub(crate) struct TaskFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub cursor: Option<String>,
    pub updated_since: Option<DateTime<Utc>>,
}

struct TaskRow {
    uuid: Uuid,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    id: i64,
    name: String,
    active: bool,
    billable_default: bool,
    /// A rate template with no currency, written with two decimals.
    default_rate_cents: Option<i64>,
}

fn task_row_to_harvest(row: &TaskRow) -> HarvestTask {
    HarvestTask {
        id: row.id,
        name: row.name.clone(),
        is_active: row.active,
        billable_by_default: row.billable_default,
        default_hourly_rate: row.default_rate_cents.map(|cents| Amount::new(cents, "")),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

async fn list_tasks(
    user: AuthUser,
    State(db): State<PgPool>,
    list: ListRequest,
    Query(filters): Query<TaskFilters>,
) -> ApiResult<HarvestPagination<HarvestTask>> {
    let window = list.window(filters.page, filters.per_page, filters.cursor.as_deref())?;
    let listed = task_reads::read(&db, &user, &filters, None, &window).await?;
    let items = listed.rows.iter().map(task_row_to_harvest).collect();
    Ok(Json(list.keyed(
        "tasks",
        &listed,
        |row| Key::id(row.uuid),
        items,
        &window,
    )))
}

async fn get_task(
    user: AuthUser,
    State(db): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<HarvestTask> {
    let id = ids::path(&db, user.org_id, Kind::Task, &id).await?;
    let listed = task_reads::read(
        &db,
        &user,
        &TaskFilters::default(),
        Some(id),
        &Window::one(),
    )
    .await?;
    let row = listed.rows.first().ok_or_else(not_found)?;

    Ok(Json(task_row_to_harvest(row)))
}

// ── Users ───────────────────────────────────────────────────────────────────

#[derive(Default, Deserialize)]
pub(crate) struct UserFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub cursor: Option<String>,
    pub updated_since: Option<DateTime<Utc>>,
}

struct UserRow {
    uuid: Uuid,
    /// Updated when the person or their stored permission state (which
    /// holds `is_admin`) changes.
    updated_at: DateTime<Utc>,
    id: i64,
    name: String,
    email: String,
    active: bool,
    /// The person's stored Administrator identity, not their grants.
    is_admin: bool,
    cost_rate_cents: Option<i64>,
    billable_rate_cents: Option<i64>,
    /// A person's rates are in the organization's currency.
    org_currency: String,
    created_at: DateTime<Utc>,
}

/// Rates follow the caller's grants (see [`user_access::begin`]), never being
/// the person.
fn user_row_to_harvest(row: &UserRow, rates: user_access::RateView) -> HarvestUser {
    // Split "First Last" into first_name / last_name
    let (first, last) = match row.name.split_once(' ') {
        Some((f, l)) => (f.to_string(), l.to_string()),
        None => (row.name.clone(), String::new()),
    };

    HarvestUser {
        id: row.id,
        first_name: first,
        last_name: last,
        email: row.email.clone(),
        is_active: row.active,
        is_admin: row.is_admin,
        cost_rate: row
            .cost_rate_cents
            .filter(|_| rates.cost)
            .map(|cents| Amount::new(cents, &row.org_currency)),
        default_hourly_rate: row
            .billable_rate_cents
            .filter(|_| rates.billable)
            .map(|cents| Amount::new(cents, &row.org_currency)),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
    }
}

async fn list_users(
    user: AuthUser,
    State(db): State<PgPool>,
    list: ListRequest,
    Query(filters): Query<UserFilters>,
) -> ApiResult<HarvestPagination<HarvestUser>> {
    users_page(&db, &user, filters, &list).await
}

/// The org's people, with the rates the caller may read and the others
/// omitted. Listing needs `PeopleReadAll`; everyone still reads their own
/// `/users/me`.
async fn users_page(
    db: &PgPool,
    caller: &AuthUser,
    filters: UserFilters,
    list: &ListRequest,
) -> ApiResult<HarvestPagination<HarvestUser>> {
    let mut access = user_access::begin(db, caller).await?;
    let result = async {
        if !access.list {
            return Err(forbidden("Listing users requires people access"));
        }
        let window = list.window(filters.page, filters.per_page, filters.cursor.as_deref())?;
        let listed = user_rows(&mut access.read, caller, &filters, None, &window).await?;
        Ok((window, listed))
    }
    .await;
    let (window, listed) = finish_read(access.read, result).await?;
    let items = listed
        .rows
        .iter()
        .map(|row| user_row_to_harvest(row, access.rates))
        .collect();
    Ok(Json(list.keyed(
        "users",
        &listed,
        |row| Key::id(row.uuid),
        items,
        &window,
    )))
}

/// The organization's people in list order, or the one `user_id` names.
async fn user_rows(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    filters: &UserFilters,
    user_id: Option<Uuid>,
    window: &Window,
) -> Result<Listed<UserRow>, ApiError> {
    let horizon = list_horizon(read, user_id).await?;
    let position = window.position();
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT u.id,u.harvest_id,u.name,u.email,u.active,
                    COALESCE(p.is_administrator, false) is_admin,
                    u.cost_rate_cents,u.billable_rate_cents,u.created_at,
                    GREATEST(u.updated_at, p.updated_at) updated_at
             FROM users u
             LEFT JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
             WHERE u.org_id = $1
               AND ($2::bool IS NULL OR u.active = $2)
               AND ($3::uuid IS NULL OR u.id = $3)
           ), changed AS MATERIALIZED (
             SELECT * FROM visible WHERE ($4::timestamptz IS NULL OR updated_at >= $4)
               AND ($9::timestamptz IS NULL OR updated_at < $9)
           ), totals AS (
             SELECT COUNT(*) total,
               CASE WHEN $7::uuid IS NULL THEN $6
                 WHEN $8 THEN GREATEST(COUNT(*) FILTER (WHERE id > $7) - $5, 0)
                 ELSE COUNT(*) FILTER (WHERE id >= $7) END start,
               CASE WHEN $7::uuid IS NOT NULL AND $8
                 THEN LEAST(COUNT(*) FILTER (WHERE id > $7), $5) ELSE $5 END take
             FROM changed
           ), page AS (
             SELECT * FROM changed ORDER BY id DESC
             LIMIT (SELECT take FROM totals) OFFSET (SELECT start FROM totals)
           )
           SELECT totals.total AS "total!",totals.start AS "start!",u.id AS "uuid?",
                  u.harvest_id AS "id?",u.name AS "name?",u.email AS "email?",u.active AS "active?",
                  u.is_admin AS "is_admin?",u.cost_rate_cents,u.billable_rate_cents,
                  (SELECT default_currency FROM organizations o WHERE o.id = $1) AS "org_currency!",
                  u.created_at AS "created_at?: chrono::DateTime<chrono::Utc>",
                  u.updated_at AS "updated_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page u ON true ORDER BY u.id DESC"#,
        caller.org_id,
        filters.is_active,
        user_id,
        filters.updated_since as Option<DateTime<Utc>>,
        window.per_page,
        window.offset,
        position.id,
        position.before,
        horizon as Option<DateTime<Utc>>,
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    let first = records
        .first()
        .ok_or_else(|| internal("Missing user count"))?;
    let (total, start) = (first.total, first.start);
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        // Only the row of an empty page has no id.
        let Some(uuid) = record.uuid else { continue };
        rows.push(UserRow {
            uuid,
            updated_at: record
                .updated_at
                .ok_or_else(|| internal("Missing user update time"))?,
            id: record.id.ok_or_else(|| internal("Missing api number"))?,
            name: record.name.ok_or_else(|| internal("Missing user name"))?,
            email: record.email.ok_or_else(|| internal("Missing user email"))?,
            active: record
                .active
                .ok_or_else(|| internal("Missing user activity"))?,
            // The caller's own identity comes from their actor, which an
            // OAuth token never makes an administrator.
            is_admin: if uuid == read.actor.id() {
                read.actor.administrator()
            } else {
                record
                    .is_admin
                    .ok_or_else(|| internal("Missing administrator flag"))?
            },
            cost_rate_cents: record.cost_rate_cents,
            billable_rate_cents: record.billable_rate_cents,
            org_currency: record.org_currency,
            created_at: record
                .created_at
                .ok_or_else(|| internal("Missing user creation time"))?,
        });
    }
    Ok(Listed { total, start, rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::testing::provision;
    use crate::testing::{SeedIds, seed};
    use horae_core::permissions::catalog::BuiltInProfile;
    use horae_core::types::EntryState;

    // ── Authorization ───────────────────────────────────────────────────────
    // This API rides the same session cookie as the SPA, so these assert that it
    // decides with the same stored grants: other people's rows need a time
    // grant that covers them, and each rate its own rate grant.

    /// Seeded projects and organizations bill in euros.
    fn eur(cents: i64) -> Amount {
        Amount::new(cents, "EUR")
    }

    fn list() -> ListRequest {
        ListRequest::test()
    }

    fn caller(ids: &SeedIds) -> AuthUser {
        AuthUser {
            user_id: ids.user_id,
            org_id: ids.org_id,
            channel: ops::Channel::Session,
        }
    }

    fn no_time_entry_filters() -> TimeEntryFilters {
        TimeEntryFilters::default()
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn members_cannot_read_own_project_rates_or_cost_overrides(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let id = insert_entry(&pool, &ids, ids.user_id, "Private costs").await;
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
            Uuid::now_v7(),
            ids.project_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 3000, cost_rate_cents = 1000 WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,1500)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
            .execute(&pool).await.unwrap();
        let member = caller(&ids);
        let Json(entry) = time_entry_by_id(&pool, &member, id).await.unwrap();
        let Json(page) = time_entries_page(&pool, &member, no_time_entry_filters(), &list())
            .await
            .unwrap();
        assert_eq!((entry.billable_rate, entry.cost_rate), (None, None));
        assert_eq!(
            (
                &page.data["time_entries"][0].billable_rate,
                &page.data["time_entries"][0].cost_rate
            ),
            (&None, &None)
        );
        assert_eq!(entry.hours, Hours(60));
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn imported_legacy_rates_survive_retry_in_harvest_list_and_detail(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 3000, cost_rate_cents = 1200 WHERE id = $1",
            ids.user_id,
        )
        .execute(&pool)
        .await
        .unwrap();
        let csv = format!(
            "Date,Client,Project,Task,Email,Hours,Billable?,Billable Rate,Currency\n\
             2026-09-07,Acme,Imported,Design,{}@test.com,1.25,Yes,80,EUR\n\
             2026-09-07,Acme,Imported,Review,{}@test.com,0.5,Yes,0,EUR\n\
             2026-09-07,Acme,Imported,Dev,{}@test.com,1,Yes,,EUR\n",
            ids.user_id, ids.user_id, ids.user_id,
        );
        for _ in 0..2 {
            let report = crate::importers::harvest::csv_source::import_body(
                &pool,
                ids.org_id,
                "EUR",
                crate::importers::harvest::csv_source::csv_body(csv.clone()),
                horae_core::importers::harvest::types::ImportMode::Commit,
            )
            .await
            .unwrap();
            assert_eq!(report.error_count(), 0, "{report:?}");
        }
        let caller = caller(&ids);
        let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
            .await
            .unwrap();
        let entries = &page.data["time_entries"];
        assert_eq!(entries.len(), 3);
        for entry in entries {
            let expected = match entry.task.name.as_str() {
                "Design" => (8000, 75),
                "Review" => (0, 30),
                "Dev" => (3000, 60),
                name => panic!("unexpected imported task: {name}"),
            };
            let id = ids::path(&pool, ids.org_id, Kind::TimeEntry, &entry.id.to_string())
                .await
                .unwrap();
            let Json(detail) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            assert_eq!(
                (&entry.billable_rate, entry.hours, &entry.cost_rate),
                (&Some(eur(expected.0)), Hours(expected.1), &Some(eur(1200)))
            );
            assert_eq!(
                (&detail.billable_rate, detail.hours, &detail.cost_rate),
                (&entry.billable_rate, entry.hours, &entry.cost_rate)
            );
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn harvest_entry_rates_respect_selected_project_mode(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        let id = insert_entry(&pool, &ids, ids.user_id, "Selected rates").await;
        let caller = caller(&ids);
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 3000 WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE projects SET rate_cents = 3500 WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, 5000)", ids.project_id, ids.task_id)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', 4000)", Uuid::now_v7(), ids.project_id, ids.user_id)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,1500)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1,$2,$3,$4,'person')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
            .execute(&pool).await.unwrap();
        for (mode, expected) in [
            ("person", eur(4000)),
            ("task", eur(5000)),
            ("project", eur(3500)),
        ] {
            sqlx::query!(
                "UPDATE project_settings SET rate_mode = $2 WHERE project_id = $1",
                ids.project_id,
                mode
            )
            .execute(&pool)
            .await
            .unwrap();
            let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
                .await
                .unwrap();
            assert_eq!(
                (
                    &entry.billable_rate,
                    &page.data["time_entries"][0].billable_rate
                ),
                (&Some(expected.clone()), &Some(expected)),
                "selected mode: {mode}"
            );
            assert_eq!(
                (&entry.cost_rate, &page.data["time_entries"][0].cost_rate),
                (&Some(eur(1500)), &Some(eur(1500)))
            );
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn harvest_entry_rates_follow_the_cascade_and_frozen_invoice_line(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
        let id = insert_entry(&pool, &ids, ids.user_id, "Rates").await;
        let caller = caller(&ids);
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 3000 WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE projects SET rate_cents = 3500 WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
            ids.project_id,
            ids.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id, role) VALUES ($1, $2, $3, 'lead')",
            Uuid::now_v7(),
            ids.project_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        for (task, assignment, expected) in [
            (None, None, eur(3500)),
            (None, Some(4000), eur(4000)),
            (Some(5000), Some(4000), eur(5000)),
            (Some(0), Some(4000), eur(0)),
            (None, Some(0), eur(0)),
        ] {
            sqlx::query!(
                "UPDATE project_tasks SET rate_cents = $2 WHERE project_id = $1",
                ids.project_id,
                task
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE assignments SET rate_cents = $2 WHERE project_id = $1",
                ids.project_id,
                assignment
            )
            .execute(&pool)
            .await
            .unwrap();
            let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
                .await
                .unwrap();
            assert_eq!(entry.billable_rate, Some(expected.clone()));
            assert_eq!(page.data["time_entries"][0].billable_rate, Some(expected));
        }
        let invoice_id = Uuid::now_v7();
        sqlx::query!("INSERT INTO invoices (id, org_id, client_id, number, currency, issued_on, due_on, total_cents) VALUES ($1, $2, $3, 'FROZEN', 'EUR', '2026-09-07', '2026-10-07', 1234)", invoice_id, ids.org_id, ids.client_id)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents) VALUES ($1, $2, $3, 'Frozen rate', 60, 1234, 1234)", Uuid::now_v7(), invoice_id, id)
            .execute(&pool).await.unwrap();
        sqlx::query!(
            "UPDATE time_entries SET invoice_id = $2, state = 'invoiced' WHERE id = $1",
            id,
            invoice_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
        let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
            .await
            .unwrap();
        assert_eq!(entry.billable_rate, Some(eur(1234)));
        assert_eq!(page.data["time_entries"][0].billable_rate, Some(eur(1234)));
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn is_billed_follows_the_state_with_or_without_an_invoice(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
        let id = insert_entry(&pool, &ids, ids.user_id, "Billed in Harvest").await;
        let caller = caller(&ids);
        let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
        assert!(!entry.is_billed);
        // An import marks time Harvest billed on an invoice it lacks this way.
        sqlx::query!(
            "UPDATE time_entries SET state = 'invoiced' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
        let mut billed = no_time_entry_filters();
        billed.is_billed = Some(true);
        let Json(page) = time_entries_page(&pool, &caller, billed, &list())
            .await
            .unwrap();
        assert!(entry.is_billed);
        assert!(page.data["time_entries"][0].is_billed);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn harvest_marks_unbilled_non_billable_contexts_consistently(pool: PgPool) {
        for project_billable in [false, true] {
            let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
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
            let caller = caller(&ids);
            let Json(entry) = time_entry_by_id(&pool, &caller, id).await.unwrap();
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
                .await
                .unwrap();
            assert!(!entry.billable);
            assert!(!page.data["time_entries"][0].billable);
            assert_eq!(entry.hours, Hours(60));
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn harvest_effective_minutes_match_sql_for_open_and_frozen_time(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
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
        let caller = caller(&ids);
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
            let Json(page) = time_entries_page(&pool, &caller, no_time_entry_filters(), &list())
                .await
                .unwrap();
            assert_eq!(entry.hours, Hours(8));
            assert_eq!(entry.rounded_hours, Hours(expected.into()));
            assert_eq!(
                page.data["time_entries"][0].rounded_hours,
                entry.rounded_hours
            );
        }
    }

    fn no_user_filters() -> UserFilters {
        UserFilters::default()
    }

    /// The status a refused call answered with. `expect_err` would demand
    /// `Debug` on every DTO just to print a body the test never looks at.
    fn refused<T>(result: ApiResult<T>, leak: &str) -> StatusCode {
        match result {
            Ok(_) => panic!("{leak}"),
            Err(error) => error.status,
        }
    }

    /// A second person in the org, carrying the cost rate this API used to hand
    /// to anyone with a session.
    async fn colleague(pool: &PgPool, ids: &SeedIds) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO users (id, org_id, email, name, cost_rate_cents) \
             VALUES ($1, $2, $3, 'Coworker', 12345)",
            id,
            ids.org_id,
            format!("{id}@test.com"),
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

    /// A refusal after `begin_read` rolls back before it is answered, so the
    /// caller's `FOR SHARE` locks do not outlive the response.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn refusals_release_the_read_locks(pool: PgPool) {
        use crate::testing::organization_lockable;
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        let theirs = insert_entry(&pool, &ids, other, "theirs").await;
        let released = |path: &str| {
            assert!(
                organization_lockable(&pool, ids.org_id),
                "{path} still holds the organization lock"
            );
        };
        let status = refused(
            users_page(&pool, &caller(&ids), no_user_filters(), &list()).await,
            "users",
        );
        assert_eq!(status, StatusCode::FORBIDDEN);
        released("users");
        let filters = TimeEntryFilters {
            user_id: Some(other.to_string()),
            ..no_time_entry_filters()
        };
        let status = refused(
            time_entries_page(&pool, &caller(&ids), filters, &list()).await,
            "time entries",
        );
        assert_eq!(status, StatusCode::FORBIDDEN);
        released("time entries");
        let status = refused(
            time_entry_by_id(&pool, &caller(&ids), theirs).await,
            "time entry",
        );
        assert_eq!(status, StatusCode::NOT_FOUND);
        released("time entry");

        sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        let refusal = task_reads::read(
            &pool,
            &caller(&ids),
            &TaskFilters::default(),
            None,
            &Window::one(),
        )
        .await
        .err()
        .expect("an inactive person read the tasks");
        // The session caller is signed out by the read's transaction.
        assert_eq!(refusal.status, StatusCode::UNAUTHORIZED);
        released("begin_read");
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_member_cannot_list_users(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        colleague(&pool, &ids).await;

        let status = refused(
            users_page(&pool, &caller(&ids), no_user_filters(), &list()).await,
            "a member read the org's pay rates",
        );

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn listed_user_rates_follow_each_rate_grant(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 9000 WHERE id = $1",
            other
        )
        .execute(&pool)
        .await
        .unwrap();

        provision(&pool, &ids, &[Permission::PeopleReadAll], false).await;
        let Json(page) = users_page(&pool, &caller(&ids), no_user_filters(), &list())
            .await
            .expect("people access was refused the user list");
        let users = &page.data["users"];
        assert_eq!(users.len(), 2);
        assert!(
            users
                .iter()
                .all(|u| u.cost_rate.is_none() && u.default_hourly_rate.is_none())
        );

        provision(
            &pool,
            &ids,
            &[Permission::PeopleReadAll, Permission::BillableRateReadAll],
            false,
        )
        .await;
        let Json(page) = users_page(&pool, &caller(&ids), no_user_filters(), &list())
            .await
            .unwrap();
        let users = &page.data["users"];
        assert!(users.iter().all(|u| u.cost_rate.is_none()));
        assert!(
            users
                .iter()
                .any(|u| u.default_hourly_rate == Some(eur(9000)))
        );

        provision(
            &pool,
            &ids,
            &[Permission::PeopleReadAll, Permission::CostRateReadAll],
            false,
        )
        .await;
        let Json(page) = users_page(&pool, &caller(&ids), no_user_filters(), &list())
            .await
            .unwrap();
        let users = &page.data["users"];
        assert!(users.iter().all(|u| u.default_hourly_rate.is_none()));
        assert!(users.iter().any(|u| u.cost_rate == Some(eur(12345))));
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn is_admin_reports_the_stored_administrator_identity(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        colleague(&pool, &ids).await;
        let Json(me) = users_me(caller(&ids), State(pool.clone())).await.unwrap();
        assert!(me.is_admin);
        let Json(page) = users_page(&pool, &caller(&ids), no_user_filters(), &list())
            .await
            .unwrap();
        let admins: Vec<bool> = page.data["users"].iter().map(|u| u.is_admin).collect();
        assert_eq!(admins.iter().filter(|admin| **admin).count(), 1);

        // Every grant without the identity is not an administrator.
        provision(&pool, &ids, Permission::ALL, false).await;
        let Json(me) = users_me(caller(&ids), State(pool.clone())).await.unwrap();
        assert!(!me.is_admin);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn an_administrators_oauth_token_is_not_an_administrator(pool: PgPool) {
        use horae_core::permissions::oauth::{Scope, ScopeSet};
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        let oauth = ops::Caller::oauth(
            ids.org_id,
            ids.user_id,
            Uuid::now_v7(),
            Uuid::now_v7(),
            ScopeSet::of(&Scope::ALL),
        )
        .unlock(Scope::CatalogRead)
        .unwrap();
        let user = AuthUser {
            channel: oauth.channel,
            ..caller(&ids)
        };
        let Json(me) = users_me(user, State(pool.clone())).await.unwrap();
        assert!(!me.is_admin);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_member_does_not_read_their_own_rates(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 9000, cost_rate_cents = 4000 WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let Json(me) = users_me(caller(&ids), State(pool.clone())).await.unwrap();
        assert_eq!((me.default_hourly_rate, me.cost_rate), (None, None));
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_member_sees_only_their_own_time_entries(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let Json(page) = time_entries_page(&pool, &caller(&ids), no_time_entry_filters(), &list())
            .await
            .expect("a member was refused their own entries");

        let entries = &page.data["time_entries"];
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].user.id,
            pagination_tests::api_id(&pool, "user", ids.user_id).await
        );
        assert_eq!(entries[0].notes.as_deref(), Some("mine"));
        // The envelope has to agree with the rows, or a client pages into
        // entries it is never shown.
        assert_eq!(page.total_entries, 1);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_manager_sees_the_whole_org(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let Json(page) = time_entries_page(&pool, &caller(&ids), no_time_entry_filters(), &list())
            .await
            .expect("a manager was refused the org's entries");

        assert_eq!(page.data["time_entries"].len(), 2);
        assert_eq!(page.total_entries, 2);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_member_cannot_filter_to_a_colleague(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, other, "theirs").await;

        let filters = TimeEntryFilters {
            user_id: Some(other.to_string()),
            ..no_time_entry_filters()
        };
        let status = refused(
            time_entries_page(&pool, &caller(&ids), filters, &list()).await,
            "a member read a colleague's entries by filter",
        );

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_member_cannot_fetch_a_colleagues_entry(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        let theirs = insert_entry(&pool, &ids, other, "theirs").await;

        // Outside the member's row set, so it reads as missing rather than
        // forbidden — the same answer an id from another org gets.
        let status = refused(
            time_entry_by_id(&pool, &caller(&ids), theirs).await,
            "a member fetched a colleague's entry by id",
        );
        assert_eq!(status, StatusCode::NOT_FOUND);

        // The grant is read inside each request, not from the session.
        provision(&pool, &ids, &[Permission::TimeReadAll], false).await;
        let Json(entry) = time_entry_by_id(&pool, &caller(&ids), theirs)
            .await
            .expect("an organization-wide time reader was refused an entry");
        assert_eq!(
            entry.id,
            pagination_tests::api_id(&pool, "time_entry", theirs).await
        );
    }

    /// Gives the seeded person and `other` the rates the redaction cases look
    /// for: a billable rate on the project and a cost rate on each person.
    async fn priced(pool: &PgPool, ids: &SeedIds, other: Uuid) {
        sqlx::query!(
            "UPDATE projects SET rate_cents = 3500 WHERE id = $1",
            ids.project_id
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE users SET cost_rate_cents = 1000 WHERE id = $1",
            ids.user_id
        )
        .execute(pool)
        .await
        .unwrap();
        // The colleague already costs 12345; give them a project override too,
        // which hangs off their assignment.
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            ids.project_id,
            other
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,1500)", Uuid::now_v7(), ids.org_id, ids.project_id, other)
            .execute(pool).await.unwrap();
    }

    /// `(notes, billable_rate, cost_rate)` for every listed entry, checked
    /// against the detail endpoint for the same entry.
    async fn rates(pool: &PgPool, ids: &SeedIds) -> Vec<(String, Option<Amount>, Option<Amount>)> {
        let Json(page) = time_entries_page(pool, &caller(ids), no_time_entry_filters(), &list())
            .await
            .unwrap();
        let mut seen = Vec::new();
        for entry in &page.data["time_entries"] {
            let id = ids::path(pool, ids.org_id, Kind::TimeEntry, &entry.id.to_string())
                .await
                .unwrap();
            let Json(detail) = time_entry_by_id(pool, &caller(ids), id).await.unwrap();
            assert_eq!(
                (&detail.billable_rate, &detail.cost_rate),
                (&entry.billable_rate, &entry.cost_rate),
                "detail and listing disagree on {:?}",
                entry.notes
            );
            seen.push((
                entry.notes.clone().unwrap_or_default(),
                entry.billable_rate.clone(),
                entry.cost_rate.clone(),
            ));
        }
        seen.sort_by(|a, b| a.0.cmp(&b.0));
        seen
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn time_access_without_rate_grants_reads_no_rates(pool: PgPool) {
        // People Admin reads every entry but holds no rate grant.
        let ids = seed(&pool, BuiltInProfile::PeopleAdmin).await;
        let other = colleague(&pool, &ids).await;
        priced(&pool, &ids, other).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        assert_eq!(
            rates(&pool, &ids).await,
            vec![("mine".into(), None, None), ("theirs".into(), None, None)]
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn billable_and_cost_rates_need_their_own_grants(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        priced(&pool, &ids, other).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, other, "theirs").await;

        provision(
            &pool,
            &ids,
            &[Permission::TimeReadAll, Permission::BillableRateReadAll],
            false,
        )
        .await;
        assert_eq!(
            rates(&pool, &ids).await,
            vec![
                ("mine".into(), Some(eur(3500)), None),
                ("theirs".into(), Some(eur(3500)), None)
            ]
        );

        provision(
            &pool,
            &ids,
            &[Permission::TimeReadAll, Permission::CostRateReadAll],
            false,
        )
        .await;
        // Cost covers project overrides too; there is no managed cost scope.
        assert_eq!(
            rates(&pool, &ids).await,
            vec![
                ("mine".into(), None, Some(eur(1000))),
                ("theirs".into(), None, Some(eur(1500)))
            ]
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn managed_grants_cover_managed_people_but_rates_only_managed_projects(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::ProjectManager).await;
        let managed = colleague(&pool, &ids).await;
        let stranger = colleague(&pool, &ids).await;
        priced(&pool, &ids, managed).await;
        insert_entry(&pool, &ids, ids.user_id, "mine").await;
        insert_entry(&pool, &ids, managed, "managed").await;
        insert_entry(&pool, &ids, stranger, "stranger").await;
        sqlx::query!(
            "INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(),
            ids.org_id,
            ids.user_id,
            managed
        )
        .execute(&pool)
        .await
        .unwrap();

        // The Project Manager profile reads managed time but no rates. Own
        // time stays visible; the stranger is out of scope.
        let only_managed_people = vec![("managed".into(), None, None), ("mine".into(), None, None)];
        assert_eq!(rates(&pool, &ids).await, only_managed_people);
        let Json(page) = time_entries_page(&pool, &caller(&ids), no_time_entry_filters(), &list())
            .await
            .unwrap();
        assert_eq!(page.total_entries, 2);

        // The managed rate grant still shows nothing on a project the reader
        // does not manage: the rate is the project's, not the person's.
        provision(
            &pool,
            &ids,
            &[
                Permission::TimeReadManaged,
                Permission::BillableRateReadManaged,
            ],
            false,
        )
        .await;
        assert_eq!(rates(&pool, &ids).await, only_managed_people);

        // The Accounting profile reads every rate without managing anything.
        provision(
            &pool,
            &ids,
            BuiltInProfile::Accounting.direct_permissions(),
            false,
        )
        .await;
        let billable = |rows: Vec<(String, Option<Amount>, Option<Amount>)>| {
            rows.into_iter()
                .map(|(notes, billable, _)| (notes, billable))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            billable(rates(&pool, &ids).await),
            vec![
                ("managed".into(), Some(eur(3500))),
                ("mine".into(), Some(eur(3500))),
                ("stranger".into(), Some(eur(3500)))
            ]
        );

        // Managing the project brings in everyone's time on it, with rates.
        provision(
            &pool,
            &ids,
            &[
                Permission::TimeReadManaged,
                Permission::BillableRateReadManaged,
            ],
            false,
        )
        .await;
        sqlx::query!(
            "INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(),
            ids.org_id,
            ids.user_id,
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            rates(&pool, &ids).await,
            vec![
                ("managed".into(), Some(eur(3500)), None),
                ("mine".into(), Some(eur(3500)), None),
                ("stranger".into(), Some(eur(3500)), None)
            ]
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_managed_time_reader_may_filter_to_anyone_but_sees_only_their_scope(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let stranger = colleague(&pool, &ids).await;
        insert_entry(&pool, &ids, stranger, "stranger").await;
        provision(&pool, &ids, &[Permission::TimeReadManaged], false).await;

        let filters = TimeEntryFilters {
            user_id: Some(stranger.to_string()),
            ..no_time_entry_filters()
        };
        let Json(page) = time_entries_page(&pool, &caller(&ids), filters, &list())
            .await
            .unwrap();
        assert_eq!(page.total_entries, 0);
        assert!(page.data["time_entries"].is_empty());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn budget_kind_needs_access_to_the_project(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let other = colleague(&pool, &ids).await;
        let id = insert_entry(&pool, &ids, other, "theirs").await;
        sqlx::query!(
            "UPDATE projects SET budget_kind = 'hours', budget_minutes = 600 WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();

        provision(&pool, &ids, &[Permission::TimeReadAll], false).await;
        let Json(entry) = time_entry_by_id(&pool, &caller(&ids), id).await.unwrap();
        assert_eq!(entry.budgeted, None);

        provision(
            &pool,
            &ids,
            &[Permission::TimeReadAll, Permission::ProjectReadAll],
            false,
        )
        .await;
        let Json(entry) = time_entry_by_id(&pool, &caller(&ids), id).await.unwrap();
        assert_eq!(entry.budgeted, Some(true));
    }
}
