use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub log_level: String,
    /// When `DEV_LOGIN=1`, skip OIDC and log in as the seeded admin user.
    pub dev_login: bool,
    /// Directory containing plugin subdirectories (each with plugin.toml + *.wasm).
    pub plugins_dir: String,
    /// Separate unprivileged login for plugin SQL. Unset disables SQL access.
    pub plugin_database_url: Option<String>,
    /// OIDC provider settings. `Some` only when all four env vars are set;
    /// production auth is enabled exactly when this is present and `dev_login`
    /// is false.
    pub oidc: Option<OidcConfig>,
    /// Mark session cookies `Secure` (send only over HTTPS). Set `HORAE_SECURE_COOKIES=1`
    /// in production, where TLS is terminated in front of (or by) the app.
    pub secure_cookies: bool,
    /// Harvest settings from the environment. An organization's stored OAuth
    /// client fills in what is missing; see `importers::harvest::client_settings`.
    pub harvest: HarvestDeployment,
    /// `HORAE_HARVEST_API=1` mounts the Harvest-compatible API (`/harvest/v2`,
    /// `/v2`). Off by default; the importer's OAuth callback does not need it.
    pub harvest_api: bool,
    /// Execution limits copied into newly enqueued jobs.
    pub job_policy: JobPolicy,
    /// Optional budget email transport; no process is started when absent.
    pub mail: Option<MailConfig>,
    /// The `/api/v1` cursor key and the per-token request limit. Holds a
    /// secret, so it is never serialized.
    #[serde(skip)]
    pub api: ApiConfig,
    /// `HORAE_PUBLIC_URL`; the OAuth authorization server runs only with it.
    pub public_url: Option<PublicUrl>,
    /// OAuth credential lifetimes and the token endpoint's limit.
    #[serde(skip)]
    pub oauth: OAuthConfig,
}

/// Settings of Horae's token-authenticated APIs (spec 022).
#[derive(Debug, Clone, Default)]
pub struct ApiConfig {
    pub cursor_key: CursorKey,
    pub rate_limit: RateLimit,
}

/// The HMAC key that signs list cursors (research R-5). Its `Debug` hides the
/// bytes.
#[derive(Clone)]
pub struct CursorKey {
    bytes: [u8; 32],
    random: bool,
}

impl CursorKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self {
            bytes,
            random: false,
        }
    }

    /// A key drawn at start, the development fallback: cursors signed with it
    /// stop working when the process restarts.
    pub fn random() -> Self {
        use chacha20poly1305::aead::OsRng;
        use chacha20poly1305::aead::rand_core::RngCore;
        let mut bytes = [0; 32];
        OsRng.fill_bytes(&mut bytes);
        Self {
            bytes,
            random: true,
        }
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn is_random(&self) -> bool {
        self.random
    }

    /// 64 hex characters, surrounding whitespace ignored.
    fn parse(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
        }
        Some(Self::new(bytes))
    }

    /// `HORAE_API_CURSOR_KEY_FILE`, else `HORAE_API_CURSOR_KEY`, else a random
    /// key. A key file that cannot be read or holds no key refuses to start.
    fn from_env() -> anyhow::Result<Self> {
        if let Some(path) = non_empty("HORAE_API_CURSOR_KEY_FILE") {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("Cannot read HORAE_API_CURSOR_KEY_FILE ({path})"))?;
            return Self::parse(&text).with_context(|| {
                format!("HORAE_API_CURSOR_KEY_FILE ({path}) must hold 64 hex characters")
            });
        }
        if let Some(value) = non_empty("HORAE_API_CURSOR_KEY") {
            return Self::parse(&value).context("HORAE_API_CURSOR_KEY must be 64 hex characters");
        }
        Ok(Self::random())
    }
}

/// Only for `#[serde(skip)]`: a configuration read from the environment
/// always says where its key came from.
impl Default for CursorKey {
    fn default() -> Self {
        Self::random()
    }
}

impl std::fmt::Debug for CursorKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CursorKey")
            .field("bytes", &REDACTED)
            .field("random", &self.random)
            .finish()
    }
}

