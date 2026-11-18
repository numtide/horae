use horae_core::importers::harvest::types::{
    EntityType, EntryErrorKind, ImportMode, PersonErrorKind, SourceKind, SyncScope,
};

/// Advisory retry prerequisites; write-side validation remains authoritative.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryAvailability {
    Available,
    UnavailableState,
    PreviousAccount,
    MissingUpload,
    #[default]
    #[serde(other)]
    Unknown,
}

/// Whether a preview can be committed from its run page, and if not, why.
/// Advisory like [`RetryAvailability`]; the commit request checks again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitAvailability {
    Available,
    /// The run already wrote business data.
    NotPreview,
    /// Only a preview that finished can be committed.
    UnavailableState,
    /// This preview was committed; [`JobStatus::committed_as`] is that run.
    Committed,
    /// A later import from the same source replaces this preview.
    NewerImport,
    PreviousAccount,
    MissingUpload,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JobStatus {
    pub id: uuid::Uuid,
    pub kind: String,
    pub status: String,
    pub phase: Option<String>,
    pub processed_count: i64,
    pub total_count: Option<i64>,
    pub report: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub retry_availability: RetryAvailability,
    /// Known from the request, before any report exists.
    #[serde(default)]
    pub mode: Option<ImportMode>,
    #[serde(default)]
    pub source: Option<SourceKind>,
    #[serde(default)]
    pub commit_availability: CommitAvailability,
    /// The commit run started from this preview, if any.
    #[serde(default)]
    pub committed_as: Option<uuid::Uuid>,
    /// What the last failed attempt ran into, when it is a Harvest problem
    /// the administrator can act on. `None` for any other failure.
    #[serde(default)]
    pub error_kind: Option<JobErrorKind>,
    /// An API run's scope; an incremental commit is a "Sync changes".
    #[serde(default)]
    pub sync: Option<SyncScope>,
}

/// A failed attempt's cause, so a page can say what to do without reading
/// `last_error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobErrorKind {
    /// Harvest refused the connection (HTTP 401 or a refused token refresh).
    ReconnectRequired,
    /// The connected Harvest user lacks permissions (HTTP 403).
    Forbidden,
    /// Harvest did not answer or answered with a server error.
    Unavailable,
}

impl JobStatus {
    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running")
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.status.as_str(), "failed" | "cancelled")
    }
}

/// A finished run's errors, grouped on the server so a page never loads every
/// error row (spec 019 FR-017).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ErrorSummary {
    pub groups: Vec<ErrorGroup>,
    pub people: Vec<MissingPerson>,
    /// Errors that belong to groups beyond the shown ones.
    pub omitted_errors: u64,
    /// People beyond the listed ones.
    pub omitted_people: u64,
}

/// Errors sharing a person problem, a time entry problem, or else the same
/// reason text.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ErrorGroup {
    pub entity: EntityType,
    pub kind: Option<PersonErrorKind>,
    #[serde(default)]
    pub entry_kind: Option<EntryErrorKind>,
    /// The reason of the first error in the group.
    pub reason: String,
    pub count: u64,
    pub samples: Vec<ErrorSample>,
}

/// One errored row of a group: where it came from and its own reason, which
/// carries values such as the hours.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ErrorSample {
    pub location: String,
    pub reason: String,
}

/// A Harvest person whose records could not be attached to an account.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MissingPerson {
    pub email: Option<String>,
    pub name: Option<String>,
    pub kind: PersonErrorKind,
    /// Time entries of this person that errored.
    pub entries: u64,
}
