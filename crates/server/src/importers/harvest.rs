//! Server-side Harvest importer: the source-agnostic engine plus the two source
//! adapters, OAuth connect flow, credential storage, and provenance-backed
//! resolve/apply (plan.md **Project Structure**).
//!
//! Both source adapters drive normalized [`SourceRow`]s through the shared
//! resolve → [`apply::apply_row`] → report pipeline. Each row is applied in its
//! own savepoint; a `DryRun` rolls back all domain transactions, so no domain
//! data, provenance or watermark persists (FR-014, research.md §7). Durable
//! previews retain simulation checkpoints separately from those transactions.

pub mod account_switch;
pub mod api_source;
pub mod apply;
pub mod client_settings;
pub mod credentials;
pub mod csv_source;
pub mod ids;
mod invoices;
pub mod oauth;
mod parents;
pub mod provenance;
pub mod report;
pub mod resolve;
mod streaming;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod sync_tests;

use chrono::Utc;
#[cfg(any(test, feature = "test-support"))]
use horae_core::importers::harvest::types::SourceKind;
use horae_core::importers::harvest::types::{EntityType, ImportMode, SourceRow, SyncScope};
use sqlx::{Connection, PgPool};
use uuid::Uuid;

#[cfg(test)]
use api_source::HarvestData;
use report::ImportReport;
use resolve::{OrgDefaults, RunCache};

use crate::config::HarvestConfig;
use crate::jobs::JobLease;

async fn begin_import_transaction(
    connection: &mut sqlx::PgConnection,
    org_id: Uuid,
) -> anyhow::Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    let mut tx = connection.begin().await?;
    // Rechecks under advisory locks (a person's email, an archived parent)
    // must see rows committed while the lock was awaited, whatever the
    // connection default.
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    // Access-changing commands lock project parents before task links. Join
    // their gate before the importer acquires any of those resource locks.
    crate::db::lock_organization(&mut tx, org_id, crate::db::OrganizationLock::Shared).await?;
    Ok(tx)
}

pub(crate) fn job_report(report: &ImportReport) -> anyhow::Result<(serde_json::Value, i64)> {
    let processed = report.summary.clients.processed()
        + report.summary.projects.processed()
        + report.summary.tasks.processed()
        + report.summary.people.processed()
        + report.summary.time_entries.processed();
    Ok((serde_json::to_value(report)?, i64::try_from(processed)?))
}

async fn finish_import(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    report: &mut ImportReport,
    lease: Option<&JobLease>,
) -> anyhow::Result<()> {
    match report.mode {
        ImportMode::DryRun => tx.rollback().await?,
        ImportMode::Commit => {
            if let Some(lease) = lease {
                lease.archive_report(&mut tx, report).await?;
                let (report, processed) = job_report(report)?;
                anyhow::ensure!(
                    lease.complete(&mut tx, &report, processed).await?,
                    "job execution interrupted or lease lost"
                );
            }
            tx.commit().await?;
        }
    }
    Ok(())
}

/// A source of normalized rows consumed lazily by the API adapter and engine
/// tests. Returning `None` ends the source; this interface alone does not imply
/// bounded memory or network streaming.
pub trait RowSource {
    fn next_row(&mut self) -> impl Future<Output = anyhow::Result<Option<SourceRow>>> + Send;
}

/// Drive hand-built test sources through the shared row pipeline, using the
/// same organization lock, transaction and preview rollback as uploaded sources.
#[cfg(any(test, feature = "test-support"))]
pub async fn run_import<S: RowSource>(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    source: SourceKind,
    mode: ImportMode,
    src: S,
) -> anyhow::Result<ImportReport> {
    let org = OrgDefaults {
        org_id,
        default_currency,
    };
    let mut connection = lock_import(pool, org_id).await?;
    let result = async {
        let mut tx = begin_import_transaction(&mut connection, org_id).await?;
        let mut report = ImportReport::new(source, mode);
        apply_rows(&mut tx, &mut RunCache::default(), &mut report, org, src).await?;
        match mode {
            ImportMode::Commit => tx.commit().await?,
            ImportMode::DryRun => tx.rollback().await?,
        }
        Ok(report)
    }
    .await;
    release_import(connection).await?;
    result
}

