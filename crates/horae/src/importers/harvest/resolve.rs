//! Resolve source rows to existing Horae rows, provenance-first with a composite
//! natural-key fallback, and create the missing ones (FR-004, FR-012, FR-017).
//!
//! Resolution is org-scoped and backed by an in-run [`RunCache`] so a client (or
//! project or task) seen on thousands of rows is resolved or created exactly once
//! (research.md §9). Natural-key comparisons use the shared `harvest_norm` SQL
//! function, which is byte-identical to `keys::normalize`, so a re-import always
//! matches an existing row instead of duplicating it. The default policy for an
//! already-existing record is to leave it unchanged and count it `Skipped` (spec
//! Assumptions: either skip or update is valid so long as it stays idempotent);
//! this keeps a second run at zero creations and is edit-robust because
//! provenance matches by Harvest id.

pub mod fields;
pub(super) mod preview;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use fields::{ClientFields, ProjectFields, TaskFields};
use horae_core::importers::harvest::convert;
use horae_core::importers::harvest::keys;
use horae_core::importers::harvest::types::{
    EntityType, EntryErrorKind, PersonError, PersonErrorKind, PersonOutcome, RowOutcome,
    SourceKind, SourceRow,
};
use uuid::Uuid;

/// A per-record failure that errors the row and continues the run (FR-018).
#[derive(Debug)]
pub struct RowFailure {
    pub reason: String,
    pub person: Option<PersonError>,
    pub entry_kind: Option<EntryErrorKind>,
}

impl RowFailure {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            person: None,
            entry_kind: None,
        }
    }

    /// A time entry failure the report groups by `kind`.
    pub fn entry(kind: EntryErrorKind, reason: impl Into<String>) -> Self {
        Self {
            entry_kind: Some(kind),
            ..Self::new(reason)
        }
    }

    /// A failure caused by the record's person, reported with who it was.
    pub fn person(
        reason: impl Into<String>,
        kind: PersonErrorKind,
        email: Option<&str>,
        name: Option<&str>,
    ) -> Self {
        Self {
            reason: reason.into(),
            person: Some(PersonError {
                kind,
                email: email.map(str::to_owned),
                name: name.map(str::to_owned).filter(|n| !n.is_empty()),
            }),
            entry_kind: None,
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

/// The three cacheable parent levels. A time entry is never cached (each row is a
/// distinct entry), so it cannot be represented here — making a stray cache write
/// against it impossible by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentKind {
    Client,
    Project,
    Task,
}

/// Email and full-name matches occupy separate namespaces within one org run.
#[derive(Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum UserKey {
    Email(String),
    FullName(String),
}

/// What a committed row contributes to the [`RunCache`]. Collected while the
/// row's savepoint is open and merged only after it commits, so a rolled-back
/// row never poisons the cache.
#[derive(Default)]
pub struct PendingCache {
    /// Parents this row resolved or created.
    pub parents: Vec<(ParentKind, String, Uuid)>,
    pub user: Option<(UserKey, Uuid)>,
    /// The account this row counted as a person in the report.
    pub person: Option<Uuid>,
    pub project_task: Option<(Uuid, Uuid)>,
    /// The natural-key slot an id-less time entry consumed (see
    /// [`RunCache::entry_slot_offset`]); bumped on commit so the next identical
    /// row in the run maps to the next stored entry instead of the same one.
    pub entry_slot: Option<String>,
}

/// Successful resolutions for one organization/run, plus consumed id-less
/// occurrences. Only merged after a row's savepoint commits. First successful
/// identity matches remain stable for the run; the next run resolves them anew.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct RunCache {
    clients: HashMap<String, Uuid>,
    projects: HashMap<String, Uuid>,
    tasks: HashMap<String, Uuid>,
    #[serde(
        serialize_with = "serialize_users",
        deserialize_with = "deserialize_users"
    )]
    users: HashMap<UserKey, Uuid>,
    /// Accounts already counted as people. A CSV person can be reached by
    /// name and by email, and must still count once.
    #[serde(default)]
    people: HashSet<Uuid>,
    project_tasks: HashSet<(Uuid, Uuid)>,
    entry_slots: keys::OccurrenceCounter,
    failed_parents: HashSet<(EntityType, i64)>,
    /// Harvest `updated_at` of entries a durable preview simulated on earlier
    /// pages and rolled back, set for the page being applied only.
    #[serde(skip)]
    simulated_entries: HashMap<i64, DateTime<Utc>>,
}

