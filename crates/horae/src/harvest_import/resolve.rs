//! Resolve source rows to existing Horae rows, provenance-first with a composite
//! natural-key fallback, and create the missing ones (FR-004, FR-012, FR-017).
//!
//! Resolution is org-scoped and backed by an in-run [`RunCache`] so a client (or
//! project or task) seen on thousands of rows is resolved or created exactly once
//! (research.md §9). The default policy for an already-existing record is to leave
//! it unchanged and count it as `Skipped` (spec Assumptions: either skip or update
//! is valid so long as it stays idempotent); this keeps a second run at zero
//! creations and is edit-robust because provenance matches by Harvest id.

use std::collections::HashMap;

use horae_core::harvest_import::convert;
use horae_core::harvest_import::keys;
use horae_core::harvest_import::types::{EntityType, RowOutcome, SourceRow};
use uuid::Uuid;

/// A per-record failure that errors the row and continues the run (FR-018).
#[derive(Debug)]
pub struct RowFailure {
    pub reason: String,
}

impl RowFailure {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl From<sqlx::Error> for RowFailure {
    fn from(e: sqlx::Error) -> Self {
        RowFailure::new(format!("database error: {e}"))
    }
}

impl From<convert::ConvertError> for RowFailure {
    fn from(e: convert::ConvertError) -> Self {
        RowFailure::new(e.to_string())
    }
}

/// Ids of parents resolved so far in this run, keyed per entity type so each
/// distinct parent is touched once. Only merged after a row's savepoint commits,
/// so a rolled-back creation never poisons the cache.
#[derive(Default)]
pub struct RunCache {
    clients: HashMap<String, Uuid>,
    projects: HashMap<String, Uuid>,
    tasks: HashMap<String, Uuid>,
}

impl RunCache {
    fn get(&self, entity: EntityType, key: &str) -> Option<Uuid> {
        self.map(entity).get(key).copied()
    }

    fn map(&self, entity: EntityType) -> &HashMap<String, Uuid> {
        match entity {
            EntityType::Client => &self.clients,
            EntityType::Project => &self.projects,
            EntityType::Task => &self.tasks,
            EntityType::TimeEntry => &self.clients, // unused; time entries are not cached
        }
    }

    fn map_mut(&mut self, entity: EntityType) -> &mut HashMap<String, Uuid> {
        match entity {
            EntityType::Client => &mut self.clients,
            EntityType::Project => &mut self.projects,
            EntityType::Task => &mut self.tasks,
            EntityType::TimeEntry => &mut self.clients,
        }
    }

    /// Merge the entries a committed row resolved/created into the run cache.
    pub fn merge(&mut self, pending: Vec<(EntityType, String, Uuid)>) {
        for (entity, key, id) in pending {
            self.map_mut(entity).insert(key, id);
        }
    }
}

/// The cache key for a parent in this run: its Harvest id when present, else its
/// composite natural key. Stable across every row that references the parent.
fn client_cache_key(row: &SourceRow) -> String {
    match row.harvest_client_id {
        Some(id) => format!("hid:{id}"),
        None => format!("nk:{}", keys::client_key(&row.client_name)),
    }
}

fn project_cache_key(row: &SourceRow) -> String {
    match row.harvest_project_id {
        Some(id) => format!("hid:{id}"),
        None => format!(
            "nk:{}",
            keys::project_key(
                &row.client_name,
                &row.project_name,
                row.project_code.as_deref()
            )
        ),
    }
}

fn task_cache_key(row: &SourceRow) -> String {
    match row.harvest_task_id {
        Some(id) => format!("hid:{id}"),
        None => format!("nk:{}", keys::task_key(&row.task_name)),
    }
}

/// What a resolution did to the entity, and the cache entry to promote on commit.
pub struct Resolved {
    pub id: Uuid,
    /// `None` when the parent was already in the run cache (not re-counted);
    /// otherwise the outcome to fold into the summary for that entity.
    pub outcome: Option<RowOutcome>,
    /// Entry to add to the run cache once the row's savepoint commits.
    pub cache_entry: Option<(EntityType, String, Uuid)>,
}

impl Resolved {
    fn cached(id: Uuid) -> Self {
        Self {
            id,
            outcome: None,
            cache_entry: None,
        }
    }
}

/// The organization defaults an import needs (currency fallback per FR-013).
#[derive(Clone, Copy)]
pub struct OrgDefaults<'a> {
    pub org_id: Uuid,
    pub default_currency: &'a str,
}

