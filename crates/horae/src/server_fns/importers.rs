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
    let admin = require_admin().await?;
    let cfg = harvest_config().await?;
    let current = harvest_connection_status().await?;

    // A random, session-bound nonce validated on the callback (CSRF, FR-022).
    let nonce = uuid::Uuid::now_v7().simple().to_string();
    let session: tower_sessions::Session =
        dioxus_fullstack::FullstackContext::extract::<tower_sessions::Session, _>().await?;
    let attempt = crate::importers::harvest::OAuthAttempt {
        nonce: nonce.clone(),
        user_id: admin.id,
        org_id: admin.org_id,
        version: crate::importers::harvest::account_switch::Version {
            account_generation: current.account_generation,
            connection_revision: current.connection_revision,
        },
    };
    // The session store uses MessagePack for serde_json::Value. Keep this as
    // JSON text so arbitrary-precision JSON numbers round-trip as integers.
    let encoded = serde_json::to_string(&attempt).map_err(server_err)?;
    session
        .insert(crate::importers::harvest::OAUTH_STATE_KEY, &encoded)
        .await
        .map_err(server_err)?;

    Ok(crate::importers::harvest::oauth::authorize_url(
        &cfg, &nonce,
    ))
}

/// Report whether the org has a usable Harvest connection (never the tokens).
#[dioxus_fullstack::post("/api/import/harvest/connection")]
pub async fn harvest_connection_status() -> Result<ConnectionStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::importers::harvest::account_switch::status(
        &state.db,
        admin.org_id,
        state.harvest.is_some(),
    )
    .await
    .map_err(server_err)
}

/// Explicit confirmation of the exact connection inspected by an administrator.
#[server]
pub async fn harvest_change_account(
    expected_account: String,
    expected_generation: i64,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::importers::harvest::account_switch::change(
        &state.db,
        admin.org_id,
        &expected_account,
        expected_generation,
        expected_revision,
    )
    .await
    .map_err(map_connection_error)
}

#[cfg(feature = "server")]
fn map_connection_error(error: anyhow::Error) -> ServerFnError {
    if let Some(policy) =
        error.downcast_ref::<crate::importers::harvest::account_switch::ChangeError>()
    {
        err(CONFLICT, policy)
    } else if matches!(
        error.downcast_ref::<crate::importers::harvest::ApiImportError>(),
        Some(crate::importers::harvest::ApiImportError::Busy)
    ) {
        err(
            CONFLICT,
            "Finish or cancel the active Harvest import before changing account",
        )
    } else {
        tracing::error!(%error, "Harvest connection change failed");
        err(INTERNAL_ERROR, "Unable to change Harvest connection")
    }
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
#[dioxus_fullstack::post("/api/import/harvest/start")]
pub async fn start_harvest_api_import(
    mode: ImportMode,
    sync: SyncScope,
    generation: Option<i64>,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let payload = crate::jobs::JobPayload::HarvestApi { mode, sync };
    let key = submission_key("api").await?;
    if state.harvest.is_none() {
        return Err(err(NOT_FOUND, "Harvest is not configured"));
    }
    let id = crate::jobs::enqueue_api(
        &state.db,
        admin.org_id,
        &payload,
        &key,
        state.job_policy,
        generation.unwrap_or(0),
    )
    .await
    .map_err(map_enqueue_error)?;
    required_import_job(&state.db, admin.org_id, id).await
}

/// Return a durable import's current state for the current organization.
#[dioxus_fullstack::post("/api/import/harvest/status")]
pub async fn get_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<Option<crate::models::JobStatus>, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::jobs::status(&state.db, admin.org_id, job_id)
        .await
        .map_err(server_err)
}

#[dioxus_fullstack::post("/api/import/harvest/history")]
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
    let key = submission_key("csv").await?;
    let body = axum::body::to_bytes(file.into_body()?, 50 * 1024 * 1024)
        .await
        .map_err(|_| err(BAD_REQUEST, "CSV upload is incomplete or exceeds 50 MiB"))?;
    let state = crate::state::global_state().await;
    if !crate::jobs::request_exists(&state.db, admin.org_id, "harvest_csv_import", &key)
        .await
        .map_err(server_err)?
    {
        crate::importers::harvest::csv_source::validate_upload_headers(&body).map_err(|_| {
            err(
                BAD_REQUEST,
                "Not a recognizable Harvest CSV; check required columns",
            )
        })?;
    }
    let id = crate::jobs::enqueue_csv(
        &state.db,
        admin.org_id,
        mode,
        body.to_vec(),
        &key,
        state.job_policy,
    )
    .await
    .map_err(map_enqueue_error)?;
    required_import_job(&state.db, admin.org_id, id).await
}