// JSON object keys cannot represent the email/full-name enum namespaces.
fn serialize_users<S: serde::Serializer>(
    users: &HashMap<UserKey, Uuid>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&users.iter().collect::<Vec<_>>(), serializer)
}

fn deserialize_users<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<HashMap<UserKey, Uuid>, D::Error> {
    let pairs: Vec<(UserKey, Uuid)> = serde::Deserialize::deserialize(deserializer)?;
    Ok(pairs.into_iter().collect())
}

impl RunCache {
    pub fn mark_failed(&mut self, entity: EntityType, id: i64) {
        self.failed_parents.insert((entity, id));
    }

    pub fn parent_failed(&self, entity: EntityType, id: i64) -> bool {
        self.failed_parents.contains(&(entity, id))
    }

    pub fn person_counted(&self, id: Uuid) -> bool {
        self.people.contains(&id)
    }

    pub fn set_simulated_entries(&mut self, entries: HashMap<i64, DateTime<Utc>>) {
        self.simulated_entries = entries;
    }

    pub fn simulated_entry(&self, harvest_id: i64) -> Option<DateTime<Utc>> {
        self.simulated_entries.get(&harvest_id).copied()
    }

    fn get(&self, kind: ParentKind, key: &str) -> Option<Uuid> {
        self.map(kind).get(key).copied()
    }

    fn map(&self, kind: ParentKind) -> &HashMap<String, Uuid> {
        match kind {
            ParentKind::Client => &self.clients,
            ParentKind::Project => &self.projects,
            ParentKind::Task => &self.tasks,
        }
    }

    fn map_mut(&mut self, kind: ParentKind) -> &mut HashMap<String, Uuid> {
        match kind {
            ParentKind::Client => &mut self.clients,
            ParentKind::Project => &mut self.projects,
            ParentKind::Task => &mut self.tasks,
        }
    }

    /// Merge the entries a committed row resolved/created into the run cache.
    pub fn merge(&mut self, pending: PendingCache) {
        for (kind, key, id) in pending.parents {
            self.map_mut(kind).insert(key, id);
        }
        if let Some((key, id)) = pending.user {
            self.users.insert(key, id);
        }
        if let Some(id) = pending.person {
            self.people.insert(id);
        }
        if let Some(pair) = pending.project_task {
            self.project_tasks.insert(pair);
        }
        if let Some(key) = pending.entry_slot {
            self.entry_slots.bump(key);
        }
    }

    /// How many id-less time entries with this natural key the run has already
    /// consumed — the zero-based `OFFSET` into the stored entries the next
    /// identical row should match.
    pub fn entry_slot_offset(&self, key: &str) -> u64 {
        self.entry_slots.seen(key)
    }
}

/// The cache key for a parent in this run: its Harvest id when present, else its
/// composite natural key. Stable across every row that references the parent.
fn client_cache_key(row: &ClientFields<'_>) -> String {
    match row.harvest_client_id {
        Some(id) => format!("hid:{id}"),
        None => format!("nk:{}", keys::client_key(row.client_name)),
    }
}

fn project_cache_key(row: &ProjectFields<'_>) -> String {
    match row.harvest_project_id {
        Some(id) => format!("hid:{id}"),
        None => format!(
            "nk:{}",
            keys::project_key(row.client_name, row.project_name, row.project_code)
        ),
    }
}

fn task_cache_key(row: &TaskFields<'_>) -> String {
    match row.harvest_task_id {
        Some(id) => format!("hid:{id}"),
        None => format!("nk:{}", keys::task_key(row.task_name)),
    }
}

/// What a resolution did to the entity, and the cache entry to promote on commit.
pub struct Resolved {
    pub id: Uuid,
    /// `None` when the parent was already in the run cache (not re-counted);
    /// otherwise the outcome to fold into the summary for that entity.
    pub outcome: Option<RowOutcome>,
    /// Entry to add to the run cache once the row's savepoint commits.
    pub cache_entry: Option<(ParentKind, String, Uuid)>,
}

