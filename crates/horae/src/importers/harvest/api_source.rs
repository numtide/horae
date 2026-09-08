//! Primary source adapter: pull Harvest's REST API into the shared `SourceRow`
//! stream (FR-023/FR-024, research.md §11, contracts/harvest-api.md).
//!
//! Catalog metadata is indexed once per run. Time entries are joined one row
//! at a time from bounded HTTP pages; catalog indexes and the error report still
//! grow with distinct catalog records and invalid rows respectively.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::importers::harvest::types::SourceRow;
use serde::Deserialize;

use super::RowSource;

pub(super) mod http;

// ── Harvest JSON shapes (only the fields the importer consumes) ───────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ApiClient {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRef {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiProject {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub client: ApiRef,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiTask {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub billable_by_default: bool,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub default_hourly_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiUser {
    pub id: i64,
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiTimeEntry {
    pub id: i64,
    pub spent_date: NaiveDate,
    pub hours: serde_json::Number,
    pub notes: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub is_billed: bool,
    pub client: Option<ApiRef>,
    pub project: ApiRef,
    pub task: ApiRef,
    pub user: ApiRef,
    pub billable_rate: Option<serde_json::Number>,
    pub cost_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// serde default for Harvest's `is_active` / `billable_by_default` flags, which
/// are `true` unless the record says otherwise.
fn yes() -> bool {
    true
}

/// Catalog metadata used to resolve source references. Tests may attach entries
/// to the same fixture; production time entries arrive only through page buffers.
#[derive(Debug, Default, Clone)]
pub struct HarvestData {
    pub clients: Vec<ApiClient>,
    pub projects: Vec<ApiProject>,
    pub tasks: Vec<ApiTask>,
    pub users: Vec<ApiUser>,
    #[cfg(test)]
    pub time_entries: Vec<ApiTimeEntry>,
}

/// Preserve JSON decimals through the page Value and typed record. The enabled
/// arbitrary_precision feature is required at BOTH deserialization stages.
/// Expand ordinary JSON exponents without changing the user-input grammar.
/// Exponents beyond the core converter's precision remain unexpanded for its
/// validation, rather than allocating an attacker-controlled output size.
pub(super) fn decimal(n: &serde_json::Number) -> String {
    let text = n.to_string();
    let Some((mantissa, exponent)) = text.split_once(['e', 'E']) else {
        return text;
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return text;
    };
    if !(-38..=38).contains(&exponent) {
        return text;
    }
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |n| ("-", n));
    let point = mantissa.find('.').unwrap_or(mantissa.len());
    let Ok(point) = i32::try_from(point) else {
        return text;
    };
    let Some(point) = point.checked_add(exponent) else {
        return text;
    };
    let mut digits = mantissa.replace('.', "");
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let point = point as usize;
        if point < digits.len() {
            digits.insert(point, '.');
        } else {
            digits.push_str(&"0".repeat(point - digits.len()));
        }
        format!("{sign}{digits}")
    }
}

/// Join the fetched collections into `SourceRow`s — one per time entry, carrying
/// its client/project/task/user fields and all Harvest ids (pure, no I/O).
#[cfg(test)]
fn assemble_rows(data: &HarvestData) -> Vec<SourceRow> {
    let lookup = RowLookup::new(data);
    data.time_entries.iter().map(|te| lookup.row(te)).collect()
}

pub(super) struct RowLookup<'a> {
    clients: HashMap<i64, &'a ApiClient>,
    projects: HashMap<i64, &'a ApiProject>,
    tasks: HashMap<i64, &'a ApiTask>,
    users: HashMap<i64, &'a ApiUser>,
}

impl<'a> RowLookup<'a> {
    pub(super) fn new(data: &'a HarvestData) -> Self {
        Self {
            clients: data.clients.iter().map(|c| (c.id, c)).collect(),
            projects: data.projects.iter().map(|p| (p.id, p)).collect(),
            tasks: data.tasks.iter().map(|t| (t.id, t)).collect(),
            users: data.users.iter().map(|u| (u.id, u)).collect(),
        }
    }

