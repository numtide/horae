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
pub mod provenance;
pub mod report;
pub mod resolve;

#[cfg(test)]
mod engine_tests;

use chrono::{DateTime, Utc};
use horae_core::harvest_import::types::{EntityType, ImportMode, SourceKind, SourceRow, SyncScope};
use sqlx::PgPool;
use uuid::Uuid;

use api_source::{ApiSource, HarvestData};
use report::ImportReport;
use resolve::{OrgDefaults, RunCache};

use crate::config::HarvestConfig;

/// A source of normalized rows the engine consumes lazily (research.md §9). Both
/// adapters implement it: the CSV adapter walks its parsed records, the API
/// adapter walks Harvest pages. Returning `None` ends the run.
pub trait RowSource {
    fn next_row(&mut self) -> impl Future<Output = anyhow::Result<Option<SourceRow>>> + Send;
}

/// Drive a source through the engine and return the run report. In `Commit` mode
/// the outer transaction is committed; in `DryRun` it is rolled back so nothing
/// persists (FR-014). Advancing the incremental watermark on a committing API run
/// is the caller's responsibility, done only after this returns success.
pub async fn run_import<S: RowSource>(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    source: SourceKind,
    mode: ImportMode,
    mut src: S,
) -> anyhow::Result<ImportReport> {
    let mut report = ImportReport::new(source, mode);
    let mut cache = RunCache::default();
    let org = OrgDefaults {
        org_id,
        default_currency,
    };

    let mut tx = pool.begin().await?;
    while let Some(row) = src.next_row().await? {
        let result = apply::apply_row(&mut tx, &mut cache, org, &row).await;
        for (entity, outcome) in &result.outcomes {
            report.record(*entity, outcome);
        }
    }

    match mode {
        ImportMode::Commit => tx.commit().await?,
        ImportMode::DryRun => tx.rollback().await?,
    }

    debug_assert!(report.reconciles());
    Ok(report)
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
    #[error("no usable Harvest connection — connect Harvest first")]
    NotConnected,
    #[error("Harvest connection expired — reconnect Harvest")]
    ReconnectRequired,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Run a full/incremental import from the Harvest API through the shared engine
/// (FR-023–FR-026). Loads the org's stored connection, refreshes an expired token
/// transparently, pulls every collection, assembles rows, runs the engine, and —
/// only on a committing run — advances the incremental watermark.
pub async fn run_api_import(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    cfg: &HarvestConfig,
    mode: ImportMode,
    sync: SyncScope,
) -> Result<ImportReport, ApiImportError> {
    let key = &cfg.encryption_key_hex;
    let mut conn = credentials::load(pool, org_id, key)
        .await?
        .ok_or(ApiImportError::NotConnected)?;

    // Transparent refresh when the access token is at or past expiry (FR-024).
    if let Some(expiry) = conn.token_expires_at
        && expiry <= Utc::now()
    {
        let cfg_owned = cfg.clone();
        let refresh_token = conn.refresh_token.clone();
        let refreshed = tokio::task::spawn_blocking(move || {
            let agent = ureq::agent();
            oauth::refresh(&agent, &cfg_owned, &refresh_token)
        })
        .await
        .map_err(|e| ApiImportError::Other(anyhow::anyhow!("refresh task panicked: {e}")))?
        .map_err(|_| ApiImportError::ReconnectRequired)?;
        credentials::update_tokens(
            pool,
            org_id,
            key,
            &refreshed.access_token,
            &refreshed.refresh_token,
            refreshed.expires_at,
        )
        .await?;
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
    let access = conn.access_token.clone();
    let account = conn.account_id.clone();
    let data = tokio::task::spawn_blocking(move || fetch_all_collections(&access, &account, since))
        .await
        .map_err(|e| ApiImportError::Other(anyhow::anyhow!("fetch task panicked: {e}")))??;

    // The highest `updated_at` we saw drives the next incremental watermark.
    let high_water = data
        .time_entries
        .iter()
        .filter_map(|te| te.updated_at)
        .max();

    let report = run_import(
        pool,
        org_id,
        default_currency,
        SourceKind::HarvestApi,
        mode,
        ApiSource::from_data(&data),
    )
    .await?;

    if mode == ImportMode::Commit {
        let mark = high_water.unwrap_or_else(Utc::now);
        advance_all_watermarks(pool, org_id, mark).await?;
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

/// Advance every entity's watermark to `mark` after a successful committing run.
async fn advance_all_watermarks(
    pool: &PgPool,
    org_id: Uuid,
    mark: DateTime<Utc>,
) -> anyhow::Result<()> {
    let marks: Vec<(EntityType, DateTime<Utc>)> =
        EntityType::ALL.iter().map(|&e| (e, mark)).collect();
    credentials::advance_watermark(pool, org_id, &marks).await
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
) -> axum::response::Redirect {
    use axum::response::Redirect;

    // The nonce is single-use: consume it regardless of outcome.
    let stored: Option<String> = session.get(OAUTH_STATE_KEY).await.ok().flatten();
    let _ = session.remove::<String>(OAUTH_STATE_KEY).await;

    let dest_ok = "/import/harvest?connected=1";
    let dest_err = "/import/harvest?error=1";

    if params.error.is_some() {
        tracing::warn!("Harvest OAuth returned error: {:?}", params.error);
        return Redirect::to(dest_err);
    }

    // CSRF: the returned `state` must equal the value we stored at start, and we
    // reject a mismatch **without exchanging the code** (research.md §10).
    let (Some(returned), Some(stored)) = (params.state.as_deref(), stored.as_deref()) else {
        tracing::warn!("Harvest callback missing state");
        return Redirect::to(dest_err);
    };
    if returned != stored {
        tracing::warn!("Harvest callback state mismatch (possible CSRF)");
        return Redirect::to(dest_err);
    }
    let Some(code) = params.code.clone() else {
        tracing::warn!("Harvest callback missing code");
        return Redirect::to(dest_err);
    };

    match complete_connect(&session, code).await {
        Ok(()) => Redirect::to(dest_ok),
        Err(e) => {
            tracing::error!("Harvest connect failed: {e}");
            Redirect::to(dest_err)
        }
    }
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