/// How many requests a token may make per window (research R-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    pub requests: u32,
    pub window: std::time::Duration,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests: 100,
            window: std::time::Duration::from_secs(15),
        }
    }
}

impl RateLimit {
    /// `<requests>/<seconds>`, both at least 1.
    pub fn parse(value: &str) -> Option<Self> {
        let (requests, seconds) = value.split_once('/')?;
        let requests: u32 = requests.parse().ok()?;
        let seconds: u64 = seconds.parse().ok()?;
        (requests > 0 && seconds > 0).then(|| Self {
            requests,
            window: std::time::Duration::from_secs(seconds),
        })
    }

    fn from_env(key: &str, default: Self) -> anyhow::Result<Self> {
        match std::env::var(key) {
            Ok(value) => Self::parse(&value)
                .with_context(|| format!("{key} must be <requests>/<seconds>, not {value:?}")),
            Err(std::env::VarError::NotPresent) => Ok(default),
            Err(error) => Err(error).with_context(|| format!("invalid {key}")),
        }
    }
}

/// `HORAE_PUBLIC_URL`: the address people and apps reach Horae at, and the
/// OAuth issuer (spec 024). Without it the authorization server is off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicUrl(String);

impl PublicUrl {
    /// An absolute `https` URL, or `http` to a loopback host, with no trailing
    /// slash, query, fragment or user information.
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        let url = url::Url::parse(value).context("not an absolute URL")?;
        let host = url.host_str().context("no host")?;
        match url.scheme() {
            "https" => {}
            "http" if is_loopback_host(host) => {}
            "http" => anyhow::bail!("http is allowed only for a loopback host"),
            scheme => anyhow::bail!("{scheme} is not https"),
        }
        anyhow::ensure!(
            url.username().is_empty() && url.password().is_none(),
            "a user name or password is not allowed"
        );
        anyhow::ensure!(
            url.query().is_none() && url.fragment().is_none(),
            "a query or fragment is not allowed"
        );
        anyhow::ensure!(!value.ends_with('/'), "remove the trailing slash");
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_env() -> anyhow::Result<Option<Self>> {
        non_empty("HORAE_PUBLIC_URL")
            .map(|value| {
                Self::parse(&value).with_context(|| format!("invalid HORAE_PUBLIC_URL {value:?}"))
            })
            .transpose()
    }
}

/// Lifetimes of OAuth credentials and the token endpoint's limit (spec 024
/// D-2, contracts/configuration.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OAuthConfig {
    pub access_ttl: std::time::Duration,
    /// A refresh token unused this long expires.
    pub refresh_idle: std::time::Duration,
    /// No refresh token outlives its authorization's approval by more.
    pub refresh_max: std::time::Duration,
    /// Requests per client to the token endpoint.
    pub token_rate_limit: RateLimit,
}

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            access_ttl: std::time::Duration::from_secs(HOUR),
            refresh_idle: std::time::Duration::from_secs(30 * DAY),
            refresh_max: std::time::Duration::from_secs(365 * DAY),
            token_rate_limit: RateLimit {
                requests: 30,
                window: std::time::Duration::from_secs(60),
            },
        }
    }
}

impl OAuthConfig {
    /// Refuses lifetimes outside sane bounds: access 60 s to 1 day, refresh
    /// idle 1 hour to 1 year, and the maximum at least the idle value.
    pub fn validate(self) -> anyhow::Result<Self> {
        let seconds = |duration: std::time::Duration| duration.as_secs();
        anyhow::ensure!(
            (60..=DAY).contains(&seconds(self.access_ttl)),
            "HORAE_OAUTH_ACCESS_TTL must be between 60 and {DAY} seconds"
        );
        anyhow::ensure!(
            (HOUR..=365 * DAY).contains(&seconds(self.refresh_idle)),
            "HORAE_OAUTH_REFRESH_IDLE must be between {HOUR} and {} seconds",
            365 * DAY
        );
        anyhow::ensure!(
            self.refresh_max >= self.refresh_idle,
            "HORAE_OAUTH_REFRESH_MAX must be at least HORAE_OAUTH_REFRESH_IDLE"
        );
        Ok(self)
    }

