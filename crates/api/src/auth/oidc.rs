//! Production OIDC authentication (authorization-code flow with PKCE).
//!
//! `GET /auth/login` serves the sign-in landing page; its button hands off to
//! `GET /auth/oidc/start`, which redirects to the provider. `GET /auth/callback`
//! completes the exchange, verifies the ID token, and maps the verified identity
//! onto a Horae user ([`horae_server::auth::oidc::provisioning`]). A sign-in refused
//! because of the account leaves a one-time notice that `/auth/login` shows.

use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Redirect};
use horae_server::auth::oidc::provisioning::{self, LoginNotice};
use horae_server::auth::oidc::{Identity, build_authorization, exchange_and_verify};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use super::page::{LoginAssets, LoginVariant, render};
use super::session::set_session_user_id;
use crate::config::AutoProvision;

/// Session key holding the in-flight authorization request's CSRF, PKCE, and
/// nonce secrets between `/auth/login` and `/auth/callback`.
const OIDC_FLOW_KEY: &str = "oidc_flow";

/// Session key holding the [`LoginNotice`] of a refused sign-in until
/// `/auth/login` shows it. Kept server-side so the email never enters a URL.
const LOGIN_NOTICE_KEY: &str = "login_notice";

/// The one-time secrets that tie a callback back to the login that started it.
#[derive(Serialize, Deserialize)]
struct OidcFlow {
    csrf: String,
    pkce_verifier: String,
    nonce: String,
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// `GET /auth/login` for OIDC deployments: serve the sign-in landing page. The
/// page's button links to `/auth/oidc/start`, which begins the flow. A notice
/// left by a refused sign-in is shown once.
pub async fn login_page(State(assets): State<LoginAssets>, session: Session) -> impl IntoResponse {
    let label = crate::state::global_state()
        .await
        .oidc
        .as_ref()
        .map(|c| c.button_label.clone())
        .unwrap_or_else(|| crate::config::DEFAULT_OIDC_BUTTON_LABEL.into());
    let notice = session
        .remove::<LoginNotice>(LOGIN_NOTICE_KEY)
        .await
        .ok()
        .flatten();
    Html(render(
        LoginVariant::Oidc { cta_label: label },
        notice.as_ref(),
        &assets,
    ))
}

/// `GET /auth/oidc/start`: begin the authorization-code flow. Stashes the
/// CSRF/PKCE/nonce secrets in the session and redirects to the provider. Falls
/// back to `/auth/login` on any setup error.
pub async fn start(session: Session) -> axum::response::Response {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        // Neither dev-login nor OIDC configured. Return a plain error rather than
        // redirecting back to `/auth/login`, which would loop.
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication is not configured.",
        )
            .into_response();
    };

    let build = tokio::task::spawn_blocking(move || build_authorization(&cfg)).await;
    let auth = match build {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            tracing::error!("OIDC login setup failed: {e}");
            return Redirect::to("/auth/login").into_response();
        }
        Err(e) => {
            tracing::error!("OIDC login task panicked: {e}");
            return Redirect::to("/auth/login").into_response();
        }
    };

    let flow = OidcFlow {
        csrf: auth.csrf,
        pkce_verifier: auth.pkce_verifier,
        nonce: auth.nonce,
    };
    if let Err(e) = session.insert(OIDC_FLOW_KEY, flow).await {
        tracing::error!("failed to store OIDC flow in session: {e}");
        return Redirect::to("/auth/login").into_response();
    }

    Redirect::to(&auth.url).into_response()
}

/// `GET /auth/callback`: validate state, exchange the code (PKCE), verify the
/// ID token and nonce, then resolve the identity onto a Horae user.
pub async fn callback(session: Session, Query(params): Query<CallbackParams>) -> impl IntoResponse {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        return Redirect::to("/auth/login");
    };

    // The flow secrets are single-use: consume them regardless of outcome.
    let flow: Option<OidcFlow> = session.get(OIDC_FLOW_KEY).await.ok().flatten();
    let _ = session.remove::<OidcFlow>(OIDC_FLOW_KEY).await;
    let Some(flow) = flow else {
        tracing::warn!("OIDC callback with no in-flight flow");
        return Redirect::to("/auth/login");
    };

    if params.error.is_some() {
        tracing::warn!("OIDC provider returned error: {:?}", params.error);
        return Redirect::to("/auth/login");
    }

    // CSRF: the returned `state` must equal the value we generated at login.
    let (Some(returned_state), Some(code)) = (params.state, params.code) else {
        tracing::warn!("OIDC callback missing state or code");
        return Redirect::to("/auth/login");
    };
    if returned_state != flow.csrf {
        tracing::warn!("OIDC callback state mismatch (possible CSRF)");
        return Redirect::to("/auth/login");
    }

    let policy = cfg.auto_provision.clone();
    // Exchange the code and verify the ID token off the async runtime (blocking).
    let identity = tokio::task::spawn_blocking(move || {
        exchange_and_verify(&cfg, code, &flow.pkce_verifier, &flow.nonce)
    })
    .await;
    let identity = match identity {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            tracing::warn!("OIDC token exchange/verification failed: {e}");
            return Redirect::to("/auth/login");
        }
        Err(e) => {
            tracing::error!("OIDC callback task panicked: {e}");
            return Redirect::to("/auth/login");
        }
    };

    match resolve_and_login(state, &session, &identity, &policy).await {
        Ok(true) => Redirect::to("/"),
        Ok(false) => Redirect::to("/auth/login"),
        Err(e) => {
            tracing::error!("OIDC user resolution failed: {e}");
            Redirect::to("/auth/login")
        }
    }
}

/// Resolve the identity before creating a session or dispatching events. A
/// refusal stores its notice for `/auth/login`.
async fn resolve_and_login(
    state: &'static crate::state::AppState,
    session: &Session,
    identity: &Identity,
    policy: &AutoProvision,
) -> anyhow::Result<bool> {
    let sign_in = match provisioning::resolve_user(&state.db, identity, policy).await? {
        Ok(sign_in) => sign_in,
        Err(notice) => {
            session.insert(LOGIN_NOTICE_KEY, notice).await?;
            return Ok(false);
        }
    };

    // Rotate the session id on the privilege change to defeat session fixation:
    // any pre-auth id an attacker may have fixed is discarded before we mark the
    // session authenticated.
    session.cycle_id().await?;
    set_session_user_id(session, sign_in.user.id).await?;

    provisioning::announce_sign_in(state, &sign_in).await?;
    Ok(true)
}
