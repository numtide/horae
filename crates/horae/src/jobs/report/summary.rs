//! Group a run's errors on the server, so the run page shows what went wrong
//! without loading every error row (spec 019 FR-017).

use std::collections::HashMap;

use horae_core::importers::harvest::types::{EntityType, PersonErrorKind, RowError};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::models::{ErrorGroup, ErrorSummary, MissingPerson};

const SHOWN_GROUPS: usize = 50;
const SHOWN_PEOPLE: usize = 500;
/// Distinct groups and people tracked before the rest are only counted, so a
/// run whose every reason differs still folds in bounded memory.
const TRACKED: usize = 10_000;
const SAMPLES: usize = 3;

#[derive(Clone, PartialEq, Eq, Hash)]
enum GroupKey {
    Person(EntityType, PersonErrorKind),
    Reason(EntityType, String),
}

/// Folds archived NDJSON chunks and then the inline errors, in order.
#[derive(Default)]
pub(crate) struct Summarizer {
    groups: HashMap<GroupKey, ErrorGroup>,
    people: HashMap<String, MissingPerson>,
    untracked_errors: u64,
    untracked_people: u64,
    /// The start of a line that continues in the next chunk.
    carry: Vec<u8>,
}

impl Summarizer {
    pub(crate) fn feed(&mut self, chunk: &[u8]) -> anyhow::Result<()> {
        let mut rest = chunk;
        while let Some(end) = rest.iter().position(|byte| *byte == b'\n') {
            let line = if self.carry.is_empty() {
                &rest[..end]
            } else {
                self.carry.extend_from_slice(&rest[..end]);
                self.carry.as_slice()
            };
            let error: RowError = serde_json::from_slice(line)?;
            self.carry.clear();
            self.add(error);
            rest = &rest[end + 1..];
        }
        self.carry.extend_from_slice(rest);
        Ok(())
    }

    pub(crate) fn add(&mut self, error: RowError) {
        let kind = error.person.as_ref().map(|person| person.kind);
        if let Some(person) = &error.person {
            self.add_person(error.entity, person);
        }
        let key = match kind {
            Some(kind) => GroupKey::Person(error.entity, kind),
            None => GroupKey::Reason(error.entity, error.reason.clone()),
        };
        let tracked = self.groups.len();
        let group = match self.groups.get_mut(&key) {
            Some(group) => group,
            None if tracked < TRACKED => self.groups.entry(key).or_insert(ErrorGroup {
                entity: error.entity,
                kind,
                reason: error.reason,
                count: 0,
                samples: Vec::new(),
            }),
            None => {
                self.untracked_errors += 1;
                return;
            }
        };
        group.count += 1;
        if group.samples.len() < SAMPLES {
            group.samples.push(error.source_location);
        }
    }

    fn add_person(
        &mut self,
        entity: EntityType,
        person: &horae_core::importers::harvest::types::PersonError,
    ) {
        let Some(key) = person
            .email
            .as_deref()
            .or(person.name.as_deref())
            .map(|key| key.trim().to_lowercase())
            .filter(|key| !key.is_empty())
        else {
            return;
        };
        let tracked = self.people.len();
        let missing = match self.people.get_mut(&key) {
            Some(missing) => missing,
            None if tracked < TRACKED => self.people.entry(key).or_insert(MissingPerson {
                email: person.email.clone(),
                name: person.name.clone(),
                kind: person.kind,
                entries: 0,
            }),
            None => {
                self.untracked_people += 1;
                return;
            }
        };
        // The person's own error explains their entries' "not imported".
        if missing.kind == PersonErrorKind::PersonNotImported {
            missing.kind = person.kind;
        }
        missing.name = missing.name.take().or_else(|| person.name.clone());
        if entity == EntityType::TimeEntry {
            missing.entries += 1;
        }
    }

    pub(crate) fn finish(self) -> anyhow::Result<ErrorSummary> {
        anyhow::ensure!(self.carry.is_empty(), "report archive ends mid-line");
        let mut groups: Vec<_> = self.groups.into_values().collect();
        groups.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.reason.cmp(&b.reason))
                .then_with(|| a.entity.as_str().cmp(b.entity.as_str()))
        });
        let omitted_errors = self.untracked_errors
            + groups
                .iter()
                .skip(SHOWN_GROUPS)
                .map(|group| group.count)
                .sum::<u64>();
        groups.truncate(SHOWN_GROUPS);
        let mut people: Vec<_> = self.people.into_values().collect();
        people.sort_by(|a, b| {
            b.entries
                .cmp(&a.entries)
                .then_with(|| a.email.cmp(&b.email))
                .then_with(|| a.name.cmp(&b.name))
        });
        let omitted_people = self.untracked_people
            + u64::try_from(people.len().saturating_sub(SHOWN_PEOPLE)).unwrap_or(u64::MAX);
        people.truncate(SHOWN_PEOPLE);
        Ok(ErrorSummary {
            groups,
            people,
            omitted_errors,
            omitted_people,
        })
    }
}