async fn apply_rows<S: RowSource>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cache: &mut RunCache,
    report: &mut ImportReport,
    org: OrgDefaults<'_>,
    mut src: S,
) -> anyhow::Result<()> {
    // A source yields one page of rows: their billing is written together.
    let mut writes = Vec::new();
    while let Some(row) = src.next_row().await? {
        writes.extend(
            apply::apply_row(tx, cache, org, &row, report.source, report.mode)
                .await
                .record_into(report),
        );
    }
    apply::write_links(tx, org.org_id, &mut writes, report).await?;

    debug_assert!(report.reconciles());
    Ok(())
}

/// An in-memory row source for tests that hand-build rows.
#[cfg(any(test, feature = "test-support"))]
pub struct VecSource {
    rows: std::vec::IntoIter<SourceRow>,
}

#[cfg(any(test, feature = "test-support"))]
impl VecSource {
    pub fn new(rows: Vec<SourceRow>) -> Self {
        Self {
            rows: rows.into_iter(),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl RowSource for VecSource {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        Ok(self.rows.next())
    }
}

/// Run-level failures (FR-003, FR-024), distinct from per-record report errors.
/// A failed download or transaction leaves no partially committed import.
#[derive(Debug, thiserror::Error)]
pub enum ApiImportError {
    #[error(
        "Another Harvest import is already running for this organization; retry when it finishes"
    )]
    Busy,
    #[error("no usable Harvest connection — connect Harvest first")]
    NotConnected,
    #[error("Harvest no longer accepts this connection — reconnect Harvest")]
    ReconnectRequired,
    #[error(
        "Harvest refused access (HTTP 403) — reconnect Harvest as a user who can see all clients, projects, people and time entries"
    )]
    Forbidden,
    /// A manager can read the rest of the account but not its invoices.
    #[error(
        "Harvest refused access to invoices (HTTP 403) — reconnect Harvest as an administrator, or as a manager who can create and edit invoices"
    )]
    InvoicesForbidden,
    /// Harvest timed out or answered with a server error; worth retrying.
    #[error("{0}")]
    Unavailable(String),
    #[error(transparent)]
    Other(anyhow::Error),
}

impl From<anyhow::Error> for ApiImportError {
    /// Unwraps a typed failure raised below an `anyhow` boundary (the HTTP
    /// thread, the token refresh) so the job runner can still see its kind.
    fn from(error: anyhow::Error) -> Self {
        let error = match error.downcast::<Self>() {
            Ok(typed) => return typed,
            Err(error) => error,
        };
        match error.downcast::<horae_harvest::Error>() {
            Ok(horae_harvest::Error::Unauthorized) => Self::ReconnectRequired,
            Ok(horae_harvest::Error::Forbidden) => Self::Forbidden,
            Ok(horae_harvest::Error::Unavailable(message)) => Self::Unavailable(message),
            Err(error) => Self::Other(error),
        }
    }
}

impl ApiImportError {
    /// Failures no retry can fix until an administrator acts in Harvest or Horae.
    pub(crate) fn is_permanent(error: &anyhow::Error) -> bool {
        matches!(
            error.downcast_ref::<Self>(),
            Some(
                Self::NotConnected
                    | Self::ReconnectRequired
                    | Self::Forbidden
                    | Self::InvoicesForbidden
            )
        )
    }

    /// A 403 while reading invoices names what the connection lacks.
    pub(super) fn on_invoices(error: anyhow::Error) -> anyhow::Error {
        match error.downcast_ref() {
            Some(horae_harvest::Error::Forbidden) => Self::InvoicesForbidden.into(),
            _ => error,
        }
    }

    /// What a failed job attempt shows the administrator, if Harvest caused it.
    pub(crate) fn job_error_kind(error: &anyhow::Error) -> Option<crate::models::JobErrorKind> {
        use crate::models::JobErrorKind;
        match error.downcast_ref::<Self>()? {
            Self::ReconnectRequired => Some(JobErrorKind::ReconnectRequired),
            Self::Forbidden => Some(JobErrorKind::Forbidden),
            Self::InvoicesForbidden => Some(JobErrorKind::InvoicesForbidden),
            Self::Unavailable(_) => Some(JobErrorKind::Unavailable),
            Self::Busy | Self::NotConnected | Self::Other(_) => None,
        }
    }
}