#[dioxus_fullstack::post("/api/import/harvest/cancel")]
pub async fn cancel_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    crate::jobs::cancel(&state.db, admin.org_id, job_id)
        .await
        .map_err(server_err)?;
    // Completion can race cancellation; report the actual retained state.
    required_import_job(&state.db, admin.org_id, job_id).await
}

#[dioxus_fullstack::post("/api/import/harvest/retry")]
pub async fn retry_harvest_import_job(
    job_id: uuid::Uuid,
) -> Result<crate::models::JobStatus, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    if crate::jobs::retry(&state.db, admin.org_id, job_id)
        .await
        .map_err(map_enqueue_error)?
    {
        required_import_job(&state.db, admin.org_id, job_id).await
    } else {
        Err(err(NOT_FOUND, "Import job not found or is not retryable"))
    }
}

#[cfg(feature = "server")]
async fn submission_key(source: &str) -> Result<String, ServerFnError> {
    let headers = dioxus_fullstack::FullstackContext::extract::<axum::http::HeaderMap, _>().await?;
    if headers.get_all("X-Horae-Idempotency-Key").iter().count() > 1 {
        return Err(err(BAD_REQUEST, "Supply a single request ID"));
    }
    let id = match headers.get("X-Horae-Idempotency-Key") {
        None => uuid::Uuid::now_v7(),
        Some(value) => {
            let value = value
                .to_str()
                .map_err(|_| err(BAD_REQUEST, "Invalid request ID"))?;
            let id =
                uuid::Uuid::parse_str(value).map_err(|_| err(BAD_REQUEST, "Invalid request ID"))?;
            validate_request_id(id, chrono::Utc::now().timestamp())?;
            id
        }
    };
    Ok(format!("{source}:{id}"))
}

#[cfg(feature = "server")]
fn validate_request_id(id: uuid::Uuid, now: i64) -> Result<(), ServerFnError> {
    let seconds = id.get_timestamp().map(|stamp| stamp.to_unix().0);
    if id.get_version_num() != 7
        || seconds.is_none_or(|seconds| {
            i64::try_from(seconds).map_or(true, |seconds| {
                seconds < now - 86_400 || seconds > now + 300
            })
        })
    {
        return Err(err(
            BAD_REQUEST,
            "Request ID must be UUIDv7 from the past 24 hours (at most five minutes ahead)",
        ));
    }
    Ok(())
}

#[cfg(feature = "server")]
fn map_enqueue_error(error: anyhow::Error) -> ServerFnError {
    if let Some(policy) =
        error.downcast_ref::<crate::importers::harvest::account_switch::ChangeError>()
    {
        let code = if matches!(
            policy,
            crate::importers::harvest::account_switch::ChangeError::NotConnected
        ) {
            NOT_FOUND
        } else {
            CONFLICT
        };
        err(code, policy)
    } else if error.is::<crate::jobs::RequestConflict>() {
        err(
            CONFLICT,
            "Request ID already belongs to different import input",
        )
    } else {
        tracing::error!(%error, "durable import submission failed");
        err(INTERNAL_ERROR, "Unable to submit import")
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

    #[test]
    fn submission_identity_rejects_expired_future_and_non_v7_keys() {
        let id = uuid::Uuid::now_v7();
        let now = chrono::Utc::now().timestamp();
        assert!(validate_request_id(id, now).is_ok());
        assert!(validate_request_id(id, now + 86_401).is_err());
        assert!(validate_request_id(id, now - 301).is_err());
        // Even a key accepted at the maximum future skew is long expired when
        // its retained job can be removed by the fixed 30-day cleanup policy.
        assert!(validate_request_id(id, now - 300).is_ok());
        assert!(validate_request_id(id, now - 300 + 30 * 86_400).is_err());
        assert!(validate_request_id(uuid::Uuid::nil(), now).is_err());
    }

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
