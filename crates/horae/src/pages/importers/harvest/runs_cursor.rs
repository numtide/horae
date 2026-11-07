//! The run history's `?before=` query: the last run of the previous page.

use std::fmt;

use uuid::Uuid;

/// Where a page of run history starts. The newest page has no cursor and so no
/// query; anything but `before=<uuid>` reads as the newest page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunsCursor(pub Option<Uuid>);

impl From<&str> for RunsCursor {
    fn from(query: &str) -> Self {
        Self(
            query
                .strip_prefix("before=")
                .and_then(|id| Uuid::parse_str(id).ok()),
        )
    }
}

impl fmt::Display for RunsCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(id) => write!(f, "before={id}"),
            None => Ok(()),
        }
    }
}
