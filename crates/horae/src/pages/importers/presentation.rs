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

pub(super) fn empty_selection() -> &'static str {
    "No report selected"
}

pub(super) fn mode_label(report: Option<&serde_json::Value>) -> &'static str {
    match report
        .and_then(|report| report.get("mode"))
        .and_then(serde_json::Value::as_str)
    {
        Some("DryRun") => "Preview",
        Some("Commit") => "Import",
        _ => "Mode unavailable",
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

    #[test]
    fn no_selection_does_not_claim_no_import_history() {
        assert_eq!(empty_selection(), "No report selected");
    }
}
