//! `GET /auth/harvest/callback`, where Harvest sends the browser back after an
//! administrator authorizes the importer.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use horae_core::importers::harvest::client::ConnectFailure;

use crate::importers::harvest::{
    ApiImportError, OAuthAttempt, account_switch, complete_connect, credentials, oauth,
};

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
) -> Response {
    // The nonce is single-use: consume it regardless of outcome.
    // Legacy nonce-only sessions deliberately fail closed after deployment.
    let stored = session
        .remove::<String>(OAUTH_STATE_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|encoded| serde_json::from_str::<OAuthAttempt>(&encoded).ok());

    let dest_ok = "/admin/importers/harvest?connected=1";
    let dest_err = |failure: ConnectFailure| {
        Redirect::to(&format!("/admin/importers/harvest?{}", failure.query())).into_response()
    };

    // Validate `state` and extract the code; a missing/mismatched state or error
    // yields no code, so we never reach the exchange (research.md §10).
    let code = match authorized_code(
        &params,
        stored.as_ref().map(|attempt| attempt.nonce.as_str()),
    ) {
        Ok(code) => code,
        Err((failure, reason)) => {
            tracing::warn!("Harvest callback rejected before exchange: {reason}");
            return dest_err(failure);
        }
    };

    let Some(attempt) = stored else {
        return dest_err(ConnectFailure::State);
    };
    // Only an authenticated admin may land a connection on their org.
    let connected = match crate::auth::session::get_session_user_id(&session).await {
        Ok(Some(user_id)) => complete_connect(user_id, code, &attempt).await,
        Ok(None) => Err(anyhow::anyhow!("no authenticated session")),
        Err(error) => Err(error.into()),
    };
    match connected {
        Ok(()) => Redirect::to(dest_ok).into_response(),
        Err(e) => {
            if let Some(response) = connection_policy_response(&e) {
                return response;
            }
            tracing::error!("Harvest connect failed: {e}");
            dest_err(connect_failure(&e))
        }
    }
}

/// What the Harvest page is told about a failed exchange or account lookup.
/// Harvest documents no token errors, and RFC 6749 §5.2 gives a code issued
/// for another redirect URL the same `invalid_grant` as an expired one, so the
/// code-expired message also names the redirect URL.
fn connect_failure(error: &anyhow::Error) -> ConnectFailure {
    if let Some(unavailable) = error.downcast_ref::<oauth::TokenUnavailable>() {
        return ConnectFailure::Other {
            status: Some(unavailable.status),
        };
    }
    match error.downcast_ref::<oauth::TokenRefused>() {
        Some(refused) => match refused.code {
            oauth::OAuthError::InvalidClient => ConnectFailure::InvalidClient,
            oauth::OAuthError::InvalidGrant => ConnectFailure::CodeExpired,
            _ => ConnectFailure::Other {
                status: Some(refused.status),
            },
        },
        None => ConnectFailure::Other { status: None },
    }
}

/// Only known, secret-free policy errors may be returned to the browser.
fn connection_policy_response(error: &anyhow::Error) -> Option<Response> {
    if let Some(credentials::ConnectionError::Unauthorized) = error.downcast_ref() {
        return Some(
            (
                StatusCode::FORBIDDEN,
                credentials::ConnectionError::Unauthorized.to_string(),
            )
                .into_response(),
        );
    }
    let message = if let Some(policy) = error.downcast_ref::<credentials::ConnectionError>() {
        policy.to_string()
    } else if let Some(policy) = error.downcast_ref::<account_switch::ChangeError>() {
        policy.to_string()
    } else if matches!(
        error.downcast_ref::<ApiImportError>(),
        Some(ApiImportError::Busy)
    ) {
        ApiImportError::Busy.to_string()
    } else {
        return None;
    };
    Some((StatusCode::CONFLICT, message).into_response())
}

