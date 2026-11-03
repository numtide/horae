//! The Harvest OAuth client an administrator stores from the importer: what the
//! page may see of it, and the rules a save must pass. The secret itself never
//! appears in these types.

use serde::{Deserialize, Serialize};

/// The only path Harvest may redirect back to (`/auth/harvest/callback`).
pub const CALLBACK_PATH: &str = "/auth/harvest/callback";

/// The server's refusal of a change that would disconnect Harvest without the
/// caller's confirmation; the page answers it by asking for confirmation.
pub const DISCONNECT_UNCONFIRMED: &str =
    "This change disconnects Harvest. Review the warning and confirm to continue.";

/// Where the client the importer uses comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientSource {
    /// `HORAE_HARVEST_CLIENT_ID` and `HORAE_HARVEST_CLIENT_SECRET`; read-only.
    Environment,
    /// Saved by an administrator for this organization.
    Stored,
    #[default]
    None,
}

/// What an administrator sees of the client. Never carries the secret.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSettings {
    pub source: ClientSource,
    pub client_id: Option<String>,
    pub secret_set: bool,
    /// False when the stored secret no longer opens with the deployment key.
    pub secret_readable: bool,
    /// When the stored client was last saved (RFC 3339).
    pub stored_at: Option<String>,
    pub redirect_url: Option<String>,
    pub redirect_from_environment: bool,
    pub encryption_key_set: bool,
    /// A stored client exists but the environment client takes precedence.
    pub stored_ignored: bool,
}

impl ClientSettings {
    /// Whether the page may offer save and remove.
    pub fn editable(&self) -> bool {
        self.encryption_key_set && self.source != ClientSource::Environment
    }

    /// A client saved while `HORAE_HARVEST_REDIRECT_URL` was set stores no
    /// redirect URL of its own; once that variable is unset it needs a save.
    pub fn needs_redirect(&self) -> bool {
        self.source == ClientSource::Stored && self.redirect_url.is_none()
    }
}

/// A save request after trimming. `client_secret: None` keeps the stored one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveClient {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_url: Option<String>,
}

/// The stored client a save replaces, as far as the rules need it.
#[derive(Debug, Clone, Copy)]
pub struct Existing<'a> {
    pub client_id: &'a str,
    pub secret_readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidClient {
    #[error("Enter the client ID from the Harvest OAuth2 app (at most 255 characters, no spaces)")]
    ClientId,
    #[error("Enter the client secret from the Harvest OAuth2 app (at most 1024 characters)")]
    Secret,
    #[error("Enter the client secret again: a new client ID needs its own secret")]
    SecretRequired,
    #[error(
        "The redirect URL must be an https address (http only for localhost) ending in {CALLBACK_PATH}"
    )]
    RedirectUrl,
}

/// Trim and validate a save. `environment_redirect` drops the submitted
/// redirect URL, since the environment value is the one Harvest must match.
pub fn validate_save(
    client_id: &str,
    client_secret: Option<&str>,
    redirect_url: &str,
    environment_redirect: bool,
    existing: Option<Existing<'_>>,
) -> Result<SaveClient, InvalidClient> {
    let client_id = client_id.trim();
    if client_id.is_empty() || client_id.len() > 255 || client_id.contains(char::is_whitespace) {
        return Err(InvalidClient::ClientId);
    }
    let client_secret = client_secret.map(str::trim).filter(|s| !s.is_empty());
    if client_secret.is_some_and(|s| s.len() > 1024) {
        return Err(InvalidClient::Secret);
    }
    let keeps_secret = existing.is_some_and(|e| e.client_id == client_id && e.secret_readable);
    if client_secret.is_none() && !keeps_secret {
        return Err(if existing.is_some() {
            InvalidClient::SecretRequired
        } else {
            InvalidClient::Secret
        });
    }
    let redirect_url = if environment_redirect {
        None
    } else {
        let url = redirect_url.trim();
        if !valid_redirect_url(url) {
            return Err(InvalidClient::RedirectUrl);
        }
        Some(url.to_owned())
    };
    Ok(SaveClient {
        client_id: client_id.to_owned(),
        client_secret: client_secret.map(str::to_owned),
        redirect_url,
    })
}

/// Harvest tokens belong to the client that issued them, so saving a
/// different client ID ends the current connection.
pub fn save_disconnects(existing: Option<&str>, client_id: &str) -> bool {
    existing.is_some_and(|current| current != client_id)
}