    fn from_env() -> anyhow::Result<Self> {
        let default = Self::default();
        Self {
            access_ttl: seconds_from_env("HORAE_OAUTH_ACCESS_TTL", default.access_ttl)?,
            refresh_idle: seconds_from_env("HORAE_OAUTH_REFRESH_IDLE", default.refresh_idle)?,
            refresh_max: seconds_from_env("HORAE_OAUTH_REFRESH_MAX", default.refresh_max)?,
            token_rate_limit: RateLimit::from_env(
                "HORAE_OAUTH_TOKEN_RATE_LIMIT",
                default.token_rate_limit,
            )?,
        }
        .validate()
    }
}

fn seconds_from_env(
    key: &str,
    default: std::time::Duration,
) -> anyhow::Result<std::time::Duration> {
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map(std::time::Duration::from_secs)
            .with_context(|| format!("{key} must be a number of seconds, not {value:?}")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error).with_context(|| format!("invalid {key}")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    pub executable: std::path::PathBuf,
    pub sender: String,
}

impl MailConfig {
    /// Both values must be supplied together. Validate without executing the
    /// program: configuration loading must never send mail.
    pub fn parse(executable: Option<&str>, sender: Option<&str>) -> anyhow::Result<Option<Self>> {
        use std::os::unix::fs::PermissionsExt;
        let (executable, sender) = match (executable, sender) {
            (None, None) => return Ok(None),
            (Some(executable), Some(sender)) => (executable, sender),
            _ => anyhow::bail!("Set both HORAE_SENDMAIL_PATH and HORAE_MAIL_FROM, or neither"),
        };
        let executable = std::path::PathBuf::from(executable);
        anyhow::ensure!(
            executable.is_absolute(),
            "HORAE_SENDMAIL_PATH must be an absolute executable path"
        );
        let metadata = std::fs::metadata(&executable).context("Cannot read HORAE_SENDMAIL_PATH")?;
        anyhow::ensure!(
            metadata.is_file() && metadata.permissions().mode() & 0o111 != 0,
            "HORAE_SENDMAIL_PATH must name an executable file"
        );
        anyhow::ensure!(
            valid_mailbox(sender),
            "HORAE_MAIL_FROM must be a plain ASCII mailbox address"
        );
        Ok(Some(Self {
            executable,
            sender: sender.to_owned(),
        }))
    }
}

/// Supported envelope addresses are unquoted ASCII mailboxes with DNS-style
/// domains. Display names, multiple recipients and command/file forms are not accepted.
pub(crate) fn valid_mailbox(address: &str) -> bool {
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    address.len() <= 254
        && (1..=64).contains(&local.len())
        && !local.starts_with(['-', '.'])
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        && !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JobPolicy {
    /// Total attempts, including the initial execution; must be in 1..=100.
    pub max_attempts: i32,
}

impl Default for JobPolicy {
    fn default() -> Self {
        Self { max_attempts: 5 }
    }
}

impl JobPolicy {
    pub fn validate(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=100).contains(&self.max_attempts),
            "job max_attempts must be between 1 and 100"
        );
        Ok(self)
    }

    fn from_env() -> anyhow::Result<Self> {
        let max_attempts = match std::env::var("HORAE_JOB_MAX_ATTEMPTS") {
            Ok(value) => value.parse().context("invalid HORAE_JOB_MAX_ATTEMPTS")?,
            Err(std::env::VarError::NotPresent) => Self::default().max_attempts,
            Err(error) => return Err(error).context("invalid HORAE_JOB_MAX_ATTEMPTS"),
        };
        Self { max_attempts }
            .validate()
            .context("invalid HORAE_JOB_MAX_ATTEMPTS")
    }
}

