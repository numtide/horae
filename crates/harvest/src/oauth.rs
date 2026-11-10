//! Harvest OAuth2 authorization-code flow (research.md §10, contracts/harvest-api.md §A).
//!
//! Harvest is a confidential client: the authorization URL carries the client id,
//! redirect URL, and a random `state` nonce (bound to the admin's session and
//! validated on the callback to defeat CSRF), and the server-side token exchange
//! authenticates with the client secret. Harvest's flow does not document PKCE
//! for confidential web apps, so the secret + `state` are the protection here.
//!
//! The token exchange, refresh, and account-id lookup are blocking `ureq` calls
//! (run under `spawn_blocking`), matching the OIDC stack's pattern.

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use url::form_urlencoded;

use crate::Error;

/// The registered Harvest OAuth2 app this server authenticates as.
#[derive(Debug, Clone, Copy)]
pub struct OAuthClient<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    /// Must match the redirect URL registered on the Harvest OAuth2 app.
    pub redirect_url: &'a str,
}

/// Harvest identity host (authorize, token, accounts).
const ID_HOST: &str = "https://id.getharvest.com";

/// Matches the data client's read timeout, so a silent identity host cannot
/// hold the import worker or the OAuth callback forever.
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// The only agent for the token and account calls: callers cannot pass one
/// without a timeout.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout(HTTP_TIMEOUT)
        .build()
}

/// The OAuth `error` code of a token endpoint refusal (RFC 6749 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthError {
    InvalidClient,
    InvalidGrant,
    InvalidRequest,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
    /// A code outside RFC 6749, or a body that is not an OAuth error.
    Other,
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidClient => "invalid_client",
            Self::InvalidGrant => "invalid_grant",
            Self::InvalidRequest => "invalid_request",
            Self::UnauthorizedClient => "unauthorized_client",
            Self::UnsupportedGrantType => "unsupported_grant_type",
            Self::InvalidScope => "invalid_scope",
            Self::Other => "unrecognised error",
        })
    }
}

/// The token endpoint answered with an error status. Only the OAuth code and
/// the status are kept: the description, the body and the request can carry
/// details that must not reach a log or the browser.
#[derive(Debug, thiserror::Error)]
#[error("token exchange refused ({code}, HTTP {status})")]
pub struct TokenRefused {
    pub code: OAuthError,
    pub status: u16,
}

/// The token endpoint was rate limited or failing (429 or 5xx): Harvest did
/// not judge the request, so it is not a refusal.
#[derive(Debug, thiserror::Error)]
#[error("Harvest token endpoint unavailable (HTTP {status})")]
pub struct TokenUnavailable {
    pub status: u16,
}

/// Whether a token endpoint status means "try later" rather than "refused".
fn unavailable(status: u16) -> bool {
    status >= 500 || status == 429
}

impl TokenRefused {
    fn read(status: u16, response: ureq::Response) -> Self {
        #[derive(Deserialize)]
        struct ErrorBody {
            error: String,
        }
        let code = response
            .into_string()
            .ok()
            .and_then(|body| serde_json::from_str::<ErrorBody>(&body).ok())
            .map_or(OAuthError::Other, |body| match body.error.as_str() {
                "invalid_client" => OAuthError::InvalidClient,
                "invalid_grant" => OAuthError::InvalidGrant,
                "invalid_request" => OAuthError::InvalidRequest,
                "unauthorized_client" => OAuthError::UnauthorizedClient,
                "unsupported_grant_type" => OAuthError::UnsupportedGrantType,
                "invalid_scope" => OAuthError::InvalidScope,
                _ => OAuthError::Other,
            });
        Self { code, status }
    }
}

/// Only the token endpoint refusing the grant means the connection is gone;
/// a network failure or a 5xx is transient and the job retries it.
fn refresh_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(status @ (400 | 401), response) => {
            let refused = TokenRefused::read(status, response);
            tracing::warn!(
                "Harvest token refresh refused ({}, HTTP {})",
                refused.code,
                refused.status
            );
            Error::Unauthorized.into()
        }
        ureq::Error::Status(status, _) if unavailable(status) => {
            Error::Unavailable(format!("Harvest token refresh failed with HTTP {status}")).into()
        }
        ureq::Error::Status(status, _) => {
            anyhow::anyhow!("Harvest token refresh failed with HTTP {status}")
        }
        ureq::Error::Transport(_) => {
            Error::Unavailable("Harvest token refresh failed or timed out".into()).into()
        }
    }
}

