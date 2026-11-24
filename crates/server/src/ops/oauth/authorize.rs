//! The authorization request (RFC 6749 §4.1, RFC 7636): checking what an app
//! asks for, and the person's Allow or Deny on the approval page.
//!
//! Errors split in two (FR-022). While Horae cannot trust the client or its
//! redirect URI, the error is shown on Horae's own page; once both match a
//! registration, every other error goes back to the app's redirect URI with
//! `error`, `state` and `iss`.

use chrono::{DateTime, Utc};
use horae_core::permissions::oauth::{RedirectUri, ScopeSet, pkce};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::audit::{self, Action};
use super::{CODE_PREFIX, Secret};
use crate::config::OAuthConfig;
use crate::ops::{Actor, Done, OpError, Tx};

/// The longest `state` echoed back to an app.
const MAX_STATE: usize = 1024;
/// How long a code can be exchanged (FR-030).
const CODE_TTL_SECONDS: i32 = 60;

const REQUEST_NOT_FOUND: &str = "This request has expired";

/// A checked authorization request, waiting in the person's session for
/// Allow or Deny. It holds no credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRequest {
    /// The app's row id.
    pub app_id: Uuid,
    pub client_id: String,
    /// Exactly as the app sent it; the code is bound to this string.
    pub redirect_uri: String,
    /// The scopes asked for, space-separated and normalized.
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: String,
    pub created_at: DateTime<Utc>,
}

impl PendingRequest {
    fn scopes(&self) -> Result<ScopeSet, OpError> {
        ScopeSet::parse_list(&self.scope).map_err(|_| OpError::not_found(REQUEST_NOT_FOUND))
    }
}

/// Why Horae shows an error on its own page instead of redirecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    MissingClient,
    UnknownClient,
    DisabledClient,
    MissingRedirectUri,
    UnregisteredRedirectUri,
}

/// An error returned to the app at its redirect URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectError {
    pub redirect_uri: String,
    /// `invalid_request`, `unsupported_response_type` or `invalid_scope`.
    pub error: &'static str,
    pub description: &'static str,
    pub state: Option<String>,
}

#[derive(Debug)]
pub enum AuthorizeError {
    Page(PageError),
    Redirect(RedirectError),
    Storage(sqlx::Error),
}

impl From<sqlx::Error> for AuthorizeError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error)
    }
}

/// Checks an authorization request's query parameters, in the order of
/// contracts/oauth-endpoints.md. Reads only.
///
/// A parameter sent twice is invalid (RFC 6749 §3.1); a repeated
/// `client_id` or `redirect_uri` cannot be trusted, so it reads as missing.
pub async fn validate_request(
    pool: &PgPool,
    query: &[(String, String)],
) -> Result<PendingRequest, AuthorizeError> {
    // A parameter sent once; `None` when it is missing or repeated.
    let param = |name: &str| {
        let mut values = query.iter().filter(|(key, _)| key == name);
        match (values.next(), values.next()) {
            (Some((_, value)), None) => Some(value.as_str()),
            _ => None,
        }
    };
    let repeated = query
        .iter()
        .enumerate()
        .any(|(i, (key, _))| query[..i].iter().any(|(earlier, _)| earlier == key));
    let client_id = param("client_id").ok_or(AuthorizeError::Page(PageError::MissingClient))?;
    let client = find_client(pool, client_id).await?;
    if !client.enabled {
        return Err(AuthorizeError::Page(PageError::DisabledClient));
    }
    let redirect_uri =
        param("redirect_uri").ok_or(AuthorizeError::Page(PageError::MissingRedirectUri))?;
    let registered = client
        .redirect_uris
        .iter()
        .any(|uri| RedirectUri::parse(uri, client.kind).is_ok_and(|uri| uri.matches(redirect_uri)));
    if !registered {
        return Err(AuthorizeError::Page(PageError::UnregisteredRedirectUri));
    }

    // From here on, errors go back to the app.
    // A state too long to echo is left out of the error that says so.
    let state = param("state").filter(|state| state.len() <= MAX_STATE);
    let refuse = |error: &'static str, description: &'static str| {
        AuthorizeError::Redirect(RedirectError {
            redirect_uri: redirect_uri.to_owned(),
            error,
            description,
            state: state.map(str::to_owned),
        })
    };
    let invalid = |description| refuse("invalid_request", description);
    let required = |name: &str, description| param(name).ok_or_else(|| invalid(description));

    if repeated {
        return Err(invalid("A parameter was sent more than once"));
    }
    if param("state").is_some_and(|state| state.len() > MAX_STATE) {
        return Err(invalid("state is longer than 1024 characters"));
    }
    match required("response_type", "response_type is required")? {
        "code" => {}
        _ => {
            return Err(refuse(
                "unsupported_response_type",
                "Only response_type=code is supported",
            ));
        }
    }
    let challenge = required("code_challenge", "A PKCE code_challenge is required")?;
    match required(
        "code_challenge_method",
        "code_challenge_method=S256 is required",
    )? {
        "S256" => {}
        _ => return Err(invalid("Only code_challenge_method=S256 is supported")),
    }
    if !pkce::well_formed_challenge(challenge) {
        return Err(invalid("code_challenge is not an S256 challenge"));
    }
    let scopes = match param("scope").filter(|scope| !scope.is_empty()) {
        None => client.scopes,
        Some(scope) => ScopeSet::parse_list(scope)
            .ok()
            .filter(|scopes| !scopes.is_empty())
            .map(ScopeSet::normalize)
            .filter(|scopes| scopes.is_subset(client.scopes))
            .ok_or_else(|| {
                refuse(
                    "invalid_scope",
                    "The app is not registered for a requested scope",
                )
            })?,
    };

    Ok(PendingRequest {
        app_id: client.id,
        client_id: client_id.to_owned(),
        redirect_uri: redirect_uri.to_owned(),
        scope: scopes.display(),
        state: state.map(str::to_owned),
        code_challenge: challenge.to_owned(),
        created_at: Utc::now(),
    })
}