impl Resolved {
    fn cached(id: Uuid) -> Self {
        Self {
            id,
            outcome: None,
            cache_entry: None,
        }
    }

    fn existing(kind: ParentKind, key: String, id: Uuid) -> Self {
        Self {
            id,
            outcome: Some(RowOutcome::Skipped),
            cache_entry: Some((kind, key, id)),
        }
    }

    fn created(kind: ParentKind, key: String, id: Uuid) -> Self {
        Self {
            id,
            outcome: Some(RowOutcome::Created),
            cache_entry: Some((kind, key, id)),
        }
    }
}

/// The organization defaults an import needs (currency fallback per FR-013).
#[derive(Clone, Copy)]
pub struct OrgDefaults<'a> {
    pub org_id: Uuid,
    pub default_currency: &'a str,
}

// ── Shared helpers (FR-026 provenance, money conversion) ──────────────────────

/// Write (or refresh) the provenance mapping for an API-sourced record; a no-op
/// for the CSV source, which carries no Harvest ids.
async fn upsert_provenance_if_api(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    entity: EntityType,
    harvest_id: Option<i64>,
    horae_id: Uuid,
    updated_at: Option<DateTime<Utc>>,
) -> Result<(), RowFailure> {
    if let Some(hid) = harvest_id {
        super::provenance::upsert(conn, org_id, entity, hid, horae_id, updated_at).await?;
    }
    Ok(())
}

/// Convert an optional decimal rate string to cents.
fn rate_cents(rate: Option<&str>) -> Result<Option<i64>, RowFailure> {
    match rate {
        Some(r) => Ok(Some(convert::money_to_cents(r)?)),
        None => Ok(None),
    }
}

// ── Client / Project / Task resolution ────────────────────────────────────────

/// Resolve or create the client for a row.
pub async fn resolve_client(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &ClientFields<'_>,
) -> Result<Resolved, RowFailure> {
    let ck = client_cache_key(row);
    if let Some(id) = cache.get(ParentKind::Client, &ck) {
        return Ok(Resolved::cached(id));
    }

    // Provenance first (API source).
    if let Some(hid) = row.harvest_client_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org.org_id, EntityType::Client, hid).await?
    {
        return Ok(Resolved::existing(ParentKind::Client, ck, id));
    }

    // Natural-key fallback: normalized name within the org.
    let nk = keys::client_key(row.client_name);
    if let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM clients WHERE org_id = $1 AND harvest_norm(name) = $2",
        org.org_id,
        nk,
    )
    .fetch_optional(&mut *conn)
    .await?
    {
        upsert_provenance_if_api(
            conn,
            org.org_id,
            EntityType::Client,
            row.harvest_client_id,
            id,
            None,
        )
        .await?;
        return Ok(Resolved::existing(ParentKind::Client, ck, id));
    }

    // Create.
    let name = keys::trim_ws(row.client_name);
    if name.is_empty() {
        return Err(RowFailure::new("client name is empty"));
    }
    let currency = currency_or(row.currency, org.default_currency);
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, address, active)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        org.org_id,
        name,
        currency,
        row.client_address,
        row.client_active,
    )
    .execute(&mut *conn)
    .await?;
    upsert_provenance_if_api(
        conn,
        org.org_id,
        EntityType::Client,
        row.harvest_client_id,
        id,
        row.harvest_updated_at,
    )
    .await?;
    Ok(Resolved::created(ParentKind::Client, ck, id))
}

