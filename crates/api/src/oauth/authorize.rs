//! `GET /oauth/authorize`: checks the request, keeps it in the session and
//! sends the browser to the approval page (contracts/oauth-endpoints.md).

use axum::extract::{RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use horae_i18n::Tr;
use tower_sessions::Session;

use super::{OAuthState, no_store, pairs, pending};
use crate::auth::page::escape_html;
use crate::ops::oauth::authorize::{self, AuthorizeError, PageError};

pub async fn authorize(
    State(state): State<OAuthState>,
    session: Session,
    RawQuery(query): RawQuery,
) -> Response {
    let query = pairs(query.unwrap_or_default().as_bytes());
    let mut response = match authorize::validate_request(&state.pool, &query).await {
        Ok(request) => match pending::store(&session, request).await {
            Ok(id) => found(&format!("/oauth/consent?request={id}")),
            Err(error) => {
                tracing::error!(%error, "Could not keep an authorization request");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(AuthorizeError::Redirect(error)) => found(&authorize::error_url(&error, &state.issuer)),
        Err(AuthorizeError::Page(error)) => {
            (StatusCode::BAD_REQUEST, Html(error_page(error))).into_response()
        }
        Err(AuthorizeError::Storage(error)) => {
            tracing::error!(%error, "Could not check an authorization request");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    };
    no_store(&mut response);
    response
}

/// A 302, the status RFC 6749's examples use for authorization responses.
fn found(location: &str) -> Response {
    match HeaderValue::from_str(location) {
        Ok(location) => (StatusCode::FOUND, [(header::LOCATION, location)]).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Horae's own page for a request it cannot send back to the app (FR-022).
fn error_page(error: PageError) -> String {
    let tr = Tr::default();
    let reason = match error {
        PageError::MissingClient => tr.oauth_error_missing_client(),
        PageError::UnknownClient => tr.oauth_error_unknown_client(),
        PageError::DisabledClient => tr.oauth_error_disabled_client(),
        PageError::MissingRedirectUri => tr.oauth_error_missing_redirect(),
        PageError::UnregisteredRedirectUri => tr.oauth_error_unregistered_redirect(),
    };
    ERROR_PAGE
        .replace("<!--LANG-->", tr.lang().tag())
        .replace("<!--TITLE-->", &escape_html(&tr.oauth_error_title()))
        .replace("<!--REASON-->", &escape_html(&reason))
        .replace("<!--HINT-->", &escape_html(&tr.oauth_error_hint()))
}

/// Served outside the web app like `/auth/login`, so it carries its own
/// styles: that page's tokens, in one card.
static ERROR_PAGE: &str = r#"<!DOCTYPE html>
<html lang="<!--LANG-->">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title><!--TITLE--></title>
  <link rel="icon" href="/favicon.ico" sizes="32x32">
  <style>
    :root {
      --ink: #100F0C; --panel: #1A1814; --text: #EFEAE0; --heading: #F6F2E9;
      --muted: #A29C8D; --border: #322E26; --gold: #D99A3C;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      min-height: 100vh; display: flex; align-items: center; justify-content: center;
      padding: 32px 16px; background: var(--ink); color: var(--text);
      font-family: 'Instrument Sans', system-ui, sans-serif;
    }
    main {
      width: 100%; max-width: 440px; padding: 32px; border-radius: 12px;
      background: var(--panel); border: 1px solid var(--border);
    }
    h1 {
      font-family: 'Newsreader', Georgia, serif; font-size: 26px; font-weight: 600;
      color: var(--heading); margin-bottom: 16px;
    }
    .reason {
      padding: 12px 14px; border-radius: 9px; font-size: 15px; line-height: 1.5;
      background: rgba(217, 154, 60, 0.12); border: 1px solid rgba(217, 154, 60, 0.35);
    }
    .hint { margin-top: 16px; font-size: 14px; line-height: 1.5; color: var(--muted); }
  </style>
</head>
<body>
  <main>
    <h1><!--TITLE--></h1>
    <p class="reason" role="alert"><!--REASON--></p>
    <p class="hint"><!--HINT--></p>
  </main>
</body>
</html>"#;

#[cfg(test)]
mod tests;
