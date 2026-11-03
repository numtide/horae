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
    /// Execution limits copied into newly enqueued jobs.
    pub job_policy: JobPolicy,
    /// Optional budget email transport; no process is started when absent.
    pub mail: Option<MailConfig>,
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
#[cfg(test)]
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
            oidc: OidcConfig::from_env(),
            secure_cookies: std::env::var("HORAE_SECURE_COOKIES")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            harvest: HarvestDeployment::from_env(),
            job_policy: JobPolicy::from_env()?,
            mail: MailConfig::parse(
                non_empty("HORAE_SENDMAIL_PATH").as_deref(),
                non_empty("HORAE_MAIL_FROM").as_deref(),
            )?,
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
    /// so `DEV_LOGIN` deployments need not set any of them.
    fn from_env() -> Option<Self> {
        Some(Self {
            issuer: non_empty("HORAE_OIDC_ISSUER")?,
            client_id: non_empty("HORAE_OIDC_CLIENT_ID")?,
            client_secret: non_empty("HORAE_OIDC_CLIENT_SECRET")?,
            redirect_url: non_empty("HORAE_OIDC_REDIRECT_URL")?,
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
        })
    }
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
}
