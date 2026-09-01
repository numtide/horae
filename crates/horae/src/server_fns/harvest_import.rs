//! Admin-only server functions for the Harvest importer (contracts/importer-api.md).
//!
//! These are the SPA's entry points: start the OAuth connect, read connection
//! status, and run an API or CSV import through the shared engine. All reject
//! non-admins with `FORBIDDEN` (FR-001) and use named status codes.

use super::*;
use horae_core::harvest_import::types::{ConnectionStatus, ImportMode, ImportReport, SyncScope};

/// Begin the Harvest OAuth2 connect: generate a per-start `state` nonce bound to
/// the admin's session and return the authorization URL for the SPA to redirect
/// to (contracts/importer-api.md §1).
#[server]
pub async fn harvest_connect_start() -> Result<String, ServerFnError> {
    require_admin().await?;
    let cfg = harvest_config().await?;

    // A random, session-bound nonce validated on the callback (CSRF, FR-022).
    let nonce = uuid::Uuid::now_v7().simple().to_string();
    let session: tower_sessions::Session =
        dioxus_fullstack::FullstackContext::extract::<tower_sessions::Session, _>().await?;
    session
        .insert(crate::harvest_import::OAUTH_STATE_KEY, &nonce)
        .await
        .map_err(server_err)?;

    Ok(crate::harvest_import::oauth::authorize_url(&cfg, &nonce))
}

/// Report whether the org has a usable Harvest connection (never the tokens).
#[server]
pub async fn harvest_connection_status() -> Result<ConnectionStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;

    let row = sqlx::query!(
        r#"SELECT harvest_account_id,
                  token_expires_at as "token_expires_at: chrono::DateTime<chrono::Utc>"
           FROM harvest_credentials WHERE org_id = $1"#,
        admin.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    Ok(match row {
        Some(r) => ConnectionStatus {
            connected: true,
            account_id: Some(r.harvest_account_id),
            token_expired: r
                .token_expires_at
                .is_some_and(|e| e <= chrono::Utc::now()),
        },
        None => ConnectionStatus::default(),
    })
}

/// Run an import from the Harvest API (primary source). Rejects up front when no
/// connection exists (FR-003); refreshes an expired token, else asks to reconnect
/// (FR-024). `DryRun` writes nothing (FR-014).
#[server]
pub async fn import_harvest_api(
    mode: ImportMode,
    sync: SyncScope,
) -> Result<ImportReport, ServerFnError> {
    let admin = require_admin().await?;
    let cfg = harvest_config().await?;
    let state = crate::state::global_state().await;
    let default_currency = org_default_currency(admin.org_id).await?;

    crate::harvest_import::run_api_import(
        &state.db,
        admin.org_id,
        &default_currency,
        &cfg,
        mode,
        sync,
    )
    .await
    .map_err(map_api_error)
}

/// Run an import from an uploaded Harvest CSV (secondary source). Rejects an
/// unrecognized/empty file up front with no writes (FR-003).
#[server]
pub async fn import_harvest_csv(
    file: Vec<u8>,
    mode: ImportMode,
) -> Result<ImportReport, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let default_currency = org_default_currency(admin.org_id).await?;

    crate::harvest_import::csv_source::import_csv(
        &state.db,
        admin.org_id,
        &default_currency,
        &file,
        mode,
    )
    .await
    .map_err(|e| err(CONFLICT, e))
}

/// The configured Harvest settings, or a clear error when the importer's API
/// source is not configured on this deployment.
#[cfg(feature = "server")]
async fn harvest_config() -> Result<crate::config::HarvestConfig, ServerFnError> {
    crate::state::global_state()
        .await
        .harvest
        .clone()
        .ok_or_else(|| err(NOT_FOUND, "Harvest importer is not configured on this server"))
}

#[cfg(feature = "server")]
async fn org_default_currency(org_id: uuid::Uuid) -> Result<String, ServerFnError> {
    let state = crate::state::global_state().await;
    let currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1",
        org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;
    Ok(currency)
}

/// Map a run-level API import failure onto a named server error.
#[cfg(feature = "server")]
fn map_api_error(e: crate::harvest_import::ApiImportError) -> ServerFnError {
    use crate::harvest_import::ApiImportError;
    match e {
        ApiImportError::NotConnected => err(NOT_FOUND, e),
        ApiImportError::ReconnectRequired => err(CONFLICT, e),
        ApiImportError::Other(inner) => server_err(inner),
    }
}
