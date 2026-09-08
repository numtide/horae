//! Secondary source adapter: parse a Harvest detailed-time-report CSV into the
//! shared `SourceRow` stream (US5, contracts/csv-format.md, research.md §1/§9).
//!
//! A CSV carries no stable Harvest ids, so every row's Harvest ids are `None` and
//! matching falls back to the composite natural key, with repeated identical rows
//! kept as distinct entries (the Nth occurrence matches the Nth stored entry) —
//! re-importing the same file is still duplicate-free, just not edit-robust the
//! way the API source is.
//! Column headers are matched case-insensitively with surrounding whitespace
//! trimmed; an unrecognized or empty file is rejected up front with no writes.

use chrono::NaiveDate;
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, RowOutcome, SourceKind, SourceRow,
};
use std::collections::HashMap;
use uuid::Uuid;

use super::report::ImportReport;
use super::{VecSource, run_import};

/// Columns that must be present for the file to be a recognizable export.
const REQUIRED: &[&str] = &["date", "client", "project", "task", "hours"];
const USER_COLUMNS: &[&str] = &["email", "user email", "first name", "last name"];

/// Errors that reject the whole file up front (FR-003).
#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("the file is empty")]
    Empty,
    #[error("not a recognizable Harvest CSV export (missing columns: {0})")]
    Unrecognized(String),
    #[error("invalid CSV user identity: {0}")]
    Identity(String),
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
    let mut index = HashMap::new();
    for (i, header) in headers.iter().enumerate() {
        let header = header.trim().to_lowercase();
        if index.insert(header.clone(), i).is_some() && USER_COLUMNS.contains(&header.as_str()) {
            return Err(CsvError::Identity(format!("duplicate column {header:?}")));
        }
    }

    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|c| !index.contains_key(*c))
        .collect();
    if !missing.is_empty() {
        return Err(CsvError::Unrecognized(missing.join(", ")));
    }
    if !USER_COLUMNS
        .iter()
        .any(|column| index.contains_key(*column))
    {
        return Err(CsvError::Unrecognized(
            "Email or First Name/Last Name".into(),
        ));
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

        let email = get(&record, "email");
        let user_email = get(&record, "user email");
        if let (Some(email), Some(alias)) = (&email, &user_email)
            && horae_core::importers::harvest::keys::normalize(email)
                != horae_core::importers::harvest::keys::normalize(alias)
        {
            errors.push(ParseErr {
                source_location: location,
                reason: "Email and User Email disagree; provide one user identity".into(),
            });
            continue;
        }
        let name = match (get(&record, "first name"), get(&record, "last name")) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            (Some(name), None) | (None, Some(name)) => Some(name),
            (None, None) => None,
        };

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

            user_email: email.or(user_email),
            user_name: name,

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

    if rows.is_empty() && errors.is_empty() {
        return Err(CsvError::Empty);
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
        assert_eq!(r.user_name.as_deref(), Some("Dana Dev"));
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
        assert!(matches!(
            parse_csv(b"Date,Client,Project,Task,Hours,Email\n"),
            Err(CsvError::Empty)
        ));
    }

    #[test]
    fn identity_columns_are_required_and_cannot_repeat() {
        assert!(matches!(
            parse_csv(b"Date,Client,Project,Task,Hours\n2026-01-15,A,P,T,1\n"),
            Err(CsvError::Unrecognized(_))
        ));
        for column in USER_COLUMNS {
            let csv = format!(
                "Date,Client,Project,Task,Hours,{column}, {} \n",
                column.to_uppercase()
            );
            assert!(matches!(
                parse_csv(csv.as_bytes()),
                Err(CsvError::Identity(_))
            ));
        }
    }

    #[test]
    fn names_stay_separate_from_optional_email() {
        let (rows, errors) = parse_csv(b"Date,Client,Project,Task,Hours,First Name,Last Name,User Email\n2026-01-15,A,P,T,1, Dana , Dev ,\n2026-01-15,A,P,T,1,Dana,,dev@acme.com\n").unwrap();
        assert!(errors.is_empty());
        assert_eq!(rows[0].user_email, None);
        assert_eq!(rows[0].user_name.as_deref(), Some("Dana Dev"));
        assert_eq!(rows[1].user_email.as_deref(), Some("dev@acme.com"));
        assert_eq!(rows[1].user_name.as_deref(), Some("Dana"));
    }

    #[test]
    fn conflicting_email_aliases_error_only_the_affected_row() {
        let (rows, errors) = parse_csv(b"Date,Client,Project,Task,Hours,Email,User Email\n2026-01-15,A,P,T,1,dev@acme.com,other@acme.com\n2026-01-15,A,P,T,1,DEV@ACME.COM, dev@acme.com \n2026-01-15,A,P,T,1,,dev@acme.com\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].source_location, "CSV line 2");
        assert!(errors[0].reason.contains("disagree"));
        assert_eq!(rows[1].user_email.as_deref(), Some("dev@acme.com"));
    }

    #[test]
    fn missing_required_columns_is_rejected() {
        let bad = "Foo,Bar\n1,2\n";
        let err = parse_csv(bad.as_bytes()).unwrap_err();
        assert!(matches!(err, CsvError::Unrecognized(_)));
    }

    #[test]
    fn bad_date_becomes_a_row_error_not_a_rejection() {
        let csv = "Date,Client,Project,Task,Hours,Billable?,Currency,Email\n\
not-a-date,Acme,Website,Design,1.5,Yes,USD,dev@acme.com\n";
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
