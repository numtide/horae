//! The editor's `?return_to=` query. It uses only `std` and `uuid`, with no UI,
//! router or crate-internal imports, so the SSR route mirrors in `tests/` can
//! include this exact file.

use std::fmt;

use uuid::Uuid;

/// Where the project editor was opened from. Only the exact canonical query
/// `return_to=/projects/<lowercase hyphenated UUID>` is kept; anything else,
/// including extra or repeated parameters, is dropped. The router hands
/// `From<&str>` the whole query decoded once, and serializes only `Display`,
/// so reparsing a serialized route preserves the destination. The query alone
/// cannot know which project is being edited: a canonical target naming another
/// project is kept, and `returns_to` is what binds it to the edited project.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditReturn(Option<Uuid>);

impl EditReturn {
    /// Return to the detail of project `id`.
    pub fn detail(id: Uuid) -> Self {
        Self(Some(id))
    }

    /// Whether this names the detail of the project being edited.
    pub fn returns_to(self, id: Uuid) -> bool {
        self.0 == Some(id)
    }
}

impl From<&str> for EditReturn {
    fn from(query: &str) -> Self {
        Self(query.strip_prefix("return_to=/projects/").and_then(|id| {
            Uuid::parse_str(id)
                .ok()
                .filter(|uuid| uuid.to_string() == id)
        }))
    }
}

impl fmt::Display for EditReturn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(id) => write!(f, "return_to=/projects/{id}"),
            None => Ok(()),
        }
    }
}
