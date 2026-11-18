//! The OIDC side of sign-in that needs no request or session: the provider
//! protocol (authorization URL, code exchange, ID-token verification),
//! [`provisioning`] of a verified identity onto a Horae user, and the operator's
//! [`link`] of an existing account. Accounts are created by admins (FR-002), so a
//! first OIDC login *links* the subject to a user matched by verified email.
//! Only when the operator lists the email's domain does a first login that
//! matches no account create one (an active member). Deactivated users are
//! denied at sign-in. The routes live in the `horae` crate's `auth::http::oidc`.

pub mod link;
pub mod provisioning;

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};

use crate::config::OidcConfig;

/// The verified identity claims we act on.
pub struct Identity {
    /// The token's `iss`, spelled exactly as the provider sent it.
    pub(crate) issuer: String,
    pub(crate) subject: String,
    pub(crate) email: Option<String>,
    /// `None` when the token has no `email_verified` claim, which some
    /// providers (tsidp) never send.
    pub(crate) email_verified: Option<bool>,
    /// The token's `name` claim, if any.
    pub(crate) name: Option<String>,
}

/// The authorization request to send the user to.
pub struct Authorization {
    pub url: String,
    pub csrf: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

/// Discover the provider and build the authorization URL + one-time secrets.
/// Blocking (network discovery) — call from `spawn_blocking`.
pub fn build_authorization(cfg: &OidcConfig) -> anyhow::Result<Authorization> {
    let client = discover_client(cfg)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok(Authorization {
        url: url.to_string(),
        csrf: csrf.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        nonce: nonce.secret().clone(),
    })
}

/// Exchange the authorization code and verify the ID token against `nonce`.
/// Blocking (network token request) — call from `spawn_blocking`.
pub fn exchange_and_verify(
    cfg: &OidcConfig,
    code: String,
    pkce_verifier: &str,
    nonce: &str,
) -> anyhow::Result<Identity> {
    let client = discover_client(cfg)?;

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
        .request(http_client_with_timeout)?;

    let id_token = token
        .id_token()
        .ok_or_else(|| anyhow::anyhow!("provider returned no ID token"))?;

    // `client_id` is always required by the base verifier; this only widens which
    // *additional* audiences are tolerated (e.g. Zitadel's project ID). With an
    // empty list the closure always returns false — i.e. strict, client_id only.
    let additional = cfg.additional_audiences.clone();
    let verifier = client
        .id_token_verifier()
        .set_other_audience_verifier_fn(move |aud| audience_is_trusted(&additional, aud.as_str()));

    let claims = id_token.claims(&verifier, &Nonce::new(nonce.to_string()))?;

    Ok(Identity {
        // `as_str` keeps the original spelling; `url().as_str()` would normalize it.
        issuer: claims.issuer().as_str().to_string(),
        subject: claims.subject().as_str().to_string(),
        email: claims.email().map(|e| e.as_str().to_string()),
        email_verified: claims.email_verified(),
        name: claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.as_str().to_string()),
    })
}

/// Whether an extra ID-token audience (one that is not `client_id`) is on the
/// operator-configured trust list.
fn audience_is_trusted(additional: &[String], aud: &str) -> bool {
    additional.iter().any(|a| a == aud)
}

/// Wall-clock bound on each OIDC HTTP call (discovery, token exchange), so a
/// slow or hung provider cannot stall a login or pin a `spawn_blocking` thread.
const OIDC_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A drop-in for `openidconnect::ureq::http_client` that bounds the request with
/// [`OIDC_HTTP_TIMEOUT`]. The upstream client uses the global `ureq::get/post`
/// with no timeout; this routes through an agent that has one.
fn http_client_with_timeout(
    request: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, openidconnect::ureq::Error> {
    use openidconnect::http::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
    use openidconnect::http::{Method, StatusCode};
    use openidconnect::ureq::Error;

    let agent = ureq::AgentBuilder::new().timeout(OIDC_HTTP_TIMEOUT).build();

    let is_post = request.method == Method::POST;
    let mut req = if is_post {
        agent.post(request.url.as_str())
    } else {
        agent.get(request.url.as_str())
    };
    for (name, value) in &request.headers {
        req = req.set(
            name.as_str(),
            value.to_str().map_err(|_| {
                Error::Other(format!(
                    "invalid {name} header value {:?}",
                    value.as_bytes()
                ))
            })?,
        );
    }

    let response = if is_post {
        req.send_bytes(&request.body)
    } else {
        req.call()
    }
    .map_err(Box::new)?;

    Ok(openidconnect::HttpResponse {
        status_code: StatusCode::from_u16(response.status())
            .map_err(|err| Error::Http(err.into()))?,
        headers: [(
            CONTENT_TYPE,
            HeaderValue::from_str(response.content_type())
                .map_err(|err| Error::Http(err.into()))?,
        )]
        .into_iter()
        .collect::<HeaderMap>(),
        body: response.into_string()?.as_bytes().into(),
    })
}

/// Discover provider metadata and build a client. Blocking (network).
fn discover_client(cfg: &OidcConfig) -> anyhow::Result<CoreClient> {
    let issuer = IssuerUrl::new(cfg.issuer.clone())?;
    let metadata = CoreProviderMetadata::discover(&issuer, http_client_with_timeout)?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(cfg.redirect_url.clone())?);
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_audiences_are_trusted_only_when_listed() {
        let allow = vec!["proj-123".to_string()];
        // Zitadel's extra project-ID audience is accepted when configured.
        assert!(audience_is_trusted(&allow, "proj-123"));
        // Anything else is rejected.
        assert!(!audience_is_trusted(&allow, "proj-999"));
        // With nothing configured, no extra audience is trusted (strict default).
        assert!(!audience_is_trusted(&[], "proj-123"));
    }
}
