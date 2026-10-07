use std::collections::HashSet;

use chrono::NaiveDate;
use dioxus::prelude::ServerFnError;
use uuid::Uuid;

use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{TimesheetPage, TimesheetQuery};

type SheetResult = Result<TimesheetPage, ServerFnError>;

/// A resource may still be Ready for the old week before its restart is polled.
pub(super) fn current(
    ready: bool,
    week_start: NaiveDate,
    loaded: &Option<(NaiveDate, SheetResult)>,
) -> Option<&SheetResult> {
    loaded
        .as_ref()
        .filter(|(week, _)| ready && *week == week_start)
        .map(|(_, result)| result)
}

/// Collect the complete visible period; never present partial pages as totals.
pub(super) async fn load(
    subject_id: Option<Uuid>,
    date_from: NaiveDate,
    date_to: NaiveDate,
    expected_requester: Option<PermissionRequester>,
    mut fetch: impl AsyncFnMut(TimesheetQuery) -> Result<TimesheetPage, ServerFnError>,
) -> Result<TimesheetPage, ServerFnError> {
    let mut query = TimesheetQuery {
        subject_id,
        date_from,
        date_to,
        after: None,
        expected_requester,
        expected_policy: None,
    };
    let mut complete = fetch(query.clone()).await?;
    if complete.subject.id != subject_id.unwrap_or(complete.requester.user_id)
        || expected_requester.is_some_and(|expected| expected != complete.requester)
    {
        return Err(ServerFnError::new(
            "Timesheet identity changed; reload the page.",
        ));
    }
    query.subject_id = Some(complete.subject.id);
    query.expected_requester = Some(complete.requester);
    query.expected_policy = Some(complete.policy);
    let mut seen = HashSet::new();
    validate_entries(&complete, date_from, date_to, &mut seen)?;
    while let Some(after) = complete.next_after.take() {
        if !(date_from..=date_to).contains(&after.spent_date)
            || query.after.as_ref().is_some_and(|previous| {
                (after.spent_date, after.created_at, after.id)
                    >= (previous.spent_date, previous.created_at, previous.id)
            })
        {
            return Err(ServerFnError::new(
                "Timesheet pagination changed; retry loading the week.",
            ));
        }
        query.after = Some(after);
        let page = fetch(query.clone()).await?;
        if page.requester != complete.requester
            || page.subject.id != complete.subject.id
            || page.policy != complete.policy
        {
            return Err(ServerFnError::new(
                "Timesheet identity changed; reload the page.",
            ));
        }
        validate_entries(&page, date_from, date_to, &mut seen)?;
        complete.entries.extend(page.entries);
        complete.next_after = page.next_after;
    }
    Ok(complete)
}