/// Build the authorization-code URL to redirect the admin to. `state` is the
/// per-start nonce the callback validates.
pub fn authorize_url(cfg: &OAuthClient, state: &str) -> String {
    let query = form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("client_id", cfg.client_id),
            ("redirect_uri", cfg.redirect_url),
            ("state", state),
            ("response_type", "code"),
        ])
        .finish();
    format!("{ID_HOST}/oauth2/authorize?{query}")
}

/// Tokens returned by the token endpoint (exchange or refresh).
#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenBody {
    access_token: String,
    refresh_token: String,
    expires_in: Option<i64>,
    scope: Option<String>,
}

impl TokenBody {
    fn into_tokens(self) -> Tokens {
        Tokens {
            expires_at: self
                .expires_in
                .map(|secs| Utc::now() + Duration::seconds(secs)),
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            scope: self.scope,
        }
    }
}

/// Exchange an authorization `code` for tokens (server-side, blocking). A
/// refusal is a [`TokenRefused`]; a 429 or 5xx is a [`TokenUnavailable`].
pub fn exchange_code(cfg: &OAuthClient, code: &str) -> anyhow::Result<Tokens> {
    exchange_code_at(&format!("{ID_HOST}{TOKEN_PATH}"), cfg, code)
}

const TOKEN_PATH: &str = "/api/v2/oauth2/token";

fn exchange_code_at(token_url: &str, cfg: &OAuthClient, code: &str) -> anyhow::Result<Tokens> {
    let body = agent()
        .post(token_url)
        .set("Accept", "application/json")
        .send_form(&[
            ("code", code),
            ("client_id", cfg.client_id),
            ("client_secret", cfg.client_secret),
            ("redirect_uri", cfg.redirect_url),
            ("grant_type", "authorization_code"),
        ])
        .map_err(|e| match e {
            ureq::Error::Status(status, _) if unavailable(status) => {
                TokenUnavailable { status }.into()
            }
            ureq::Error::Status(status, response) => TokenRefused::read(status, response).into(),
            ureq::Error::Transport(e) => anyhow::anyhow!("Harvest token exchange failed: {e}"),
        })?
        .into_string()?;
    let parsed: TokenBody = serde_json::from_str(&body)?;
    Ok(parsed.into_tokens())
}

/// Refresh an expired access token with the stored refresh token (FR-024). A
/// refused grant is [`Error::Unauthorized`]; anything else is a retryable error.
pub fn refresh(cfg: &OAuthClient, refresh_token: &str) -> anyhow::Result<Tokens> {
    refresh_at(&format!("{ID_HOST}{TOKEN_PATH}"), cfg, refresh_token)
}

fn refresh_at(token_url: &str, cfg: &OAuthClient, refresh_token: &str) -> anyhow::Result<Tokens> {
    let body = agent()
        .post(token_url)
        .set("Accept", "application/json")
        .send_form(&[
            ("refresh_token", refresh_token),
            ("client_id", cfg.client_id),
            ("client_secret", cfg.client_secret),
            ("grant_type", "refresh_token"),
        ])
        .map_err(refresh_error)?
        .into_string()?;
    let parsed: TokenBody = serde_json::from_str(&body)?;
    Ok(parsed.into_tokens())
}

#[derive(Debug, Deserialize)]
struct AccountsBody {
    accounts: Vec<Account>,
}

#[derive(Debug, Deserialize)]
struct Account {
    id: i64,
    #[serde(default)]
    product: Option<String>,
}

