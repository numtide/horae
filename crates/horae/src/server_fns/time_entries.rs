//! Time-entry server functions.

use super::*;
#[cfg(feature = "server")]
use crate::ops::{self, Caller};

/// The Timesheet's session check: a signed-out person gets 401, anything else
/// reads as the sheet being unavailable.
#[cfg(feature = "server")]
async fn timesheet_user() -> Result<crate::models::User, ServerFnError> {
    require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Timesheet operation failed");
            server_err("Timesheet is unavailable")
        }
    })
}

/// Eligible, writable tracking choices for one explicitly selected person.
#[server]
pub async fn load_timesheet_tracking(
    context: crate::models::scoped_time::TimesheetWriteContext,
) -> Result<Vec<crate::models::scoped_time::TimesheetTrackingOption>, ServerFnError> {
    let user = timesheet_user().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::tracking(&mut read.tx, &read.actor, &context).await;
    Ok(read.finish(result).await?)
}

/// Apply one person-bound operation; client expectations never grant authority.
/// Returns a notice for the person when the operation was adjusted, e.g. a
/// stopped timer clamped to the daily limit.
#[server]
pub async fn apply_timesheet_command(
    context: crate::models::scoped_time::TimesheetWriteContext,
    command: crate::models::scoped_time::TimesheetCommand,
) -> Result<Option<String>, ServerFnError> {
    let user = timesheet_user().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    let change = write.commit(result, &*state.plugins).await?;
    for project in change.projects {
        tokio::spawn(check_project_budget(state, project));
    }
    Ok(change.notice)
}

/// Resolve a Timesheet subject and its rows without a separate identity bootstrap.
#[server]
pub async fn load_timesheet_page(
    query: crate::models::scoped_time::TimesheetQuery,
) -> Result<crate::models::scoped_time::TimesheetPage, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate Timesheet read");
            server_err("Timesheet is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time_reads::sheet(&mut read.tx, &read.actor, &query).await;
    Ok(read.finish(result).await?)
}

/// Minimal authorized person labels; selection never grants entry access.
#[server]
pub async fn list_timesheet_people(
    query: crate::models::scoped_time::TimesheetPeopleQuery,
) -> Result<crate::models::scoped_time::TimesheetPeoplePage, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate timesheet people read");
            server_err("Timesheet people are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time_reads::people(&mut read.tx, &read.actor, &query).await;
    Ok(read.finish(result).await?)
}

/// Read a bounded page of time entries under current canonical permissions.
#[server]
pub async fn list_visible_time_entries(
    query: crate::models::scoped_time::TimeEntryQuery,
) -> Result<crate::models::scoped_time::TimeEntryPage, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate time-entry read");
            server_err("Time entries are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time_reads::entries(&mut read.tx, &read.actor, &query).await;
    Ok(read.finish(result).await?)
}

/// Only the session user's active, assigned project/task combinations; no rates.
#[server]
pub async fn list_time_entry_contexts()
-> Result<Vec<crate::models::time_entry::TimeEntryContext>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::contexts(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// Start a timer for the given project and task; a second running timer is a
/// conflict.
#[server]
pub async fn start_timer(
    project_id: String,
    task_id: String,
    notes: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::start_timer(
        &mut write.tx,
        &write.actor,
        project_id,
        task_id,
        notes.as_deref(),
    )
    .await;
    Ok(write.commit(result, &*state.plugins).await?)
}

/// Stop a running timer and record elapsed minutes, clamped to what is left of
/// the day; the notice, when present, says how much the daily limit cut.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<(TimeEntry, Option<String>), ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::stop_timer(&mut write.tx, &write.actor, entry_id).await;
    let (entry, notice) = write.commit(result, &*state.plugins).await?;
    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok((entry, notice))
}

/// Return the currently running timer for the authenticated user, if any.
#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::time::current_timer(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// The timer operations run as the session adapters run them, for tests that
/// name only the person. An unknown person gets an organization of their own,
/// so the actor carries no authority.
#[cfg(all(test, feature = "server"))]
async fn session_write(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> ops::WriteTx {
    let org_id = sqlx::query_scalar!("SELECT org_id FROM users WHERE id=$1", user_id)
        .fetch_optional(pool)
        .await
        .unwrap()
        .unwrap_or_else(uuid::Uuid::now_v7);
    ops::begin(pool, Caller::session(org_id, user_id))
        .await
        .unwrap()
}

/// New own time for tests: a running entry starts the timer (which files it on
/// today, with no minutes, start or billability request); any other is created
/// on the person's own Timesheet.
#[cfg(all(test, feature = "server"))]
struct NewTimeEntry<'a> {
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    spent_date: chrono::NaiveDate,
    minutes: i32,
    notes: Option<&'a str>,
    billable: bool,
    start_minute: Option<i32>,
    is_running: bool,
}