/// The Harvest client one request uses: the environment's settings, completed
/// by the organization's stored client. Its `Debug` hides the secret and key.
#[derive(Clone)]
pub struct HarvestConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Where Harvest redirects back after authorization — must match the value
    /// registered on the Harvest OAuth2 app and end in `/auth/harvest/callback`.
    pub redirect_url: String,
    /// 32-byte AEAD key (hex-encoded, 64 hex chars) that seals the stored OAuth
    /// tokens at rest. Rotating it makes existing tokens undecryptable — recovery
    /// is to reconnect (data-model.md).
    pub encryption_key_hex: String,
}

/// Harvest settings read from `HORAE_HARVEST_CLIENT_ID`,
/// `HORAE_HARVEST_CLIENT_SECRET`, `HORAE_HARVEST_REDIRECT_URL`, and
/// `HORAE_HARVEST_ENC_KEY`. Each is optional; the importer's API source is
/// available when these and the organization's stored client together supply
/// a client, a redirect URL and the key.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct HarvestDeployment {
    /// The client ID and secret, set only when both variables are.
    pub client: Option<HarvestClient>,
    pub redirect_url: Option<String>,
    /// Seals tokens and the stored client secret. Never stored or serialized.
    #[serde(skip_serializing, default)]
    pub encryption_key_hex: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HarvestClient {
    pub client_id: String,
    #[serde(skip_serializing, default)]
    pub client_secret: String,
}

/// Stands in for a secret in `Debug` output, so one `{:?}` of the
/// configuration cannot leak it into a log.
const REDACTED: &str = "[redacted]";

impl std::fmt::Debug for HarvestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarvestConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &REDACTED)
            .field("redirect_url", &self.redirect_url)
            .field("encryption_key_hex", &REDACTED)
            .finish()
    }
}

impl std::fmt::Debug for HarvestDeployment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarvestDeployment")
            .field("client", &self.client)
            .field("redirect_url", &self.redirect_url)
            .field(
                "encryption_key_hex",
                &self.encryption_key_hex.as_ref().map(|_| REDACTED),
            )
            .finish()
    }
}

impl std::fmt::Debug for HarvestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HarvestClient")
            .field("client_id", &self.client_id)
            .field("client_secret", &REDACTED)
            .finish()
    }
}

/// Tests configure the whole client through the environment.
#[cfg(any(test, feature = "test-support"))]
impl From<HarvestConfig> for HarvestDeployment {
    fn from(cfg: HarvestConfig) -> Self {
        Self {
            client: Some(HarvestClient {
                client_id: cfg.client_id,
                client_secret: cfg.client_secret,
            }),
            redirect_url: Some(cfg.redirect_url),
            encryption_key_hex: Some(cfg.encryption_key_hex),
        }
    }
}

/// OIDC provider configuration, read from `HORAE_OIDC_ISSUER`,
/// `HORAE_OIDC_CLIENT_ID`, `HORAE_OIDC_CLIENT_SECRET`, and `HORAE_OIDC_REDIRECT_URL`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    /// Extra `aud` values to trust in the ID token beyond `client_id`. Some
    /// providers (e.g. Zitadel) add the project ID to `aud`; without listing it
    /// here the strict verifier rejects the token. Empty means strict
    /// (client_id only). Set `HORAE_OIDC_ADDITIONAL_AUDIENCES` (comma-separated).
    pub additional_audiences: Vec<String>,
    /// Label on the sign-in page's SSO button. Deployments front a named provider
    /// (e.g. "Continue with Okta", "Sign in with Google"), so this is overridable
    /// via `HORAE_OIDC_BUTTON_LABEL`; it defaults to [`DEFAULT_OIDC_BUTTON_LABEL`].
    pub button_label: String,
    /// Which first sign-ins may create their own account. Off unless the
    /// operator lists domains.
    pub auto_provision: AutoProvision,
}

/// Operator policy for creating accounts at first OIDC sign-in.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoProvision {
    /// Lowercased email domains whose people may create an active member
    /// account by signing in (`HORAE_OIDC_AUTO_PROVISION_DOMAINS`). Empty
    /// means provisioning is off.
    pub domains: Vec<String>,
    /// Treat a token with no `email_verified` claim as verified, for
    /// provisioning only (`HORAE_OIDC_TRUST_UNVERIFIED_EMAIL`). Meant for
    /// providers such as tsidp whose users cannot choose their address.
    pub trust_unverified_email: bool,
}