/// Resolve or create the client for a row.
pub async fn resolve_client(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
) -> Result<Resolved, RowFailure> {
    let ck = client_cache_key(row);
    if let Some(id) = cache.get(EntityType::Client, &ck) {
        return Ok(Resolved::cached(id));
    }

    // Provenance first (API source).
    if let Some(hid) = row.harvest_client_id
        && let Some(id) = super::provenance::lookup(&mut *conn, org.org_id, EntityType::Client, hid)
            .await?
    {
        return Ok(existing(EntityType::Client, ck, id));
    }

    // Natural-key fallback: normalized name within the org.
    let nk = keys::client_key(&row.client_name);
    if let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM clients WHERE org_id = $1 AND LOWER(TRIM(name)) = $2",
        org.org_id,
        nk,
    )
    .fetch_optional(&mut *conn)
    .await?
    {
        if let Some(hid) = row.harvest_client_id {
            super::provenance::upsert(&mut *conn, org.org_id, EntityType::Client, hid, id, None)
                .await?;
        }
        return Ok(existing(EntityType::Client, ck, id));
    }

    // Create.
    let name = row.client_name.trim();
    if name.is_empty() {
        return Err(RowFailure::new("client name is empty"));
    }
    let currency = currency_or(row.currency.as_deref(), org.default_currency);
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, address, active)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        org.org_id,
        name,
        currency,
        row.client_address.as_deref(),
        row.client_active,
    )
    .execute(&mut *conn)
    .await?;
    if let Some(hid) = row.harvest_client_id {
        super::provenance::upsert(
            &mut *conn,
            org.org_id,
            EntityType::Client,
            hid,
            id,
            row.harvest_updated_at,
        )
        .await?;
    }
    Ok(created(EntityType::Client, ck, id))
}

/// Resolve or create the project for a row (client already resolved).
pub async fn resolve_project(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    client_id: Uuid,
    row: &SourceRow,
) -> Result<Resolved, RowFailure> {
    let ck = project_cache_key(row);
    if let Some(id) = cache.get(EntityType::Project, &ck) {
        return Ok(Resolved::cached(id));
    }

    if let Some(hid) = row.harvest_project_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org.org_id, EntityType::Project, hid).await?
    {
        return Ok(existing(EntityType::Project, ck, id));
    }

    // Natural key: code when present, else (client, name).
    let existing_id = match row.project_code.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(code) => sqlx::query_scalar!(
            "SELECT id FROM projects WHERE org_id = $1 AND LOWER(TRIM(code)) = $2",
            org.org_id,
            code.to_lowercase(),
        )
        .fetch_optional(&mut *conn)
        .await?,
        None => sqlx::query_scalar!(
            "SELECT id FROM projects
             WHERE org_id = $1 AND client_id = $2 AND LOWER(TRIM(name)) = $3",
            org.org_id,
            client_id,
            keys::normalize(&row.project_name),
        )
        .fetch_optional(&mut *conn)
        .await?,
    };
    if let Some(id) = existing_id {
        if let Some(hid) = row.harvest_project_id {
            super::provenance::upsert(&mut *conn, org.org_id, EntityType::Project, hid, id, None)
                .await?;
        }
        return Ok(existing(EntityType::Project, ck, id));
    }

    let name = row.project_name.trim();
    if name.is_empty() {
        return Err(RowFailure::new("project name is empty"));
    }
    // Project currency defaults from the client (FR-013).
    let client_currency: String =
        sqlx::query_scalar!("SELECT currency FROM clients WHERE id = $1", client_id)
            .fetch_one(&mut *conn)
            .await?;
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, code, name, currency, starts_on, ends_on, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        id,
        org.org_id,
        client_id,
        row.project_code.as_deref().map(str::trim).filter(|c| !c.is_empty()),
        name,
        client_currency,
        row.project_starts_on as Option<chrono::NaiveDate>,
        row.project_ends_on as Option<chrono::NaiveDate>,
        row.project_active,
    )
    .execute(&mut *conn)
    .await?;
    if let Some(hid) = row.harvest_project_id {
        super::provenance::upsert(
            &mut *conn,
            org.org_id,
            EntityType::Project,
            hid,
            id,
            row.harvest_updated_at,
        )
        .await?;
    }
    Ok(created(EntityType::Project, ck, id))
}

