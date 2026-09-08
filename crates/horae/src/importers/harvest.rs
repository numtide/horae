//! Server-side Harvest importer: the source-agnostic engine plus the two source
//! adapters, OAuth connect flow, credential storage, and provenance-backed
//! resolve/apply (plan.md **Project Structure**).
//!
//! The engine ([`run_import`]) drives a stream of [`SourceRow`]s — produced by
//! either the API adapter or the CSV adapter — through resolve → apply → report.
//! It never learns which adapter produced a row. Each row is applied in its own
//! savepoint (see [`apply`]); a `DryRun` runs the whole stream inside a
//! transaction that is rolled back, so nothing persists — not data, not
//! provenance, not the watermark (FR-014, research.md §7).

pub mod api_source;
pub mod apply;
pub mod credentials;
pub mod csv_source;
pub mod oauth;
mod parents;
pub mod provenance;
pub mod report;
pub mod resolve;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod sync_tests;

use chrono::{DateTime, Utc};
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, SourceKind, SourceRow, SyncScope,
};
use sqlx::{Acquire, PgPool};
use uuid::Uuid;

use api_source::{ApiSource, HarvestData};
use report::ImportReport;
use resolve::{OrgDefaults, RunCache};

use crate::config::HarvestConfig;

/// A source of normalized rows the engine consumes lazily (research.md §9). Both
/// adapters implement it over parsed records. Returning `None` ends the run;
/// this interface alone does not imply bounded memory or network streaming.
pub trait RowSource {
    fn next_row(&mut self) -> impl Future<Output = anyhow::Result<Option<SourceRow>>> + Send;
}

/// Drive a source through the engine and return the run report. In `Commit` mode
/// the outer transaction is committed; in `DryRun` it is rolled back so nothing
/// persists (FR-014). API synchronization uses the same row pipeline inside a
/// transaction shared with its watermark update.
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
        let mut tx = connection.begin().await?;
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
    while let Some(row) = src.next_row().await? {
        let result = apply::apply_row(tx, cache, org, &row).await;
        for (entity, outcome) in &result.outcomes {
            report.record(*entity, outcome);
        }
    }

    debug_assert!(report.reconciles());
    Ok(())
}

/// An in-memory row source over a `Vec` — used by the CSV adapter (after parsing)
/// and by integration tests that hand-build rows.
pub struct VecSource {
    rows: std::vec::IntoIter<SourceRow>,
}

impl VecSource {
    pub fn new(rows: Vec<SourceRow>) -> Self {
        Self {
            rows: rows.into_iter(),
        }
    }
}

impl RowSource for VecSource {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        Ok(self.rows.next())
    }
}

/// Why an API import could not even start (FR-003, FR-024). Distinct from the
/// per-record errors inside a report — these reject the whole run up front.
#[derive(Debug, thiserror::Error)]
pub enum ApiImportError {
    #[error(
        "Another Harvest import is already running for this organization; retry when it finishes"
    )]
    Busy,
    #[error("no usable Harvest connection — connect Harvest first")]
    NotConnected,
    #[error("Harvest connection expired — reconnect Harvest")]
    ReconnectRequired,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
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
    // Closing the socket does not wait for PostgreSQL to release session locks.
    // Await the unlock so an immediate retry cannot see a completed import as busy.
    // SQLx flushes any rollback queued by a dropped transaction before this query.
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