/// What the approval page shows about a pending request.
#[derive(Debug, Clone, PartialEq)]
pub struct Consent {
    pub app_name: String,
    pub built_in: bool,
    pub scopes: ScopeSet,
    /// The redirect URI's host, or its scheme for an app scheme.
    pub return_to: String,
    pub person_name: String,
    pub organization_name: String,
}

/// The approval page's details. A request whose app was disabled or removed
/// since reads as not found.
pub async fn details(
    tx: &mut Tx,
    actor: &Actor,
    pending: &PendingRequest,
) -> Result<Consent, OpError> {
    actor.require_session()?;
    let row = sqlx::query!(
        "SELECT c.name, c.built_in, u.name AS person_name, o.name AS organization_name
         FROM oauth_clients c
         JOIN users u ON u.org_id = c.org_id AND u.id = $3
         JOIN organizations o ON o.id = c.org_id
         WHERE c.id = $1 AND c.org_id = $2 AND c.enabled",
        pending.app_id,
        actor.org_id(),
        actor.id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(REQUEST_NOT_FOUND))?;
    Ok(Consent {
        app_name: row.name,
        built_in: row.built_in,
        scopes: pending.scopes()?,
        return_to: return_to_display(&pending.redirect_uri),
        person_name: row.person_name,
        organization_name: row.organization_name,
    })
}

/// Allow: a new authorization of the actor for the app (R-10), and a code
/// bound to it, the request's redirect URI and its challenge. The answer is
/// the address to send the browser to, which carries the code.
pub async fn approve(
    tx: &mut Tx,
    actor: &Actor,
    pending: &PendingRequest,
    issuer: &str,
    config: &OAuthConfig,
) -> Result<Done<String>, OpError> {
    actor.require_session()?;
    let scopes = pending.scopes()?;
    let client = sqlx::query!(
        "SELECT name, client_id, scopes FROM oauth_clients
         WHERE id = $1 AND org_id = $2 AND enabled
         FOR SHARE",
        pending.app_id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(REQUEST_NOT_FOUND))?;
    // The app may have lost a scope since the request was checked.
    let allowed = ScopeSet::parse_each(client.scopes.iter().map(String::as_str))
        .map_err(|_| OpError::not_found(REQUEST_NOT_FOUND))?;
    if !scopes.is_subset(allowed) {
        return Err(OpError::not_found(REQUEST_NOT_FOUND));
    }
    let grant_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO oauth_grants (id, org_id, client_id, user_id, scopes, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))",
        grant_id,
        actor.org_id(),
        pending.app_id,
        actor.id(),
        &scopes.names() as &[&str],
        config.refresh_max.as_secs_f64(),
    )
    .execute(&mut **tx)
    .await?;
    let code = Secret::generate(CODE_PREFIX);
    sqlx::query!(
        "INSERT INTO oauth_codes (id, org_id, grant_id, code_sha256, redirect_uri, code_challenge,
                                  expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7))",
        Uuid::now_v7(),
        actor.org_id(),
        grant_id,
        code.digest(),
        pending.redirect_uri,
        pending.code_challenge,
        f64::from(CODE_TTL_SECONDS),
    )
    .execute(&mut **tx)
    .await?;
    let data = serde_json::json!({
        "app_name": client.name,
        "client_id": client.client_id,
        "person_id": actor.id(),
        "scopes": scopes.names(),
    });
    audit::record(tx, actor, Action::GrantApproved, grant_id, data).await?;
    Ok(Done::new(response_url(
        &pending.redirect_uri,
        &[("code", code.expose())],
        pending.state.as_deref(),
        issuer,
    )))
}

