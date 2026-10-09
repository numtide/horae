use super::*;
use crate::models::scoped_time::TimeEntryQuery;
use crate::server_fns::permissions::time_entries::{TimeReadError, read};
use crate::server_fns::test_seed::time_entry;
use horae_core::types::EntryState;

fn query() -> TimeEntryQuery {
    TimeEntryQuery {
        date_from: "2026-09-01".parse().unwrap(),
        date_to: "2026-09-30".parse().unwrap(),
        user_id: None,
        project_id: None,
        after: None,
    }
}

async fn set_grants(pool: &PgPool, actor: Uuid, selection: PermissionSelection) {
    let values: Vec<String> =
        serde_json::from_value(serde_json::to_value(selection).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2, is_administrator=false WHERE user_id=$1",
        actor,
        &values
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_distinguish_project_management_from_membership_and_people_scope(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let outsider = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let second_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Unmanaged','EUR')", second_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let mut rows = Vec::new();
    for user_id in [ids.user_id, target, outsider] {
        for project_id in [ids.project_id, second_project] {
            let entry_ids = SeedIds {
                user_id,
                project_id,
                ..ids
            };
            rows.push(time_entry(&pool, &entry_ids, EntryState::Open).await);
        }
    }
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,user_id,project_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.user_id,
        second_project
    )
    .execute(&pool)
    .await
    .unwrap();
    set_grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[Permission::TimeReadManaged]),
    )
    .await;
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    let actual: std::collections::BTreeSet<_> = page.entries.iter().map(|row| row.id).collect();
    assert_eq!(actual, rows[..5].iter().copied().collect());
    assert_eq!(page.entries.len(), 5);
    set_grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[
            Permission::ProjectReadAll,
            Permission::PeopleReadAll,
            Permission::CostRateReadAll,
        ]),
    )
    .await;
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|row| row.id)
            .collect::<std::collections::BTreeSet<_>>(),
        rows[..2].iter().copied().collect()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_finish_before_competing_revocation_when_read_admitted_first(pool: PgPool) {
    let ids = fixture(&pool).await;
    let visible = time_entry(&pool, &ids, EntryState::Open).await;
    let mut entry_lock = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *entry_lock)
        .await
        .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *entry_lock)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let pending = tokio::spawn(async move { read(&db, ids.org_id, ids.user_id, &query()).await });
    wait_for_blocked(&pool, blocker).await;
    let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker).fetch_one(&pool).await.unwrap();
    let db = pool.clone();
    let writer = tokio::spawn(async move {
        let mut tx = db.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    });
    wait_for_blocked(&pool, reader_pid.unwrap()).await;
    entry_lock.rollback().await.unwrap();
    let page = pending.await.unwrap().unwrap();
    assert_eq!(
        page.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        [visible]
    );
    writer.await.unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &query()).await,
        Err(TimeReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_cancel_without_leaking_locks_or_changing_pool_defaults(pool: PgPool) {
    let ids = fixture(&pool).await;
    let reader = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let db = reader.clone();
    let pending = tokio::spawn(async move { read(&db, ids.org_id, ids.user_id, &query()).await });
    wait_for_blocked(&pool, pid).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    writer.rollback().await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(6),
        read(&reader, ids.org_id, ids.user_id, &query()),
    )
    .await
    .unwrap()
    .unwrap();
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly").fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_exclude_foreign_parents_but_preserve_archived_history(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let history = SeedIds {
        user_id: target,
        ..ids
    };
    let visible = time_entry(&pool, &history, EntryState::Open).await;
    time_entry(&pool, &foreign, EntryState::Open).await;
    for invalid in [
        SeedIds {
            user_id: foreign.user_id,
            ..ids
        },
        SeedIds {
            project_id: foreign.project_id,
            ..ids
        },
        SeedIds {
            task_id: foreign.task_id,
            ..ids
        },
    ] {
        time_entry(&pool, &invalid, EntryState::Open).await;
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(
        page.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        [visible]
    );
    assert_eq!(page.entries[0].user_name, "Person");
    assert_eq!(page.entries[0].project_name, "Widget");
    assert_eq!(page.entries[0].task_name, "Dev");
    assert_eq!(page.entries[0].client_name, "Acme");
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        foreign.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        read(&pool, ids.org_id, ids.user_id, &query())
            .await
            .unwrap()
            .entries
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_page_ties_without_loss_and_filters_never_expand_scope(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    set_grants(&pool, ids.user_id, BuiltInProfile::Member.selection()).await;
    let mut expected = Vec::new();
    for _ in 0..501 {
        expected.push(time_entry(&pool, &ids, EntryState::Open).await);
    }
    let other_ids = SeedIds {
        user_id: other,
        ..ids
    };
    time_entry(&pool, &other_ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET created_at='2026-09-07 12:00:00+00' WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    expected.sort_unstable_by(|a, b| b.cmp(a));
    let first = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(
        first.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        expected[..500]
    );
    let cursor = first.next_after.unwrap();
    sqlx::query!("DELETE FROM time_entries WHERE id=$1", cursor.id)
        .execute(&pool)
        .await
        .unwrap();
    let mut next = query();
    next.after = Some(cursor);
    let second = read(&pool, ids.org_id, ids.user_id, &next).await.unwrap();
    assert_eq!(
        second.entries.iter().map(|row| row.id).collect::<Vec<_>>(),
        expected[500..]
    );
    assert!(second.next_after.is_none());
    let mut filter = query();
    filter.date_from = "2026-09-07".parse().unwrap();
    filter.date_to = filter.date_from;
    filter.project_id = Some(ids.project_id);
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, &filter)
            .await
            .unwrap()
            .entries
            .len(),
        500
    );
    assert!(
        read(&pool, ids.org_id, ids.user_id, &filter)
            .await
            .unwrap()
            .next_after
            .is_none()
    );
    for user in [other, Uuid::now_v7()] {
        filter.user_id = Some(user);
        assert!(
            read(&pool, ids.org_id, ids.user_id, &filter)
                .await
                .unwrap()
                .entries
                .is_empty()
        );
    }
    filter.user_id = None;
    filter.project_id = Some(Uuid::now_v7());
    assert!(
        read(&pool, ids.org_id, ids.user_id, &filter)
            .await
            .unwrap()
            .entries
            .is_empty()
    );
    filter.project_id = None;
    filter.date_from = "2026-09-08".parse().unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &filter).await,
        Err(TimeReadError::InvalidQuery)
    ));
    next.date_to = "2026-09-06".parse().unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &next).await,
        Err(TimeReadError::InvalidQuery)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_recheck_grants_relationships_and_activity_after_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let target_ids = SeedIds {
        user_id: target,
        ..ids
    };
    time_entry(&pool, &target_ids, EntryState::Open).await;
    for mode in 0..3 {
        set_grants(
            &pool,
            ids.user_id,
            PermissionSelection::new(&[Permission::TimeReadManaged]),
        )
        .await;
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        let mut revoke = pool.begin().await.unwrap();
        if mode == 2 {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&mut *revoke)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            if mode == 0 {
                let values: Vec<String> = serde_json::from_value(
                    serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
                )
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                    ids.user_id,
                    &values
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
            } else {
                sqlx::query!(
                    "DELETE FROM project_management_assignments WHERE manager_id=$1",
                    ids.user_id
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
            }
        }
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let pending =
            tokio::spawn(async move { read(&db, ids.org_id, ids.user_id, &query()).await });
        wait_for_blocked(&pool, pid).await;
        revoke.commit().await.unwrap();
        let result = pending.await.unwrap();
        if mode == 2 {
            assert!(matches!(result, Err(TimeReadError::Forbidden)));
        } else {
            assert!(result.unwrap().entries.is_empty());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_fail_closed_for_policy_identity_and_invalid_state(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    assert!(matches!(
        read(&pool, ids.org_id, foreign.user_id, &query()).await,
        Err(TimeReadError::Forbidden)
    ));
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        let result = read(&pool, ids.org_id, ids.user_id, &query()).await;
        assert!(if version == 0 {
            matches!(result, Err(TimeReadError::Forbidden))
        } else {
            matches!(result, Err(TimeReadError::Unavailable))
        });
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['invalid_grant'] WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &query()).await,
        Err(TimeReadError::Storage(_))
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, &query()).await,
        Err(TimeReadError::Unavailable)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn time_reads_use_effective_profile_scope_and_union_without_duplicates(pool: PgPool) {
    let ids = fixture(&pool).await;
    let own = time_entry(&pool, &ids, EntryState::Open).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let target_ids = SeedIds {
        user_id: target,
        ..ids
    };
    let managed = time_entry(&pool, &target_ids, EntryState::Open).await;
    let unrelated = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let unrelated_ids = SeedIds {
        user_id: unrelated,
        ..ids
    };
    let other = time_entry(&pool, &unrelated_ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id)
         VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        target
    )
    .execute(&pool)
    .await
    .unwrap();
    for &profile in BuiltInProfile::ALL {
        let grants: Vec<String> =
            serde_json::from_value(serde_json::to_value(profile.selection()).unwrap()).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2, is_administrator=$3 WHERE user_id=$1",
            ids.user_id,
            &grants,
            profile == BuiltInProfile::Administrator
        )
        .execute(&pool)
        .await
        .unwrap();
        let page = read(&pool, ids.org_id, ids.user_id, &query())
            .await
            .unwrap();
        let expected = match profile {
            BuiltInProfile::Member => vec![own],
            BuiltInProfile::ProjectManager => vec![own, managed],
            _ => vec![own, managed, other],
        };
        let actual: std::collections::BTreeSet<_> = page.entries.iter().map(|row| row.id).collect();
        assert_eq!(actual, expected.into_iter().collect(), "{profile:?}");
        assert_eq!(page.entries.len(), actual.len());
    }
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
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id)
         VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let page = read(&pool, ids.org_id, ids.user_id, &query())
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 3);
    assert_eq!(page.requester.user_id, ids.user_id);
    assert_eq!(page.requester.org_id, ids.org_id);
}
