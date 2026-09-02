//! Primary source adapter: pull Harvest's REST API into the shared `SourceRow`
//! stream (FR-023/FR-024, research.md §11, contracts/harvest-api.md).
//!
//! The adapter separates two concerns so the mapping stays testable without a
//! network: [`assemble_rows`] is a **pure** join of already-fetched Harvest
//! collections into `SourceRow`s (unit-tested against fixture JSON), while
//! [`fetch_all`] does the paginated, rate-limit-aware HTTP with a bearer token.
//! The parent collections (clients, projects, tasks, task assignments, users) are
//! bounded and fetched in full; time entries are the large collection and are
//! joined against those maps.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::importers::harvest::types::SourceRow;
use serde::Deserialize;

use super::RowSource;

// ── Harvest JSON shapes (only the fields the importer consumes) ───────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ApiClient {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: Option<String>,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiTask {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub billable_by_default: bool,
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
    pub hours: f64,
    pub notes: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub is_billed: bool,
    pub client: Option<ApiRef>,
    pub project: ApiRef,
    pub task: ApiRef,
    pub user: ApiRef,
    pub billable_rate: Option<f64>,
    pub cost_rate: Option<f64>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// serde default for Harvest's `is_active` / `billable_by_default` flags, which
/// are `true` unless the record says otherwise.
fn yes() -> bool {
    true
}

/// A full set of fetched collections, ready to assemble into rows. Public so a
/// test can build it from fixtures.
#[derive(Debug, Default)]
pub struct HarvestData {
    pub clients: Vec<ApiClient>,
    pub projects: Vec<ApiProject>,
    pub tasks: Vec<ApiTask>,
    pub users: Vec<ApiUser>,
    pub time_entries: Vec<ApiTimeEntry>,
}

/// Render a Harvest JSON number back to a decimal string for the exact
/// [`horae_core::importers::harvest::convert`] helpers — never an `f64` in the
/// conversion path. Harvest sends at most 2 decimals for hours and money; six
/// digits is more than enough to reproduce the source value.
fn decimal(n: f64) -> String {
    // Trim trailing zeros so "1.500000" → "1.5" (keeps the exact value).
    let s = format!("{n:.6}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Join the fetched collections into `SourceRow`s — one per time entry, carrying
/// its client/project/task/user fields and all Harvest ids (pure, no I/O).
pub fn assemble_rows(data: &HarvestData) -> Vec<SourceRow> {
    let clients: HashMap<i64, &ApiClient> = data.clients.iter().map(|c| (c.id, c)).collect();
    let projects: HashMap<i64, &ApiProject> = data.projects.iter().map(|p| (p.id, p)).collect();
    let tasks: HashMap<i64, &ApiTask> = data.tasks.iter().map(|t| (t.id, t)).collect();
    let users: HashMap<i64, &ApiUser> = data.users.iter().map(|u| (u.id, u)).collect();

    let mut rows = Vec::with_capacity(data.time_entries.len());
    for te in &data.time_entries {
        let project = projects.get(&te.project.id);
        let client = project
            .and_then(|p| clients.get(&p.client.id))
            .or_else(|| te.client.as_ref().and_then(|c| clients.get(&c.id)));
        let task = tasks.get(&te.task.id);
        let user = users.get(&te.user.id);

        let client_name = client
            .map(|c| c.name.clone())
            .or_else(|| te.client.as_ref().and_then(|c| c.name.clone()))
            .unwrap_or_default();
        let currency = client.and_then(|c| c.currency.clone());

        rows.push(SourceRow {
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
            hours: decimal(te.hours),
            notes: te.notes.clone(),
            billable: te.billable,
            invoiced: te.is_billed,

            billable_rate: te.billable_rate.map(decimal),
            billable_amount: None,
            cost_rate: te.cost_rate.map(decimal),
            cost_amount: None,
            currency,

            harvest_updated_at: te.updated_at,
            source_location: format!("time_entry {}", te.id),
        });
    }
    rows
}

/// An assembled, in-memory API source. Parents are bounded; time entries are the
/// bulk and are streamed out one at a time from the assembled vector.
pub struct ApiSource {
    rows: std::vec::IntoIter<SourceRow>,
}

impl ApiSource {
    pub fn from_data(data: &HarvestData) -> Self {
        Self {
            rows: assemble_rows(data).into_iter(),
        }
    }
}

impl RowSource for ApiSource {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        Ok(self.rows.next())
    }
}

// ── HTTP layer (blocking ureq, run under spawn_blocking) ──────────────────────

/// Harvest's API v2 data host.
const API_BASE: &str = "https://api.harvestapp.com/v2";
/// Number of records per page (Harvest caps `per_page` at 100 for v2 lists).
const PER_PAGE: u32 = 100;

/// A single-collection paginator, following Harvest's `next_page` links to
/// completion and honoring an HTTP 429 `Retry-After` (FR-023). Blocking.
pub fn fetch_all(
    agent: &ureq::Agent,
    access_token: &str,
    account_id: &str,
    collection: &str,
    updated_since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let mut url = format!("{API_BASE}/{collection}?per_page={PER_PAGE}&page={page}");
        if let Some(since) = updated_since {
            url.push_str(&format!("&updated_since={}", since.to_rfc3339()));
        }
        let body = get_with_backoff(agent, &url, access_token, account_id)?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(items) = json.get(collection).and_then(|v| v.as_array()) {
            out.extend(items.iter().cloned());
        }
        // Harvest returns `next_page: null` on the last page.
        match json.get("next_page").and_then(|v| v.as_u64()) {
            Some(next) => page = next as u32,
            None => break,
        }
    }
    Ok(out)
}

