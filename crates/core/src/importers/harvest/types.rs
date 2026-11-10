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

/// The five Harvest/Horae entity levels the importer creates, in FK-safe order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Client,
    Project,
    Task,
    Person,
    TimeEntry,
}

impl EntityType {
    pub const ALL: [EntityType; 5] = [
        EntityType::Client,
        EntityType::Project,
        EntityType::Task,
        EntityType::Person,
        EntityType::TimeEntry,
    ];

    /// The lowercase token used in the `harvest_import_map.harvest_entity_type`
    /// enum column (`client|project|task|person|time_entry`).
    pub fn as_str(self) -> &'static str {
        match self {
            EntityType::Client => "client",
            EntityType::Project => "project",
            EntityType::Task => "task",
            EntityType::Person => "person",
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
    // fallback when email is absent. Only an email can create an account.
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
    /// A Harvest timer still running: its hours are partial, so it is reported
    /// and left for a later sync instead of imported (API source only).
    pub running: bool,

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
        /// Set when the failure concerns a person, so reports can group it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        person: Option<PersonError>,
        /// Set for a time entry problem the report groups by kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entry_kind: Option<EntryErrorKind>,
    },
}

/// Why a time entry was not imported, for problems that recur across many
/// entries. The row's reason keeps its own detail (the hours, the refusal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryErrorKind {
    /// The timer is still running in Harvest; a later sync brings it.
    TimerRunning,
    /// Harvest has negative hours, which Horae does not store.
    NegativeHours,
    /// The entry changed in Horae (edited, submitted, approved) after import,
    /// so the Harvest edit was not applied.
    ChangedInHorae,
}

/// Why a record failed because of its person, and who that person is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonError {
    pub kind: PersonErrorKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonErrorKind {
    /// No Horae account has the email.
    UnmatchedEmail,
    /// No single Horae account has the name (CSV rows without an email).
    UnmatchedName,
    /// More than one Horae account matches.
    AmbiguousPerson,
    /// The Harvest person has no email.
    MissingEmail,
    /// Another Harvest person in the same account has the same email.
    DuplicateEmail,
    /// The entry's Harvest person failed to import earlier in the run.
    PersonNotImported,
    /// An account the import cannot use, such as one in another workspace,
    /// already has the email.
    EmailInUse,
}

/// A Harvest person the run linked to a Horae account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportPerson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harvest_id: Option<i64>,
    pub horae_id: uuid::Uuid,
    /// Absent only for a CSV person matched by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub name: String,
    pub outcome: PersonOutcome,
    /// Harvest's own active flag, for the administrator; never applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harvest_active: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonOutcome {
    Created,
    Matched,
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

/// Counts for each of the five entity types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub clients: EntityCounts,
    pub projects: EntityCounts,
    pub tasks: EntityCounts,
    /// Absent from reports written before people were imported.
    #[serde(default)]
    pub people: EntityCounts,
    pub time_entries: EntityCounts,
}

impl ImportSummary {
    /// Mutable access to the counts for one entity type.
    pub fn counts_mut(&mut self, entity: EntityType) -> &mut EntityCounts {
        match entity {
            EntityType::Client => &mut self.clients,
            EntityType::Project => &mut self.projects,
            EntityType::Task => &mut self.tasks,
            EntityType::Person => &mut self.people,
            EntityType::TimeEntry => &mut self.time_entries,
        }
    }

    pub fn counts(&self, entity: EntityType) -> &EntityCounts {
        match entity {
            EntityType::Client => &self.clients,
            EntityType::Project => &self.projects,
            EntityType::Task => &self.tasks,
            EntityType::Person => &self.people,
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
    #[serde(default)]
    pub account_generation: i64,
    #[serde(default)]
    pub connection_revision: i64,
    #[serde(default)]
    pub has_provenance: bool,
    #[serde(default)]
    pub active_imports: i64,
}

impl ConnectionStatus {
    /// A snapshot for presentation; the server rechecks it at confirmation.
    pub fn change_account_blocker(&self) -> Option<&'static str> {
        if self.account_id.is_none() {
            Some("No Harvest account is bound. Use Connect Harvest.")
        } else if self.has_provenance {
            Some(
                "Harvest data has already been imported. Changing accounts requires a separate data migration.",
            )
        } else if self.active_imports > 0 {
            Some(
                "Finish or explicitly cancel queued and running Harvest imports before changing account.",
            )
        } else {
            None
        }
    }

    /// Why Disconnect is unavailable, for presentation. The server refuses
    /// only while a run holds the import session; a queued or retrying run
    /// would fail later without the credentials, so the page waits for both.
    pub fn disconnect_blocker(&self) -> Option<&'static str> {
        (self.active_imports > 0).then_some(
            "Finish or explicitly cancel queued and running Harvest imports before disconnecting.",
        )
    }
}