/// One import session per organization, shared by API and CSV across processes.
/// The session lock spans token refresh (autocommit) and the separate data
/// transaction. Closing the connection on every exit, including cancellation,
/// prevents a session lock from ever leaking back into the shared pool.
async fn lock_import(
    pool: &PgPool,
    org_id: Uuid,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, ApiImportError> {
    let mut connection = pool.acquire().await.map_err(anyhow::Error::from)?;
    connection.close_on_drop();
    let acquired = sqlx::query_scalar!(
        r#"SELECT pg_try_advisory_lock(hashtextextended($1, 0)) as "acquired!""#,
        format!("horae:harvest-import:{org_id}"),
    )
    .fetch_one(&mut *connection)
    .await
    .map_err(anyhow::Error::from)?;
    if !acquired {
        return Err(ApiImportError::Busy);
    }
    Ok(connection)
}

async fn release_import(
    mut connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
) -> anyhow::Result<()> {
    loop {
        match connection.flush().await {
            Ok(()) => break,
            // A cancelled RELEASE can reach PostgreSQL before SQLx updates its
            // depth. Drain the resulting stale rollback, never reuse this session.
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("3B001") => {
                tracing::debug!("draining an interrupted import savepoint rollback");
            }
            Err(error) => return Err(error.into()),
        }
    }
    // A cancelled BEGIN may likewise precede SQLx's transaction bookkeeping.
    // Roll back server state before releasing the run reservation.
    sqlx::query!("ROLLBACK").execute(&mut *connection).await?;
    // Closing the socket does not wait for PostgreSQL to release session locks.
    // Await the unlock so an immediate retry cannot see a completed import as busy.
    sqlx::query!("SELECT pg_advisory_unlock_all()")
        .execute(&mut *connection)
        .await?;
    connection.close().await?;
    Ok(())
}

/// A cancelled waiter cannot cancel blocking HTTP work. Keep the import
/// session with the worker until it really exits, not with the waiting future.
async fn blocking_import_call<T: Send + 'static>(
    connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
    call: impl FnOnce() -> T + Send + 'static,
) -> Result<(sqlx::pool::PoolConnection<sqlx::Postgres>, T), ApiImportError> {
    tokio::task::spawn_blocking(move || {
        let result = call();
        (connection, result)
    })
    .await
    .map_err(|e| ApiImportError::Other(anyhow::anyhow!("Harvest HTTP task failed: {e}")))
}

