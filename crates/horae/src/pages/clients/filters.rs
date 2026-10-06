use crate::models::client::ClientSummary;

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum ClientScope {
    #[default]
    Active,
    Inactive,
    All,
}

impl ClientScope {
    pub(super) fn matches(self, active: bool) -> bool {
        match self {
            Self::Active => active,
            Self::Inactive => !active,
            Self::All => true,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Active => "Active clients",
            Self::Inactive => "Inactive clients",
            Self::All => "All clients",
        }
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
