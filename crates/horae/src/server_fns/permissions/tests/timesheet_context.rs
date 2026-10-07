use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{TimesheetPolicy, TimesheetQuery};
use crate::server_fns::permissions::time_entries::{TimeReadError, sheet};
use crate::server_fns::test_seed::time_entry;
use horae_core::types::EntryState;

fn query(subject_id: Option<Uuid>) -> TimesheetQuery {
    TimesheetQuery {
        subject_id,
        date_from: "2026-09-01".parse().unwrap(),
        date_to: "2026-09-30".parse().unwrap(),
        after: None,
        expected_requester: None,
        expected_policy: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_resolves_legacy_own_context_without_canonical_state(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let result = sheet(&pool, ids.org_id, ids.user_id, &query(None))
        .await
        .unwrap();
    assert_eq!(result.policy, TimesheetPolicy::LegacyOwn);
    assert_eq!(
        result.requester,
        PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id
        }
    );
    assert_eq!(result.subject.id, ids.user_id);
    assert_eq!(result.subject.name, "Test User");
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        [entry]
    );
    assert_eq!(result.entries[0].project_name, "Widget");
    assert_eq!(result.entries[0].task_name, "Dev");
    assert_eq!(result.entries[0].client_name, "Acme");
    assert_eq!(result.next_after, None);
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &query(Some(Uuid::now_v7()))).await,
        Err(TimeReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_selects_empty_managed_participant_without_widening_record_scope(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TimeReadManaged])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2, is_administrator=false WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,user_id,project_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        target,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let selected = query(Some(target));
    let empty = sheet(&pool, ids.org_id, ids.user_id, &selected)
        .await
        .unwrap();
    assert_eq!(empty.policy, TimesheetPolicy::Scoped);
    assert_eq!(empty.requester.user_id, ids.user_id);
    assert_eq!(empty.subject.id, target);
    assert!(empty.entries.is_empty());

    let visible = time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    let second_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Unmanaged','EUR')", second_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            project_id: second_project,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let page = sheet(&pool, ids.org_id, ids.user_id, &selected)
        .await
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        [visible]
    );
    assert!(page.entries.iter().all(|entry| entry.user_id == target));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_rejects_changed_requester_or_policy_and_inaccessible_subject(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let initial = sheet(&pool, ids.org_id, ids.user_id, &query(Some(target)))
        .await
        .unwrap();
    let mut request = query(Some(target));
    request.expected_requester = Some(initial.requester);
    request.expected_policy = Some(initial.policy);
    assert!(
        sheet(&pool, ids.org_id, ids.user_id, &request)
            .await
            .is_ok()
    );
    assert!(matches!(
        sheet(&pool, ids.org_id, target, &request).await,
        Err(TimeReadError::Forbidden)
    ));
    assert!(matches!(
        sheet(&pool, foreign.org_id, foreign.user_id, &request).await,
        Err(TimeReadError::Forbidden)
    ));
    for subject in [Uuid::now_v7(), foreign.user_id] {
        assert!(matches!(
            sheet(&pool, ids.org_id, ids.user_id, &query(Some(subject))).await,
            Err(TimeReadError::Forbidden)
        ));
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &request).await,
        Err(TimeReadError::Forbidden)
    ));
    request.subject_id = None;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 0 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &request).await,
        Err(TimeReadError::Forbidden)
    ));
    assert_eq!(
        sheet(&pool, ids.org_id, ids.user_id, &query(None))
            .await
            .unwrap()
            .policy,
        TimesheetPolicy::LegacyOwn
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_exhausts_pages_and_does_not_reuse_scope_or_silently_change_policy(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    for _ in 0..501 {
        time_entry(
            &pool,
            &SeedIds {
                user_id: target,
                ..ids
            },
            EntryState::Open,
        )
        .await;
    }
    let mut request = query(Some(target));
    let first = sheet(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(first.entries.len(), 500);
    request.after = first.next_after;
    request.expected_requester = Some(first.requester);
    request.expected_policy = Some(first.policy);
    assert!(request.after.is_some());
    let last = sheet(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(last.entries.len(), 1);
    assert_eq!(last.next_after, None);
    assert!(
        !first
            .entries
            .iter()
            .any(|entry| entry.id == last.entries[0].id)
    );

    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TimeReadOwn])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2, is_administrator=false WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &request).await,
        Err(TimeReadError::Forbidden)
    ));
    let mut invalid = query(None);
    invalid.date_to = "2026-08-31".parse().unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &invalid).await,
        Err(TimeReadError::InvalidQuery)
    ));
    invalid = query(None);
    invalid.after = request.after.clone();
    invalid.date_from = "2026-09-08".parse().unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &invalid).await,
        Err(TimeReadError::InvalidQuery)
    ));
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1", ids.user_id).execute(&pool).await.unwrap();
    assert!(matches!(
        sheet(&pool, ids.org_id, ids.user_id, &query(None)).await,
        Err(TimeReadError::Storage(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_rechecks_selected_activity_after_waiting_for_archive(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut archive = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(&mut *archive)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *archive)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let reader =
        tokio::spawn(
            async move { sheet(&db, ids.org_id, ids.user_id, &query(Some(target))).await },
        );
    wait_for_blocked(&pool, pid).await;
    archive.commit().await.unwrap();
    assert!(matches!(
        reader.await.unwrap(),
        Err(TimeReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn sheet_holds_selected_activity_through_entry_delivery_and_releases_on_cancel(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let visible = time_entry(
        &pool,
        &SeedIds {
            user_id: target,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    for cancel in [false, true] {
        let mut block_entries = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *block_entries)
            .await
            .unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *block_entries)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let reader =
            tokio::spawn(
                async move { sheet(&db, ids.org_id, ids.user_id, &query(Some(target))).await },
            );
        wait_for_blocked(&pool, blocker).await;
        let pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker).fetch_one(&pool).await.unwrap().unwrap();
        let db = pool.clone();
        let archive = tokio::spawn(async move {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
                .execute(&db)
                .await
                .unwrap();
        });
        wait_for_blocked(&pool, pid).await;
        if cancel {
            reader.abort();
            assert!(reader.await.unwrap_err().is_cancelled());
            // SQLx queues rollback behind the blocked query on task cancellation.
            // Release our table barrier so cleanup can run without racing the
            // statement timeout against the archive completion deadline.
            block_entries.rollback().await.unwrap();
        } else {
            block_entries.rollback().await.unwrap();
            let page = reader.await.unwrap().unwrap();
            assert_eq!(
                page.entries
                    .iter()
                    .map(|entry| entry.id)
                    .collect::<Vec<_>>(),
                [visible]
            );
        }
        tokio::time::timeout(Duration::from_secs(5), archive)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            sheet(&pool, ids.org_id, ids.user_id, &query(Some(target))).await,
            Err(TimeReadError::Forbidden)
        ));
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", target)
            .execute(&pool)
            .await
            .unwrap();
    }
}
