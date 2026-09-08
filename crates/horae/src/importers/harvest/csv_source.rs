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
#[cfg(test)]
use horae_core::importers::harvest::types::ImportMode;
use horae_core::importers::harvest::types::SourceRow;
use std::collections::HashMap;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
use super::report::ImportReport;

mod upload;
pub use upload::import_body;

/// Columns that must be present for the file to be a recognizable export.
const REQUIRED: &[&str] = &["date", "client", "project", "task", "hours"];
const USER_COLUMNS: &[&str] = &["email", "user email", "first name", "last name"];

/// Run-level failures, distinct from recoverable record validation errors.
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

/// Deliver records as they are read. Transport failures reject the whole run;
/// malformed records remain reportable row errors.
fn read_csv(
    input: impl std::io::Read,
    mut emit: impl FnMut(Result<SourceRow, ParseErr>) -> anyhow::Result<()>,
) -> Result<(), CsvError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(input);

    let headers = reader
        .headers()
        .map_err(|e| CsvError::Other(e.into()))?
        .clone();
    if headers.iter().all(|header| header.trim().is_empty()) {
        return Err(CsvError::Empty);
    }
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

    let mut seen_record = false;

    for (i, record) in reader.records().enumerate() {
        seen_record = true;
        // Harvest's data rows start at CSV line 2 (after the header).
        let line = i + 2;
        let location = format!("CSV line {line}");
        let record = match record {
            Ok(r) => r,
            Err(e) if e.is_io_error() => return Err(CsvError::Other(e.into())),
            Err(e) => {
                emit(Err(ParseErr {
                    source_location: location,
                    reason: format!("malformed CSV row: {e}"),
                }))?;
                continue;
            }
        };

        let date_str = get(&record, "date");
        let spent_date = match date_str.as_deref().map(parse_date) {
            Some(Ok(d)) => d,
            Some(Err(reason)) => {
                emit(Err(ParseErr {
                    source_location: location,
                    reason,
                }))?;
                continue;
            }
            None => {
                emit(Err(ParseErr {
                    source_location: location,
                    reason: "missing Date".to_string(),
                }))?;
                continue;
            }
        };

        let email = get(&record, "email");
        let user_email = get(&record, "user email");
        if let (Some(email), Some(alias)) = (&email, &user_email)
            && horae_core::importers::harvest::keys::normalize(email)
                != horae_core::importers::harvest::keys::normalize(alias)
        {
            emit(Err(ParseErr {
                source_location: location,
                reason: "Email and User Email disagree; provide one user identity".into(),
            }))?;
            continue;
        }
        let name = match (get(&record, "first name"), get(&record, "last name")) {
            (Some(first), Some(last)) => Some(format!("{first} {last}")),
            (Some(name), None) | (None, Some(name)) => Some(name),
            (None, None) => None,
        };

        emit(Ok(SourceRow {
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
        }))?;
    }

    if !seen_record {
        return Err(CsvError::Empty);
    }
    Ok(())
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
#[cfg(test)]
pub async fn import_csv(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    default_currency: &str,
    bytes: &[u8],
    mode: ImportMode,
) -> Result<ImportReport, CsvError> {
    import_body(
        pool,
        org_id,
        default_currency,
        axum::body::Body::from(bytes.to_vec()),
        mode,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_csv(bytes: &[u8]) -> Result<(Vec<SourceRow>, Vec<ParseErr>), CsvError> {
        let mut rows = Vec::new();
        let mut errors = Vec::new();
        read_csv(bytes, |record| {
            match record {
                Ok(row) => rows.push(row),
                Err(error) => errors.push(error),
            }
            Ok(())
        })?;
        Ok((rows, errors))
    }

    #[test]
    fn delivers_a_row_before_reading_the_rest_of_the_file() {
        use std::cell::Cell;
        use std::io::{self, Read};

        struct Gated<'a> {
            first: &'a [u8],
            tail: &'a [u8],
            delivered: &'a Cell<usize>,
        }
        impl Read for Gated<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if !self.first.is_empty() {
                    return self.first.read(buffer);
                }
                if self.delivered.get() == 0 {
                    return Err(io::Error::other(
                        "read ahead before delivering the first row",
                    ));
                }
                self.tail.read(buffer)
            }
        }

        let delivered = Cell::new(0);
        let input = Gated {
            first: b"Date,Client,Project,Task,Hours,Email\n2026-01-15,A,P,T,1,dev@acme.com\n",
            tail: b"2026-01-16,A,P,T,2,dev@acme.com\n",
            delivered: &delivered,
        };
        read_csv(input, |row| {
            assert!(row.is_ok(), "{row:?}");
            delivered.set(delivered.get() + 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(delivered.get(), 2);
    }

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
    fn csv_records_preserve_multiline_unicode_across_single_byte_reads() {
        use std::io::{self, Read};
        struct OneByte<'a>(&'a [u8]);
        impl Read for OneByte<'_> {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                let count = output.len().min(1);
                self.0.read(&mut output[..count])
            }
        }
        let csv = "Date,Client,Project,Task,Hours,Email,Notes\r\n2026-01-15,A,P,T,1,d@a.com,\"mañana, revisión\nsegunda línea\"\r\n2026-01-16,A,P,T,2,d@a.com,fin\r\n";
        let mut rows = Vec::new();
        read_csv(OneByte(csv.as_bytes()), |row| {
            rows.push(row.unwrap());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            rows[0].notes.as_deref(),
            Some("mañana, revisión\nsegunda línea")
        );
        assert_eq!(rows[1].source_location, "CSV line 3");
        assert_eq!(rows[1].notes.as_deref(), Some("fin"));
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
    fn invalid_utf8_errors_one_record_and_continues() {
        let bytes = b"Date,Client,Project,Task,Hours,Email,Notes\n2026-01-15,A,P,T,1,d@a.com,\xff\n2026-01-16,A,P,T,2,d@a.com,valid\n";
        let (rows, errors) = parse_csv(bytes).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].source_location, "CSV line 2");
        assert!(errors[0].reason.contains("malformed CSV row"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].notes.as_deref(), Some("valid"));
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
