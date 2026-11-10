//! Harvest OAuth2 authorization-code flow (research.md §10, contracts/harvest-api.md §A).
//! The HTTP calls live in `horae_harvest::oauth`; this module feeds them the
//! deployment's [`HarvestConfig`] and decides when a stored token is refreshed.

use chrono::{DateTime, Duration, Utc};
use horae_harvest::oauth::OAuthClient;
pub use horae_harvest::oauth::{
    OAuthError, TokenRefused, TokenUnavailable, Tokens, fetch_account_id,
};

use crate::config::HarvestConfig;

fn client(cfg: &HarvestConfig) -> OAuthClient<'_> {
    OAuthClient {
        client_id: &cfg.client_id,
        client_secret: &cfg.client_secret,
        redirect_url: &cfg.redirect_url,
    }
}

/// Margin for refreshing before expiry. A data-API 401 ends the job as
/// "reconnect", so the token must outlive the run, rate-limit waits included.
const REFRESH_AHEAD: Duration = Duration::days(1);

/// Whether the stored access token must be refreshed before a run starts.
pub fn needs_refresh(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    expires_at.is_some_and(|expiry| expiry <= now + REFRESH_AHEAD)
}

/// Build the authorization-code URL to redirect the admin to. `state` is the
/// per-start nonce the callback validates.
pub fn authorize_url(cfg: &HarvestConfig, state: &str) -> String {
    horae_harvest::oauth::authorize_url(&client(cfg), state)
}

/// Exchange an authorization `code` for tokens (server-side, blocking). A
/// refusal is a [`TokenRefused`]; a 429 or 5xx is a [`TokenUnavailable`].
pub fn exchange_code(cfg: &HarvestConfig, code: &str) -> anyhow::Result<Tokens> {
    horae_harvest::oauth::exchange_code(&client(cfg), code)
}

/// Refresh an expired access token with the stored refresh token (FR-024). A
/// refused grant becomes `ReconnectRequired`; anything else is a retryable error.
pub fn refresh(cfg: &HarvestConfig, refresh_token: &str) -> anyhow::Result<Tokens> {
    horae_harvest::oauth::refresh(&client(cfg), refresh_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_that_would_expire_during_the_run_is_refreshed_first() {
        let now = Utc::now();
        for (expires_at, refresh) in [
            (None, false),
            (Some(now - Duration::minutes(1)), true),
            (Some(now + Duration::hours(1)), true),
            (Some(now + Duration::hours(23)), true),
            (Some(now + Duration::days(2)), false),
        ] {
            assert_eq!(needs_refresh(expires_at, now), refresh, "{expires_at:?}");
        }
    }
}
