use super::*;
use crate::models::permission_audit::AuditCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns::permissions::audit::page;

async fn receipts(pool: &PgPool, ids: &SeedIds, count: i64) -> Vec<Uuid> {
    let mut records = Vec::new();
    for revision in 0..count {
        let command = TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: revision,
            action: TemplateAction::Create {
                name: format!("Historical template {revision}"),
                grants: BuiltInProfile::Member.selection().iter().collect(),
            },
        };
        super::super::templates::execute(pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap();
        records.push(receipt_id(pool, command.request_id).await);
    }
    sqlx::query!(
        "UPDATE permission_change_receipts SET created_at='2026-09-07 12:00:00+00' WHERE org_id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    records.sort_unstable_by(|a, b| b.cmp(a));
    records
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn history_pages_scope_before_limit_and_use_stable_exclusive_timestamp_id_bounds(
    pool: PgPool,
) {
    let ids = fixture(&pool, true).await;
    let expected = receipts(&pool, &ids, 27).await;
    let foreign = fixture(&pool, true).await;
    let foreign_ids = receipts(&pool, &foreign, 3).await;
    let first = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert_eq!(
        first.requester,
        PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id
        }
    );
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[..25]
    );
    let cursor = first.next_after.unwrap();
    assert_eq!(cursor.id, expected[24]);
    let second = page(
        &pool,
        ids.org_id,
        ids.user_id,
        Some(&cursor),
        Some(first.requester),
    )
    .await
    .unwrap();
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[25..]
    );
    assert!(second.next_after.is_none());
    for entry in first.entries.iter().chain(&second.entries) {
        assert_eq!(
            Some(entry.clone()),
            read(&pool, ids.org_id, ids.user_id, entry.id)
                .await
                .unwrap()
        );
        let wire = serde_json::to_value(entry).unwrap();
        for private in ["request_id", "intent", "result"] {
            assert!(wire.get(private).is_none());
        }
    }
    // Neither a foreign receipt nor a nonexistent ID must be resolved as a record.
    for id in [foreign_ids[0], Uuid::from_u128(u128::MAX)] {
        let bound = AuditCursor {
            created_at: cursor.created_at,
            id,
        };
        let bounded = page(&pool, ids.org_id, ids.user_id, Some(&bound), None)
            .await
            .unwrap();
        let expected: Vec<_> = expected
            .iter()
            .copied()
            .filter(|entry| *entry < id)
            .take(25)
            .collect();
        assert_eq!(
            bounded
                .entries
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            expected
        );
    }
    let oldest = second.entries.last().unwrap();
    let end = AuditCursor {
        created_at: oldest.created_at,
        id: oldest.id,
    };
    let empty = page(&pool, ids.org_id, ids.user_id, Some(&end), None)
        .await
        .unwrap();
    assert!(empty.entries.is_empty() && empty.next_after.is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn history_page_rejects_malformed_history_without_partial_success(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let records = receipts(&pool, &ids, 3).await;
    sqlx::query!(
        "UPDATE permission_change_receipts SET audit='{}'::jsonb WHERE id=$1",
        records[1]
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        page(&pool, ids.org_id, ids.user_id, None, None).await,
        Err(AuditReadError::InvalidDocument)
    ));
    assert!(
        read(&pool, ids.org_id, ids.user_id, records[0])
            .await
            .unwrap()
            .is_some()
    );
    assert_unlocked(&pool, &ids).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn history_page_reauthorizes_empty_and_continuation_requests_and_binds_requester(
    pool: PgPool,
) {
    let ids = fixture(&pool, true).await;
    let empty = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert!(empty.entries.is_empty() && empty.next_after.is_none());
    let nonadmin = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    for expected in [
        PermissionRequester {
            org_id: Uuid::now_v7(),
            user_id: ids.user_id,
        },
        PermissionRequester {
            org_id: ids.org_id,
            user_id: nonadmin,
        },
    ] {
        assert!(matches!(
            page(&pool, ids.org_id, ids.user_id, None, Some(expected)).await,
            Err(AuditReadError::Forbidden)
        ));
    }
    assert!(matches!(
        page(&pool, ids.org_id, nonadmin, None, None).await,
        Err(AuditReadError::Forbidden)
    ));
    let records = receipts(&pool, &ids, 26).await;
    let first = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert_eq!(first.entries[0].id, records[0]);
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        page(
            &pool,
            ids.org_id,
            ids.user_id,
            first.next_after.as_ref(),
            Some(first.requester)
        )
        .await,
        Err(AuditReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn history_page_waits_for_revocation_commit_or_rollback_at_each_gate(pool: PgPool) {
    for direct_activity in [false, true] {
        for commit in [false, true] {
            let ids = fixture(&pool, true).await;
            receipts(&pool, &ids, 1).await;
            let mut change = pool.begin().await.unwrap();
            if direct_activity {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *change)
                    .await
                    .unwrap();
            } else {
                crate::db::lock_organization(
                    &mut change,
                    ids.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&mut *change)
                .await
                .unwrap();
            }
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *change)
                .await
                .unwrap();
            let db = pool.clone();
            let mut reader = tokio::task::JoinSet::new();
            reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
            wait_for_blocked(&pool, blocker).await;
            if commit {
                change.commit().await.unwrap();
            } else {
                change.rollback().await.unwrap();
            }
            let result = reader.join_next().await.unwrap().unwrap();
            if commit {
                assert!(matches!(result, Err(AuditReadError::Forbidden)));
            } else {
                assert_eq!(result.unwrap().entries.len(), 1);
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn admitted_history_page_finishes_before_revocation_at_each_gate(pool: PgPool) {
    for direct_activity in [false, true] {
        let ids = fixture(&pool, true).await;
        let records = receipts(&pool, &ids, 1).await;
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *hold)
            .await
            .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *hold)
            .await
            .unwrap();
        let db = pool.clone();
        let mut reader = tokio::task::JoinSet::new();
        reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
        wait_for_blocked(&pool, holder).await;
        let reader_pid=sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",holder).fetch_one(&pool).await.unwrap().unwrap();
        let db = pool.clone();
        let mut writer = tokio::task::JoinSet::new();
        writer.spawn(async move {
            let mut tx = db.begin().await.unwrap();
            if direct_activity {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            } else {
                crate::db::lock_organization(
                    &mut tx,
                    ids.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&mut *tx)
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
        });
        wait_for_blocked(&pool, reader_pid).await;
        hold.commit().await.unwrap();
        assert_eq!(
            reader.join_next().await.unwrap().unwrap().unwrap().entries[0].id,
            records[0]
        );
        writer.join_next().await.unwrap().unwrap();
        assert!(matches!(
            page(&pool, ids.org_id, ids.user_id, None, None).await,
            Err(AuditReadError::Forbidden)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancelling_history_page_releases_both_authority_gates(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    receipts(&pool, &ids, 1).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *hold)
        .await
        .unwrap();
    let db = pool.clone();
    let mut reader = tokio::task::JoinSet::new();
    reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
    wait_for_blocked(&pool, holder).await;
    reader.abort_all();
    assert!(
        reader
            .join_next()
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    hold.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut tx = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut tx,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    })
    .await
    .unwrap();
    assert_unlocked(&pool, &ids).await;
}
