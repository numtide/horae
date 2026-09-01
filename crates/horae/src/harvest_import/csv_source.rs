//! Secondary source adapter: parse a Harvest detailed-time-report CSV into the
//! shared `SourceRow` stream (US5, contracts/csv-format.md, research.md §1/§9).
//!
//! A CSV carries no stable Harvest ids, so every row's Harvest ids are `None` and
//! matching falls back to the composite natural key — re-importing the same file
//! is still duplicate-free, just not edit-robust the way the API source is.
//! Column headers are matched case-insensitively with surrounding whitespace
//! trimmed; an unrecognized or empty file is rejected up front with no writes.

use chrono::NaiveDate;
use horae_core::harvest_import::types::{
    EntityType, ImportMode, RowOutcome, SourceKind, SourceRow,
};
use std::collections::HashMap;
use uuid::Uuid;

use super::report::ImportReport;
use super::{VecSource, run_import};

/// Columns that must be present for the file to be a recognizable export.
const REQUIRED: &[&str] = &["date", "client", "project", "task", "hours"];

/// Errors that reject the whole file up front (FR-003).
#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("the file is empty")]
    Empty,
    #[error("not a recognizable Harvest CSV export (missing columns: {0})")]
    Unrecognized(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// A per-row parse failure (bad date etc.) that becomes a record error, not an
/// up-front rejection.
#[derive(Debug)]
struct ParseErr {
    source_location: String,
    reason: String,
}

/// Parse the CSV bytes into good rows plus per-row parse errors. Rejects the file
/// up front if it is empty or missing required columns.
fn parse_csv(bytes: &[u8]) -> Result<(Vec<SourceRow>, Vec<ParseErr>), CsvError> {
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Err(CsvError::Empty);
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(bytes);

    let headers = reader
        .headers()
        .map_err(|e| CsvError::Other(e.into()))?
        .clone();
    let index: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.trim().to_lowercase(), i))
        .collect();

    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|c| !index.contains_key(*c))
        .collect();
    if !missing.is_empty() {
        return Err(CsvError::Unrecognized(missing.join(", ")));
    }

    let get = |rec: &csv::StringRecord, col: &str| -> Option<String> {
        index
            .get(col)
            .and_then(|&i| rec.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for (i, record) in reader.records().enumerate() {
        // Harvest's data rows start at CSV line 2 (after the header).
        let line = i + 2;
        let location = format!("CSV line {line}");
        let record = match record {
            Ok(r) => r,
            Err(e) => {
                errors.push(ParseErr {
                    source_location: location,
                    reason: format!("malformed CSV row: {e}"),
                });
                continue;
            }
        };

        let date_str = get(&record, "date");
        let spent_date = match date_str.as_deref().map(parse_date) {
            Some(Ok(d)) => d,
            Some(Err(reason)) => {
                errors.push(ParseErr {
                    source_location: location,
                    reason,
                });
                continue;
            }
            None => {
                errors.push(ParseErr {
                    source_location: location,
                    reason: "missing Date".to_string(),
                });
                continue;
            }
        };

        let email = get(&record, "email").or_else(|| {
            // Fall back to a "First Last" name when there is no email column.
            match (get(&record, "first name"), get(&record, "last name")) {
                (Some(f), Some(l)) => Some(format!("{f} {l}")),
                (Some(f), None) => Some(f),
                (None, Some(l)) => Some(l),
                (None, None) => None,
            }
        });

        rows.push(SourceRow {
            harvest_client_id: None,
            harvest_project_id: None,
            harvest_task_id: None,
            harvest_time_entry_id: None,
            harvest_user_id: None,

            client_name: get(&record, "client").unwrap_or_default(),
            client_address: None,
            client_active: true,

            project_name: get(&record, "project").unwrap_or_default(),
            project_code: get(&record, "project code"),
            project_active: true,
            project_starts_on: None,
            project_ends_on: None,

            task_name: get(&record, "task").unwrap_or_default(),
            task_billable_default: parse_bool(get(&record, "billable?").as_deref()),

            user_email: email,
            user_name: None,

            spent_date,
            hours: get(&record, "hours").unwrap_or_default(),
            notes: get(&record, "notes"),
            billable: parse_bool(get(&record, "billable?").as_deref()),
            invoiced: parse_bool(get(&record, "invoiced?").as_deref()),

            billable_rate: get(&record, "billable rate"),
            billable_amount: get(&record, "billable amount"),
            cost_rate: get(&record, "cost rate"),
            cost_amount: get(&record, "cost amount"),
            currency: get(&record, "currency"),

            harvest_updated_at: None,
            source_location: location,
        });
    }

    Ok((rows, errors))
}

/// Parse a `YYYY-MM-DD` date, returning a human reason on failure.
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("invalid date {s:?} (expected YYYY-MM-DD)"))
}