/// Resolve or create the org-level task for a row.
pub async fn resolve_task(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
) -> Result<Resolved, RowFailure> {
    let ck = task_cache_key(row);
    if let Some(id) = cache.get(EntityType::Task, &ck) {
        return Ok(Resolved::cached(id));
    }

    if let Some(hid) = row.harvest_task_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org.org_id, EntityType::Task, hid).await?
    {
        return Ok(existing(EntityType::Task, ck, id));
    }

    let nk = keys::task_key(&row.task_name);
    if let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM tasks WHERE org_id = $1 AND LOWER(TRIM(name)) = $2",
        org.org_id,
        nk,
    )
    .fetch_optional(&mut *conn)
    .await?
    {
        if let Some(hid) = row.harvest_task_id {
            super::provenance::upsert(&mut *conn, org.org_id, EntityType::Task, hid, id, None)
                .await?;
        }
        return Ok(existing(EntityType::Task, ck, id));
    }

    let name = row.task_name.trim();
    if name.is_empty() {
        return Err(RowFailure::new("task name is empty"));
    }
    let default_rate_cents = match row.billable_rate.as_deref() {
        Some(r) => Some(convert::money_to_cents(r)?),
        None => None,
    };
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, active)
         VALUES ($1, $2, $3, $4, $5, true)",
        id,
        org.org_id,
        name,
        row.task_billable_default,
        default_rate_cents,
    )
    .execute(&mut *conn)
    .await?;
    if let Some(hid) = row.harvest_task_id {
        super::provenance::upsert(
            &mut *conn,
            org.org_id,
            EntityType::Task,
            hid,
            id,
            row.harvest_updated_at,
        )
        .await?;
    }
    Ok(created(EntityType::Task, ck, id))
}

/// Ensure the task is enabled on the project (FR-009). Not counted in the summary
/// — it is a link, not one of the four entity levels.
pub async fn ensure_project_task(
    conn: &mut sqlx::PgConnection,
    project_id: Uuid,
    task_id: Uuid,
    row: &SourceRow,
) -> Result<(), RowFailure> {
    let rate_cents = match row.billable_rate.as_deref() {
        Some(r) => Some(convert::money_to_cents(r)?),
        None => None,
    };
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (project_id, task_id) DO NOTHING",
        project_id,
        task_id,
        row.billable,
        rate_cents,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Resolve the Horae user for a row by email (FR-010). Never provisions; an
/// unmatched user errors the row.
pub async fn resolve_user(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    row: &SourceRow,
) -> Result<Uuid, RowFailure> {
    let email = row
        .user_email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .ok_or_else(|| RowFailure::new("time entry has no user email to match"))?;

    let id = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND LOWER(TRIM(email)) = $2",
        org_id,
        email.to_lowercase(),
    )
    .fetch_optional(&mut *conn)
    .await?;

    id.ok_or_else(|| RowFailure::new(format!("no Horae user matches email {email:?}")))
}

fn existing(entity: EntityType, ck: String, id: Uuid) -> Resolved {
    Resolved {
        id,
        outcome: Some(RowOutcome::Skipped),
        cache_entry: Some((entity, ck, id)),
    }
}

fn created(entity: EntityType, ck: String, id: Uuid) -> Resolved {
    Resolved {
        id,
        outcome: Some(RowOutcome::Created),
        cache_entry: Some((entity, ck, id)),
    }
}

/// The row's currency when it is a plausible 3-letter code, else the org default.
fn currency_or(row_currency: Option<&str>, default: &str) -> String {
    match row_currency.map(str::trim).filter(|c| c.len() == 3) {
        Some(c) => c.to_uppercase(),
        None => default.to_string(),
    }
}
