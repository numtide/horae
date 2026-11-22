//! OAuth 2.0 rules (spec 024): the scopes an app may ask for and the
//! permissions each one lets through, which redirect URIs an app may register
//! and how a request's URI is matched against them, and the PKCE S256 check.

use std::net::{Ipv4Addr, Ipv6Addr};

use super::catalog::{Permission, PermissionSelection};

/// One scope of the `scope` parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    TimeRead,
    TimeWrite,
    CatalogRead,
}

impl Scope {
    pub const ALL: [Self; 3] = [Self::TimeRead, Self::TimeWrite, Self::CatalogRead];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TimeRead => "time:read",
            Self::TimeWrite => "time:write",
            Self::CatalogRead => "catalog:read",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|scope| scope.as_str() == value)
    }

    const fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The permissions this scope lets through (research R-6). Each list is
    /// closed under prerequisites.
    const fn permissions(self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Self::TimeRead => &[TimeReadOwn, TimeReadManaged, TimeReadAll],
            Self::TimeWrite => &[
                TimeReadOwn,
                TimeReadManaged,
                TimeReadAll,
                TimeWriteOwn,
                TimeWriteManaged,
                TimeWriteAll,
            ],
            Self::CatalogRead => &[
                ProjectReadManaged,
                ProjectReadAll,
                ClientReadAll,
                TaskReadAll,
                PeopleReadManaged,
                PeopleReadAll,
            ],
        }
    }
}

/// A scope named in a request or a registration that Horae does not define.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Unknown scope {0:?}")]
pub struct UnknownScope(pub String);

/// A set of scopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ScopeSet(u8);

impl ScopeSet {
    pub const EMPTY: Self = Self(0);

    pub fn of(scopes: &[Scope]) -> Self {
        Self(scopes.iter().fold(0, |bits, scope| bits | scope.bit()))
    }

    /// A space-separated `scope` parameter (RFC 6749 §3.3). Repeated scopes
    /// count once; an empty string is the empty set.
    ///
    /// # Errors
    /// The first scope Horae does not define.
    pub fn parse_list(value: &str) -> Result<Self, UnknownScope> {
        Self::parse_each(value.split(' ').filter(|scope| !scope.is_empty()))
    }

    /// Scopes stored one per array element.
    ///
    /// # Errors
    /// The first scope Horae does not define.
    pub fn parse_each<'a>(scopes: impl IntoIterator<Item = &'a str>) -> Result<Self, UnknownScope> {
        scopes.into_iter().try_fold(Self::EMPTY, |set, scope| {
            Scope::parse(scope)
                .map(|scope| set.with(scope))
                .ok_or_else(|| UnknownScope(scope.to_owned()))
        })
    }

    #[must_use]
    pub const fn with(self, scope: Scope) -> Self {
        Self(self.0 | scope.bit())
    }

    pub const fn contains(self, scope: Scope) -> bool {
        self.0 & scope.bit() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn is_subset(self, of: Self) -> bool {
        self.0 & !of.0 == 0
    }

    /// `time:write` includes `time:read` (FR-060).
    #[must_use]
    pub const fn normalize(self) -> Self {
        if self.contains(Scope::TimeWrite) {
            self.with(Scope::TimeRead)
        } else {
            self
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Scope> {
        Scope::ALL
            .into_iter()
            .filter(move |scope| self.contains(*scope))
    }

    /// The names in a fixed order, for storage in a `text[]` column.
    pub fn names(self) -> Vec<&'static str> {
        self.iter().map(Scope::as_str).collect()
    }

    /// The space-separated form of the `scope` parameter.
    pub fn display(self) -> String {
        self.names().join(" ")
    }

    /// Every permission these scopes let through. An OAuth request acts with
    /// the person's grants intersected with this, so a scope only narrows.
    pub fn ceiling(self) -> PermissionSelection {
        let permissions: Vec<Permission> = self
            .iter()
            .flat_map(|scope| scope.permissions().iter().copied())
            .collect();
        PermissionSelection::closure_of(&permissions)
    }
}

/// Whether an app can keep a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientKind {
    /// A native or browser app: no secret, PKCE only.
    Public,
    /// A server-side app with a secret.
    Confidential,
}

impl ClientKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Confidential => "confidential",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Self::Public, Self::Confidential]
            .into_iter()
            .find(|kind| kind.as_str() == value)
    }
}