/// Drive a CLI-submitted durable job through the production API adapter using
/// the existing loopback Harvest fixture, with no production URL override.
#[cfg(any(test, feature = "test-support"))]
pub async fn complete_cli_api_fixture(
    pool: &PgPool,
    org: Uuid,
    id: Uuid,
    email: String,
    cfg: &HarvestConfig,
    mode: ImportMode,
) {
    use api_source::http::{
        ApiHttp,
        test_server::{Response, Server},
    };
    use serde_json::json;
    let server = Server::start(move |url| {
        let collection = url.path().rsplit('/').next().unwrap();
        let items = match collection {
            "clients" => json!([{"id":101,"name":"CLI API Client","currency":"USD"}]),
            "projects" => json!([{"id":102,"name":"CLI API Project","client":{"id":101}}]),
            "tasks" => json!([{"id":103,"name":"CLI API Task"}]),
            "users" => json!([{"id":104,"email":email}]),
            "invoices" => json!([]),
            "time_entries" => json!([{
                "id":105,"spent_date":"2026-09-14","hours":1,
                "notes":"cli-api-acceptance","project":{"id":102},
                "task":{"id":103},"user":{"id":104}
            }]),
            _ => panic!("unexpected collection {collection}"),
        };
        Response::json(json!({collection:items,"next_page":null,"links":{"next":null}}))
    });
    let (lease, stop) = crate::jobs::claim_lease_for_test(pool).await;
    assert_eq!(
        crate::jobs::status(pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .status,
        "running"
    );
    crate::jobs::run_claimed(pool, &lease, stop, async {
        let report = run_api_import_with_http(
            pool,
            org,
            "USD",
            cfg,
            mode,
            SyncScope::Full,
            ApiHttp::local(server.base.clone()),
            Some(&lease),
        )
        .await?;
        job_report(&report)
    })
    .await
    .unwrap();
}

/// Run a full/incremental import from the Harvest API through the shared engine
/// (FR-023–FR-026). Loads the org's stored connection, refreshes an expired token
/// transparently, indexes catalogs, streams time-entry pages through the engine, and —
/// only on an error-free committing run — advances the incremental watermark.
#[cfg(test)]
pub async fn run_api_import(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    cfg: &HarvestConfig,
    mode: ImportMode,
    sync: SyncScope,
) -> Result<ImportReport, ApiImportError> {
    run_api_import_with_http(
        pool,
        org_id,
        default_currency,
        cfg,
        mode,
        sync,
        api_source::http::ApiHttp::new()?,
        None,
    )
    .await
}

pub(crate) async fn run_api_import_with_lease(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    cfg: &HarvestConfig,
    mode: ImportMode,
    sync: SyncScope,
    lease: &JobLease,
) -> Result<ImportReport, ApiImportError> {
    run_api_import_with_http(
        pool,
        org_id,
        default_currency,
        cfg,
        mode,
        sync,
        api_source::http::ApiHttp::new()?,
        Some(lease),
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "shares the HTTP test seam with synchronous and leased imports"
)]
async fn run_api_import_with_http(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    cfg: &HarvestConfig,
    mode: ImportMode,
    sync: SyncScope,
    http: api_source::http::ApiHttp,
    lease: Option<&JobLease>,
) -> Result<ImportReport, ApiImportError> {
    if let Some(lease) = lease {
        lease.check_organization(org_id)?;
    }
    let mut connection = lock_import(pool, org_id).await?;
    let key = &cfg.encryption_key_hex;
    if let Some(lease) = lease
        && let Err(error) = lease.check_account_generation(&mut connection).await
    {
        release_import(connection).await?;
        return Err(error.into());
    }
    let loaded = credentials::load(&mut *connection, org_id, key)
        .await
        .map_err(ApiImportError::from)
        .and_then(|value| value.ok_or(ApiImportError::NotConnected));
    let mut conn = match loaded {
        Ok(value) => value,
        Err(error) => {
            release_import(connection).await?;
            return Err(error);
        }
    };

    // Transparent refresh before the access token expires (FR-024).
    if oauth::needs_refresh(conn.token_expires_at, Utc::now()) {
        let cfg_owned = cfg.clone();
        let refresh_token = conn.refresh_token.clone();
        let (returned_connection, refreshed) = blocking_import_call(connection, move || {
            oauth::refresh(&cfg_owned, &refresh_token)
        })
        .await?;
        connection = returned_connection;
        let refreshed = match refreshed {
            Ok(value) => value,
            Err(error) => {
                release_import(connection).await?;
                return Err(error.into());
            }
        };
        let updated = credentials::update_tokens(
            &mut *connection,
            org_id,
            key,
            &refreshed.access_token,
            &refreshed.refresh_token,
            refreshed.expires_at,
        )
        .await;
        if let Err(error) = updated {
            release_import(connection).await?;
            return Err(error.into());
        }
        conn.access_token = refreshed.access_token;
        conn.refresh_token = refreshed.refresh_token;
        conn.token_expires_at = refreshed.expires_at;
    }

    // Per-entity `updated_since` for an incremental run (FR-025).
    let since = match sync {
        SyncScope::Full => None,
        SyncScope::Incremental => conn.watermark_for(EntityType::TimeEntry),
    };
    let invoice_mark = conn.watermark_for(EntityType::Invoice);

    // A bounded queue joins blocking HTTP pages to the async row pipeline.
    let capture_started_at = Utc::now();
    if let Some(lease) = lease {
        return streaming::durable::run(
            connection,
            org_id,
            streaming::durable::Request {
                account_id: conn.account_id,
                currency: default_currency.to_owned(),
                sync,
                since,
                invoice_mark,
                captured_at: capture_started_at,
            },
            mode,
            conn.access_token,
            http,
            lease,
        )
        .await
        .map_err(Into::into);
    }
    let voided = match invoices::voided_versions(&mut connection, org_id).await {
        Ok(voided) => voided,
        Err(error) => {
            release_import(connection).await?;
            return Err(anyhow::Error::from(error).into());
        }
    };
    let window = api_source::EntryWindow::new(since, invoice_mark, voided);
    streaming::run(
        connection,
        org_id,
        default_currency,
        mode,
        capture_started_at,
        move |pages| streaming::fetch(http, &conn.access_token, &conn.account_id, window, pages),
        lease,
    )
    .await
    .map_err(Into::into)
}

// ── OAuth connect ─────────────────────────────────────────────────────────────

/// What `harvest_connect_start` keeps in the admin's session for the callback
/// (`crate::http::harvest_oauth`).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct OAuthAttempt {
    pub nonce: String,
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub version: account_switch::Version,
}

