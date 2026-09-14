//! Admin-only server functions for the Harvest importer (contracts/importer-api.md).
//!
//! These are the SPA's entry points: start the OAuth connect, read connection
//! status, and enqueue durable API or CSV imports. All reject
//! non-admins with `FORBIDDEN` (FR-001) and use named status codes.

use super::*;
use horae_core::importers::harvest::types::{ConnectionStatus, ImportMode, SyncScope};

mod csv_upload;
pub use csv_upload::CsvUpload;

#[cfg(all(test, feature = "server"))]
mod authorization_tests;

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
        .insert(crate::importers::harvest::OAUTH_STATE_KEY, &nonce)
        .await
        .map_err(server_err)?;

    Ok(crate::importers::harvest::oauth::authorize_url(
        &cfg, &nonce,
    ))
}

/// Report whether the org has a usable Harvest connection (never the tokens).
#[server]
pub async fn harvest_connection_status() -> Result<ConnectionStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let configured = state.harvest.is_some();

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
            configured,
            connected: true,
            account_id: Some(r.harvest_account_id),
            token_expired: r.token_expires_at.is_some_and(|e| e <= chrono::Utc::now()),
        },
        None => ConnectionStatus {
            configured,
            ..ConnectionStatus::default()
        },
    })
}

/// Disconnect Harvest: remove OAuth secrets, retaining the original account
/// binding for a safe reconnect (contracts/importer-api.md). Admin-only (FR-001).
#[server]
pub async fn harvest_disconnect() -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;

    crate::importers::harvest::credentials::disconnect(&state.db, admin.org_id)
        .await
        .map_err(map_api_error)
}

/// Enqueue an asynchronous Harvest API import and return its ID and current state.
/// The organization is derived from the authenticated administrator's session.
#[server]
pub async fn start_harvest_api_import(
    mode: ImportMode,
    sync: SyncScope,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let payload = crate::jobs::JobPayload::HarvestApi { mode, sync };
    let key = format!("api:{}", uuid::Uuid::now_v7());
    let id = crate::jobs::enqueue(&state.db, admin.org_id, &payload, &key, state.job_policy)
        .await
        .map_err(server_err)?;
    required_import_job(&state.db, admin.org_id, id).await
}

/// Return a durable import's current state for the current organization.
#[server]
pub async fn get_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<Option<crate::models::JobStatus>, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::jobs::status(&state.db, admin.org_id, job_id)
        .await
        .map_err(server_err)
}

#[server]
pub async fn list_harvest_import_jobs(
    before: Option<uuid::Uuid>,
    limit: Option<i64>,
) -> Result<Vec<crate::models::JobStatus>, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::jobs::list(&state.db, admin.org_id, limit.unwrap_or(20), before)
        .await
        .map_err(server_err)
}

/// Buffer and enqueue a CSV import so the request body is not tied to the
/// lifetime of the background worker.
#[dioxus_fullstack::post("/api/import/harvest/csv-job/{mode}")]
pub async fn start_harvest_csv_import(
    mode: ImportMode,
    file: CsvUpload,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let body = axum::body::to_bytes(file.into_body()?, 50 * 1024 * 1024)
        .await
        .map_err(server_err)?;
    let state = crate::state::global_state().await;
    // Uploads have no stable source identifier; never deduplicate unrelated
    // files merely because their byte lengths happen to match.
    let key = format!("csv:{}", uuid::Uuid::now_v7());
    let id = crate::jobs::enqueue_csv(
        &state.db,
        admin.org_id,
        mode,
        body.to_vec(),
        &key,
        state.job_policy,
    )
    .await
    .map_err(server_err)?;
    required_import_job(&state.db, admin.org_id, id).await
}

#[server]
pub async fn cancel_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    if crate::jobs::cancel(&state.db, admin.org_id, job_id)
        .await
        .map_err(server_err)?
    {
        required_import_job(&state.db, admin.org_id, job_id).await
    } else {
        Err(err(
            NOT_FOUND,
            "Import job not found or is already complete",
        ))
    }
}

#[server]
pub async fn retry_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    if crate::jobs::retry(&state.db, admin.org_id, job_id)
        .await
        .map_err(server_err)?
    {
        required_import_job(&state.db, admin.org_id, job_id).await
    } else {
        Err(err(NOT_FOUND, "Import job not found or is not retryable"))
    }
}

#[cfg(feature = "server")]
async fn required_import_job(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    job_id: uuid::Uuid,
) -> Result<crate::models::JobStatus, ServerFnError> {
    crate::jobs::status(pool, org_id, job_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| err(NOT_FOUND, "Import job not found"))
}

/// The configured Harvest settings, or a clear error when the importer's API
/// source is not configured on this deployment.
#[cfg(feature = "server")]
async fn harvest_config() -> Result<crate::config::HarvestConfig, ServerFnError> {
    crate::state::global_state()
        .await
        .harvest
        .clone()
        .ok_or_else(|| {
            err(
                NOT_FOUND,
                "Harvest importer is not configured on this server",
            )
        })
}

/// Map a run-level API import failure onto a named server error.
#[cfg(feature = "server")]
fn map_api_error(e: crate::importers::harvest::ApiImportError) -> ServerFnError {
    use crate::importers::harvest::ApiImportError;
    match e {
        ApiImportError::NotConnected => err(NOT_FOUND, e),
        ApiImportError::ReconnectRequired | ApiImportError::Busy => err(CONFLICT, e),
        ApiImportError::Other(inner) => server_err(inner),
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn csv_route_rejects_invalid_modes_and_headers_without_reading_uploads() {
        use axum::{Router, body::Body, extract::Request, middleware};
        use dioxus::prelude::{DioxusRouterExt, dioxus_server::FullstackState};
        let router = Router::new()
            .register_server_functions()
            .with_state(FullstackState::headless())
            .layer(middleware::from_fn(
                |request: Request, next: middleware::Next| async move {
                    let request = request.map(|_| {
                        Body::from_stream(futures_util::stream::poll_fn(
                            |_| -> std::task::Poll<
                                Option<Result<axum::body::Bytes, std::io::Error>>,
                            > {
                                panic!("rejected CSV request body was read");
                            },
                        ))
                    });
                    next.run(request).await
                },
            ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut server = tokio::task::JoinSet::new();
        server.spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = dioxus_fullstack::ClientRequest::new_reqwest_client();
        for route in ["csv", "csv-job"] {
            for (mode, header, expected) in [
                ("invalid", "csv", BAD_REQUEST),
                ("dryrun", "csv", BAD_REQUEST),
                ("DryRun", "wrong", FORBIDDEN),
                ("Commit", "wrong", FORBIDDEN),
                ("DryRun/Commit", "csv", NOT_FOUND),
            ] {
                let response = client
                    .post(format!(
                        "http://{address}/api/import/harvest/{route}/{mode}"
                    ))
                    .header("X-Horae-Import", header)
                    .body("not CSV")
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .unwrap();
                let expected = if route == "csv" { NOT_FOUND } else { expected };
                assert_eq!(response.status().as_u16(), expected, "{route}/{mode}");
            }
        }
        server.abort_all();
        while server.join_next().await.is_some() {}
    }

    #[test]
    fn concurrent_api_import_returns_a_retryable_conflict() {
        let error = map_api_error(crate::importers::harvest::ApiImportError::Busy);
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
    }
}