/// GET a Harvest URL with the required headers, retrying on HTTP 429 per the
/// `Retry-After` header (bounded attempts). Blocking.
fn get_with_backoff(
    agent: &ureq::Agent,
    url: &str,
    access_token: &str,
    account_id: &str,
) -> anyhow::Result<String> {
    const MAX_ATTEMPTS: u32 = 6;
    for attempt in 1..=MAX_ATTEMPTS {
        let resp = agent
            .get(url)
            .set("Authorization", &format!("Bearer {access_token}"))
            .set("Harvest-Account-Id", account_id)
            .set("User-Agent", "Horae Importer (support@horae.app)")
            .set("Accept", "application/json")
            .call();
        match resp {
            Ok(r) => return Ok(r.into_string()?),
            Err(ureq::Error::Status(429, r)) if attempt < MAX_ATTEMPTS => {
                let wait = r
                    .header("Retry-After")
                    .and_then(|h| h.parse::<u64>().ok())
                    .unwrap_or(2);
                std::thread::sleep(std::time::Duration::from_secs(wait.min(30)));
            }
            Err(e) => return Err(anyhow::anyhow!("Harvest GET {url} failed: {e}")),
        }
    }
    Err(anyhow::anyhow!("Harvest GET {url} exhausted retries"))
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
        assert_eq!(r0.billable_rate.as_deref(), Some("150"));
    }

    #[test]
    fn hours_render_as_exact_decimals() {
        assert_eq!(decimal(1.5), "1.5");
        assert_eq!(decimal(0.25), "0.25");
        assert_eq!(decimal(2.0), "2");
        assert_eq!(decimal(0.0), "0");
        assert_eq!(decimal(150.0), "150");
    }

    #[test]
    fn assembled_rows_convert_to_exact_minutes() {
        use horae_core::importers::harvest::convert::hours_to_minutes;
        let rows = assemble_rows(&fixture());
        assert_eq!(hours_to_minutes(&rows[0].hours).unwrap(), 90);
        assert_eq!(hours_to_minutes(&rows[1].hours).unwrap(), 15);
    }
}