/// Decide whether a callback may proceed to the token exchange. Returns the
/// authorization code only when the provider reported no error, a `state` was
/// stored for this session, and the returned `state` matches it exactly. A
/// missing or mismatched `state` is rejected **without** the code (CSRF, FR-022).
/// A rejection carries what the page is told and what the log says.
fn authorized_code(
    params: &CallbackParams,
    stored_state: Option<&str>,
) -> Result<String, (ConnectFailure, &'static str)> {
    // State first, errors included (RFC 6749 §4.1.2.1 returns it with them):
    // only this session's own attempt may say how it ended.
    let state = |reason| (ConnectFailure::State, reason);
    let stored = stored_state.ok_or(state("no state stored for this session"))?;
    let returned = params
        .state
        .as_deref()
        .ok_or(state("callback missing state"))?;
    if returned != stored {
        return Err(state("state mismatch (possible CSRF)"));
    }
    // Only RFC 6749 §4.1.2.1 codes are matched; anything else Harvest sends
    // stays out of the log and the redirect.
    match params.error.as_deref() {
        None => {}
        Some("access_denied") => return Err((ConnectFailure::Denied, "admin denied access")),
        Some("invalid_redirect_uri" | "redirect_uri_mismatch") => {
            return Err((
                ConnectFailure::RedirectMismatch,
                "provider rejected the redirect URL",
            ));
        }
        Some(_) => {
            return Err((
                ConnectFailure::Other { status: None },
                "provider returned an error",
            ));
        }
    }
    params.code.clone().ok_or((
        ConnectFailure::Other { status: None },
        "callback missing code",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn account_policy_failures_return_safe_actionable_conflicts() {
        for error in [
            anyhow::Error::from(credentials::ConnectionError::AccountChange),
            anyhow::Error::from(credentials::ConnectionError::UnidentifiedProvenance),
            anyhow::Error::from(ApiImportError::Busy),
        ] {
            let response = connection_policy_response(&error).unwrap();
            assert_eq!(response.status(), StatusCode::CONFLICT);
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), error.to_string().as_bytes());
        }
        assert!(connection_policy_response(&anyhow::anyhow!("secret upstream payload")).is_none());
    }

    #[tokio::test]
    async fn revoked_callback_authority_returns_only_a_safe_forbidden_message() {
        let error = anyhow::Error::from(credentials::ConnectionError::Unauthorized)
            .context("private callback context must not be returned");
        let response = connection_policy_response(&error).unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(
            body.as_ref(),
            credentials::ConnectionError::Unauthorized
                .to_string()
                .as_bytes()
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
        assert_eq!(rejection(&p, Some("nonce")), ConnectFailure::State);
    }

    fn rejection(p: &CallbackParams, stored: Option<&str>) -> ConnectFailure {
        authorized_code(p, stored).unwrap_err().0
    }

    #[test]
    fn missing_stored_state_is_rejected() {
        let p = params(None, Some("nonce"), Some("the-code"));
        assert_eq!(rejection(&p, None), ConnectFailure::State);
    }

    #[test]
    fn missing_returned_state_is_rejected() {
        let p = params(None, None, Some("the-code"));
        assert_eq!(rejection(&p, Some("nonce")), ConnectFailure::State);
    }

    #[test]
    fn provider_errors_are_rejected_before_exchange() {
        for (error, failure) in [
            ("access_denied", ConnectFailure::Denied),
            ("invalid_redirect_uri", ConnectFailure::RedirectMismatch),
            ("redirect_uri_mismatch", ConnectFailure::RedirectMismatch),
            ("server_error", ConnectFailure::Other { status: None }),
            ("<private>", ConnectFailure::Other { status: None }),
        ] {
            let p = params(Some(error), Some("nonce"), Some("the-code"));
            assert_eq!(rejection(&p, Some("nonce")), failure, "{error}");
        }
    }

    #[test]
    fn a_provider_error_without_this_sessions_state_is_a_state_failure() {
        for (returned, stored) in [
            (None, Some("nonce")),
            (Some("attacker"), Some("nonce")),
            (Some("nonce"), None),
        ] {
            let p = params(Some("access_denied"), returned, None);
            assert_eq!(
                rejection(&p, stored),
                ConnectFailure::State,
                "{returned:?} {stored:?}"
            );
        }
    }

    #[test]
    fn valid_state_but_missing_code_is_rejected() {
        let p = params(None, Some("nonce"), None);
        assert_eq!(
            rejection(&p, Some("nonce")),
            ConnectFailure::Other { status: None }
        );
    }

    #[test]
    fn a_refused_exchange_tells_the_page_why() {
        use oauth::{OAuthError, TokenRefused};
        let refused = |code, status| anyhow::Error::from(TokenRefused { code, status });
        for (error, failure) in [
            (
                refused(OAuthError::InvalidClient, 401),
                ConnectFailure::InvalidClient,
            ),
            (
                refused(OAuthError::InvalidGrant, 400),
                ConnectFailure::CodeExpired,
            ),
            (
                refused(OAuthError::InvalidRequest, 400),
                ConnectFailure::Other { status: Some(400) },
            ),
            (
                refused(OAuthError::Other, 403),
                ConnectFailure::Other { status: Some(403) },
            ),
            (
                anyhow::Error::from(oauth::TokenUnavailable { status: 503 }),
                ConnectFailure::Other { status: Some(503) },
            ),
            (
                anyhow::anyhow!("Harvest accounts lookup failed"),
                ConnectFailure::Other { status: None },
            ),
        ] {
            assert_eq!(connect_failure(&error), failure, "{error}");
        }
    }
}