/// Why a redirect URI cannot be registered (FR-011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RedirectUriError {
    #[error("Enter an absolute URI")]
    Invalid,
    #[error("A redirect URI cannot have a fragment")]
    Fragment,
    #[error("A redirect URI cannot carry a user name or password")]
    UserInfo,
    #[error("A redirect URI cannot contain wildcards")]
    Wildcard,
    #[error("Use 127.0.0.1 or [::1] instead of localhost")]
    Localhost,
    #[error("http is allowed only to 127.0.0.1 or [::1]")]
    HttpNotLoopback,
    #[error("Use https, a loopback http URI, or an app scheme such as com.example.app:/callback")]
    Scheme,
    #[error("Only public apps may use loopback or app-scheme redirect URIs")]
    PublicOnly,
}

/// A registered redirect URI, kept exactly as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectUri {
    value: String,
    /// For a loopback `http` URI, the length of `http://<address>`: the port
    /// that follows may differ at request time (RFC 8252 §7.3).
    loopback_authority: Option<usize>,
}

const LOOPBACK_AUTHORITIES: [&str; 2] = ["http://127.0.0.1", "http://[::1]"];

impl RedirectUri {
    /// Validates a URI for registration by an app of `kind`.
    ///
    /// # Errors
    /// The first rule of FR-011 the URI breaks.
    pub fn parse(value: &str, kind: ClientKind) -> Result<Self, RedirectUriError> {
        if value.contains('*') {
            return Err(RedirectUriError::Wildcard);
        }
        if value.contains('#') {
            return Err(RedirectUriError::Fragment);
        }
        let url = url::Url::parse(value).map_err(|_| RedirectUriError::Invalid)?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(RedirectUriError::UserInfo);
        }
        if url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("localhost"))
        {
            return Err(RedirectUriError::Localhost);
        }
        let public_only = |uri: Self| match kind {
            ClientKind::Public => Ok(uri),
            ClientKind::Confidential => Err(RedirectUriError::PublicOnly),
        };
        match url.scheme() {
            "https" if url.host().is_some() => Ok(Self {
                value: value.to_owned(),
                loopback_authority: None,
            }),
            "https" => Err(RedirectUriError::Invalid),
            "http" => {
                let loopback = matches!(
                    url.host(),
                    Some(url::Host::Ipv4(ip)) if ip == Ipv4Addr::LOCALHOST
                ) || matches!(
                    url.host(),
                    Some(url::Host::Ipv6(ip)) if ip == Ipv6Addr::LOCALHOST
                );
                // The URL parser also reads `127.1` or `0x7f.0.0.1` as the
                // loopback address; only the literal forms are accepted, so
                // matching can compare strings.
                let authority = LOOPBACK_AUTHORITIES
                    .into_iter()
                    .find(|authority| {
                        value.strip_prefix(authority).is_some_and(|rest| {
                            rest.is_empty() || rest.starts_with([':', '/', '?'])
                        })
                    })
                    .filter(|_| loopback)
                    .ok_or(RedirectUriError::HttpNotLoopback)?;
                public_only(Self {
                    value: value.to_owned(),
                    loopback_authority: Some(authority.len()),
                })
            }
            // A private-use scheme in reverse domain name form (RFC 8252 §7.1).
            scheme if scheme.contains('.') => public_only(Self {
                value: value.to_owned(),
                loopback_authority: None,
            }),
            _ => Err(RedirectUriError::Scheme),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Whether a request's `redirect_uri` is this one: the exact string, or for
    /// a loopback URI the exact string with any port (FR-010).
    pub fn matches(&self, requested: &str) -> bool {
        if requested == self.value {
            return true;
        }
        let Some(authority) = self.loopback_authority else {
            return false;
        };
        let (prefix, registered_rest) = self.value.split_at(authority);
        let registered_rest = strip_port(registered_rest);
        requested
            .strip_prefix(prefix)
            .is_some_and(|rest| strip_port(rest) == registered_rest)
    }
}

/// What follows an authority without its `:<port>`, if it has one.
fn strip_port(rest: &str) -> &str {
    let Some(after_colon) = rest.strip_prefix(':') else {
        return rest;
    };
    let digits = after_colon.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=5).contains(&digits) {
        &after_colon[digits..]
    } else {
        // Not a port: leave it, so it can never equal a registered path.
        rest
    }
}

/// Proof Key for Code Exchange, S256 only (RFC 7636).
pub mod pkce {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    /// Whether `verifier` is well formed (43 to 128 unreserved characters,
    /// RFC 7636 §4.1) and `BASE64URL(SHA256(verifier))` equals `challenge`,
    /// compared in constant time.
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        let well_formed = (43..=128).contains(&verifier.len())
            && verifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte));
        if !well_formed {
            return false;
        }
        let computed = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        computed.as_bytes().ct_eq(challenge.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod properties;