/// Resolve the Harvest account id the tokens authorize (required on every data
/// call). Prefers a Harvest-product account; falls back to the first listed.
pub fn fetch_account_id(access_token: &str) -> anyhow::Result<String> {
    let body = agent()
        .get(&format!("{ID_HOST}/api/v2/accounts"))
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("User-Agent", "Horae Importer (support@horae.app)")
        .set("Accept", "application/json")
        .call()
        .map_err(|e| anyhow::anyhow!("Harvest accounts lookup failed: {e}"))?
        .into_string()?;
    let parsed: AccountsBody = serde_json::from_str(&body)?;
    let account = parsed
        .accounts
        .iter()
        .find(|a| a.product.as_deref() == Some("harvest"))
        .or_else(|| parsed.accounts.first())
        .ok_or_else(|| anyhow::anyhow!("Harvest returned no accounts for this connection"))?;
    Ok(account.id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> OAuthClient<'static> {
        OAuthClient {
            client_id: "abc123",
            client_secret: "secret",
            redirect_url: "https://horae.example.com/auth/harvest/callback",
        }
    }

    #[test]
    fn authorize_url_carries_state_and_encoded_redirect() {
        let url = authorize_url(&cfg(), "nonce-xyz");
        assert!(url.starts_with("https://id.getharvest.com/oauth2/authorize?"));
        assert!(url.contains("client_id=abc123"));
        assert!(url.contains("state=nonce-xyz"));
        assert!(url.contains("response_type=code"));
        // The redirect URL is percent-encoded (":" and "/" escaped).
        assert!(
            url.contains(
                "redirect_uri=https%3A%2F%2Fhorae.example.com%2Fauth%2Fharvest%2Fcallback"
            )
        );
    }

    #[test]
    fn authorize_url_preserves_query_values_without_injecting_parameters() {
        for value in ["", "a-b_c.d~e", "a+b c", "x/y:z?&=;%#", "mañana 東京 🕒"] {
            let redirect_url = format!("https://horae.example.com/callback?next={value}");
            let config = OAuthClient {
                client_id: value,
                redirect_url: &redirect_url,
                ..cfg()
            };
            let url = url::Url::parse(&authorize_url(&config, value)).unwrap();

            assert_eq!(url.origin().ascii_serialization(), ID_HOST);
            assert_eq!(url.path(), "/oauth2/authorize");
            assert_eq!(url.fragment(), None);
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                vec![
                    ("client_id".into(), value.into()),
                    ("redirect_uri".into(), config.redirect_url.into()),
                    ("state".into(), value.into()),
                    ("response_type".into(), "code".into()),
                ],
                "query values must round-trip for {value:?}",
            );
        }
    }

    #[test]
    fn only_a_refused_grant_asks_for_a_reconnect() {
        let status =
            |code| ureq::Error::Status(code, ureq::Response::new(code, "status", "").unwrap());
        // Port 1 on loopback refuses the connection: a transport failure.
        let unreachable = ureq::get("http://127.0.0.1:1/").call().unwrap_err();
        // What the job shows: reconnect, try again later, or the plain error.
        for (error, kind) in [
            (status(400), "reconnect"),
            (status(401), "reconnect"),
            (status(429), "unavailable"),
            (status(500), "unavailable"),
            (status(503), "unavailable"),
            (unreachable, "unavailable"),
            (status(404), "other"),
        ] {
            let mapped = refresh_error(error);
            let found = match mapped.downcast_ref::<Error>() {
                Some(Error::Unauthorized) => "reconnect",
                Some(Error::Unavailable(_)) => "unavailable",
                _ => "other",
            };
            assert_eq!(found, kind, "{mapped}");
        }
    }

    /// What a fake token endpoint puts beside the error code, none of which
    /// may reach an error, a log line or the browser.
    const PRIVATE: [&str; 3] = ["private-description", "private-uri", "private-state"];

    /// A token endpoint that answers one request with `status` and `body`.
    fn fake_token_endpoint(status: u16, body: &'static str) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/api/v2/oauth2/token",
            listener.local_addr().unwrap()
        );
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let _ = write!(
                stream,
                "HTTP/1.1 {status} Refused\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            // Closing with unread request bytes would reset the connection
            // before the client reads the answer.
            let _ = stream.shutdown(std::net::Shutdown::Write);
            let _ = std::io::copy(&mut stream, &mut std::io::sink());
        });
        url
    }

    fn refusal(code: &str) -> &'static str {
        format!(
            r#"{{"error":"{code}","error_description":"private-description","error_uri":"private-uri","state":"private-state"}}"#
        )
        .leak()
    }

    #[test]
    fn a_refused_exchange_reports_only_the_oauth_code_and_status() {
        for (status, code, expected) in [
            (401, "invalid_client", OAuthError::InvalidClient),
            (400, "invalid_grant", OAuthError::InvalidGrant),
            (400, "invalid_request", OAuthError::InvalidRequest),
            (400, "unauthorized_client", OAuthError::UnauthorizedClient),
            (
                400,
                "unsupported_grant_type",
                OAuthError::UnsupportedGrantType,
            ),
            (400, "invalid_scope", OAuthError::InvalidScope),
            (400, "something_new", OAuthError::Other),
        ] {
            let url = fake_token_endpoint(status, refusal(code));
            let error = exchange_code_at(&url, &cfg(), "the-code").unwrap_err();
            let refused = error.downcast_ref::<TokenRefused>().unwrap();
            assert_eq!((refused.code, refused.status), (expected, status), "{code}");
            let shown = format!("{error} {error:?} {error:#}");
            for private in PRIVATE.iter().chain(&["the-code", "secret"]) {
                assert!(!shown.contains(private), "{code} leaked {private}: {shown}");
            }
        }
    }

    #[test]
    fn the_log_line_names_the_code_and_status() {
        let url = fake_token_endpoint(401, refusal("invalid_client"));
        let error = exchange_code_at(&url, &cfg(), "the-code").unwrap_err();
        assert_eq!(
            error.to_string(),
            "token exchange refused (invalid_client, HTTP 401)"
        );
        let url = fake_token_endpoint(400, refusal("something_new"));
        let error = exchange_code_at(&url, &cfg(), "the-code").unwrap_err();
        assert_eq!(
            error.to_string(),
            "token exchange refused (unrecognised error, HTTP 400)"
        );
    }

    #[test]
    fn an_unparseable_refusal_keeps_only_the_status() {
        for body in ["<html>private-description</html>", "", r#"{"error":7}"#] {
            let url = fake_token_endpoint(400, body);
            let error = exchange_code_at(&url, &cfg(), "the-code").unwrap_err();
            let refused = error.downcast_ref::<TokenRefused>().unwrap();
            assert_eq!((refused.code, refused.status), (OAuthError::Other, 400));
            assert!(!format!("{error:?}").contains("private"), "{body}");
        }
    }

    #[test]
    fn an_unavailable_token_endpoint_is_not_a_refusal() {
        for status in [429, 500, 503] {
            let url = fake_token_endpoint(status, refusal("invalid_grant"));
            let error = exchange_code_at(&url, &cfg(), "the-code").unwrap_err();
            assert!(error.downcast_ref::<TokenRefused>().is_none(), "{status}");
            let unavailable = error.downcast_ref::<TokenUnavailable>().unwrap();
            assert_eq!(unavailable.status, status);
            assert_eq!(
                error.to_string(),
                format!("Harvest token endpoint unavailable (HTTP {status})")
            );
            assert!(!format!("{error:?}").contains("private"), "{status}");
        }
    }

    #[test]
    fn a_refused_refresh_still_asks_for_a_reconnect() {
        for (status, code) in [(400, "invalid_grant"), (401, "invalid_client")] {
            let url = fake_token_endpoint(status, refusal(code));
            let error = refresh_at(&url, &cfg(), "the-refresh-token").unwrap_err();
            assert!(!format!("{error:?}").contains("private"));
            assert!(matches!(
                error.downcast_ref::<Error>(),
                Some(Error::Unauthorized)
            ));
        }
    }

    /// `agent()` is private and every token call builds it, so this covers the
    /// refresh, the code exchange and the account lookup.
    #[test]
    fn token_requests_give_up_on_a_silent_server() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        // Accept the connection and never answer it.
        std::thread::spawn(move || {
            let _held = listener.accept();
            std::thread::sleep(std::time::Duration::from_secs(120));
        });
        let (done, result) = std::sync::mpsc::channel();
        std::thread::spawn(move || done.send(agent().get(&url).call().is_err()));
        assert_eq!(
            result.recv_timeout(HTTP_TIMEOUT + std::time::Duration::from_secs(15)),
            Ok(true)
        );
    }
}
