//! Truthful labels for observed import data; unknown metadata stays unknown.

use horae_core::importers::harvest::types::{EntityType, ImportMode, ImportReport};

pub(super) fn source_label(kind: &str) -> &'static str {
    match kind {
        "harvest_api_import" => "Harvest API",
        "harvest_csv_import" => "CSV file",
        _ => "Unknown source",
    }
}

pub(super) fn state_label(state: &str) -> &'static str {
    match state {
        "queued" => "Queued",
        "running" => "Running",
        "succeeded" => "Completed",
        "failed" => "Failed",
        "cancelled" => "Cancelled",
        _ => "Status unavailable",
    }
}

pub(super) fn partial_report(state: &str) -> bool {
    matches!(state, "failed" | "cancelled")
}

pub(super) fn mode_label(
    mode: Option<horae_core::importers::harvest::types::ImportMode>,
) -> &'static str {
    use horae_core::importers::harvest::types::ImportMode;
    match mode {
        Some(ImportMode::DryRun) => "Preview",
        Some(ImportMode::Commit) => "Import",
        None => "Mode unavailable",
    }
}

/// What a run is called: a preview, an import, or a sync of later changes.
pub(super) fn run_label(job: &crate::models::JobStatus) -> &'static str {
    use horae_core::importers::harvest::types::SyncScope;
    if job.mode == Some(ImportMode::Commit) && job.sync == Some(SyncScope::Incremental) {
        "Sync"
    } else {
        mode_label(job.mode)
    }
}

/// `21529` → `"21,529"`.
pub(super) fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Records a run writes (new or updated) and records it cannot import.
/// Records already in Horae count as neither.
pub(super) fn ready_and_skipped(report: &ImportReport) -> (u64, u64) {
    EntityType::ALL
        .iter()
        .map(|&entity| report.summary.counts(entity))
        .fold((0, 0), |(ready, skipped), counts| {
            (
                ready + counts.created + counts.updated,
                skipped + counts.errored,
            )
        })
}

/// What a running job is doing and how far it got. `total` is `None` while
/// the phase's size is unknown, and the page then shows the count alone.
#[derive(Debug, PartialEq)]
pub(super) struct Progress {
    pub label: &'static str,
    pub done: Option<u64>,
    pub total: Option<u64>,
}

pub(super) fn progress(job: &crate::models::JobStatus, report: Option<&ImportReport>) -> Progress {
    let preview = job.mode != Some(ImportMode::Commit);
    let processed = u64::try_from(job.processed_count).unwrap_or(0);
    let total = job.total_count.and_then(|total| u64::try_from(total).ok());
    let (label, done) = match job.phase.as_deref() {
        // A queued retry keeps the phase it reached; only a fresh job has none.
        None if job.status == "queued" => ("Waiting to start…", None),
        Some("cancelling") => ("Stopping after the current batch…", None),
        Some(phase) if phase.starts_with("download_") => (
            "Reading clients, projects, tasks and people from Harvest",
            None,
        ),
        Some("catalog") if preview => (
            "Checking clients, projects, tasks and people",
            Some(processed),
        ),
        Some("catalog") => (
            "Writing clients, projects, tasks and people",
            Some(processed),
        ),
        Some("time_entries") => (
            if preview {
                "Checking time entries"
            } else {
                "Writing time entries"
            },
            // The job's count spans every phase; this phase's is the entries'.
            Some(report.map_or(processed, |report| report.summary.time_entries.processed())),
        ),
        _ if preview => ("Reading the source", Some(processed)),
        _ => ("Writing to Horae", Some(processed)),
    };
    Progress {
        label,
        done,
        total: total.filter(|_| done.is_some()),
    }
}

pub(super) fn commit_reason(availability: crate::models::CommitAvailability) -> &'static str {
    use crate::models::CommitAvailability;
    match availability {
        CommitAvailability::NewerImport => {
            "A newer import from this source replaced this preview. Start a new preview to commit."
        }
        CommitAvailability::PreviousAccount => {
            "This preview belongs to a previous Harvest account. Start a new preview for the current account."
        }
        CommitAvailability::MissingUpload => {
            "The CSV upload is no longer retained. Choose the file again and start a new preview."
        }
        CommitAvailability::UnavailableState => "Only a preview that finished can be committed.",
        CommitAvailability::Unknown => {
            "Commit availability is unknown. Reload the page or update Horae."
        }
        CommitAvailability::Available
        | CommitAvailability::NotPreview
        | CommitAvailability::Committed => "",
    }
}

pub(super) fn person_kind_label(
    kind: horae_core::importers::harvest::types::PersonErrorKind,
) -> &'static str {
    use horae_core::importers::harvest::types::PersonErrorKind;
    match kind {
        PersonErrorKind::UnmatchedEmail => "No Horae account has this email",
        PersonErrorKind::UnmatchedName => "No Horae account has this name",
        PersonErrorKind::AmbiguousPerson => "Several Horae accounts could match",
        PersonErrorKind::MissingEmail => "The Harvest person has no email",
        PersonErrorKind::DuplicateEmail => "Several Harvest people share this email",
        PersonErrorKind::PersonNotImported => "The person was not imported",
        PersonErrorKind::EmailInUse => "Another account uses this email",
    }
}

/// A time entry problem's heading and what to do about it.
pub(super) fn entry_kind_text(
    kind: horae_core::importers::harvest::types::EntryErrorKind,
) -> (&'static str, &'static str) {
    use horae_core::importers::harvest::types::EntryErrorKind;
    match kind {
        EntryErrorKind::NegativeHours => (
            "Time entries with negative hours, skipped",
            "Usually a correction added in Harvest instead of editing the original entry. Horae doesn't store negative time. Edit the original entry in Harvest, delete the negative one, then preview again.",
        ),
        EntryErrorKind::TimerRunning => (
            "Timers still running in Harvest",
            "They come across on the next sync, once stopped.",
        ),
        EntryErrorKind::ChangedInHorae => (
            "Entries changed in both places",
            "Horae kept its version because someone edited, submitted or approved them here.",
        ),
    }
}