/// Deny: the app hears `access_denied`. Nothing is stored.
pub fn deny(pending: &PendingRequest, issuer: &str) -> String {
    response_url(
        &pending.redirect_uri,
        &[("error", "access_denied")],
        pending.state.as_deref(),
        issuer,
    )
}

/// The address of an error the app hears at its redirect URI.
pub fn error_url(error: &RedirectError, issuer: &str) -> String {
    response_url(
        &error.redirect_uri,
        &[
            ("error", error.error),
            ("error_description", error.description),
        ],
        error.state.as_deref(),
        issuer,
    )
}

/// The redirect URI with `params`, `state` and `iss` (RFC 9207) added to its
/// query. The URI matched a registration, so it parses.
fn response_url(
    redirect_uri: &str,
    params: &[(&str, &str)],
    state: Option<&str>,
    issuer: &str,
) -> String {
    let Ok(mut url) = url::Url::parse(redirect_uri) else {
        return redirect_uri.to_owned();
    };
    {
        let mut query = url.query_pairs_mut();
        query.extend_pairs(params);
        if let Some(state) = state {
            query.append_pair("state", state);
        }
        query.append_pair("iss", issuer);
    }
    url.into()
}

/// Where Allow sends the person, as the approval page names it: the host of
/// an `https` or loopback URI, the scheme of an app scheme.
fn return_to_display(redirect_uri: &str) -> String {
    match url::Url::parse(redirect_uri) {
        Ok(url) => match url.host_str() {
            Some(host) if matches!(url.scheme(), "http" | "https") => host.to_owned(),
            _ => url.scheme().to_owned(),
        },
        Err(_) => redirect_uri.to_owned(),
    }
}

struct Client {
    id: Uuid,
    kind: horae_core::permissions::oauth::ClientKind,
    redirect_uris: Vec<String>,
    scopes: ScopeSet,
    enabled: bool,
}

/// The app a `client_id` names. A registered id is unique across
/// organizations; the built-in id is the same in each.
//
// shortcut: with several organizations the built-in id is ambiguous and reads
// as unknown; pick the organization from the request's host once tenants
// have their own subdomains.
async fn find_client(pool: &PgPool, client_id: &str) -> Result<Client, AuthorizeError> {
    let mut rows = sqlx::query!(
        r#"SELECT id, kind::text AS "kind!", redirect_uris, scopes, enabled
           FROM oauth_clients WHERE client_id = $1
           LIMIT 2"#,
        client_id,
    )
    .fetch_all(pool)
    .await?;
    let unknown = AuthorizeError::Page(PageError::UnknownClient);
    if rows.len() != 1 {
        return Err(unknown);
    }
    let row = rows.remove(0);
    let (Some(kind), Ok(scopes)) = (
        horae_core::permissions::oauth::ClientKind::parse(&row.kind),
        ScopeSet::parse_each(row.scopes.iter().map(String::as_str)),
    ) else {
        tracing::error!("Invalid stored OAuth client");
        return Err(unknown);
    };
    Ok(Client {
        id: row.id,
        kind,
        redirect_uris: row.redirect_uris,
        scopes,
        enabled: row.enabled,
    })
}

#[cfg(test)]
mod tests;