/// Exchange the code, resolve the account id, and persist the encrypted tokens
/// for the signed-in admin's org. Errors leave no credentials written.
pub async fn complete_connect(
    user_id: Uuid,
    code: String,
    attempt: &OAuthAttempt,
) -> anyhow::Result<()> {
    let state = crate::state::global_state().await;
    let org_id = attempt.org_id;
    // A refusal before the token exchange; storing the tokens checks again
    // under the locks. Someone outside the attempt's organization loads with
    // no authority.
    let actor = crate::ops::Actor::load_unlocked(
        &mut *state.db.acquire().await?,
        crate::ops::Caller::session(org_id, user_id),
    )
    .await?;
    if user_id != attempt.user_id || !actor.administrator() {
        anyhow::bail!("admin access required to connect Harvest");
    }
    // A client change after the attempt started bumps the connection revision,
    // which is checked below and again when the tokens are stored.
    let cfg = client_settings::resolve(&state.db, &state.harvest, org_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Harvest importer is not configured"))?;
    let current = account_switch::status(&state.db, org_id, true).await?;
    anyhow::ensure!(
        attempt.version
            == account_switch::Version {
                account_generation: current.account_generation,
                connection_revision: current.connection_revision
            },
        account_switch::ChangeError::Stale
    );

    // Exchange + account lookup off the async runtime (blocking ureq).
    let cfg_owned = cfg.clone();
    let (tokens, account_id) = tokio::task::spawn_blocking(move || {
        let tokens = oauth::exchange_code(&cfg_owned, &code)?;
        let account_id = oauth::fetch_account_id(&tokens.access_token)?;
        anyhow::Ok((tokens, account_id))
    })
    .await??;

    credentials::store_for_attempt(
        &state.db,
        org_id,
        attempt.user_id,
        &cfg.encryption_key_hex,
        &account_id,
        &tokens.access_token,
        &tokens.refresh_token,
        tokens.expires_at,
        tokens.scope.as_deref(),
        attempt.version,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod error_tests {
    use super::ApiImportError;

    #[test]
    fn harvest_client_failures_become_their_import_errors() {
        use anyhow::Context;
        for (error, expected) in [
            (horae_harvest::Error::Unauthorized, "reconnect"),
            (horae_harvest::Error::Forbidden, "forbidden"),
            (horae_harvest::Error::Unavailable("later".into()), "later"),
        ] {
            let wrapped = Err::<(), _>(error).context("fetching clients").unwrap_err();
            let found = match ApiImportError::from(wrapped) {
                ApiImportError::ReconnectRequired => "reconnect".into(),
                ApiImportError::Forbidden => "forbidden".into(),
                ApiImportError::Unavailable(message) => message,
                other => panic!("unmapped: {other}"),
            };
            assert_eq!(found, expected);
        }
        assert!(matches!(
            ApiImportError::from(anyhow::anyhow!("plain")),
            ApiImportError::Other(_)
        ));
    }
}
