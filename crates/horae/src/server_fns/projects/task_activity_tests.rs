use super::canonical_read_tests::fixture;
use super::*;
use crate::server_fns::test_seed::{time_entry, wait_for_blocked};
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn project_task_activity_migration_preserves_legacy_and_archives_canonical_links(
    pool: PgPool,
) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|migration| migration.version < 48)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let legacy = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    let (canonical, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    for ids in [&legacy, &canonical] {
        sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,true,1200)", ids.project_id, ids.task_id).execute(&pool).await.unwrap();
        time_entry(&pool, ids, EntryState::Open).await;
    }
    let legacy_revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM projects WHERE id=$1",
        legacy.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    for (ids, active) in [(&legacy, true), (&canonical, false)] {
        let row = sqlx::query!("SELECT active,billable,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2", ids.project_id, ids.task_id).fetch_one(&pool).await.unwrap();
        assert_eq!(
            (row.active, row.billable, row.rate_cents),
            (active, true, Some(1200))
        );
        assert_eq!(
            sqlx::query_scalar!(
                "SELECT sum(minutes)::bigint FROM time_entries WHERE org_id=$1",
                ids.org_id
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            Some(60)
        );
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT edit_revision FROM projects WHERE id=$1",
            legacy.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        legacy_revision
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn global_restore_does_not_restore_project_tracking_and_archive_invalidates_editor(
    pool: PgPool,
) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,true,1200)", ids.project_id, ids.task_id)
        .execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        uuid::Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM projects WHERE id=$1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut failures = Vec::new();
    for active in [false, true] {
        set_task_active_record(&pool, ids.org_id, ids.user_id, ids.task_id, active)
            .await
            .unwrap();
        let trackable = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM time_entry_contexts WHERE user_id=$1 AND project_id=$2 AND task_id=$3)", ids.user_id, ids.project_id, ids.task_id)
            .fetch_one(&pool).await.unwrap().unwrap();
        if trackable {
            failures.push("Global restore implicitly restored project tracking");
        }
        let current_revision = sqlx::query_scalar!(
            "SELECT edit_revision FROM projects WHERE id=$1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if current_revision <= revision {
            failures.push("Global archive did not invalidate the project editor");
        }
        let saved = sqlx::query!(
            "SELECT billable,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            ids.project_id,
            ids.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((saved.billable, saved.rate_cents), (true, Some(1200)));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn activity_separates_session_rates_from_events_and_preserves_noops(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=8000,default_rate_currency='EUR' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    for (active, changed) in [(false, true), (false, false), (true, true), (true, false)] {
        let before = sqlx::query_scalar!("SELECT xmin::text FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let (task, event) =
            set_task_active_record(&pool, ids.org_id, ids.user_id, ids.task_id, active)
                .await
                .unwrap();
        assert_eq!((task.active, task.default_rate_cents), (active, None));
        assert_eq!(event.is_some(), changed);
        if let Some(event) = event {
            let payload = match event {
                crate::plugin::AppEvent::TaskReactivated { org_id, task, .. } => {
                    assert_eq!(org_id, ids.org_id);
                    assert!(active);
                    task
                }
                crate::plugin::AppEvent::TaskDeactivated { org_id, task, .. } => {
                    assert_eq!(org_id, ids.org_id);
                    assert!(!active);
                    task
                }
                other => panic!("unexpected activity event: {other:?}"),
            };
            assert_eq!(payload.default_rate_cents, Some(8000));
        }
        let saved = sqlx::query!("SELECT xmin::text AS version,default_rate_cents,default_rate_currency FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(&pool).await.unwrap();
        assert_eq!(saved.version != before, changed);
        assert_eq!(
            (
                saved.default_rate_cents,
                saved.default_rate_currency.as_deref()
            ),
            (Some(8000), Some("EUR"))
        );
        let history = sqlx::query!(
            "SELECT minutes,billable,task_id FROM time_entries WHERE id=$1",
            entry
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (history.minutes, history.billable, history.task_id),
            (60, true, ids.task_id)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn activity_rejects_unavailable_authority_even_for_noops(pool: PgPool) {
    for case in [
        "missing",
        "malformed",
        "unknown_policy",
        "inactive",
        "no_grant",
    ] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&[Permission::TaskWriteAll]),
        )
        .await;
        match case {
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "malformed" => {
                sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown.permission'] WHERE user_id=$1", ids.user_id)
                    .execute(&pool).await.unwrap();
            }
            "unknown_policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version=2 WHERE id=$1",
                    ids.org_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "no_grant" => {
                let stored: Vec<String> = serde_json::from_value(
                    serde_json::to_value(PermissionSelection::new(&[])).unwrap(),
                )
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                    ids.user_id,
                    &stored
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        for active in [false, true] {
            let result =
                set_task_active_record(&pool, ids.org_id, ids.user_id, ids.task_id, active).await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{case}: {result:?}"
            );
        }
        assert!(
            sqlx::query_scalar!("SELECT active FROM tasks WHERE id=$1", ids.task_id)
                .fetch_one(&pool)
                .await
                .unwrap()
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn activity_observes_revocation_after_organization_wait(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    lock_organization(&mut hold, ids.org_id, OrganizationLock::AccessChange)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        set_task_active_record(&pending_pool, ids.org_id, ids.user_id, ids.task_id, false).await
    });
    wait_for_blocked(&pool, holder).await;
    let stored: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(&[])).unwrap())
            .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &stored
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    hold.commit().await.unwrap();
    let result = pending.await.unwrap();
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
    assert!(
        sqlx::query_scalar!("SELECT active FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn archive_waits_for_existing_time_writer_and_observes_committed_timer(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let mut writer = pool.begin().await.unwrap();
    lock_organization(&mut writer, ids.org_id, OrganizationLock::Shared)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET is_running=true,started_at=now() WHERE id=$1",
        entry
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        set_task_active_record(&pending_pool, ids.org_id, ids.user_id, ids.task_id, false).await
    });
    wait_for_blocked(&pool, holder).await;
    writer.commit().await.unwrap();
    let error = pending.await.unwrap().unwrap_err();
    assert!(
        matches!(error, ServerFnError::ServerError { code: CONFLICT, .. }),
        "{error:?}"
    );
    assert!(!error.to_string().contains(&entry.to_string()));
    assert!(!error.to_string().contains(&ids.user_id.to_string()));
    assert!(
        sqlx::query_scalar!("SELECT active FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn archive_excludes_new_tracking_writers_before_task_lock_and_commit(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM tasks WHERE id=$1 FOR UPDATE", ids.task_id)
        .fetch_one(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        set_task_active_record(&pending_pool, ids.org_id, ids.user_id, ids.task_id, false).await
    });
    wait_for_blocked(&pool, holder).await;
    let mut writer = pool.begin().await.unwrap();
    let error = sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR SHARE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    writer.rollback().await.unwrap();
    hold.commit().await.unwrap();
    let (task, event) = pending.await.unwrap().unwrap();
    assert!(!task.active);
    assert!(event.is_some());
}