#[cfg(all(test, feature = "server"))]
async fn insert_time_entry(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    input: NewTimeEntry<'_>,
) -> Result<TimeEntry, ServerFnError> {
    use crate::models::permission_editor::PermissionRequester;
    use crate::models::scoped_time::{
        TimesheetCommand, TimesheetEntryInput, TimesheetWriteContext,
    };
    use crate::plugin::AppEvent;

    if input.is_running {
        return start_timer_record(pool, user_id, input.project_id, input.task_id, input.notes)
            .await;
    }
    let mut write = session_write(pool, user_id).await;
    let context = TimesheetWriteContext {
        expected_requester: PermissionRequester {
            org_id: write.actor.org_id(),
            user_id,
        },
        subject_id: user_id,
    };
    let command = TimesheetCommand::Create {
        entry: TimesheetEntryInput {
            project_id: input.project_id,
            task_id: input.task_id,
            spent_date: input.spent_date,
            minutes: input.minutes,
            notes: input.notes.map(str::to_owned),
            billable: input.billable,
            start_minute: input.start_minute,
        },
    };
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    let events = crate::ops::testing::Recorder::default();
    write.commit(result, &events).await?;
    let [AppEvent::TimeEntryCreated { time_entry, .. }] = &events.take()[..] else {
        panic!("a created entry announces itself once");
    };
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
           FROM time_entries WHERE id = $1"#,
        time_entry.id,
    )
    .fetch_one(pool)
    .await
    .unwrap())
}

#[cfg(all(test, feature = "server"))]
async fn start_timer_record(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    notes: Option<&str>,
) -> Result<TimeEntry, ServerFnError> {
    let mut write = session_write(pool, user_id).await;
    let result =
        ops::time::start_timer(&mut write.tx, &write.actor, project_id, task_id, notes).await;
    Ok(write
        .commit(result, &crate::ops::testing::Recorder::default())
        .await?)
}

#[cfg(all(test, feature = "server"))]
async fn stop_entry_timer(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    entry_id: uuid::Uuid,
) -> Result<(TimeEntry, Option<String>), ServerFnError> {
    let mut write = session_write(pool, user_id).await;
    let result = ops::time::stop_timer(&mut write.tx, &write.actor, entry_id).await;
    Ok(write
        .commit(result, &crate::ops::testing::Recorder::default())
        .await?)
}