    fn row(&self, te: &ApiTimeEntry) -> SourceRow {
        let project = self.projects.get(&te.project.id);
        let client = project
            .and_then(|p| self.clients.get(&p.client.id))
            .or_else(|| te.client.as_ref().and_then(|c| self.clients.get(&c.id)));
        let task = self.tasks.get(&te.task.id);
        let user = self.users.get(&te.user.id);

        let client_name = client
            .map(|c| c.name.clone())
            .or_else(|| te.client.as_ref().and_then(|c| c.name.clone()))
            .unwrap_or_default();
        let currency = client.and_then(|c| c.currency.clone());

        SourceRow {
            harvest_client_id: client.map(|c| c.id).or(te.client.as_ref().map(|c| c.id)),
            harvest_project_id: Some(te.project.id),
            harvest_task_id: Some(te.task.id),
            harvest_time_entry_id: Some(te.id),
            harvest_user_id: Some(te.user.id),

            client_name,
            client_address: client.and_then(|c| c.address.clone()),
            client_active: client.map(|c| c.is_active).unwrap_or(true),

            project_name: project
                .map(|p| p.name.clone())
                .or_else(|| te.project.name.clone())
                .unwrap_or_default(),
            project_code: project.and_then(|p| p.code.clone()),
            project_active: project.map(|p| p.is_active).unwrap_or(true),
            project_starts_on: project.and_then(|p| p.starts_on),
            project_ends_on: project.and_then(|p| p.ends_on),

            task_name: task
                .map(|t| t.name.clone())
                .or_else(|| te.task.name.clone())
                .unwrap_or_default(),
            task_billable_default: task.map(|t| t.billable_by_default).unwrap_or(true),

            user_email: user.map(|u| u.email.clone()),
            user_name: user.map(|u| {
                format!("{} {}", u.first_name, u.last_name)
                    .trim()
                    .to_string()
            }),

            spent_date: te.spent_date,
            hours: decimal(&te.hours),
            notes: te.notes.clone(),
            billable: te.billable,
            invoiced: te.is_billed,

            billable_rate: te.billable_rate.as_ref().map(decimal),
            billable_amount: None,
            cost_rate: te.cost_rate.as_ref().map(decimal),
            cost_amount: None,
            currency,

            harvest_updated_at: te.updated_at,
            source_location: format!("time_entry {}", te.id),
        }
    }
}

/// Join only the next record; do not build a second collection of SourceRows.
pub(super) struct ApiSource<'a> {
    rows: std::slice::Iter<'a, ApiTimeEntry>,
    lookup: &'a RowLookup<'a>,
}

impl<'a> ApiSource<'a> {
    pub(super) fn new(lookup: &'a RowLookup<'a>, rows: &'a [ApiTimeEntry]) -> Self {
        Self {
            rows: rows.iter(),
            lookup,
        }
    }
}