/// Default text on the OIDC sign-in button when `HORAE_OIDC_BUTTON_LABEL` is
/// unset — provider-agnostic, since Horae does not know the provider's name.
pub const DEFAULT_OIDC_BUTTON_LABEL: &str = "Continue with SSO";

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/horae".into()),
            log_level: std::env::var("HORAE_LOG").unwrap_or_else(|_| "info".into()),
            dev_login: std::env::var("DEV_LOGIN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            plugins_dir: std::env::var("HORAE_PLUGINS_DIR").unwrap_or_else(|_| "plugins".into()),
            plugin_database_url: non_empty("HORAE_PLUGIN_DATABASE_URL"),
            oidc: OidcConfig::from_env()?,
            secure_cookies: std::env::var("HORAE_SECURE_COOKIES")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            harvest: HarvestDeployment::from_env(),
            harvest_api: std::env::var("HORAE_HARVEST_API")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            job_policy: JobPolicy::from_env()?,
            mail: MailConfig::parse(
                non_empty("HORAE_SENDMAIL_PATH").as_deref(),
                non_empty("HORAE_MAIL_FROM").as_deref(),
            )?,
            api: ApiConfig {
                cursor_key: CursorKey::from_env()?,
                rate_limit: RateLimit::from_env("HORAE_API_RATE_LIMIT", RateLimit::default())?,
            },
            public_url: PublicUrl::from_env()?,
            oauth: OAuthConfig::from_env()?,
        })
    }
}

impl HarvestDeployment {
    /// A client ID without its secret (or the reverse) is ignored, as a partial
    /// configuration always was.
    fn from_env() -> Self {
        Self {
            client: non_empty("HORAE_HARVEST_CLIENT_ID")
                .zip(non_empty("HORAE_HARVEST_CLIENT_SECRET"))
                .map(|(client_id, client_secret)| HarvestClient {
                    client_id,
                    client_secret,
                }),
            redirect_url: non_empty("HORAE_HARVEST_REDIRECT_URL"),
            encryption_key_hex: non_empty("HORAE_HARVEST_ENC_KEY"),
        }
    }
}

