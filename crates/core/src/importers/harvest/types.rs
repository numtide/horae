//! Source-agnostic types shared by both import adapters and the engine.
//!
//! A [`SourceRow`] is the single normalized record both the API adapter and the
//! CSV adapter produce; the engine never learns which adapter made it. Decimal
//! values (hours, money) travel as their original *string* form so the exact
//! [`super::convert`] helpers turn them into integers with no `f64` in between.
//! [`RowOutcome`], [`EntityCounts`], and [`ImportSummary`] are the raw material
//! of the run report; [`ImportMode`] and [`SyncScope`] are the two-state mode
//! flags (named enums, never `Option<bool>`, per repo convention).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// The four Harvest/Horae entity levels the importer creates, in FK-safe order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Client,
    Project,
    Task,
    TimeEntry,
}

impl EntityType {
    pub const ALL: [EntityType; 4] = [
        EntityType::Client,
        EntityType::Project,
        EntityType::Task,
        EntityType::TimeEntry,
    ];

    /// The lowercase token used in the `harvest_import_map.harvest_entity_type`
    /// enum column (`client|project|task|time_entry`).
    pub fn as_str(self) -> &'static str {
        match self {
            EntityType::Client => "client",
            EntityType::Project => "project",
            EntityType::Task => "task",
            EntityType::TimeEntry => "time_entry",
        }
    }
}

/// Whether a run writes (`Commit`) or only previews (`DryRun`). A dry-run resolves
/// and plans against live data but persists nothing — no rows, no provenance, no
/// watermark (FR-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportMode {
    DryRun,
    Commit,
}

/// Whether an API pull fetches everything (`Full`) or only records changed since
/// the stored watermark (`Incremental`, FR-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncScope {
    Full,
    Incremental,
}

/// Which source produced a run's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    HarvestApi,
    Csv,
}

/// The normalized record both adapters produce (data-model.md).
///
/// Harvest ids are `Some` for the API source (they drive provenance matching)
/// and `None` for the CSV source. Decimal fields are the source's original text
/// so [`super::convert`] can turn them into exact integers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    // Provenance ids — API source only; `None` for CSV.
    pub harvest_client_id: Option<i64>,
    pub harvest_project_id: Option<i64>,
    pub harvest_task_id: Option<i64>,
    pub harvest_time_entry_id: Option<i64>,
    pub harvest_user_id: Option<i64>,

    // Client
    pub client_name: String,
    pub client_address: Option<String>,
    pub client_active: bool,

    // Project
    pub project_name: String,
    pub project_code: Option<String>,
    pub project_active: bool,
    pub project_starts_on: Option<NaiveDate>,
    pub project_ends_on: Option<NaiveDate>,

    // Task
    pub task_name: String,
    pub task_billable_default: bool,

    // Person — resolved to a Horae user by email (FR-010).
    pub user_email: Option<String>,
    pub user_name: Option<String>,

    // Entry
    pub spent_date: NaiveDate,
    /// Decimal hours as text (e.g. "1.5"); converted via `hours_to_minutes`.
    pub hours: String,
    pub notes: Option<String>,
    pub billable: bool,
    /// Harvest's invoiced flag — informational only, never couples to a Horae
    /// invoice (FR-016).
    pub invoiced: bool,

    // Money — decimal amounts as text; converted via `money_to_cents`.
    pub billable_rate: Option<String>,
    pub billable_amount: Option<String>,
    pub cost_rate: Option<String>,
    pub cost_amount: Option<String>,
    /// ISO 4217 currency code.
    pub currency: Option<String>,

    /// Harvest's `updated_at` for the time entry — stored on provenance to drive
    /// the incremental `updated_since` watermark (API source only).
    pub harvest_updated_at: Option<DateTime<Utc>>,

    /// Where this row came from, for the error report: a Harvest id (API) or a
    /// CSV line number.
    pub source_location: String,
}

