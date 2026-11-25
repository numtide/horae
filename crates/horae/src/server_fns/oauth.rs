//! The OAuth approval page's server functions: session adapters over
//! `ops::oauth::authorize` (spec 024, contracts/server-functions.md). Each
//! reads the pending request from the session that started it; Allow and
//! Deny remove it, so a second click finds nothing.

use super::*;
use crate::models::oauth::ConsentDetails;
#[cfg(feature = "server")]
use crate::ops::{self, Caller};
#[cfg(feature = "server")]
use tower_sessions::Session;

/// What the approval page shows about request `request`.
#[server]
pub async fn oauth_consent_details(request: String) -> Result<ConsentDetails, ServerFnError> {
    let caller = session_caller().await?;
    let session = session().await?;
    details(
        &crate::state::global_state().await.db,
        &session,
        caller,
        &request,
    )
    .await
}

/// Allow: the address, carrying the code, to send the browser to.
#[server]
pub async fn approve_oauth_request(request: String) -> Result<String, ServerFnError> {
    let caller = session_caller().await?;
    let session = session().await?;
    let state = crate::state::global_state().await;
    let issuer = state.public_url.as_ref().map(|url| url.as_str());
    approve(&state.db, &session, caller, &request, issuer, &state.oauth).await
}

/// Deny: the address that tells the app the person said no.
#[server]
pub async fn deny_oauth_request(request: String) -> Result<String, ServerFnError> {
    session_caller().await?;
    let session = session().await?;
    let state = crate::state::global_state().await;
    let issuer = state.public_url.as_ref().map(|url| url.as_str());
    deny(&session, &request, issuer).await
}

#[cfg(feature = "server")]
const EXPIRED: &str = "This request has expired. Go back to the app and start again.";

#[cfg(feature = "server")]
async fn session() -> Result<Session, ServerFnError> {
    dioxus_fullstack::FullstackContext::extract::<Session, _>().await
}

#[cfg(feature = "server")]
async fn details(
    db: &sqlx::PgPool,
    session: &Session,
    caller: Caller,
    request_id: &str,
) -> Result<ConsentDetails, ServerFnError> {
    let pending = horae_api::oauth::pending::get(session, request_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| not_found(EXPIRED))?;
    let mut read = ops::begin_read(db, caller).await.map_err(op_error)?;
    let result = ops::oauth::authorize::details(&mut read.tx, &read.actor, &pending).await;
    let consent = read.finish(result).await.map_err(op_error)?;
    Ok(ConsentDetails {
        request_id: request_id.to_owned(),
        app_name: consent.app_name,
        built_in: consent.built_in,
        scopes: consent
            .scopes
            .names()
            .into_iter()
            .map(str::to_owned)
            .collect(),
        return_to_display: consent.return_to,
        person_name: consent.person_name,
        organization_name: consent.organization_name,
    })
}

// Approvals raise no plugin events, so nothing is dispatched after the commit.
#[cfg(feature = "server")]
async fn approve(
    db: &sqlx::PgPool,
    session: &Session,
    caller: Caller,
    request_id: &str,
    issuer: Option<&str>,
    config: &crate::config::OAuthConfig,
) -> Result<String, ServerFnError> {
    let issuer = issuer.ok_or_else(|| not_found(EXPIRED))?;
    let pending = horae_api::oauth::pending::take(session, request_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| not_found(EXPIRED))?;
    let mut write = ops::begin(db, caller).await.map_err(op_error)?;
    let result =
        ops::oauth::authorize::approve(&mut write.tx, &write.actor, &pending, issuer, config).await;
    write.commit(result, &ops::NoEvents).await.map_err(op_error)
}

#[cfg(feature = "server")]
async fn deny(
    session: &Session,
    request_id: &str,
    issuer: Option<&str>,
) -> Result<String, ServerFnError> {
    let issuer = issuer.ok_or_else(|| not_found(EXPIRED))?;
    let pending = horae_api::oauth::pending::take(session, request_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| not_found(EXPIRED))?;
    Ok(ops::oauth::authorize::deny(&pending, issuer))
}

#[cfg(all(test, feature = "server"))]
mod tests;
