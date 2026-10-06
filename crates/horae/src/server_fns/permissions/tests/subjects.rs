use super::*;
use crate::server_fns::permissions::editor;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_pages_preserve_all_local_identities_with_only_display_fields(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let mut expected = vec![ids.user_id];
    for _ in 0..52 {
        expected.push(person(&pool, ids.org_id, false, BuiltInProfile::Member).await);
    }
    let inactive = expected[17];
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", inactive)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        inactive
    )
    .execute(&pool)
    .await
    .unwrap();
    expected.sort_unstable();
    let first = editor::subjects(&pool, ids.org_id, ids.user_id, None)
        .await
        .unwrap();
    assert_eq!(first.subjects.len(), 50);
    assert_eq!(first.next_after, Some(expected[49]));
    let last = editor::subjects(&pool, ids.org_id, ids.user_id, first.next_after)
        .await
        .unwrap();
    assert_eq!(last.subjects.len(), 3);
    assert_eq!(last.next_after, None);
    let actual: Vec<_> = first
        .subjects
        .iter()
        .chain(&last.subjects)
        .map(|row| row.id)
        .collect();
    assert_eq!(actual, expected);
    assert!(!actual.contains(&foreign.user_id));
    let row = first
        .subjects
        .iter()
        .chain(&last.subjects)
        .find(|row| row.id == inactive)
        .unwrap();
    assert_eq!(
        serde_json::to_value(row).unwrap(),
        serde_json::json!({
            "id":inactive,"name":"Person","active":false
        })
    );
    assert_eq!(first.requester.org_id, ids.org_id);
    assert_eq!(first.requester.user_id, ids.user_id);
    let mut keys: Vec<_> = serde_json::to_value(&first)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(keys, ["next_after", "requester", "subjects"]);
    assert!(
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, inactive)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_cursor_is_only_a_bound_and_exact_page_has_no_continuation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let mut expected = Vec::new();
    for _ in 0..50 {
        expected.push(person(&pool, ids.org_id, false, BuiltInProfile::Member).await);
    }
    let page = editor::subjects(&pool, ids.org_id, ids.user_id, Some(ids.user_id))
        .await
        .unwrap();
    assert_eq!(page.subjects.len(), 50);
    assert_eq!(page.next_after, None);
    let foreign_cursor = editor::subjects(&pool, ids.org_id, ids.user_id, Some(foreign.user_id))
        .await
        .unwrap();
    assert_eq!(foreign_cursor.subjects, page.subjects);
    let empty = editor::subjects(&pool, ids.org_id, ids.user_id, Some(Uuid::max()))
        .await
        .unwrap();
    assert!(empty.subjects.is_empty());
    assert_eq!(empty.next_after, None);
    let removed = expected[24];
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        removed
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("DELETE FROM users WHERE id=$1", removed)
        .execute(&pool)
        .await
        .unwrap();
    let remaining = editor::subjects(&pool, ids.org_id, ids.user_id, Some(removed))
        .await
        .unwrap();
    assert_eq!(
        remaining
            .subjects
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        expected[25..]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_discovery_requires_current_explicit_identity_and_supported_policy(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let all_grants = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", all_grants)
        .execute(&pool)
        .await
        .unwrap();
    for actor in [all_grants, foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            editor::subjects(&pool, ids.org_id, actor, None).await,
            Err(ProfileCommandError::Forbidden)
        ));
        assert!(matches!(
            editor::subjects(&pool, ids.org_id, actor, Some(Uuid::max())).await,
            Err(ProfileCommandError::Forbidden)
        ));
    }
    for policy in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=$2 WHERE id=$1",
            ids.org_id,
            policy
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            editor::subjects(&pool, ids.org_id, ids.user_id, None).await,
            Err(ProfileCommandError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private-invalid-grant'] WHERE user_id=$1", ids.user_id)
        .execute(&pool).await.unwrap();
    assert!(matches!(
        editor::subjects(&pool, ids.org_id, ids.user_id, None).await,
        Err(ProfileCommandError::Storage(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_discovery_rechecks_revocation_after_gate_wait_and_between_pages(pool: PgPool) {
    let ids = fixture(&pool).await;
    let page = editor::subjects(&pool, ids.org_id, ids.user_id, None)
        .await
        .unwrap();
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *revoke)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(&mut *revoke)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let connection = pool.clone();
    let pending = tokio::spawn(async move {
        editor::subjects(
            &connection,
            ids.org_id,
            ids.user_id,
            Some(page.subjects[0].id),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProfileCommandError::Forbidden)
    ));
    assert!(matches!(
        editor::subjects(&pool, ids.org_id, ids.user_id, None).await,
        Err(ProfileCommandError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_discovery_rechecks_direct_deactivation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&mut *revoke)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let connection = pool.clone();
    let pending =
        tokio::spawn(
            async move { editor::subjects(&connection, ids.org_id, ids.user_id, None).await },
        );
    wait_for_blocked(&pool, holder).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProfileCommandError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn subject_discovery_cancellation_releases_connection_and_preserves_pool_defaults(
    pool: PgPool,
) {
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
    let connection = reader.clone();
    let pending =
        tokio::spawn(
            async move { editor::subjects(&connection, ids.org_id, ids.user_id, None).await },
        );
    wait_for_blocked(&pool, holder).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    writer.rollback().await.unwrap();
    let page = tokio::time::timeout(
        Duration::from_secs(6),
        editor::subjects(&reader, ids.org_id, ids.user_id, None),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(page.subjects.len(), 1);
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&reader).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    reader.close().await;
}