fn validate_entries(
    page: &TimesheetPage,
    date_from: NaiveDate,
    date_to: NaiveDate,
    seen: &mut HashSet<Uuid>,
) -> Result<(), ServerFnError> {
    if page.entries.iter().any(|entry| {
        entry.user_id != page.subject.id
            || !(date_from..=date_to).contains(&entry.spent_date)
            || !seen.insert(entry.id)
    }) {
        return Err(ServerFnError::new(
            "Timesheet rows changed; retry loading the week.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::scoped_time::{
        TimeEntryCursor, TimesheetPerson, TimesheetPolicy, VisibleTimeEntry,
    };

    fn page() -> TimesheetPage {
        let id = Uuid::now_v7();
        TimesheetPage {
            requester: PermissionRequester {
                org_id: Uuid::now_v7(),
                user_id: id,
            },
            subject: TimesheetPerson {
                id,
                name: "Selected person".into(),
            },
            policy: TimesheetPolicy::Scoped,
            entries: Vec::new(),
            next_after: None,
        }
    }

    fn date() -> NaiveDate {
        "2026-09-07".parse().unwrap()
    }

    fn cursor() -> TimeEntryCursor {
        TimeEntryCursor {
            spent_date: date(),
            created_at: chrono::Utc::now(),
            id: Uuid::now_v7(),
        }
    }

    fn entry(subject: Uuid, minutes: i32) -> VisibleTimeEntry {
        VisibleTimeEntry {
            id: Uuid::now_v7(),
            user_id: subject,
            user_name: "Person".into(),
            project_id: Uuid::now_v7(),
            project_name: "Archived project".into(),
            task_id: Uuid::now_v7(),
            task_name: "Archived task".into(),
            client_id: Uuid::now_v7(),
            client_name: "Historical client".into(),
            spent_date: date(),
            minutes,
            rounded_minutes: None,
            notes: None,
            billable: true,
            is_running: false,
            started_at: None,
            start_minute: None,
            sort_order: 0,
            state: horae_core::types::EntryState::Open,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn old_week_results_and_errors_are_not_current_even_before_restart() {
        for result in [Ok(page()), Err(ServerFnError::new("Old week's failure"))] {
            let loaded = Some((date(), result));
            assert!(current(true, date().succ_opt().unwrap(), &loaded).is_none());
            assert!(current(false, date(), &loaded).is_none());
            assert!(current(true, date(), &loaded).is_some());
        }
    }

    #[tokio::test]
    async fn completes_pages_while_pinning_subject_requester_and_policy() {
        let mut first = page();
        first.entries.push(entry(first.subject.id, 90));
        first.next_after = Some(cursor());
        let mut last = first.clone();
        last.entries = vec![entry(first.subject.id, 45)];
        last.next_after = None;
        let mut queries = Vec::new();
        let loaded = load(None, date(), date(), None, async |query| {
            queries.push(query);
            Ok(if queries.len() == 1 {
                first.clone()
            } else {
                last.clone()
            })
        })
        .await
        .unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[1].subject_id, Some(first.subject.id));
        assert_eq!(queries[1].expected_requester, Some(first.requester));
        assert_eq!(queries[1].expected_policy, Some(first.policy));
        assert_eq!(queries[1].after, first.next_after);
        assert_eq!(loaded.next_after, None);
        assert_eq!(
            loaded
                .entries
                .iter()
                .map(|entry| entry.minutes)
                .sum::<i32>(),
            135
        );
        assert_eq!(loaded.entries, [first.entries, last.entries].concat());
    }

    #[tokio::test]
    async fn later_page_failure_does_not_return_partial_totals() {
        let mut first = page();
        first.entries.push(entry(first.subject.id, 90));
        first.next_after = Some(cursor());
        let mut calls = 0;
        let result = load(None, date(), date(), None, async |_| {
            calls += 1;
            if calls == 1 {
                Ok(first.clone())
            } else {
                Err(ServerFnError::new("Access revoked"))
            }
        })
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn refuses_a_changed_requester_subject_or_policy_between_pages() {
        for changed in 0..3 {
            let mut first = page();
            first.next_after = Some(cursor());
            let mut last = first.clone();
            last.next_after = None;
            match changed {
                0 => last.requester.user_id = Uuid::now_v7(),
                1 => last.subject.id = Uuid::now_v7(),
                _ => last.policy = TimesheetPolicy::LegacyOwn,
            }
            let mut calls = 0;
            assert!(
                load(None, date(), date(), None, async |_| {
                    calls += 1;
                    Ok(if calls == 1 {
                        first.clone()
                    } else {
                        last.clone()
                    })
                })
                .await
                .is_err()
            );
        }
    }

    #[tokio::test]
    async fn rejects_a_nonadvancing_cursor_instead_of_looping() {
        let mut first = page();
        first.next_after = Some(cursor());
        let mut calls = 0;
        assert!(
            load(None, date(), date(), None, async |_| {
                calls += 1;
                assert!(calls <= 2, "must not loop on a repeated cursor");
                Ok(first.clone())
            })
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_the_wrong_first_subject_or_requester() {
        let first = page();
        assert!(
            load(Some(Uuid::now_v7()), date(), date(), None, async |_| Ok(
                first.clone()
            ))
            .await
            .is_err()
        );
        assert!(
            load(
                None,
                date(),
                date(),
                Some(PermissionRequester {
                    user_id: Uuid::now_v7(),
                    ..first.requester
                }),
                async |_| Ok(first.clone())
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_mixed_subjects_out_of_window_rows_and_duplicates() {
        for invalid in 0..3 {
            let mut first = page();
            let mut row = entry(first.subject.id, 30);
            match invalid {
                0 => row.user_id = Uuid::now_v7(),
                1 => row.spent_date = date().succ_opt().unwrap(),
                _ => first.entries.push(row.clone()),
            }
            first.entries.push(row);
            assert!(
                load(None, date(), date(), None, async |_| Ok(first.clone()))
                    .await
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn rejects_rows_repeated_across_pages() {
        let mut first = page();
        first.entries.push(entry(first.subject.id, 30));
        first.next_after = Some(cursor());
        let mut last = first.clone();
        last.next_after = None;
        let mut calls = 0;
        assert!(
            load(None, date(), date(), None, async |_| {
                calls += 1;
                Ok(if calls == 1 {
                    first.clone()
                } else {
                    last.clone()
                })
            })
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn empty_authorized_sheet_is_a_complete_result() {
        let first = page();
        assert_eq!(
            load(
                Some(first.subject.id),
                date(),
                date(),
                Some(first.requester),
                async |_| Ok(first.clone())
            )
            .await
            .unwrap(),
            first
        );
    }
}