/// `Yes`/`No`/`true`/`1` (case-insensitive) → bool; anything else is `false`.
fn parse_bool(s: Option<&str>) -> bool {
    matches!(
        s.map(|v| v.trim().to_lowercase()).as_deref(),
        Some("yes") | Some("true") | Some("1") | Some("y")
    )
}

/// Import a Harvest CSV through the shared engine. Rejects an empty/unrecognized
/// file up front (FR-003); good rows run through the engine, parse-failed rows are
/// folded into the report as record errors so totals still reconcile (FR-021).
pub async fn import_csv(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    default_currency: &str,
    bytes: &[u8],
    mode: ImportMode,
) -> Result<ImportReport, CsvError> {
    let (rows, parse_errors) = parse_csv(bytes)?;

    let mut report = run_import(
        pool,
        org_id,
        default_currency,
        SourceKind::Csv,
        mode,
        VecSource::new(rows),
    )
    .await
    .map_err(CsvError::Other)?;

    for e in parse_errors {
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: e.source_location,
                reason: e.reason,
            },
        );
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Date,Client,Project,Project Code,Task,Notes,Hours,Billable?,Invoiced?,First Name,Last Name,Email,Billable Rate,Billable Amount,Cost Rate,Cost Amount,Currency\n\
2026-01-15,Acme,Website,WEB,Design,kickoff,1.5,Yes,No,Dana,Dev,dev@acme.com,150,225,80,120,USD\n\
2026-01-16,Acme,Website,WEB,Design,,0.25,No,No,Dana,Dev,dev@acme.com,,,,,USD\n";

    #[test]
    fn parses_rows_case_insensitively() {
        let (rows, errors) = parse_csv(SAMPLE.as_bytes()).unwrap();
        assert!(errors.is_empty());
        assert_eq!(rows.len(), 2);
        let r = &rows[0];
        assert_eq!(r.client_name, "Acme");
        assert_eq!(r.project_name, "Website");
        assert_eq!(r.project_code.as_deref(), Some("WEB"));
        assert_eq!(r.task_name, "Design");
        assert_eq!(r.user_email.as_deref(), Some("dev@acme.com"));
        assert_eq!(r.hours, "1.5");
        assert!(r.billable);
        assert_eq!(r.currency.as_deref(), Some("USD"));
        assert_eq!(r.source_location, "CSV line 2");
        // No Harvest ids on the CSV source.
        assert_eq!(r.harvest_client_id, None);
    }

    #[test]
    fn empty_file_is_rejected() {
        assert!(matches!(parse_csv(b"   \n"), Err(CsvError::Empty)));
    }

    #[test]
    fn missing_required_columns_is_rejected() {
        let bad = "Foo,Bar\n1,2\n";
        let err = parse_csv(bad.as_bytes()).unwrap_err();
        assert!(matches!(err, CsvError::Unrecognized(_)));
    }

    #[test]
    fn bad_date_becomes_a_row_error_not_a_rejection() {
        let csv = "Date,Client,Project,Task,Hours,Billable?,Currency\n\
not-a-date,Acme,Website,Design,1.5,Yes,USD\n";
        let (rows, errors) = parse_csv(csv.as_bytes()).unwrap();
        assert!(rows.is_empty());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].source_location, "CSV line 2");
    }

    #[test]
    fn parse_bool_accepts_yes_variants() {
        assert!(parse_bool(Some("Yes")));
        assert!(parse_bool(Some("YES")));
        assert!(parse_bool(Some("true")));
        assert!(!parse_bool(Some("No")));
        assert!(!parse_bool(None));
    }
}