/// What the administrator can do about a person, when adding them is not it.
pub(super) fn person_fix(
    kind: horae_core::importers::harvest::types::PersonErrorKind,
) -> &'static str {
    use horae_core::importers::harvest::types::PersonErrorKind;
    match kind {
        PersonErrorKind::UnmatchedEmail | PersonErrorKind::UnmatchedName => "",
        PersonErrorKind::MissingEmail => {
            "Add an email to this person in Harvest, then preview again."
        }
        PersonErrorKind::DuplicateEmail => "Give each of these Harvest people their own email.",
        PersonErrorKind::AmbiguousPerson => {
            "Make the email or name unique in Horae, or change it in Harvest."
        }
        PersonErrorKind::EmailInUse => "Change the email in Harvest or on the other account.",
        PersonErrorKind::PersonNotImported => "Fix the person's own error first.",
    }
}

pub(super) fn retry_reason(availability: crate::models::RetryAvailability) -> &'static str {
    use crate::models::RetryAvailability;
    match availability {
        RetryAvailability::PreviousAccount => {
            "This import belongs to a previous Harvest account. Its report remains available; start a new preview for the current account."
        }
        RetryAvailability::MissingUpload => {
            "Upload is no longer retained. Choose the CSV file again and start a new preview."
        }
        RetryAvailability::Unknown => {
            "Retry availability is unknown. Refresh history or update Horae before retrying."
        }
        RetryAvailability::UnavailableState => {
            "This import is not in a retryable state. Refresh history to check its status."
        }
        RetryAvailability::Available => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::importers::harvest::types::{ImportMode, SourceKind};

    #[test]
    fn source_labels_do_not_guess_unknown_importers() {
        for (kind, label) in [
            ("harvest_api_import", "Harvest API"),
            ("harvest_csv_import", "CSV file"),
            ("future", "Unknown source"),
        ] {
            assert_eq!(source_label(kind), label);
        }
    }

    #[test]
    fn states_have_readable_unknown_fallbacks() {
        for (state, label) in [
            ("queued", "Queued"),
            ("running", "Running"),
            ("succeeded", "Completed"),
            ("failed", "Failed"),
            ("cancelled", "Cancelled"),
            ("future", "Status unavailable"),
        ] {
            assert_eq!(state_label(state), label);
        }
    }

    #[test]
    fn counts_group_thousands() {
        for (n, text) in [(0, "0"), (999, "999"), (1_000, "1,000"), (21_529, "21,529")] {
            assert_eq!(count(n), text);
        }
        assert_eq!(count(1_234_567), "1,234,567");
    }

    fn running(
        phase: Option<&str>,
        processed: i64,
        total: Option<i64>,
    ) -> crate::models::JobStatus {
        crate::models::JobStatus {
            id: uuid::Uuid::nil(),
            kind: "harvest_api_import".into(),
            status: "running".into(),
            phase: phase.map(Into::into),
            processed_count: processed,
            total_count: total,
            report: None,
            last_error: None,
            created_at: chrono::DateTime::UNIX_EPOCH,
            finished_at: None,
            retry_availability: Default::default(),
            mode: Some(ImportMode::DryRun),
            source: Some(SourceKind::HarvestApi),
            commit_availability: Default::default(),
            committed_as: None,
            error_kind: None,
            sync: Some(horae_core::importers::harvest::types::SyncScope::Full),
        }
    }

    #[test]
    fn progress_counts_the_current_phase_against_its_own_total() {
        let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::DryRun);
        report.summary.clients.created = 229;
        report.summary.time_entries.created = 8_412;
        let job = running(Some("time_entries"), 8_641, Some(21_529));
        assert_eq!(
            progress(&job, Some(&report)),
            Progress {
                label: "Checking time entries",
                done: Some(8_412),
                total: Some(21_529),
            }
        );
        let mut import = running(Some("catalog"), 180, Some(229));
        import.mode = Some(ImportMode::Commit);
        assert_eq!(
            progress(&import, None),
            Progress {
                label: "Writing clients, projects, tasks and people",
                done: Some(180),
                total: Some(229),
            }
        );
    }

    #[test]
    fn progress_without_a_known_size_shows_no_total() {
        let download = progress(&running(Some("download_projects"), 0, None), None);
        assert_eq!((download.done, download.total), (None, None));
        let mut queued = running(None, 0, None);
        queued.status = "queued".into();
        assert_eq!(progress(&queued, None).label, "Waiting to start…");
        let csv = progress(&running(Some("time_entries"), 504, None), None);
        assert_eq!((csv.done, csv.total), (Some(504), None));
    }

    #[test]
    fn interrupted_report_is_partial_regardless_of_retry_prerequisites() {
        for (state, partial) in [("failed", true), ("cancelled", true), ("succeeded", false)] {
            assert_eq!(partial_report(state), partial);
        }
    }

    #[test]
    fn ready_counts_writes_and_skipped_counts_errors() {
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::DryRun);
        assert_eq!(ready_and_skipped(&report), (0, 0));
        report.summary.clients.created = 2;
        report.summary.projects.updated = 3;
        report.summary.time_entries.skipped = 4;
        report.summary.time_entries.errored = 1;
        assert_eq!(ready_and_skipped(&report), (5, 1));
    }
}