/// Resolve or create the project for a row (client already resolved).
pub async fn resolve_project(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    client_id: Uuid,
    row: &ProjectFields<'_>,
) -> Result<Resolved, RowFailure> {
    let ck = project_cache_key(row);
    if let Some(id) = cache.get(ParentKind::Project, &ck) {
        return Ok(Resolved::cached(id));
    }

    if let Some(hid) = row.harvest_project_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org.org_id, EntityType::Project, hid).await?
    {
        return Ok(Resolved::existing(ParentKind::Project, ck, id));
    }

    // Natural key: code when present, else (client, name).
    let code = row
        .project_code
        .map(keys::trim_ws)
        .filter(|c| !c.is_empty());
    let existing_id = match code {
        Some(code) => {
            sqlx::query_scalar!(
                "SELECT id FROM projects WHERE org_id = $1 AND harvest_norm(code) = $2",
                org.org_id,
                keys::normalize(code),
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        None => {
            sqlx::query_scalar!(
                "SELECT id FROM projects
             WHERE org_id = $1 AND client_id = $2 AND harvest_norm(name) = $3",
                org.org_id,
                client_id,
                keys::normalize(row.project_name),
            )
            .fetch_optional(&mut *conn)
            .await?
        }
    };
    if let Some(id) = existing_id {
        upsert_provenance_if_api(
            conn,
            org.org_id,
            EntityType::Project,
            row.harvest_project_id,
            id,
            None,
        )
        .await?;
        return Ok(Resolved::existing(ParentKind::Project, ck, id));
    }

    let name = keys::trim_ws(row.project_name);
    if name.is_empty() {
        return Err(RowFailure::new("project name is empty"));
    }
    // Project currency defaults from the client (FR-013). The share lock orders
    // this creation against a client archive, which locks the same row.
    let client = sqlx::query!(
        "SELECT currency, active FROM clients WHERE id = $1 FOR SHARE",
        client_id
    )
    .fetch_one(&mut *conn)
    .await?;
    if row.project_active && !client.active {
        // Archived clients keep only archived projects; the record is reported,
        // not created archived or used to restore the client.
        return Err(RowFailure::new(
            "client is archived; restore it before importing an active project",
        ));
    }
    let client_currency = client.currency;
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, code, name, currency, starts_on, ends_on, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        id,
        org.org_id,
        client_id,
        code,
        name,
        client_currency,
        row.project_starts_on as Option<chrono::NaiveDate>,
        row.project_ends_on as Option<chrono::NaiveDate>,
        row.project_active,
    )
    .execute(&mut *conn)
    .await?;
    upsert_provenance_if_api(
        conn,
        org.org_id,
        EntityType::Project,
        row.harvest_project_id,
        id,
        row.harvest_updated_at,
    )
    .await?;
    Ok(Resolved::created(ParentKind::Project, ck, id))
}

/// Resolve or create the org-level task for a row.
pub async fn resolve_task(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &TaskFields<'_>,
) -> Result<Resolved, RowFailure> {
    let ck = task_cache_key(row);
    if let Some(id) = cache.get(ParentKind::Task, &ck) {
        return Ok(Resolved::cached(id));
    }

    if let Some(hid) = row.harvest_task_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org.org_id, EntityType::Task, hid).await?
    {
        return Ok(Resolved::existing(ParentKind::Task, ck, id));
    }

    let nk = keys::task_key(row.task_name);
    if let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM tasks WHERE org_id = $1 AND harvest_norm(name) = $2",
        org.org_id,
        nk,
    )
    .fetch_optional(&mut *conn)
    .await?
    {
        upsert_provenance_if_api(
            conn,
            org.org_id,
            EntityType::Task,
            row.harvest_task_id,
            id,
            None,
        )
        .await?;
        return Ok(Resolved::existing(ParentKind::Task, ck, id));
    }

    let name = keys::trim_ws(row.task_name);
    if name.is_empty() {
        return Err(RowFailure::new("task name is empty"));
    }
    let default_rate_cents = rate_cents(row.billable_rate)?;
    let rate_currency = row
        .rate_currency
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| {
            default_rate_cents.is_some()
                && value.len() == 3
                && value.bytes().all(|byte| byte.is_ascii_uppercase())
        });
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, active, default_rate_currency, is_default)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        id,
        org.org_id,
        name,
        row.task_billable_default,
        default_rate_cents,
        row.task_active,
        rate_currency,
        row.task_is_default,
    )
    .execute(&mut *conn)
    .await?;
    upsert_provenance_if_api(
        conn,
        org.org_id,
        EntityType::Task,
        row.harvest_task_id,
        id,
        row.harvest_updated_at,
    )
    .await?;
    Ok(Resolved::created(ParentKind::Task, ck, id))
}

