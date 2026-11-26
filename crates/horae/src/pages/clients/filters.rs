//! The Clients list's filters and its URL query: `?status=archived&currency=EUR&q=<search>`.

use std::fmt;

use crate::models::client::ClientSummary;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClientScope {
    #[default]
    Active,
    Inactive,
    All,
}

impl ClientScope {
    pub(super) const ALL: [Self; 3] = [Self::Active, Self::Inactive, Self::All];

    pub(super) fn matches(self, active: bool) -> bool {
        match self {
            Self::Active => active,
            Self::Inactive => !active,
            Self::All => true,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "archived",
            Self::All => "all",
        }
    }
}

/// What the Clients list shows, all of it in the URL so a view can be shared
/// and Back returns to it. The default is the active clients.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientFilter {
    pub scope: ClientScope,
    /// A currency code; empty for every currency.
    pub currency: String,
    pub search: String,
}

impl ClientFilter {
    /// Every client: no status, currency or search.
    pub(super) fn cleared() -> Self {
        Self {
            scope: ClientScope::All,
            ..Self::default()
        }
    }
}

/// Reads the query leniently: a value it does not know is left out. The
/// router decodes the query once before this, so the search, written last,
/// is everything after `q=`, `&` and `=` included.
impl From<&str> for ClientFilter {
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
                    filter.scope = ClientScope::ALL
                        .into_iter()
                        .find(|s| s.as_str() == value)
                        .unwrap_or_default()
                }
                "currency" => filter.currency = value.trim().to_owned(),
                _ => {}
            }
        }
        filter
    }
}

impl fmt::Display for ClientFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.scope != ClientScope::Active {
            parts.push(format!("status={}", self.scope.as_str()));
        }
        if !self.currency.is_empty() {
            parts.push(format!("currency={}", self.currency));
        }
        if !self.search.is_empty() {
            // The router encodes what it must and decodes the query once
            // before `from`, but leaves `%` alone, so a literal `%` is escaped.
            parts.push(format!("q={}", self.search.replace('%', "%25")));
        }
        f.write_str(&parts.join("&"))
    }
}

pub(super) fn matches_query_currency(client: &ClientSummary, query: &str, currency: &str) -> bool {
    client
        .client
        .name
        .to_lowercase()
        .contains(&query.trim().to_lowercase())
        && (currency.is_empty()
            || client.client.currency.trim() == currency
            || client
                .project_currencies
                .iter()
                .any(|c| c.trim() == currency))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Client;
    use uuid::Uuid;

    fn summary(
        name: &str,
        active: bool,
        currency: &str,
        project_currencies: &[&str],
    ) -> ClientSummary {
        ClientSummary {
            client: Client {
                id: Uuid::now_v7(),
                org_id: Uuid::nil(),
                name: name.into(),
                currency: currency.into(),
                address: None,
                tax_id: None,
                active,
                created_at: chrono::DateTime::UNIX_EPOCH,
            },
            active_projects: 0,
            total_projects: 0,
            project_currencies: project_currencies.iter().map(|s| (*s).into()).collect(),
        }
    }

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
        let filter = ClientFilter {
            scope: ClientScope::Inactive,
            currency: "EUR".into(),
            search: "A&B = 100% #2".into(),
        };
        assert_eq!(
            ClientFilter::from(decoded(&filter.to_string()).as_str()),
            filter
        );
    }

    #[test]
    fn the_default_writes_no_query_and_unknown_values_are_dropped() {
        assert_eq!(ClientFilter::default().to_string(), "");
        assert_eq!(
            ClientFilter::from("status=gone&shape=round"),
            ClientFilter::default()
        );
        assert_eq!(ClientFilter::from("status=all").scope, ClientScope::All);
    }

    #[test]
    fn name_search_ignores_case_and_surrounding_whitespace() {
        let client = summary("Rún Collective", true, "EUR", &[]);
        assert!(matches_query_currency(&client, "  RÚN  ", ""));
        assert!(!matches_query_currency(&client, "absent", ""));
    }

    #[test]
    fn preferred_or_visible_project_currency_matches_but_query_still_applies() {
        let client = summary("Studio", true, "EUR", &["USD", "GBP"]);
        for currency in ["EUR", "USD", "GBP", ""] {
            assert!(matches_query_currency(&client, "studio", currency));
            assert!(!matches_query_currency(&client, "another", currency));
        }
        assert!(!matches_query_currency(&client, "", "CHF"));
    }

    #[test]
    fn lifecycle_counts_share_query_and_currency_filters() {
        let list = [
            summary("Acme", true, "EUR", &["USD"]),
            summary("Acme", false, "USD", &[]),
            summary("Other", true, "USD", &[]),
            summary("Acme", false, "CHF", &[]),
        ];
        let counts: Vec<_> = [ClientScope::Active, ClientScope::Inactive, ClientScope::All]
            .into_iter()
            .map(|scope| {
                list.iter()
                    .filter(|row| {
                        matches_query_currency(row, "Acme", "USD")
                            && scope.matches(row.client.active)
                    })
                    .count()
            })
            .collect();
        assert_eq!(counts, [1, 1, 2]);
    }

    #[test]
    fn clearing_query_preserves_selected_currency_and_lifecycle() {
        let client = summary("A <long> client", false, "EUR", &[]);
        assert!(matches_query_currency(&client, " ", "EUR"));
        assert!(!ClientScope::Active.matches(client.client.active));
        assert!(ClientScope::Inactive.matches(client.client.active));
        assert!(!matches_query_currency(&client, "", "USD"));
    }
}
