//! The run report type. The data and its reconciliation live in `horae-core`
//! (pure, so they cross the `#[server]` boundary onto the web target); this
//! module re-exports it under the server-side importer for local use (FR-021).

pub use horae_core::importers::harvest::types::ImportReport;

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::importers::harvest::types::{
        EntityCounts, EntityType, ImportMode, PersonError, PersonErrorKind, PersonOutcome,
        ReportPerson, RowOutcome, SourceKind,
    };

    fn report() -> ImportReport {
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        report.record(EntityType::Client, &RowOutcome::Created);
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: "record 2".into(),
                reason: "invalid date".into(),
                person: None,
                entry_kind: None,
            },
        );
        report
    }

    #[test]
    fn reports_without_people_fields_still_load() {
        let legacy = serde_json::json!({
            "version": 1,
            "source": "harvest_api",
            "mode": "Commit",
            "summary": {
                "clients": {"created": 1, "updated": 0, "skipped": 0, "errored": 0},
                "projects": {"created": 0, "updated": 0, "skipped": 0, "errored": 0},
                "tasks": {"created": 0, "updated": 0, "skipped": 0, "errored": 0},
                "time_entries": {"created": 0, "updated": 0, "skipped": 0, "errored": 1}
            },
            "row_errors": [{
                "source_location": "time_entry 1",
                "entity": "time_entry",
                "reason": "no Horae user matches email \"a@b.c\""
            }]
        });
        let report: ImportReport = serde_json::from_value(legacy).unwrap();
        assert_eq!(report.summary.people, EntityCounts::default());
        assert!(report.people.is_empty());
        assert_eq!(report.row_errors[0].person, None);
        assert!(report.reconciles());
    }

    #[test]
    fn person_details_round_trip_and_reconcile() {
        let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit);
        report.record(EntityType::Person, &RowOutcome::Created);
        report.record(
            EntityType::Person,
            &RowOutcome::Errored {
                source_location: "person 7".into(),
                reason: "Harvest person has no email".into(),
                person: Some(PersonError {
                    kind: PersonErrorKind::MissingEmail,
                    email: None,
                    name: Some("No Mail".into()),
                }),
                entry_kind: None,
            },
        );
        report.people.push(ReportPerson {
            harvest_id: Some(6),
            horae_id: uuid::Uuid::nil(),
            email: Some("dana@example.com".into()),
            name: "Dana Dev".into(),
            outcome: PersonOutcome::Created,
            harvest_active: Some(false),
        });
        assert!(report.reconciles());
        let encoded = serde_json::to_value(&report).unwrap();
        assert_eq!(encoded["summary"]["people"]["created"], 1);
        assert_eq!(encoded["row_errors"][0]["entity"], "person");
        assert_eq!(encoded["row_errors"][0]["person"]["kind"], "missing_email");
        assert!(encoded["row_errors"][0]["person"].get("email").is_none());
        assert_eq!(encoded["people"][0]["outcome"], "created");
        assert_eq!(
            serde_json::from_value::<ImportReport>(encoded).unwrap(),
            report
        );
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
