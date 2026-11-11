//! The Harvest-compatible API's response shapes (the app serves them under
//! `/harvest/v2`), with hours and money as exact JSON numbers: written from
//! integer minutes and minor units as decimal text, never through `f64`
//! (Constitution I, decimal wire formats). This crate enables serde_json's
//! `arbitrary_precision`, so a `Number` parsed from that text is written back
//! verbatim.

use horae_core::duration::format_hours2;
use horae_core::money::format_major_units;
use serde::{Serialize, Serializer};
use std::collections::HashMap;

/// Integer minutes, written as hours rounded half up to two decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hours(pub i64);

/// Integer minor units, written in major units with the currency's own
/// decimals (`money::format_major_units`). An unknown currency gets two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    pub cents: i64,
    pub currency: String,
}

impl Amount {
    pub fn new(cents: i64, currency: &str) -> Self {
        Self {
            cents,
            currency: currency.to_owned(),
        }
    }
}

fn number<S: Serializer>(text: String, serializer: S) -> Result<S::Ok, S::Error> {
    text.parse::<serde_json::Number>()
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

impl Serialize for Hours {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        number(format_hours2(self.0), serializer)
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        number(format_major_units(self.cents, &self.currency), serializer)
    }
}

// ── Pagination envelope ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestPagination<T: Serialize> {
    #[serde(flatten)]
    pub data: HashMap<String, Vec<T>>,
    pub per_page: i64,
    pub total_pages: i64,
    pub total_entries: i64,
    pub page: i64,
    pub next_page: Option<i64>,
    pub previous_page: Option<i64>,
    pub links: HarvestLinks,
}

impl<T: Serialize> HarvestPagination<T> {
    pub fn new(
        key: &str,
        items: Vec<T>,
        page: i64,
        per_page: i64,
        total_entries: i64,
        base_url: &str,
    ) -> Self {
        let total_pages = if total_entries == 0 {
            1
        } else {
            (total_entries - 1) / per_page + 1
        };
        let next_page = if page < total_pages {
            Some(page + 1)
        } else {
            None
        };
        let previous_page = if page > 1 { Some(page - 1) } else { None };

        let links = HarvestLinks {
            first: format!("{}?page=1&per_page={}", base_url, per_page),
            next: next_page.map(|p| format!("{}?page={}&per_page={}", base_url, p, per_page)),
            previous: previous_page
                .map(|p| format!("{}?page={}&per_page={}", base_url, p, per_page)),
            last: format!("{}?page={}&per_page={}", base_url, total_pages, per_page),
        };

        let mut data = HashMap::new();
        data.insert(key.to_string(), items);

        Self {
            data,
            per_page,
            total_pages,
            total_entries,
            page,
            next_page,
            previous_page,
            links,
        }
    }

    /// Keep the original filter values on every link, while the envelope owns
    /// the canonical page window. Encoding parsed pairs preserves literal `+`,
    /// delimiters, Unicode, and repeated extension parameters.
    pub fn with_query(mut self, query: Option<&str>) -> Self {
        let Some(query) = query else { return self };
        let mut encoded = url::form_urlencoded::Serializer::new(String::new());
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key != "page" && key != "per_page" {
                encoded.append_pair(&key, &value);
            }
        }
        let filters = encoded.finish();
        if !filters.is_empty() {
            for link in [
                Some(&mut self.links.first),
                Some(&mut self.links.last),
                self.links.next.as_mut(),
                self.links.previous.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                link.push('&');
                link.push_str(&filters);
            }
        }
        self
    }
}

#[derive(Serialize)]
pub struct HarvestLinks {
    pub first: String,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub last: String,
}

// ── Resource DTOs ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct HarvestRef {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct HarvestProjectRef {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
}

// ── Time Entry ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestTimeEntry {
    pub id: i64,
    pub spent_date: String,
    pub hours: Hours,
    pub rounded_hours: Hours,
    /// Wall-clock start/end (Harvest tracks by start/end times); null when the
    /// entry is duration-only (no start time).
    pub started_time: Option<String>,
    pub ended_time: Option<String>,
    pub notes: Option<String>,
    pub is_locked: bool,
    pub locked_reason: Option<String>,
    pub is_closed: bool,
    pub is_billed: bool,
    pub is_running: bool,
    pub timer_started_at: Option<String>,
    pub billable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budgeted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billable_rate: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_rate: Option<Amount>,
    pub created_at: String,
    pub updated_at: String,
    pub user: HarvestRef,
    pub client: HarvestRef,
    pub project: HarvestProjectRef,
    pub task: HarvestRef,
    pub approval_status: String,
}

// ── Project ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestProject {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    pub is_active: bool,
    pub is_billable: bool,
    pub bill_by: String,
    pub budget_by: String,
    pub budget: Option<Budget>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub client: HarvestRef,
}

/// A project budget in its own unit: hours for an hours budget, money for an
/// amount budget.
#[derive(Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Budget {
    Hours(Hours),
    Amount(Amount),
}

// ── Client ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestClient {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestTask {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub billable_by_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_hourly_rate: Option<Amount>,
    pub created_at: String,
    pub updated_at: String,
}

// ── User ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestUser {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub is_active: bool,
    pub is_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_rate: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_hourly_rate: Option<Amount>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_and_amounts_are_exact_json_numbers() {
        let json = serde_json::to_string(&(
            Hours(1),
            Hours(90),
            Amount::new(1234, "EUR"),
            Amount::new(-5, "USD"),
            Amount::new(100_000, "JPY"),
            Amount::new(1234, "BHD"),
        ))
        .unwrap();
        assert_eq!(json, "[0.02,1.50,12.34,-0.05,1000,12.340]");
    }

    #[test]
    fn large_amounts_keep_every_digit() {
        let json = serde_json::to_string(&Amount::new(i64::MAX, "EUR")).unwrap();
        assert_eq!(json, "92233720368547758.07");
    }
}
