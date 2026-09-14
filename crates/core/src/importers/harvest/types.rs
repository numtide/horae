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

impl std::fmt::Display for ImportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DryRun => "DryRun",
            Self::Commit => "Commit",
        })
    }
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

    // Person — resolved by unique email; CSV alone permits a unique full-name
    // fallback when email is absent. Neither source creates users (FR-010).
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
#[serde(try_from = "ImportReportWire")]
pub struct ImportReport {
    version: u16,
    pub source: SourceKind,
    pub mode: ImportMode,
    pub summary: ImportSummary,
    pub row_errors: Vec<RowError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_archive: Option<ErrorArchive>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ErrorArchive {
    count: u64,
    chunks: u64,
}

#[derive(Deserialize)]
struct ImportReportWire {
    #[serde(default = "report_version")]
    version: u16,
    source: SourceKind,
    mode: ImportMode,
    summary: ImportSummary,
    row_errors: Vec<RowError>,
    error_archive: Option<ErrorArchive>,
}

fn report_version() -> u16 {
    1
}

impl TryFrom<ImportReportWire> for ImportReport {
    type Error = &'static str;

    fn try_from(wire: ImportReportWire) -> Result<Self, Self::Error> {
        if !matches!(wire.version, 1 | 2) {
            return Err("unsupported import report version");
        }
        if let Some(archive) = &wire.error_archive
            && (wire.version != 2 || archive.count == 0 || archive.chunks == 0)
        {
            return Err("invalid import report error archive");
        }
        let report = Self {
            version: wire.version,
            source: wire.source,
            mode: wire.mode,
            summary: wire.summary,
            row_errors: wire.row_errors,
            error_archive: wire.error_archive,
        };
        if !report.reconciles() {
            return Err("import report error counts do not reconcile");
        }
        Ok(report)
    }
}

impl ImportReport {
    pub fn new(source: SourceKind, mode: ImportMode) -> Self {
        Self {
            version: report_version(),
            source,
            mode,
            summary: ImportSummary::default(),
            row_errors: Vec::new(),
            error_archive: None,
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

    /// True when the report's two independent error tallies agree: the per-entity
    /// `errored` counts sum to the inline and archived [`RowError`] details.
    ///
    /// [`Self::record`] writes both from the same outcome — it bumps the entity's
    /// `errored` bucket and pushes a `RowError`. A mismatch therefore means a row
    /// was counted as errored without being reported, or reported without being
    /// counted, which would corrupt the run report (FR-019, FR-021, SC-005).
    pub fn reconciles(&self) -> bool {
        let errored = EntityType::ALL
            .iter()
            .map(|&e| self.summary.counts(e).errored)
            .try_fold(0_u64, u64::checked_add);
        errored.is_some()
            && errored
                == self
                    .archived_error_count()
                    .checked_add(self.row_errors.len() as u64)
    }

    /// Total records that errored across all entity types.
    pub fn error_count(&self) -> u64 {
        self.archived_error_count() + self.row_errors.len() as u64
    }

    pub fn archived_error_count(&self) -> u64 {
        self.error_archive
            .as_ref()
            .map_or(0, |archive| archive.count)
    }

    pub fn archived_error_chunks(&self) -> u64 {
        self.error_archive
            .as_ref()
            .map_or(0, |archive| archive.chunks)
    }

    /// Replace inline details only after the caller has persisted them with the
    /// report. The returned metadata describes all archived details, not a delta.
    pub fn archive_errors(&mut self, total_chunks: u64) -> Result<(), &'static str> {
        if !self.reconciles()
            || self.row_errors.is_empty()
            || total_chunks <= self.archived_error_chunks()
        {
            return Err("invalid import report archive progress");
        }
        self.error_archive = Some(ErrorArchive {
            count: self.error_count(),
            chunks: total_chunks,
        });
        self.row_errors.clear();
        self.version = 2;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archived_errors_and_new_inline_errors_reconcile_across_batches() {
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        let error = RowOutcome::Errored {
            source_location: "record 1".into(),
            reason: "invalid date".into(),
        };
        report.record(EntityType::TimeEntry, &error);
        let summary = report.summary;
        assert!(report.archive_errors(0).is_err());
        assert_eq!(report.row_errors.len(), 1);
        report.archive_errors(2).unwrap();
        assert_eq!(report.summary, summary);
        assert_eq!(report.archived_error_count(), 1);
        assert_eq!(report.archived_error_chunks(), 2);
        assert!(report.reconciles());
        report.record(EntityType::Client, &error);
        assert_eq!(report.error_count(), 2);
        assert!(report.reconciles());
        assert!(report.archive_errors(2).is_err());
        assert_eq!(report.row_errors.len(), 1);
        report.archive_errors(3).unwrap();
        assert_eq!(report.error_count(), 2);
        assert!(report.row_errors.is_empty());
        assert!(report.reconciles());
        report.summary.clients.errored += 1;
        assert!(!report.reconciles());
    }

    #[test]
    fn import_modes_display_their_wire_names() {
        assert_eq!(ImportMode::DryRun.to_string(), "DryRun");
        assert_eq!(ImportMode::Commit.to_string(), "Commit");
    }

    fn errored(entity: EntityType) -> (EntityType, RowOutcome) {
        (
            entity,
            RowOutcome::Errored {
                source_location: format!("{}:1", entity.as_str()),
                reason: "bad row".into(),
            },
        )
    }

    #[test]
    fn reconciles_when_errored_counts_match_collected_errors() {
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        report.record(EntityType::Client, &RowOutcome::Created);
        report.record(EntityType::Project, &RowOutcome::Skipped);
        let (e, o) = errored(EntityType::TimeEntry);
        report.record(e, &o);
        let (e, o) = errored(EntityType::Task);
        report.record(e, &o);

        assert!(report.reconciles());
        assert_eq!(report.error_count(), 2);
    }

    #[test]
    fn does_not_reconcile_when_an_errored_count_has_no_reported_error() {
        // A bucket incremented without a matching `RowError` (the drift the
        // invariant is meant to catch) must fail reconciliation.
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        report.summary.time_entries.errored = 1;

        assert!(!report.reconciles());
    }
}