/// `https://host[:port]/auth/harvest/callback`, with no user info, query or
/// fragment: anything else would send the authorization code elsewhere.
/// Plain `http` is accepted only for a loopback host, where the code never
/// crosses the network.
fn valid_redirect_url(url: &str) -> bool {
    let (rest, secure) = match url.strip_prefix("https://") {
        Some(rest) => (rest, true),
        None => match url.strip_prefix("http://") {
            Some(rest) => (rest, false),
            None => return false,
        },
    };
    let Some(authority) = rest.strip_suffix(CALLBACK_PATH) else {
        return false;
    };
    // The host is everything before an optional `:port`; IPv6 is bracketed.
    let (host, port) = match authority.strip_prefix('[') {
        Some(inner) => match inner.split_once(']') {
            Some((address, port)) => (&authority[..address.len() + 2], port),
            None => return false,
        },
        None => authority
            .find(':')
            .map_or((authority, ""), |colon| authority.split_at(colon)),
    };
    let port_ok = port.is_empty()
        || port
            .strip_prefix(':')
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    !host.is_empty()
        && port_ok
        && url.len() <= 2048
        && !authority.contains(['/', '@', '?', '#', '\\'])
        && !url.contains(char::is_whitespace)
        && (secure || matches!(host, "localhost" | "127.0.0.1" | "[::1]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://horae.example.com/auth/harvest/callback";

    fn save(
        id: &str,
        secret: Option<&str>,
        existing: Option<Existing<'_>>,
    ) -> Result<SaveClient, InvalidClient> {
        validate_save(id, secret, URL, false, existing)
    }

    #[test]
    fn first_save_trims_and_requires_a_secret() {
        assert_eq!(
            save("  id  ", Some(" secret "), None),
            Ok(SaveClient {
                client_id: "id".into(),
                client_secret: Some("secret".into()),
                redirect_url: Some(URL.into()),
            })
        );
        assert_eq!(save("id", None, None), Err(InvalidClient::Secret));
        assert_eq!(save("id", Some("   "), None), Err(InvalidClient::Secret));
    }

    #[test]
    fn client_id_and_secret_are_bounded_and_not_blank() {
        assert_eq!(save(" ", Some("s"), None), Err(InvalidClient::ClientId));
        assert_eq!(save("a b", Some("s"), None), Err(InvalidClient::ClientId));
        assert_eq!(
            save(&"a".repeat(256), Some("s"), None),
            Err(InvalidClient::ClientId)
        );
        assert!(save(&"a".repeat(255), Some("s"), None).is_ok());
        assert_eq!(
            save("id", Some(&"s".repeat(1025)), None),
            Err(InvalidClient::Secret)
        );
    }

    #[test]
    fn blank_secret_keeps_only_a_readable_secret_of_the_same_client() {
        let same = Existing {
            client_id: "id",
            secret_readable: true,
        };
        assert_eq!(save("id", None, Some(same)).unwrap().client_secret, None);
        assert_eq!(
            save("other", None, Some(same)),
            Err(InvalidClient::SecretRequired)
        );
        let unreadable = Existing {
            client_id: "id",
            secret_readable: false,
        };
        assert_eq!(
            save("id", None, Some(unreadable)),
            Err(InvalidClient::SecretRequired)
        );
        assert!(save("other", Some("new"), Some(same)).is_ok());
    }

    #[test]
    fn redirect_url_must_be_the_harvest_callback_on_a_plain_origin() {
        for url in [
            URL,
            "https://horae.example.com:8443/auth/harvest/callback",
            "http://localhost:8080/auth/harvest/callback",
            "http://localhost/auth/harvest/callback",
            "http://127.0.0.1:8080/auth/harvest/callback",
            "http://[::1]:8080/auth/harvest/callback",
            "https://[::1]:8443/auth/harvest/callback",
        ] {
            assert!(
                validate_save("id", Some("s"), url, false, None).is_ok(),
                "{url}"
            );
        }
        for url in [
            "",
            "ftp://horae.example.com/auth/harvest/callback",
            "https:///auth/harvest/callback",
            "https://horae.example.com/auth/harvest/callback?x=1",
            "https://horae.example.com/auth/harvest/callback#x",
            "https://horae.example.com/other/auth/harvest/callback",
            "https://user@horae.example.com/auth/harvest/callback",
            "https://horae.example.com/auth/harvest",
            "https://horae.example.com /auth/harvest/callback",
            // Plain http leaves loopback only for an explicit loopback host.
            "http://horae.example.com/auth/harvest/callback",
            "http://10.0.0.5:8080/auth/harvest/callback",
            "http://localhost.example.com/auth/harvest/callback",
            "http://127.0.0.1.example.com/auth/harvest/callback",
            "http://[::2]/auth/harvest/callback",
            "https://horae.example.com:/auth/harvest/callback",
            "https://horae.example.com:x/auth/harvest/callback",
            "https://[::1/auth/harvest/callback",
        ] {
            assert_eq!(
                validate_save("id", Some("s"), url, false, None),
                Err(InvalidClient::RedirectUrl),
                "{url}"
            );
        }
    }

    #[test]
    fn environment_redirect_url_replaces_the_submitted_one() {
        let saved = validate_save("id", Some("s"), "not a url", true, None).unwrap();
        assert_eq!(saved.redirect_url, None);
    }

    #[test]
    fn only_a_different_client_id_disconnects() {
        assert!(!save_disconnects(None, "id"));
        assert!(!save_disconnects(Some("id"), "id"));
        assert!(save_disconnects(Some("id"), "other"));
    }

    #[test]
    fn only_a_stored_client_without_any_redirect_url_needs_one() {
        let mut settings = ClientSettings {
            source: ClientSource::Stored,
            ..Default::default()
        };
        assert!(settings.needs_redirect());
        settings.redirect_url = Some(URL.into());
        assert!(!settings.needs_redirect());
        settings.redirect_url = None;
        settings.source = ClientSource::Environment;
        assert!(!settings.needs_redirect());
    }

    #[test]
    fn environment_client_and_missing_key_are_not_editable() {
        let mut settings = ClientSettings {
            encryption_key_set: true,
            ..Default::default()
        };
        assert!(settings.editable());
        settings.source = ClientSource::Environment;
        assert!(!settings.editable());
        settings.source = ClientSource::Stored;
        settings.encryption_key_set = false;
        assert!(!settings.editable());
    }
}