/// Ensure a retained project link exists (FR-009), without restoring archived
/// links. Not counted in the summary — it is not one of the four entity levels.
pub async fn ensure_project_task(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    project_id: Uuid,
    task_id: Uuid,
    row: &SourceRow,
) -> Result<(), RowFailure> {
    // A cached link must not hide a malformed rate on a later source row.
    let rate = rate_cents(row.billable_rate.as_deref())?;
    if cache.project_tasks.contains(&(project_id, task_id)) {
        return Ok(());
    }
    // Configured overrides are denominated in the project currency. Existing
    // links ignore incoming rates, and legacy projects keep their import rules.
    if rate.is_some()
        && let Some(project) = sqlx::query!(
            "SELECT p.currency, o.default_currency FROM projects p
             JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
             JOIN organizations o ON o.id = p.org_id
             WHERE p.id = $1 AND NOT EXISTS (
               SELECT 1 FROM project_tasks pt WHERE pt.project_id = p.id AND pt.task_id = $2
             ) FOR SHARE OF p",
            project_id,
            task_id,
        )
        .fetch_optional(&mut *conn)
        .await?
        && !row
            .currency
            .as_deref()
            .map(str::trim)
            .filter(|currency| !currency.is_empty())
            .unwrap_or(&project.default_currency)
            .eq_ignore_ascii_case(&project.currency)
    {
        return Err(RowFailure::new(
            "imported task rate currency differs from the configured project currency",
        ));
    }
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents, active)
         VALUES ($1, $2, $3, $4, (
           SELECT o.permission_policy_version=0 OR (o.permission_policy_version=1 AND t.active)
           FROM tasks t JOIN projects p ON p.org_id=t.org_id
           JOIN organizations o ON o.id=p.org_id WHERE p.id=$1 AND t.id=$2
         ))
         ON CONFLICT (project_id, task_id) DO NOTHING",
        project_id,
        task_id,
        row.billable,
        rate,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// A Harvest person as the people phase sees it.
pub struct PersonFields<'a> {
    pub harvest_id: Option<i64>,
    /// Trimmed and non-empty.
    pub email: &'a str,
    pub name: &'a str,
}

/// Resolve a Harvest person to an account: its Harvest identity link first,
/// then a unique normalized email (active or not), else a new deactivated
/// member. Never changes an account it matched.
pub async fn resolve_person(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    person: &PersonFields<'_>,
) -> Result<(UserKey, Uuid, PersonOutcome), RowFailure> {
    let email = keys::normalize(person.email);
    let key = UserKey::Email(email.clone());
    if let Some(hid) = person.harvest_id
        && let Some(id) =
            super::provenance::lookup(&mut *conn, org_id, EntityType::Person, hid).await?
        && sqlx::query_scalar!(
            "SELECT id FROM users WHERE id = $1 AND org_id = $2",
            id,
            org_id
        )
        .fetch_optional(&mut *conn)
        .await?
        .is_some()
    {
        return Ok((key, id, PersonOutcome::Matched));
    }
    let matches = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND harvest_norm(email) = $2 LIMIT 2",
        org_id,
        email,
    )
    .fetch_all(&mut *conn)
    .await?;
    let (id, outcome) = match matches.as_slice() {
        [id] => (*id, PersonOutcome::Matched),
        [] => create_person(conn, org_id, person.email, person.name).await?,
        _ => return Err(ambiguous_person(person.email, person.name)),
    };
    if let Some(hid) = person.harvest_id {
        // An email match must not take over an account that already holds the
        // history of another Harvest person, e.g. after an email moved between
        // people in Harvest.
        if outcome == PersonOutcome::Matched
            && let Some(other) = sqlx::query_scalar!(
                "SELECT harvest_id FROM harvest_import_map
                 WHERE org_id = $1 AND harvest_entity_type = 'person'
                   AND horae_id = $2 AND harvest_id <> $3
                 LIMIT 1",
                org_id,
                id,
                hid,
            )
            .fetch_optional(&mut *conn)
            .await?
        {
            return Err(RowFailure::person(
                format!(
                    "the Horae account with email {:?} already belongs to Harvest person {other}",
                    person.email
                ),
                PersonErrorKind::AmbiguousPerson,
                Some(person.email),
                Some(person.name),
            ));
        }
        super::provenance::upsert(&mut *conn, org_id, EntityType::Person, hid, id, None).await?;
    }
    Ok((key, id, outcome))
}

