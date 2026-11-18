//! New own time and the tracking picker, for every profile.

use super::*;

async fn linked_seed(pool: &PgPool, profile: BuiltInProfile) -> SeedIds {
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

fn manual_entry(ids: &SeedIds) -> NewTimeEntry<'static> {
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
            let mut read =
                crate::ops::begin_read(&pool, crate::ops::Caller::session(ids.org_id, ids.user_id))
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
    assert!(matches!(err, OpError::Conflict(_)), "{err:?}");
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
        crate::testing::wait_for_blocked(&pool, blocker).await;
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
