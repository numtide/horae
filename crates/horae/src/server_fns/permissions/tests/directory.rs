use super::*;
use crate::models::people::{PeopleActivity, PeopleCursor};
use crate::server_fns::permissions::directory::{DirectoryError, read};

async fn grants(pool: &PgPool, actor: Uuid, selection: PermissionSelection) {
    let values: Vec<String> =
        serde_json::from_value(serde_json::to_value(selection).unwrap()).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        actor,
        &values
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn manage(pool: &PgPool, org: Uuid, actor: Uuid, target: Uuid) {
    sqlx::query!(
        "INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id)
        VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        org,
        actor,
        target
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_uses_each_profiles_people_grants_not_legacy_role_or_other_scope(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    manage(&pool, ids.org_id, ids.user_id, target).await;
    for &profile in BuiltInProfile::ALL {
        sqlx::query!(
            "UPDATE person_permission_states SET is_administrator=$2 WHERE user_id=$1",
            ids.user_id,
            profile == BuiltInProfile::Administrator
        )
        .execute(&pool)
        .await
        .unwrap();
        grants(&pool, ids.user_id, profile.selection()).await;
        let result = read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None).await;
        match profile {
            BuiltInProfile::PeopleAdmin
            | BuiltInProfile::ExecutiveManager
            | BuiltInProfile::Administrator => {
                let page = result.unwrap();
                let actual: std::collections::BTreeSet<_> =
                    page.people.iter().map(|row| row.id).collect();
                assert_eq!(actual, [ids.user_id, target].into_iter().collect());
            }
            _ => assert!(
                matches!(result, Err(DirectoryError::Forbidden)),
                "{profile:?}: {result:?}"
            ),
        }
    }
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[Permission::PeopleReadManaged]),
    )
    .await;
    let page = read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None)
        .await
        .unwrap();
    assert_eq!(
        page.people.iter().map(|row| row.id).collect::<Vec<_>>(),
        [target]
    );
    assert_eq!(page.requester.org_id, ids.org_id);
    assert_eq!(page.requester.user_id, ids.user_id);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_rechecks_grant_revocation_and_direct_deactivation_after_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    for deactivate in [false, true] {
        grants(&pool, ids.user_id, BuiltInProfile::PeopleAdmin.selection()).await;
        let mut revoke = pool.begin().await.unwrap();
        if deactivate {
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
            let selection: Vec<String> = serde_json::from_value(
                serde_json::to_value(BuiltInProfile::Member.selection()).unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &selection
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
        }
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        let db = pool.clone();
        let pending = tokio::spawn(async move {
            read(&db, ids.org_id, ids.user_id, PeopleActivity::All, None).await
        });
        wait_for_blocked(&pool, holder).await;
        revoke.commit().await.unwrap();
        assert!(matches!(
            pending.await.unwrap(),
            Err(DirectoryError::Forbidden)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_cancellation_releases_connection_and_preserves_pool_defaults(pool: PgPool) {
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
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let db = reader.clone();
    let pending =
        tokio::spawn(
            async move { read(&db, ids.org_id, ids.user_id, PeopleActivity::All, None).await },
        );
    wait_for_blocked(&pool, holder).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    writer.rollback().await.unwrap();
    let page = tokio::time::timeout(
        Duration::from_secs(6),
        read(&reader, ids.org_id, ids.user_id, PeopleActivity::All, None),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(page.people.len(), 1);
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly").fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_orders_by_name_then_id_and_accepts_deleted_cursor_without_extra_page(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[Permission::PeopleReadManaged]),
    )
    .await;
    let mut expected = Vec::new();
    for index in (0..50).rev() {
        let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        sqlx::query!(
            "UPDATE users SET name=$2 WHERE id=$1",
            target,
            format!("Person {index:02}")
        )
        .execute(&pool)
        .await
        .unwrap();
        manage(&pool, ids.org_id, ids.user_id, target).await;
        expected.push(target);
    }
    expected.reverse();
    let page = read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None)
        .await
        .unwrap();
    assert_eq!(page.next_after, None);
    assert_eq!(
        page.people.iter().map(|row| row.id).collect::<Vec<_>>(),
        expected
    );
    let cursor = PeopleCursor {
        name: "Person 24".into(),
        id: expected[24],
    };
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE managed_user_id=$1",
        cursor.id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        cursor.id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("DELETE FROM users WHERE id=$1", cursor.id)
        .execute(&pool)
        .await
        .unwrap();
    let page = read(
        &pool,
        ids.org_id,
        ids.user_id,
        PeopleActivity::All,
        Some(&cursor),
    )
    .await
    .unwrap();
    assert_eq!(
        page.people.iter().map(|row| row.id).collect::<Vec<_>>(),
        expected[25..]
    );
    let invalid = PeopleCursor {
        name: "bad\0name".into(),
        id: ids.user_id,
    };
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            PeopleActivity::All,
            Some(&invalid)
        )
        .await,
        Err(DirectoryError::InvalidCursor)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_scopes_before_activity_pagination_and_only_returns_basic_identity(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[Permission::PeopleReadManaged]),
    )
    .await;
    let mut visible = Vec::new();
    for index in 0..53 {
        let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        // Hidden people must not fill a page or influence its continuation.
        person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        manage(&pool, ids.org_id, ids.user_id, target).await;
        visible.push(target);
        if index == 52 {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query!(
                "DELETE FROM person_permission_states WHERE user_id=$1",
                target
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    }
    visible.sort_unstable();
    let first = read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None)
        .await
        .unwrap();
    assert_eq!(first.people.len(), 50);
    let cursor = first.next_after.as_ref().unwrap();
    assert_eq!((cursor.name.as_str(), cursor.id), ("Person", visible[49]));
    let next = read(
        &pool,
        ids.org_id,
        ids.user_id,
        PeopleActivity::All,
        Some(cursor),
    )
    .await
    .unwrap();
    assert_eq!(next.next_after, None);
    assert_eq!(
        first
            .people
            .iter()
            .chain(&next.people)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        visible
    );
    for row in first.people.iter().chain(&next.people) {
        let value = serde_json::to_value(row).unwrap();
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(keys, ["active", "email", "id", "name"]);
        assert_ne!(row.id, foreign.user_id);
    }
    let archived = read(
        &pool,
        ids.org_id,
        ids.user_id,
        PeopleActivity::Archived,
        None,
    )
    .await
    .unwrap();
    assert_eq!(archived.people.len(), 1);
    assert!(!archived.people[0].active);
    let active_tail = read(
        &pool,
        ids.org_id,
        ids.user_id,
        PeopleActivity::Active,
        Some(cursor),
    )
    .await
    .unwrap();
    assert_eq!(active_tail.people.len(), 2);
    assert!(active_tail.people.iter().all(|row| row.active));
    let foreign_bound = PeopleCursor {
        name: "Person".into(),
        id: foreign.user_id,
    };
    let bounded = read(
        &pool,
        ids.org_id,
        ids.user_id,
        PeopleActivity::All,
        Some(&foreign_bound),
    )
    .await
    .unwrap();
    assert_eq!(bounded, first);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_does_not_use_project_management_and_rechecks_relationship_removal(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    grants(
        &pool,
        ids.user_id,
        PermissionSelection::new(&[Permission::PeopleReadManaged]),
    )
    .await;
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
    assert!(
        read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None)
            .await
            .unwrap()
            .people
            .is_empty()
    );
    manage(&pool, ids.org_id, ids.user_id, target).await;
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, PeopleActivity::All, None)
            .await
            .unwrap()
            .people
            .len(),
        1
    );
    let mut removal = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *removal)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&mut *removal)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *removal)
        .await
        .unwrap()
        .unwrap();
    let db = pool.clone();
    let pending =
        tokio::spawn(
            async move { read(&db, ids.org_id, ids.user_id, PeopleActivity::All, None).await },
        );
    wait_for_blocked(&pool, holder).await;
    removal.commit().await.unwrap();
    assert!(pending.await.unwrap().unwrap().people.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn directory_denies_legacy_foreign_inactive_and_invalid_authority_even_for_empty_pages(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let cursor = PeopleCursor {
        name: "zzzz".into(),
        id: Uuid::max(),
    };
    for (org, actor) in [
        (ids.org_id, foreign.user_id),
        (foreign.org_id, ids.user_id),
        (ids.org_id, Uuid::now_v7()),
    ] {
        assert!(matches!(
            read(&pool, org, actor, PeopleActivity::All, Some(&cursor)).await,
            Err(DirectoryError::Forbidden)
        ));
    }
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            read(
                &pool,
                ids.org_id,
                ids.user_id,
                PeopleActivity::All,
                Some(&cursor)
            )
            .await
            .is_err()
        );
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            PeopleActivity::All,
            Some(&cursor)
        )
        .await,
        Err(DirectoryError::Forbidden)
    ));
    sqlx::query!(
        "UPDATE users SET active=true, org_role='admin' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    grants(&pool, ids.user_id, BuiltInProfile::Member.selection()).await;
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            PeopleActivity::All,
            Some(&cursor)
        )
        .await,
        Err(DirectoryError::Forbidden)
    ));
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown_private_grant'] WHERE user_id=$1", ids.user_id).execute(&pool).await.unwrap();
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            PeopleActivity::All,
            Some(&cursor)
        )
        .await,
        Err(DirectoryError::Storage(_))
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            PeopleActivity::All,
            Some(&cursor)
        )
        .await,
        Err(DirectoryError::Unavailable)
    ));
}
