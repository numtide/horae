//! The queries the OAuth callback and its banner put on Harvest pages.

use std::fmt;

use horae_core::importers::harvest::client::ConnectFailure;

/// The Harvest page's query: why the last connection attempt failed, if it
/// did. Anything else (`?connected=1`, the old `?error=1`) reads as nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConnectOutcome(pub Option<ConnectFailure>);

impl From<&str> for ConnectOutcome {
    fn from(query: &str) -> Self {
        Self(ConnectFailure::parse(query))
    }
}

impl fmt::Display for ConnectOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(failure) => f.write_str(&failure.query()),
            None => Ok(()),
        }
    }
}

/// The OAuth app page's query: `focus=secret` opens it ready for a new secret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SecretFocus(pub bool);

impl From<&str> for SecretFocus {
    fn from(query: &str) -> Self {
        Self(query == "focus=secret")
    }
}

impl fmt::Display for SecretFocus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 {
            f.write_str("focus=secret")
        } else {
            Ok(())
        }
    }
}
