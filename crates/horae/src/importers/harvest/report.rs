//! The run report type. The data and its reconciliation live in `horae-core`
//! (pure, so they cross the `#[server]` boundary onto the web target); this
//! module re-exports it under the server-side importer for local use (FR-021).

pub use horae_core::importers::harvest::types::ImportReport;

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::importers::harvest::types::{EntityType, ImportMode, RowOutcome, SourceKind};

    fn report() -> ImportReport {
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        report.record(EntityType::Client, &RowOutcome::Created);
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: "record 2".into(),
                reason: "invalid date".into(),
            },
        );
        report
    }

    #[test]
    fn serialized_report_has_a_supported_schema_version() {
        let report = report();
        let encoded = serde_json::to_value(&report).unwrap();
        assert_eq!(encoded["version"], 1);
        assert_eq!(
            serde_json::from_value::<ImportReport>(encoded).unwrap(),
            report
        );
    }

    #[test]
    fn legacy_report_preserves_its_counts_and_error_details() {
        let report = report();
        let mut encoded = serde_json::to_value(&report).unwrap();
        encoded.as_object_mut().unwrap().remove("version");
        assert_eq!(
            serde_json::from_value::<ImportReport>(encoded).unwrap(),
            report
        );
    }

    #[test]
    fn report_rejects_unsupported_and_invalid_versions() {
        for version in [
            serde_json::json!(3),
            serde_json::json!(0),
            serde_json::json!("1"),
            serde_json::Value::Null,
        ] {
            let mut encoded = serde_json::to_value(report()).unwrap();
            encoded["version"] = version;
            assert!(
                serde_json::from_value::<ImportReport>(encoded).is_err(),
                "unsupported report version must be rejected"
            );
        }
    }

    #[test]
    fn archived_report_round_trips_without_losing_error_accounting() {
        let mut report = report();
        report.archive_errors(1).unwrap();
        assert!(report.row_errors.is_empty());
        assert_eq!(report.error_count(), 1);
        assert!(report.reconciles());
        let encoded = serde_json::to_value(&report).unwrap();
        assert_eq!(encoded["version"], 2);
        assert_eq!(
            serde_json::from_value::<ImportReport>(encoded).unwrap(),
            report
        );
    }

    #[test]
    fn archive_metadata_cannot_masquerade_as_a_legacy_or_inconsistent_report() {
        let mut report = report();
        report.archive_errors(1).unwrap();
        let encoded = serde_json::to_value(report).unwrap();
        for (path, value) in [
            ("/version", serde_json::json!(1)),
            ("/error_archive/count", serde_json::json!(0)),
            ("/error_archive/count", serde_json::json!(2)),
            ("/error_archive/chunks", serde_json::json!(0)),
        ] {
            let mut invalid = encoded.clone();
            *invalid.pointer_mut(path).unwrap() = value;
            assert!(
                serde_json::from_value::<ImportReport>(invalid).is_err(),
                "{path}"
            );
        }
    }
}
