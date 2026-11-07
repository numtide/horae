//! Truthful labels for observed import data; unknown metadata stays unknown.

use horae_core::importers::harvest::types::{EntityType, ImportReport};

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

pub(super) fn phase_label(phase: Option<&str>) -> &'static str {
    match phase {
        Some("importing") => "Reading source",
        Some(phase) if phase.starts_with("download_") => "Downloading from Harvest",
        Some("catalog") => "Clients, projects, tasks and people",
        Some("clients") => "Clients",
        Some("projects") => "Projects",
        Some("tasks") => "Tasks",
        Some("time_entries") => "Time entries",
        Some("cancelling") => "Cancellation requested",
        _ => "Phase unavailable",
    }
}

pub(super) fn partial_report(state: &str) -> bool {
    matches!(state, "failed" | "cancelled")
}

pub(super) fn result_summary(report: &ImportReport) -> String {
    let mut totals = [0_u128; 4];
    for entity in EntityType::ALL {
        let counts = report.summary.counts(entity);
        for (total, value) in totals.iter_mut().zip([
            counts.created,
            counts.updated,
            counts.skipped,
            counts.errored,
        ]) {
            *total += u128::from(value);
        }
    }
    let [created, updated, skipped, errored] = totals;
    format!("{created} created · {updated} updated · {skipped} skipped · {errored} errored")
}

pub(super) fn mode_label(
    mode: Option<horae_core::importers::harvest::types::ImportMode>,
) -> &'static str {
    use horae_core::importers::harvest::types::ImportMode;
    match mode {
        Some(ImportMode::DryRun) => "Dry run",
        Some(ImportMode::Commit) => "Commit",
        None => "Mode unavailable",
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
    fn states_and_phases_have_readable_unknown_fallbacks() {
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
        for (phase, label) in [
            (Some("time_entries"), "Time entries"),
            (Some("cancelling"), "Cancellation requested"),
            (Some("importing"), "Reading source"),
            (Some("future"), "Phase unavailable"),
            (None, "Phase unavailable"),
        ] {
            assert_eq!(phase_label(phase), label);
        }
    }

    #[test]
    fn interrupted_report_is_partial_regardless_of_retry_prerequisites() {
        for (state, partial) in [("failed", true), ("cancelled", true), ("succeeded", false)] {
            assert_eq!(partial_report(state), partial);
        }
    }

    #[test]
    fn summaries_distinguish_zero_skipped_and_mixed_outcomes() {
        for mode in [ImportMode::DryRun, ImportMode::Commit] {
            let mut report = ImportReport::new(SourceKind::Csv, mode);
            assert_eq!(
                result_summary(&report),
                "0 created · 0 updated · 0 skipped · 0 errored"
            );
            report.summary.time_entries.skipped = 4;
            assert_eq!(
                result_summary(&report),
                "0 created · 0 updated · 4 skipped · 0 errored"
            );
            report.summary.clients.created = 2;
            report.summary.projects.updated = 3;
            report.summary.time_entries.errored = 1;
            assert_eq!(
                result_summary(&report),
                "2 created · 3 updated · 4 skipped · 1 errored"
            );
        }
    }
}