/// Summarize one job's archived and inline errors, read within the caller's
/// admin access transaction. `None` when the job is not this organization's.
pub(crate) async fn summarize(
    connection: &mut PgConnection,
    org_id: Uuid,
    job_id: Uuid,
) -> anyhow::Result<Option<ErrorSummary>> {
    let Some(job) = super::super::status(&mut *connection, org_id, job_id).await? else {
        return Ok(None);
    };
    let Some(report) = job.report else {
        return Ok(Some(ErrorSummary::default()));
    };
    let report: horae_core::importers::harvest::types::ImportReport =
        serde_json::from_value(report)?;
    let end = i64::try_from(report.archived_error_chunks())?;
    let mut summary = Summarizer::default();
    let mut next = 0;
    while next < end {
        let page = super::chunks(&mut *connection, org_id, job_id, next, end).await?;
        next += i64::try_from(page.len())?;
        for chunk in &page {
            summary.feed(chunk)?;
        }
    }
    for error in report.row_errors {
        summary.add(error);
    }
    summary.finish().map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::importers::harvest::types::PersonError;

    fn error(entity: EntityType, location: &str, reason: &str) -> RowError {
        RowError {
            source_location: location.into(),
            entity,
            reason: reason.into(),
            person: None,
        }
    }

    fn person_error(
        entity: EntityType,
        location: &str,
        kind: PersonErrorKind,
        email: Option<&str>,
        name: Option<&str>,
    ) -> RowError {
        RowError {
            person: Some(PersonError {
                kind,
                email: email.map(Into::into),
                name: name.map(Into::into),
            }),
            ..error(entity, location, &format!("{kind:?} {email:?}"))
        }
    }

    fn ndjson(errors: &[RowError]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for error in errors {
            serde_json::to_writer(&mut bytes, error).unwrap();
            bytes.push(b'\n');
        }
        bytes
    }

    #[test]
    fn lines_split_across_chunks_are_reassembled() {
        let errors: Vec<_> = (0..7)
            .map(|n| error(EntityType::TimeEntry, &format!("row {n}"), "bad hours"))
            .collect();
        let bytes = ndjson(&errors);
        for size in [1, 5, 64, bytes.len()] {
            let mut summary = Summarizer::default();
            for chunk in bytes.chunks(size) {
                summary.feed(chunk).unwrap();
            }
            let summary = summary.finish().unwrap();
            assert_eq!(summary.groups.len(), 1, "chunk size {size}");
            assert_eq!(summary.groups[0].count, 7);
            assert_eq!(summary.groups[0].samples, ["row 0", "row 1", "row 2"]);
        }
    }

    #[test]
    fn an_archive_ending_mid_line_is_an_error() {
        let bytes = ndjson(&[error(EntityType::Client, "row 1", "x")]);
        let mut summary = Summarizer::default();
        summary.feed(&bytes[..bytes.len() - 2]).unwrap();
        assert!(summary.finish().is_err());
    }

    #[test]
    fn person_errors_group_by_kind_and_others_by_reason() {
        let mut summary = Summarizer::default();
        for (n, email) in ["a@x.com", "b@x.com", "a@x.com"].into_iter().enumerate() {
            summary.add(person_error(
                EntityType::TimeEntry,
                &format!("row {n}"),
                PersonErrorKind::UnmatchedEmail,
                Some(email),
                None,
            ));
        }
        summary.add(error(EntityType::TimeEntry, "row 9", "bad hours"));
        summary.add(error(EntityType::Project, "row 9", "bad hours"));
        let summary = summary.finish().unwrap();
        let groups: Vec<_> = summary
            .groups
            .iter()
            .map(|group| (group.entity, group.kind, group.count))
            .collect();
        assert_eq!(
            groups,
            [
                (
                    EntityType::TimeEntry,
                    Some(PersonErrorKind::UnmatchedEmail),
                    3
                ),
                (EntityType::Project, None, 1),
                (EntityType::TimeEntry, None, 1),
            ]
        );
        let people: Vec<_> = summary
            .people
            .iter()
            .map(|person| (person.email.as_deref(), person.entries))
            .collect();
        assert_eq!(people, [(Some("a@x.com"), 2), (Some("b@x.com"), 1)]);
    }

    #[test]
    fn a_person_takes_the_kind_of_their_own_error_and_counts_entries_only() {
        let mut summary = Summarizer::default();
        for n in 0..2 {
            summary.add(person_error(
                EntityType::TimeEntry,
                &format!("time_entry {n}"),
                PersonErrorKind::PersonNotImported,
                None,
                Some("No Mail"),
            ));
        }
        summary.add(person_error(
            EntityType::Person,
            "person 7",
            PersonErrorKind::MissingEmail,
            None,
            Some("No Mail"),
        ));
        let summary = summary.finish().unwrap();
        assert_eq!(
            summary.people,
            [MissingPerson {
                email: None,
                name: Some("No Mail".into()),
                kind: PersonErrorKind::MissingEmail,
                entries: 2,
            }]
        );
    }

    #[test]
    fn caps_report_what_they_leave_out() {
        let mut summary = Summarizer::default();
        for n in 0..(SHOWN_GROUPS + 5) {
            summary.add(error(EntityType::TimeEntry, "row", &format!("reason {n}")));
        }
        for n in 0..(SHOWN_PEOPLE + 3) {
            summary.add(person_error(
                EntityType::TimeEntry,
                "row",
                PersonErrorKind::UnmatchedEmail,
                Some(&format!("p{n}@x.com")),
                None,
            ));
        }
        let summary = summary.finish().unwrap();
        assert_eq!(summary.groups.len(), SHOWN_GROUPS);
        // One person group holds the people's errors; six reason groups fall off.
        assert_eq!(summary.omitted_errors, 6);
        assert_eq!(summary.people.len(), SHOWN_PEOPLE);
        assert_eq!(summary.omitted_people, 3);
    }
}