fn ambiguous_person(email: &str, name: &str) -> RowFailure {
    RowFailure::person(
        format!("ambiguous person email {email:?}: multiple Horae users match"),
        PersonErrorKind::AmbiguousPerson,
        Some(email),
        Some(name),
    )
}

/// Create the account that owns a person's imported history. It is a
/// deactivated member with no rates and no sign-in identity, so an import never
/// grants access; an administrator reactivates the people who still work there.
///
/// Account emails are unique only case-sensitively, so creation takes the
/// email's lock and checks again, ignoring case, before inserting. An account
/// another request created meanwhile is matched instead; one elsewhere is
/// reported. The "user created" event is queued with the account, so it is
/// delivered exactly when the account commits.
async fn create_person(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    email: &str,
    name: &str,
) -> Result<(Uuid, PersonOutcome), RowFailure> {
    // The import needs to know whose account holds the email, not only that
    // one does, so it reads them under the shared lock itself, also ignoring
    // surrounding whitespace.
    crate::db::lock_new_user_email(&mut *conn, email).await?;
    let existing = sqlx::query!(
        "SELECT id, org_id FROM users WHERE lower(btrim(email)) = lower(btrim($1)) LIMIT 2",
        email,
    )
    .fetch_all(&mut *conn)
    .await?;
    match existing.as_slice() {
        [] => {}
        [user] if user.org_id == org_id => return Ok((user.id, PersonOutcome::Matched)),
        [_, _] if existing.iter().all(|user| user.org_id == org_id) => {
            return Err(ambiguous_person(email, name));
        }
        _ => return Err(email_in_use(email, name)),
    }
    let name = keys::trim_ws(name);
    let name = if name.is_empty() { email } else { name };
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role, active)
         VALUES ($1, $2, $3, $4, 'member', false)",
        id,
        org_id,
        email,
        name,
    )
    .execute(&mut *conn)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => email_in_use(email, name),
        _ => error.into(),
    })?;
    let event = crate::plugin::AppEvent::UserCreated {
        occurred_at: Utc::now(),
        org_id,
        user: crate::plugin::event::UserPayload {
            id,
            email: email.to_owned(),
            name: name.to_owned(),
            org_role: "member".into(),
            method: None,
        },
    };
    let event = serde_json::to_value(event).map_err(|e| RowFailure::new(e.to_string()))?;
    crate::jobs::enqueue_outbox(&mut *conn, org_id, "user_created", event)
        .await
        .map_err(|e| RowFailure::new(e.to_string()))?;
    Ok((id, PersonOutcome::Created))
}

fn email_in_use(email: &str, name: &str) -> RowFailure {
    RowFailure::person(
        format!("an account outside this import already uses the email {email:?}"),
        PersonErrorKind::EmailInUse,
        Some(email),
        Some(name),
    )
}