#[cfg(test)]
mod account_switch_tests {
    use super::ConnectionStatus;

    #[test]
    fn account_switch_requires_binding_and_no_imports_or_active_work() {
        let mut status = ConnectionStatus::default();
        assert!(status.change_account_blocker().is_some());
        status.account_id = Some("original".into());
        assert!(status.change_account_blocker().is_none());
        status.has_provenance = true;
        assert!(
            status
                .change_account_blocker()
                .unwrap()
                .contains("migration")
        );
        status.has_provenance = false;
        status.active_imports = 1;
        assert!(status.change_account_blocker().unwrap().contains("cancel"));
        status.active_imports = 0;
        status.connected = true;
        status.token_expired = true;
        assert!(status.change_account_blocker().is_none());
    }

    #[test]
    fn disconnecting_waits_for_queued_and_running_imports() {
        let mut status = ConnectionStatus {
            connected: true,
            account_id: Some("original".into()),
            has_provenance: true,
            ..ConnectionStatus::default()
        };
        assert_eq!(status.disconnect_blocker(), None);
        status.active_imports = 1;
        assert!(
            status
                .disconnect_blocker()
                .unwrap()
                .contains("disconnecting")
        );
    }
}

/// A single per-record error for the report (FR-019).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowError {
    pub source_location: String,
    pub entity: EntityType,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<PersonError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_kind: Option<EntryErrorKind>,
}

/// A record created from Harvest that could not take its Harvest id as its id
/// in the Harvest-compatible API, because another record of its kind already
/// has that id, so it got `api_id` instead (FR-016).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdNotice {
    pub entity: EntityType,
    pub harvest_id: i64,
    pub api_id: i64,
}

/// Notices a report keeps; the rest are only counted. Notices are never
/// archived, so they count against the report's 16 KiB budget for good: 20
/// take about 1.4 KB.
pub const ID_NOTICES_KEPT: usize = 20;

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
    /// One row per Harvest person the run resolved; bounded by the people in
    /// the source, not by its entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<ReportPerson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub id_notices: Vec<IdNotice>,
    /// Notices past [`ID_NOTICES_KEPT`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub omitted_id_notices: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_archive: Option<ErrorArchive>,
}

fn is_zero(count: &u64) -> bool {
    *count == 0
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
    #[serde(default)]
    people: Vec<ReportPerson>,
    #[serde(default)]
    id_notices: Vec<IdNotice>,
    #[serde(default)]
    omitted_id_notices: u64,
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
            people: wire.people,
            id_notices: wire.id_notices,
            omitted_id_notices: wire.omitted_id_notices,
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
            people: Vec::new(),
            id_notices: Vec::new(),
            omitted_id_notices: 0,
            error_archive: None,
        }
    }

    pub fn note_id(&mut self, notice: IdNotice) {
        if self.id_notices.len() < ID_NOTICES_KEPT {
            self.id_notices.push(notice);
        } else {
            self.omitted_id_notices += 1;
        }
    }

    /// Fold one entity outcome into the summary, collecting the error detail when
    /// the outcome is `Errored`.
    pub fn record(&mut self, entity: EntityType, outcome: &RowOutcome) {
        self.summary.counts_mut(entity).record(outcome);
        if let RowOutcome::Errored {
            source_location,
            reason,
            person,
            entry_kind,
        } = outcome
        {
            self.row_errors.push(RowError {
                source_location: source_location.clone(),
                entity,
                reason: reason.clone(),
                person: person.clone(),
                entry_kind: *entry_kind,
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
            person: None,
            entry_kind: None,
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
    fn id_notices_are_kept_up_to_a_limit_and_the_rest_counted() {
        let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit);
        let notice = |harvest_id| IdNotice {
            entity: EntityType::Client,
            harvest_id,
            api_id: harvest_id + 1,
        };
        for harvest_id in 0..ID_NOTICES_KEPT as i64 + 2 {
            report.note_id(notice(harvest_id));
        }

        assert_eq!(report.id_notices.len(), ID_NOTICES_KEPT);
        assert_eq!(report.id_notices[0], notice(0));
        assert_eq!(report.omitted_id_notices, 2);
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
                person: None,
                entry_kind: None,
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