impl OidcConfig {
    /// Returns `Some` only when all four OIDC env vars are present; a partial
    /// configuration is treated as "OIDC not configured" rather than a hard error,
    /// so `DEV_LOGIN` deployments need not set any of them. A malformed
    /// provisioning domain list is an error, so a typo cannot silently open or
    /// close sign-up.
    fn from_env() -> anyhow::Result<Option<Self>> {
        let auto_provision = AutoProvision {
            domains: parse_auto_provision_domains(
                non_empty("HORAE_OIDC_AUTO_PROVISION_DOMAINS")
                    .as_deref()
                    .unwrap_or_default(),
            )?,
            trust_unverified_email: std::env::var("HORAE_OIDC_TRUST_UNVERIFIED_EMAIL")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        };
        let (Some(issuer), Some(client_id), Some(client_secret), Some(redirect_url)) = (
            non_empty("HORAE_OIDC_ISSUER"),
            non_empty("HORAE_OIDC_CLIENT_ID"),
            non_empty("HORAE_OIDC_CLIENT_SECRET"),
            non_empty("HORAE_OIDC_REDIRECT_URL"),
        ) else {
            return Ok(None);
        };
        Ok(Some(Self {
            issuer,
            client_id,
            client_secret,
            redirect_url,
            additional_audiences: non_empty("HORAE_OIDC_ADDITIONAL_AUDIENCES")
                .map(|v| {
                    v.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            button_label: non_empty("HORAE_OIDC_BUTTON_LABEL")
                .unwrap_or_else(|| DEFAULT_OIDC_BUTTON_LABEL.into()),
            auto_provision,
        }))
    }
}

/// Parse `HORAE_OIDC_AUTO_PROVISION_DOMAINS`: comma-separated domains, trimmed
/// and lowercased. Empty input means provisioning is off. There is no
/// wildcard: without an approval step, "every domain" would let anyone the
/// provider authenticates join.
fn parse_auto_provision_domains(raw: &str) -> anyhow::Result<Vec<String>> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    raw.split(',')
        .map(|item| {
            let domain = item.trim();
            if domain.is_empty()
                || domain == "*"
                || domain.contains('@')
                || domain.contains(char::is_whitespace)
            {
                anyhow::bail!(
                    "HORAE_OIDC_AUTO_PROVISION_DOMAINS must be a comma-separated list of email \
                     domains (e.g. example.com,example.org); {item:?} is not a domain"
                );
            }
            Ok(domain.to_ascii_lowercase())
        })
        .collect()
}

/// An environment variable's value if it is set and non-empty.
fn non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Whether a bind address only accepts connections from the machine itself.
/// Anything that is not a loopback IP — including a hostname we cannot resolve
/// here — counts as reachable from elsewhere.
fn is_loopback_host(host: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

/// `DEV_LOGIN=1` registers `/auth/dev-login`, which hands an admin session to
/// anyone who asks for one. That is only ever safe when nothing outside the
/// machine can reach the port, so refuse to start rather than serve an open
/// admin door on a public interface.
pub fn check_dev_login_bind(dev_login: bool, host: &str) -> anyhow::Result<()> {
    if dev_login && !is_loopback_host(host) {
        anyhow::bail!(
            "DEV_LOGIN=1 grants an admin session to anyone who can reach the server, \
             but the bind address is {host}. Bind a loopback address (127.0.0.1) or \
             unset DEV_LOGIN."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_dev_login_bind;

    mod environment;

    #[test]
    fn harvest_secrets_stay_out_of_debug_and_serialized_configuration() {
        let effective = super::HarvestConfig {
            client_id: "client-id".into(),
            client_secret: "client-secret".into(),
            redirect_url: "https://horae.example/auth/harvest/callback".into(),
            encryption_key_hex: "encryption-key".into(),
        };
        let deployment = super::HarvestDeployment::from(effective.clone());
        for rendered in [
            format!("{effective:?}"),
            format!("{deployment:?}"),
            serde_json::to_string(&deployment).unwrap(),
        ] {
            assert!(rendered.contains("client-id"), "{rendered}");
            assert!(
                !rendered.contains("client-secret") && !rendered.contains("encryption-key"),
                "{rendered}"
            );
        }
    }

    #[test]
    fn dev_login_is_allowed_on_loopback() {
        for host in ["127.0.0.1", "::1", "[::1]", "localhost", "127.0.0.53"] {
            assert!(
                check_dev_login_bind(true, host).is_ok(),
                "refused the loopback bind {host}"
            );
        }
    }

    #[test]
    fn dev_login_is_refused_off_loopback() {
        // A wildcard bind, a routable address, and a hostname we cannot resolve.
        for host in ["0.0.0.0", "::", "192.168.1.10", "horae.example.com"] {
            assert!(
                check_dev_login_bind(true, host).is_err(),
                "served the dev-login bypass on {host}"
            );
        }
    }

    #[test]
    fn without_dev_login_any_bind_is_fine() {
        assert!(check_dev_login_bind(false, "0.0.0.0").is_ok());
    }

    #[test]
    fn auto_provision_domains_are_trimmed_and_lowercased() {
        assert_eq!(
            super::parse_auto_provision_domains(" Example.COM, example.org ").unwrap(),
            ["example.com", "example.org"]
        );
    }

    #[test]
    fn empty_auto_provision_domains_turn_provisioning_off() {
        for raw in ["", "  "] {
            assert!(super::parse_auto_provision_domains(raw).unwrap().is_empty());
        }
    }

    #[test]
    fn malformed_auto_provision_domains_are_refused() {
        for raw in [
            "*",
            "a.com,,b.com",
            "a.com,",
            "ada@a.com",
            "a .com",
            "a.com,*",
        ] {
            let error = super::parse_auto_provision_domains(raw).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("HORAE_OIDC_AUTO_PROVISION_DOMAINS"),
                "{raw}: {error}"
            );
        }
    }
}