/// Match one org user, including inactive historical users. Email is authoritative;
/// only CSV rows without email may use an unambiguous full-name fallback. A CSV
/// email with no match creates the person (returned with its outcome, which the
/// API source never has: its people are resolved before the entries). Never
/// guesses between normalized duplicates or creates a person from a name.
pub async fn resolve_user(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org_id: Uuid,
    row: &SourceRow,
    source: SourceKind,
) -> Result<(UserKey, Uuid, Option<PersonOutcome>), RowFailure> {
    let email = row
        .user_email
        .as_deref()
        .map(keys::trim_ws)
        .filter(|e| !e.is_empty());
    let name = row
        .user_name
        .as_deref()
        .map(keys::trim_ws)
        .filter(|n| !n.is_empty());
    let (key, field, value) = if let Some(email) = email {
        (UserKey::Email(keys::normalize(email)), "email", email)
    } else {
        if source != SourceKind::Csv {
            return Err(RowFailure::person(
                "time entry has no user email to match",
                PersonErrorKind::MissingEmail,
                None,
                name,
            ));
        }
        let name = name.ok_or_else(|| {
            RowFailure::person(
                "time entry has no user email or full name to match",
                PersonErrorKind::MissingEmail,
                None,
                None,
            )
        })?;
        (UserKey::FullName(keys::normalize(name)), "name", name)
    };
    if let Some(&id) = cache.users.get(&key) {
        return Ok((key, id, None));
    }
    let csv_outcome = |outcome| (source == SourceKind::Csv).then_some(outcome);
    // Two matches are enough to prove ambiguity; do not arbitrarily select one.
    // Missing/ambiguous identities and database errors are never cached.
    let matches = match &key {
        UserKey::Email(email) => {
            sqlx::query_scalar!(
                "SELECT id FROM users WHERE org_id = $1 AND harvest_norm(email) = $2 LIMIT 2",
                org_id,
                email,
            )
            .fetch_all(&mut *conn)
            .await?
        }
        UserKey::FullName(name) => {
            sqlx::query_scalar!(
                "SELECT id FROM users WHERE org_id = $1 AND harvest_norm(name) = $2 LIMIT 2",
                org_id,
                name,
            )
            .fetch_all(&mut *conn)
            .await?
        }
    };
    match (matches.as_slice(), email) {
        ([id], _) => Ok((key, *id, csv_outcome(PersonOutcome::Matched))),
        ([], Some(email)) if source == SourceKind::Csv => {
            let (id, outcome) = create_person(conn, org_id, email, name.unwrap_or(email)).await?;
            Ok((key, id, csv_outcome(outcome)))
        }
        ([], _) => Err(RowFailure::person(
            format!("no Horae user matches {field} {value:?}"),
            if email.is_some() {
                PersonErrorKind::UnmatchedEmail
            } else {
                PersonErrorKind::UnmatchedName
            },
            email,
            name,
        )),
        (_, _) => Err(RowFailure::person(
            format!(
                "ambiguous user {field} {value:?}: multiple Horae users match; provide a unique email or resolve duplicate identities"
            ),
            PersonErrorKind::AmbiguousPerson,
            email,
            name,
        )),
    }
}

/// The row's currency when it is a plausible 3-letter code, else the org default.
fn currency_or(row_currency: Option<&str>, default: &str) -> String {
    match row_currency.map(str::trim).filter(|c| c.len() == 3) {
        Some(c) => c.to_uppercase(),
        None => default.to_string(),
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;

    #[test]
    fn run_cache_round_trip_preserves_identity_namespaces_and_occurrences() {
        let client = Uuid::now_v7();
        let project = Uuid::now_v7();
        let task = Uuid::now_v7();
        let email_user = Uuid::now_v7();
        let named_user = Uuid::now_v7();
        let mut cache = RunCache::default();
        cache.merge(PendingCache {
            parents: vec![
                (ParentKind::Client, "client".into(), client),
                (ParentKind::Project, "project".into(), project),
                (ParentKind::Task, "task".into(), task),
            ],
            user: Some((UserKey::Email("same-key".into()), email_user)),
            person: Some(email_user),
            project_task: Some((project, task)),
            entry_slot: Some("identical-entry".into()),
        });
        cache.merge(PendingCache {
            user: Some((UserKey::FullName("same-key".into()), named_user)),
            entry_slot: Some("identical-entry".into()),
            ..Default::default()
        });
        cache.mark_failed(EntityType::Project, 42);
        let restored: RunCache =
            serde_json::from_value(serde_json::to_value(&cache).unwrap()).unwrap();
        assert_eq!(restored.clients, cache.clients);
        assert_eq!(restored.projects, cache.projects);
        assert_eq!(restored.tasks, cache.tasks);
        assert_eq!(restored.users, cache.users);
        assert_eq!(restored.project_tasks, cache.project_tasks);
        assert!(restored.person_counted(email_user));
        assert_eq!(restored.entry_slot_offset("identical-entry"), 2);
        assert!(restored.parent_failed(EntityType::Project, 42));
    }
}
