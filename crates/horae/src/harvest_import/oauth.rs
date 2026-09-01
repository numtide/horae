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

use crate::config::HarvestConfig;

/// Harvest identity host (authorize, token, accounts).
const ID_HOST: &str = "https://id.getharvest.com";

/// Build the authorization-code URL to redirect the admin to. `state` is the
/// per-start nonce the callback validates.
pub fn authorize_url(cfg: &HarvestConfig, state: &str) -> String {
    format!(
        "{ID_HOST}/oauth2/authorize?client_id={}&redirect_uri={}&state={}&response_type=code",
        encode(&cfg.client_id),
        encode(&cfg.redirect_url),
        encode(state),
    )
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

/// Exchange an authorization `code` for tokens (server-side, blocking).
pub fn exchange_code(agent: &ureq::Agent, cfg: &HarvestConfig, code: &str) -> anyhow::Result<Tokens> {
    let body = agent
        .post(&format!("{ID_HOST}/api/v2/oauth2/token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("code", code),
            ("client_id", &cfg.client_id),
            ("client_secret", &cfg.client_secret),
            ("redirect_uri", &cfg.redirect_url),
            ("grant_type", "authorization_code"),
        ])
        .map_err(|e| anyhow::anyhow!("Harvest token exchange failed: {e}"))?
        .into_string()?;
    let parsed: TokenBody = serde_json::from_str(&body)?;
    Ok(parsed.into_tokens())
}

/// Refresh an expired access token with the stored refresh token (FR-024). A
/// failure here means the connection is revoked/expired — the caller rejects the
/// run with "reconnect Harvest".
pub fn refresh(agent: &ureq::Agent, cfg: &HarvestConfig, refresh_token: &str) -> anyhow::Result<Tokens> {
    let body = agent
        .post(&format!("{ID_HOST}/api/v2/oauth2/token"))
        .set("Accept", "application/json")
        .send_form(&[
            ("refresh_token", refresh_token),
            ("client_id", &cfg.client_id),
            ("client_secret", &cfg.client_secret),
            ("grant_type", "refresh_token"),
        ])
        .map_err(|e| anyhow::anyhow!("Harvest token refresh failed: {e}"))?
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
pub fn fetch_account_id(agent: &ureq::Agent, access_token: &str) -> anyhow::Result<String> {
    let body = agent
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

/// Percent-encode a query value (encode everything outside the unreserved set).
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> HarvestConfig {
        HarvestConfig {
            client_id: "abc123".into(),
            client_secret: "secret".into(),
            redirect_url: "https://horae.example.com/auth/harvest/callback".into(),
            encryption_key_hex: "00".repeat(32),
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
        assert!(url.contains("redirect_uri=https%3A%2F%2Fhorae.example.com%2Fauth%2Fharvest%2Fcallback"));
    }

    #[test]
    fn encode_escapes_reserved_but_keeps_unreserved() {
        assert_eq!(encode("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(encode("a b"), "a%20b");
        assert_eq!(encode("x/y:z"), "x%2Fy%3Az");
    }
}