impl RowSource for ApiSource<'_> {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        Ok(self.rows.next().map(|te| self.lookup.row(te)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> HarvestData {
        // A minimal two-entry dataset exercising the join.
        let clients: Vec<ApiClient> = serde_json::from_str(
            r#"[{"id":1,"name":"Acme","is_active":true,"address":"1 Road","currency":"USD","updated_at":"2026-01-01T00:00:00Z"}]"#,
        )
        .unwrap();
        let projects: Vec<ApiProject> = serde_json::from_str(
            r#"[{"id":10,"name":"Website","code":"WEB","is_active":true,"client":{"id":1,"name":"Acme"},"starts_on":null,"ends_on":null,"updated_at":"2026-01-01T00:00:00Z"}]"#,
        )
        .unwrap();
        let tasks: Vec<ApiTask> = serde_json::from_str(
            r#"[{"id":100,"name":"Design","billable_by_default":true,"default_hourly_rate":150.0,"updated_at":"2026-01-01T00:00:00Z"}]"#,
        )
        .unwrap();
        let users: Vec<ApiUser> = serde_json::from_str(
            r#"[{"id":1000,"email":"dev@acme.com","first_name":"Dana","last_name":"Dev"}]"#,
        )
        .unwrap();
        let time_entries: Vec<ApiTimeEntry> = serde_json::from_str(
            r#"[
              {"id":5000,"spent_date":"2026-01-15","hours":1.5,"notes":"kickoff","billable":true,"is_billed":true,
               "client":{"id":1},"project":{"id":10},"task":{"id":100},"user":{"id":1000},
               "billable_rate":150.0,"cost_rate":80.0,"updated_at":"2026-01-16T00:00:00Z"},
              {"id":5001,"spent_date":"2026-01-16","hours":0.25,"notes":null,"billable":false,"is_billed":false,
               "client":{"id":1},"project":{"id":10},"task":{"id":100},"user":{"id":1000},
               "billable_rate":null,"cost_rate":null,"updated_at":"2026-01-16T00:00:00Z"}
            ]"#,
        )
        .unwrap();
        HarvestData {
            clients,
            projects,
            tasks,
            users,
            time_entries,
        }
    }

    #[test]
    fn assembles_rows_joining_parents_by_id() {
        let rows = assemble_rows(&fixture());
        assert_eq!(rows.len(), 2);

        let r0 = &rows[0];
        assert_eq!(r0.harvest_time_entry_id, Some(5000));
        assert_eq!(r0.client_name, "Acme");
        assert_eq!(r0.project_name, "Website");
        assert_eq!(r0.project_code.as_deref(), Some("WEB"));
        assert_eq!(r0.task_name, "Design");
        assert_eq!(r0.user_email.as_deref(), Some("dev@acme.com"));
        assert_eq!(r0.currency.as_deref(), Some("USD"));
        assert_eq!(r0.hours, "1.5");
        assert!(r0.billable);
        // Harvest's billed flag is captured as informational only.
        assert!(r0.invoiced);
        assert_eq!(r0.billable_rate.as_deref(), Some("150.0"));
    }

    #[test]
    fn hours_render_as_exact_decimals() {
        for value in ["1.5", "0.25", "2.0", "0.0", "150.0", "1.004999999999999999"] {
            assert_eq!(decimal(&value.parse().unwrap()), value);
        }
    }

    #[test]
    fn json_exponents_expand_exactly_with_bounded_output() {
        use horae_core::importers::harvest::convert::{hours_to_minutes, money_to_cents};
        for (json, expected) in [
            ("1.5e2", "150"),
            ("1e-2", "0.01"),
            ("1.25e+1", "12.5"),
            ("-1e-2", "-0.01"),
            ("1e0", "1"),
            ("9.223372036854775807e16", "92233720368547758.07"),
        ] {
            assert_eq!(decimal(&json.parse().unwrap()), expected);
        }
        assert_eq!(
            money_to_cents(&decimal(&"9.223372036854775807e16".parse().unwrap())),
            Ok(i64::MAX)
        );
        assert_eq!(
            hours_to_minutes(&decimal(&"1.5e-1".parse().unwrap())),
            Ok(9)
        );
        for json in ["1e2147483647", "1e-2147483648", "1e99999999999999999999"] {
            let value = decimal(&json.parse().unwrap());
            assert!(value.len() < 32);
            assert!(hours_to_minutes(&value).is_err());
            assert!(money_to_cents(&value).is_err());
        }
    }

    #[test]
    fn assembled_rows_convert_to_exact_minutes() {
        use horae_core::importers::harvest::convert::hours_to_minutes;
        let rows = assemble_rows(&fixture());
        assert_eq!(hours_to_minutes(&rows[0].hours).unwrap(), 90);
        assert_eq!(hours_to_minutes(&rows[1].hours).unwrap(), 15);
    }

    #[test]
    fn http_page_preserves_decimal_values_before_integer_conversion() {
        use horae_core::importers::harvest::convert::{hours_to_minutes, money_to_cents};
        use http::test_server::{Response, Server};

        // Raw JSON is intentional: json!(a_float) would already lose the source
        // precision before the real HTTP parser ever sees it.
        let server = Server::start(|_| Response {
            status: 200,
            headers: vec![],
            body: br#"{"time_entries":[{
              "id":5000,"spent_date":"2026-01-15",
              "hours":0.00833333333333333333333333333333333334,
              "project":{"id":10},"task":{"id":100},"user":{"id":1000},
              "billable_rate":1.004999999999999999,
              "cost_rate":92233720368547758.07
            }],"links":{"next":null}}"#
                .to_vec(),
        });
        let data = fixture();
        let lookup = RowLookup::new(&data);
        let mut rows = Vec::new();
        http::ApiHttp::local(server.base.clone())
            .pages::<ApiTimeEntry>(
                "token",
                "account",
                "time_entries",
                None,
                || false,
                |entries| {
                    rows.extend(entries.iter().map(|entry| lookup.row(entry)));
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(hours_to_minutes(&row.hours), Ok(1));
        assert_eq!(
            money_to_cents(row.billable_rate.as_deref().unwrap()),
            Ok(100)
        );
        assert_eq!(
            money_to_cents(row.cost_rate.as_deref().unwrap()),
            Ok(i64::MAX)
        );
    }
}