/// The per-record result of a run (data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowOutcome {
    Created,
    Updated,
    /// Matched an existing record and left unchanged.
    Skipped,
    /// Could not be applied; carries its source location and a human reason.
    Errored {
        source_location: String,
        reason: String,
    },
}

/// Per-entity-type counts. Invariant: `processed = created + updated + skipped +
/// errored` (FR-021).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityCounts {
    pub created: u64,
    pub updated: u64,
    pub skipped: u64,
    pub errored: u64,
}

impl EntityCounts {
    pub fn processed(&self) -> u64 {
        self.created + self.updated + self.skipped + self.errored
    }

    /// Fold one outcome for one entity type into the counts.
    pub fn record(&mut self, outcome: &RowOutcome) {
        match outcome {
            RowOutcome::Created => self.created += 1,
            RowOutcome::Updated => self.updated += 1,
            RowOutcome::Skipped => self.skipped += 1,
            RowOutcome::Errored { .. } => self.errored += 1,
        }
    }
}

/// Counts for each of the four entity types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub clients: EntityCounts,
    pub projects: EntityCounts,
    pub tasks: EntityCounts,
    pub time_entries: EntityCounts,
}

impl ImportSummary {
    /// Mutable access to the counts for one entity type.
    pub fn counts_mut(&mut self, entity: EntityType) -> &mut EntityCounts {
        match entity {
            EntityType::Client => &mut self.clients,
            EntityType::Project => &mut self.projects,
            EntityType::Task => &mut self.tasks,
            EntityType::TimeEntry => &mut self.time_entries,
        }
    }

    pub fn counts(&self, entity: EntityType) -> &EntityCounts {
        match entity {
            EntityType::Client => &self.clients,
            EntityType::Project => &self.projects,
            EntityType::Task => &self.tasks,
            EntityType::TimeEntry => &self.time_entries,
        }
    }
}

/// Whether the org has a usable Harvest connection, for the admin screen. Never
/// carries the tokens themselves (FR-022).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionStatus {
    /// Whether the Harvest API source is configured on this deployment (OAuth
    /// credentials present). When false the admin screen offers only CSV import.
    pub configured: bool,
    pub connected: bool,
    pub account_id: Option<String>,
    /// True when the stored access token is known to be past expiry (a re-sync
    /// will refresh it transparently, or ask to reconnect if refresh fails).
    pub token_expired: bool,
}

/// A single per-record error for the report (FR-019).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowError {
    pub source_location: String,
    pub entity: EntityType,
    pub reason: String,
}

/// The result of an import run, returned by every surface (server fn + CLI). A
/// pure data type so it crosses the `#[server]` boundary and compiles on the web
/// target as well as the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub source: SourceKind,
    pub mode: ImportMode,
    pub summary: ImportSummary,
    pub row_errors: Vec<RowError>,
}

impl ImportReport {
    pub fn new(source: SourceKind, mode: ImportMode) -> Self {
        Self {
            source,
            mode,
            summary: ImportSummary::default(),
            row_errors: Vec::new(),
        }
    }

    /// Fold one entity outcome into the summary, collecting the error detail when
    /// the outcome is `Errored`.
    pub fn record(&mut self, entity: EntityType, outcome: &RowOutcome) {
        self.summary.counts_mut(entity).record(outcome);
        if let RowOutcome::Errored {
            source_location,
            reason,
        } = outcome
        {
            self.row_errors.push(RowError {
                source_location: source_location.clone(),
                entity,
                reason: reason.clone(),
            });
        }
    }

    /// True when every entity type reconciles: `processed` equals the sum of its
    /// four buckets (FR-021, SC-005).
    pub fn reconciles(&self) -> bool {
        EntityType::ALL.iter().all(|&e| {
            let c = self.summary.counts(e);
            c.processed() == c.created + c.updated + c.skipped + c.errored
        })
    }

    /// Total records that errored across all entity types.
    pub fn error_count(&self) -> usize {
        self.row_errors.len()
    }
}