#[cfg(all(test, feature = "server"))]
async fn fetch_time_entry_contexts(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Vec<crate::models::time_entry::TimeEntryContext>, ServerFnError> {
    let mut write = session_write(pool, user_id).await;
    let result = ops::time::contexts(&mut write.tx, &write.actor)
        .await
        .map(ops::Done::new);
    Ok(write
        .commit(result, &crate::ops::testing::Recorder::default())
        .await?)
}

#[cfg(all(test, feature = "server"))]
mod update_tests;

#[cfg(all(test, feature = "server"))]
mod activity_tests;

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        CONFLICT, NewTimeEntry, fetch_time_entry_contexts, insert_time_entry, start_timer_record,
    };
    use crate::server_fns::test_seed::seed;
    use dioxus::prelude::ServerFnError;
    use horae_core::permissions::catalog::BuiltInProfile;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn linked_seed(
        pool: &PgPool,
        profile: BuiltInProfile,
    ) -> crate::server_fns::test_seed::SeedIds {
        let ids = seed(pool, profile).await;
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
            ids.project_id,
            ids.task_id
        )
        .execute(pool)
        .await
        .unwrap();
        ids
    }

    fn manual_entry(ids: &crate::server_fns::test_seed::SeedIds) -> NewTimeEntry<'static> {
        NewTimeEntry {
            project_id: ids.project_id,
            task_id: ids.task_id,
            spent_date: "2026-09-07".parse().unwrap(),
            minutes: 60,
            notes: None,
            billable: true,
            start_minute: None,
            is_running: false,
        }
    }

    /// Every profile tracks on projects it is assigned to; only the stored
    /// Administrator state also offers unassigned projects. The legacy picker
    /// and the Timesheet's own choices agree.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn picker_offers_assigned_projects_and_every_project_to_administrators(pool: PgPool) {
        use crate::models::permission_editor::PermissionRequester;
        use crate::models::scoped_time::TimesheetWriteContext;

        for profile in BuiltInProfile::ALL.iter().copied() {
            for assigned in [false, true] {
                let ids = linked_seed(&pool, profile).await;
                if assigned {
                    sqlx::query!(
                        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
                        Uuid::now_v7(),
                        ids.project_id,
                        ids.user_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                let expected = usize::from(assigned || profile == BuiltInProfile::Administrator);
                let picker = fetch_time_entry_contexts(&pool, ids.user_id).await.unwrap();
                assert_eq!(picker.len(), expected, "{profile:?} assigned={assigned}");
                let own = TimesheetWriteContext {
                    expected_requester: PermissionRequester {
                        org_id: ids.org_id,
                        user_id: ids.user_id,
                    },
                    subject_id: ids.user_id,
                };
                let mut read = crate::ops::begin_read(
                    &pool,
                    crate::ops::Caller::session(ids.org_id, ids.user_id),
                )
                .await
                .unwrap();
                let choices = crate::ops::time::tracking(&mut read.tx, &read.actor, &own).await;
                let choices = read.finish(choices).await.unwrap();
                assert_eq!(choices.len(), expected, "{profile:?} assigned={assigned}");
                let started = insert_time_entry(&pool, ids.user_id, manual_entry(&ids)).await;
                assert_eq!(started.is_ok(), expected == 1, "{profile:?} {started:?}");
            }
        }
    }

    /// A disabled project-task link closes the shared view for every profile,
    /// Administrator included.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn disabled_project_task_link_closes_the_view_for_every_profile(pool: PgPool) {
        for profile in BuiltInProfile::ALL.iter().copied() {
            let ids = linked_seed(&pool, profile).await;
            sqlx::query!(
                "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let offered = fetch_time_entry_contexts(&pool, ids.user_id).await.unwrap();
            assert_eq!(offered.len(), 1, "{profile:?}");
            sqlx::query!(
                "UPDATE project_tasks SET active = false WHERE project_id = $1",
                ids.project_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let offered = fetch_time_entry_contexts(&pool, ids.user_id).await.unwrap();
            assert!(offered.is_empty(), "{profile:?}");
            for is_running in [false, true] {
                let input = NewTimeEntry {
                    is_running,
                    ..manual_entry(&ids)
                };
                let refused = insert_time_entry(&pool, ids.user_id, input).await;
                assert!(refused.is_err(), "{profile:?} running={is_running}");
            }
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_timer_is_dated_in_the_persons_zone(pool: PgPool) {
        use chrono::Timelike;
        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
        // UTC+14 or UTC−12, whichever is on another date than UTC right now.
        let (zone, hours) = if chrono::Utc::now().hour() >= 11 {
            ("Pacific/Kiritimati", 14)
        } else {
            ("Etc/GMT+12", -12)
        };
        sqlx::query!(
            "UPDATE users SET timezone = $2 WHERE id = $1",
            ids.user_id,
            zone
        )
        .execute(&pool)
        .await
        .unwrap();
        let day = || (chrono::Utc::now() + chrono::Duration::hours(hours)).date_naive();
        let before = day();
        let entry = start_timer_record(&pool, ids.user_id, ids.project_id, ids.task_id, None)
            .await
            .unwrap();
        assert!([before, day()].contains(&entry.spent_date), "{entry:?}");
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_and_picker_reject_inactive_or_foreign_contexts(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
        let other = linked_seed(&pool, BuiltInProfile::Administrator).await;
        assert_eq!(
            fetch_time_entry_contexts(&pool, ids.user_id)
                .await
                .unwrap()
                .len(),
            1
        );
        for (project_id, task_id) in [
            (ids.project_id, other.task_id),
            (other.project_id, other.task_id),
            (ids.project_id, Uuid::now_v7()),
        ] {
            for is_running in [false, true] {
                let input = NewTimeEntry {
                    project_id,
                    task_id,
                    is_running,
                    ..manual_entry(&ids)
                };
                assert!(insert_time_entry(&pool, ids.user_id, input).await.is_err());
            }
        }
        for inactive in ["client", "project", "task", "user"] {
            sqlx::query!(
                "UPDATE clients SET active = ($2 <> 'client') WHERE id = $1",
                ids.client_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE projects SET active = ($2 <> 'project') WHERE id = $1",
                ids.project_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE tasks SET active = ($2 <> 'task') WHERE id = $1",
                ids.task_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE users SET active = ($2 <> 'user') WHERE id = $1",
                ids.user_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            // An inactive person is refused outright; otherwise the view is empty.
            let offered = fetch_time_entry_contexts(&pool, ids.user_id).await;
            assert!(
                offered.as_ref().map_or(inactive == "user", Vec::is_empty),
                "{inactive}: {offered:?}"
            );
            for is_running in [false, true] {
                assert!(
                    insert_time_entry(
                        &pool,
                        ids.user_id,
                        NewTimeEntry {
                            is_running,
                            ..manual_entry(&ids)
                        }
                    )
                    .await
                    .is_err(),
                    "{inactive}"
                );
            }
        }
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM time_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(0)
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_and_picker_require_a_project_task_link(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        assert!(
            fetch_time_entry_contexts(&pool, ids.user_id)
                .await
                .unwrap()
                .is_empty()
        );
        for is_running in [false, true] {
            assert!(
                insert_time_entry(
                    &pool,
                    ids.user_id,
                    NewTimeEntry {
                        is_running,
                        ..manual_entry(&ids)
                    }
                )
                .await
                .is_err()
            );
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_uses_effective_billability_for_manual_entries_and_timers(pool: PgPool) {
        for project_billable in [false, true] {
            for task_billable in [false, true] {
                for requested_billable in [false, true] {
                    for is_running in [false, true] {
                        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
                        sqlx::query!("UPDATE projects SET project_type = CASE WHEN $2 THEN 'time_and_materials'::project_type ELSE 'non_billable'::project_type END WHERE id = $1", ids.project_id, project_billable).execute(&pool).await.unwrap();
                        sqlx::query!("UPDATE project_tasks SET billable = $3 WHERE project_id = $1 AND task_id = $2", ids.project_id, ids.task_id, task_billable).execute(&pool).await.unwrap();
                        let context = fetch_time_entry_contexts(&pool, ids.user_id).await.unwrap();
                        assert_eq!(context[0].billable, project_billable && task_billable);
                        let entry = insert_time_entry(
                            &pool,
                            ids.user_id,
                            NewTimeEntry {
                                billable: requested_billable,
                                is_running,
                                ..manual_entry(&ids)
                            },
                        )
                        .await
                        .unwrap();
                        // A timer always asks for billable time.
                        assert_eq!(
                            entry.billable,
                            (requested_billable || is_running) && project_billable && task_billable
                        );
                        assert_eq!(entry.is_running, is_running);
                        assert_eq!(entry.started_at.is_some(), is_running);
                    }
                }
            }
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn concurrent_timer_starts_insert_exactly_one_entry(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
        let (first, second) = tokio::join!(
            insert_time_entry(
                &pool,
                ids.user_id,
                NewTimeEntry {
                    is_running: true,
                    ..manual_entry(&ids)
                }
            ),
            insert_time_entry(
                &pool,
                ids.user_id,
                NewTimeEntry {
                    is_running: true,
                    ..manual_entry(&ids)
                }
            ),
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM time_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(1)
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn inactive_project_cannot_accept_new_time_even_for_admin(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
        sqlx::query!(
            "UPDATE projects SET active = false WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_err()
        );
    }

    // New time is checked through the actual insert and the shared selector view.

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn unassigned_member_is_forbidden(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Member).await;

        let err = insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
            .await
            .expect_err("an unassigned member must be refused");
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, CONFLICT),
            other => panic!("expected a 409 ServerError, got {other:?}"),
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn assigned_member_may_log_time(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Member).await;
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            ids.project_id,
            ids.user_id,
        )
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn admin_may_log_time_without_assignment(pool: PgPool) {
        let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;

        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn submission_barrier_blocks_manual_entries_and_timer_starts(pool: PgPool) {
        for is_running in [false, true] {
            let ids = linked_seed(&pool, BuiltInProfile::Administrator).await;
            let mut submission = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *submission)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
                ids.user_id,
            ).execute(&mut *submission).await.unwrap();
            let run_pool = pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move {
                let mut input = manual_entry(&ids);
                input.is_running = is_running;
                insert_time_entry(&run_pool, ids.user_id, input).await
            });
            crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
            submission.commit().await.unwrap();
            let entry = tokio::time::timeout(std::time::Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(entry.is_running, is_running);
        }
    }
}