/// Run a full/incremental import from the Harvest API through the shared engine
/// (FR-023–FR-026). Loads the org's stored connection, refreshes an expired token
/// transparently, pulls every collection, assembles rows, runs the engine, and —
/// only on an error-free committing run — advances the incremental watermark.
pub async fn run_api_import(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    cfg: &HarvestConfig,
    mode: ImportMode,
    sync: SyncScope,
) -> Result<ImportReport, ApiImportError> {
    let mut connection = lock_import(pool, org_id).await?;
    let key = &cfg.encryption_key_hex;
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

    // Transparent refresh when the access token is at or past expiry (FR-024).
    if let Some(expiry) = conn.token_expires_at
        && expiry <= Utc::now()
    {
        let cfg_owned = cfg.clone();
        let refresh_token = conn.refresh_token.clone();
        let (returned_connection, refreshed) = blocking_import_call(connection, move || {
            let agent = ureq::agent();
            oauth::refresh(&agent, &cfg_owned, &refresh_token)
        })
        .await?;
        connection = returned_connection;
        let refreshed = match refreshed {
            Ok(value) => value,
            Err(_) => {
                release_import(connection).await?;
                return Err(ApiImportError::ReconnectRequired);
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

    // Fetch all collections off the async runtime (blocking ureq).
    let capture_started_at = Utc::now();
    let access = conn.access_token.clone();
    let account = conn.account_id.clone();
    let (mut connection, data) = blocking_import_call(connection, move || {
        fetch_all_collections(&access, &account, since)
    })
    .await?;
    let result = async {
        let data = data?;
        apply_api_data(
            &mut connection,
            org_id,
            default_currency,
            mode,
            &data,
            capture_started_at,
        )
        .await
    }
    .await;
    release_import(connection).await?;
    result.map_err(Into::into)
}

async fn apply_api_data(
    connection: &mut sqlx::PgConnection,
    org_id: Uuid,
    default_currency: &str,
    mode: ImportMode,
    data: &HarvestData,
    capture_started_at: DateTime<Utc>,
) -> anyhow::Result<ImportReport> {
    // A missing timestamp cannot certify coverage. Empty responses likewise
    // carry no source timestamp from which to advance the cursor.
    let high_water = data
        .time_entries
        .iter()
        .filter_map(|te| te.updated_at)
        .max();

    let mut tx = connection.begin().await?;
    let org = OrgDefaults {
        org_id,
        default_currency,
    };
    let mut cache = RunCache::default();
    let mut report = ImportReport::new(SourceKind::HarvestApi, mode);
    parents::apply(&mut tx, &mut cache, org, data, &mut report).await?;
    apply_rows(
        &mut tx,
        &mut cache,
        &mut report,
        org,
        ApiSource::from_data(data),
    )
    .await?;

    if mode == ImportMode::Commit
        && report.error_count() == 0
        && data
            .time_entries
            .iter()
            .all(|entry| entry.updated_at.is_some())
        && let Some(high_water) = high_water
        && let Some(mark) = high_water
            .min(capture_started_at)
            .checked_sub_signed(chrono::Duration::seconds(1))
    {
        // Re-fetch changes made during capture, including a one-second overlap
        // for timestamp precision and boundary inclusivity. Provenance makes
        // those retries idempotent. Parents are always fetched in full.
        credentials::advance_watermark(&mut *tx, org_id, &[(EntityType::TimeEntry, mark)]).await?;
    }
    match mode {
        ImportMode::Commit => tx.commit().await?,
        ImportMode::DryRun => tx.rollback().await?,
    }

    Ok(report)
}

/// Fetch every Harvest collection into a [`HarvestData`] (blocking). Parents in
/// full; time entries filtered by `updated_since` on an incremental run.
fn fetch_all_collections(
    access_token: &str,
    account_id: &str,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<HarvestData> {
    use api_source::*;
    let agent = ureq::agent();
    let clients = parse_collection::<ApiClient>(&agent, access_token, account_id, "clients", None)?;
    let projects =
        parse_collection::<ApiProject>(&agent, access_token, account_id, "projects", None)?;
    let tasks = parse_collection::<ApiTask>(&agent, access_token, account_id, "tasks", None)?;
    let users = parse_collection::<ApiUser>(&agent, access_token, account_id, "users", None)?;
    let time_entries =
        parse_collection::<ApiTimeEntry>(&agent, access_token, account_id, "time_entries", since)?;
    Ok(HarvestData {
        clients,
        projects,
        tasks,
        users,
        time_entries,
    })
}

/// Fetch one collection and deserialize each item into `T`.
fn parse_collection<T: serde::de::DeserializeOwned>(
    agent: &ureq::Agent,
    access_token: &str,
    account_id: &str,
    collection: &str,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<T>> {
    let items = api_source::fetch_all(agent, access_token, account_id, collection, since)?;
    items
        .into_iter()
        .map(|v| serde_json::from_value(v).map_err(Into::into))
        .collect()
}

// ── OAuth connect: session nonce + callback route ─────────────────────────────

/// Session key holding the per-start `state` nonce between `harvest_connect_start`
/// and the callback, bound to the initiating admin's session (research.md §10).
pub const OAUTH_STATE_KEY: &str = "harvest_oauth_state";

/// The plain Axum route the browser is redirected to after authorizing on
/// Harvest — a redirect target, so it cannot be a `#[server]` fn (Constitution
/// IV). Registered beside `auth::router()`.
pub fn callback_router() -> axum::Router {
    use axum::routing::get;
    axum::Router::new().route("/auth/harvest/callback", get(oauth_callback))
}

#[derive(serde::Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// `GET /auth/harvest/callback`: validate `state`, exchange the code, resolve the
/// account id, and store the encrypted credentials (FR-022). Redirects into the
/// admin "Import from Harvest" screen.
async fn oauth_callback(
    session: tower_sessions::Session,
    axum::extract::Query(params): axum::extract::Query<CallbackParams>,
) -> axum::response::Response {
    use axum::response::{IntoResponse, Redirect};

    // The nonce is single-use: consume it regardless of outcome.
    let stored: Option<String> = session.get(OAUTH_STATE_KEY).await.ok().flatten();
    let _ = session.remove::<String>(OAUTH_STATE_KEY).await;

    let dest_ok = "/admin/importers?connected=1";
    let dest_err = "/admin/importers?error=1";

    // Validate `state` and extract the code; a missing/mismatched state or error
    // yields no code, so we never reach the exchange (research.md §10).
    let code = match authorized_code(&params, stored.as_deref()) {
        Ok(code) => code,
        Err(reason) => {
            tracing::warn!("Harvest callback rejected before exchange: {reason}");
            return Redirect::to(dest_err).into_response();
        }
    };

    match complete_connect(&session, code).await {
        Ok(()) => Redirect::to(dest_ok).into_response(),
        Err(e) => {
            if let Some(response) = connection_conflict_response(&e) {
                return response;
            }
            tracing::error!("Harvest connect failed: {e}");
            Redirect::to(dest_err).into_response()
        }
    }
}

/// Only known, secret-free policy errors may be returned to the browser.
fn connection_conflict_response(error: &anyhow::Error) -> Option<axum::response::Response> {
    use axum::response::IntoResponse;
    let message = if let Some(policy) = error.downcast_ref::<credentials::ConnectionError>() {
        policy.to_string()
    } else if matches!(
        error.downcast_ref::<ApiImportError>(),
        Some(ApiImportError::Busy)
    ) {
        ApiImportError::Busy.to_string()
    } else {
        return None;
    };
    Some((axum::http::StatusCode::CONFLICT, message).into_response())
}

/// Decide whether a callback may proceed to the token exchange. Returns the
/// authorization code only when the provider reported no error, a `state` was
/// stored for this session, and the returned `state` matches it exactly. A
/// missing or mismatched `state` is rejected **without** the code (CSRF, FR-022).
fn authorized_code(
    params: &CallbackParams,
    stored_state: Option<&str>,
) -> Result<String, &'static str> {
    if params.error.is_some() {
        return Err("provider returned an error");
    }
    let stored = stored_state.ok_or("no state stored for this session")?;
    let returned = params.state.as_deref().ok_or("callback missing state")?;
    if returned != stored {
        return Err("state mismatch (possible CSRF)");
    }
    params.code.clone().ok_or("callback missing code")
}

/// Exchange the code, resolve the account id, and persist the encrypted tokens
/// for the acting admin's org. Errors leave no credentials written.
async fn complete_connect(session: &tower_sessions::Session, code: String) -> anyhow::Result<()> {
    let state = crate::state::global_state().await;
    let cfg = state
        .harvest
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Harvest importer is not configured"))?;

    // Only an authenticated admin may land a connection on their org.
    let user_id = crate::auth::session::get_session_user_id(session)
        .await
        .ok_or_else(|| anyhow::anyhow!("no authenticated session"))?;
    let user = sqlx::query!(
        r#"SELECT org_id, org_role::text as "role!" FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| anyhow::anyhow!("user not found"))?;
    if user.role != "admin" {
        anyhow::bail!("admin access required to connect Harvest");
    }

    // Exchange + account lookup off the async runtime (blocking ureq).
    let cfg_owned = cfg.clone();
    let (tokens, account_id) = tokio::task::spawn_blocking(move || {
        let agent = ureq::agent();
        let tokens = oauth::exchange_code(&agent, &cfg_owned, &code)?;
        let account_id = oauth::fetch_account_id(&agent, &tokens.access_token)?;
        anyhow::Ok((tokens, account_id))
    })
    .await??;

    credentials::store(
        &state.db,
        user.org_id,
        &cfg.encryption_key_hex,
        &account_id,
        &tokens.access_token,
        &tokens.refresh_token,
        tokens.expires_at,
        tokens.scope.as_deref(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod oauth_callback_tests {
    use super::*;

    #[tokio::test]
    async fn account_policy_failures_return_safe_actionable_conflicts() {
        for error in [
            anyhow::Error::from(credentials::ConnectionError::AccountChange),
            anyhow::Error::from(credentials::ConnectionError::UnidentifiedProvenance),
            anyhow::Error::from(ApiImportError::Busy),
        ] {
            let response = connection_conflict_response(&error).unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), error.to_string().as_bytes());
        }
        assert!(
            connection_conflict_response(&anyhow::anyhow!("secret upstream payload")).is_none()
        );
    }

    fn params(error: Option<&str>, state: Option<&str>, code: Option<&str>) -> CallbackParams {
        CallbackParams {
            error: error.map(str::to_string),
            state: state.map(str::to_string),
            code: code.map(str::to_string),
        }
    }

    #[test]
    fn valid_matching_state_yields_the_code() {
        let p = params(None, Some("nonce"), Some("the-code"));
        assert_eq!(authorized_code(&p, Some("nonce")).unwrap(), "the-code");
    }

    #[test]
    fn mismatched_state_is_rejected_without_a_code() {
        let p = params(None, Some("attacker"), Some("the-code"));
        assert!(authorized_code(&p, Some("nonce")).is_err());
    }

    #[test]
    fn missing_stored_state_is_rejected() {
        let p = params(None, Some("nonce"), Some("the-code"));
        assert!(authorized_code(&p, None).is_err());
    }

    #[test]
    fn missing_returned_state_is_rejected() {
        let p = params(None, None, Some("the-code"));
        assert!(authorized_code(&p, Some("nonce")).is_err());
    }

    #[test]
    fn provider_error_is_rejected_before_exchange() {
        let p = params(Some("access_denied"), Some("nonce"), Some("the-code"));
        assert!(authorized_code(&p, Some("nonce")).is_err());
    }

    #[test]
    fn valid_state_but_missing_code_is_rejected() {
        let p = params(None, Some("nonce"), None);
        assert!(authorized_code(&p, Some("nonce")).is_err());
    }
}
