//! The Projects list's URL query: `?status=archived&client=<id>&tag=<id>&q=<search>`.

use std::fmt;

use uuid::Uuid;

/// Which projects the list shows by status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectScope {
    #[default]
    Active,
    /// Active projects with a budget.
    Budgeted,
    Archived,
}

impl ProjectScope {
    pub const ALL: [Self; 3] = [Self::Active, Self::Budgeted, Self::Archived];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Budgeted => "budgeted",
            Self::Archived => "archived",
        }
    }
}

/// What the Projects list shows, all of it in the URL so a view can be
/// shared and Back returns to it. The default is the active projects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectFilter {
    pub scope: ProjectScope,
    pub client: Option<Uuid>,
    pub tag: Option<Uuid>,
    pub search: String,
}

/// Reads the query leniently: a value it does not know is left out. The
/// router decodes the query once before this, so the search, written last,
/// is everything after `q=`, `&` and `=` included.
impl From<&str> for ProjectFilter {
    fn from(query: &str) -> Self {
        let mut filter = Self::default();
        let mut rest = query;
        while !rest.is_empty() {
            if let Some(search) = rest.strip_prefix("q=") {
                filter.search = search.to_owned();
                break;
            }
            let (pair, tail) = rest.split_once('&').unwrap_or((rest, ""));
            rest = tail;
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            match key {
                "status" => {
                    filter.scope = ProjectScope::ALL
                        .into_iter()
                        .find(|s| s.as_str() == value)
                        .unwrap_or_default()
                }
                "client" => filter.client = Uuid::parse_str(value).ok(),
                "tag" => filter.tag = Uuid::parse_str(value).ok(),
                _ => {}
            }
        }
        filter
    }
}

impl fmt::Display for ProjectFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.scope != ProjectScope::Active {
            parts.push(format!("status={}", self.scope.as_str()));
        }
        if let Some(client) = self.client {
            parts.push(format!("client={client}"));
        }
        if let Some(tag) = self.tag {
            parts.push(format!("tag={tag}"));
        }
        if !self.search.is_empty() {
            // The router encodes what it must and decodes the query once
            // before `from`, but leaves `%` alone, so a literal `%` is escaped.
            parts.push(format!("q={}", self.search.replace('%', "%25")));
        }
        f.write_str(&parts.join("&"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the router hands `from`: the query decoded once.
    fn decoded(written: &str) -> String {
        let mut out = String::new();
        let mut chars = written.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                out.push(u8::from_str_radix(&hex, 16).unwrap() as char);
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn every_filter_survives_the_url() {
        let filter = ProjectFilter {
            scope: ProjectScope::Budgeted,
            client: Some(Uuid::now_v7()),
            tag: Some(Uuid::now_v7()),
            search: "A&B = 100% #2".into(),
        };
        assert_eq!(
            ProjectFilter::from(decoded(&filter.to_string()).as_str()),
            filter
        );
    }

    #[test]
    fn the_default_writes_no_query_and_unknown_values_are_dropped() {
        assert_eq!(ProjectFilter::default().to_string(), "");
        assert_eq!(
            ProjectFilter::from("status=lost&client=nope&tag=nope&shape=round"),
            ProjectFilter::default()
        );
    }
}
