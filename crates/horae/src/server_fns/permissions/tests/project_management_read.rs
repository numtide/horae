use super::*;
use crate::server_fns::permissions::project_management::read;
use serde_json::json;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_preserves_the_complete_retained_set_without_private_fields(pool: PgPool) {
    let ids = fixture(&pool).await;
    let first = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let second = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[second, first]),
    )
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false, email='private@example.test', oidc_subject='private-subject', cost_rate_cents=12345, billable_rate_cents=54321 WHERE id=$1", first).execute(&pool).await.unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=ANY($1)",
        &[first, second]
    )
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
    let before = revision(&pool, ids.org_id).await;
    let actual = read(&pool, ids.org_id, ids.user_id, ids.project_id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        json!({
            "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
            "project_id":ids.project_id,"access_revision":before,
            "managers":[{"id":first,"name":"Manager","active":false},{"id":second,"name":"Manager","active":true}]
        })
    );
    assert_eq!(revision(&pool, ids.org_id).await, before);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_requires_current_project_edit_scope_not_identity_or_membership(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    for grants in [
        vec![],
        vec![Permission::ProjectReadAll],
        vec![Permission::PeopleWriteAll],
        vec![Permission::ProjectWriteManaged],
    ] {
        permissions(&pool, ids.org_id, ids.user_id, &grants).await;
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, ids.project_id).await,
            Err(ProjectManagersError::Forbidden)
        ));
    }
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=true WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, ids.project_id).await,
        Err(ProjectManagersError::Forbidden)
    ));
    permissions(
        &pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectWriteAll],
    )
    .await;
    execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[ids.user_id]),
    )
    .await
    .unwrap();
    permissions(
        &pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectWriteManaged],
    )
    .await;
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .managers[0]
            .id,
        ids.user_id
    );
    for project in [foreign.project_id, Uuid::now_v7()] {
        assert!(matches!(
            read(&pool, ids.org_id, ids.user_id, project).await,
            Err(ProjectManagersError::Forbidden)
        ));
    }
    let mut remove = request(ids.project_id, &[]);
    remove.expected_access_revision = revision(&pool, ids.org_id).await;
    assert!(
        execute(&pool, ids.org_id, ids.user_id, &remove)
            .await
            .unwrap()
            .changed
    );
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, ids.project_id).await,
        Err(ProjectManagersError::Forbidden)
    ));
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &remove).await,
        Err(ProjectManagersError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_rechecks_revocation_after_the_organization_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        ids.org_id
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { read(&pending_pool, ids.org_id, ids.user_id, ids.project_id).await },
        );
    wait_for_blocked(&pool, holder).await;
    let floor: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &floor
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    hold.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProjectManagersError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_holds_authority_and_cancellation_releases_it(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE project_management_assignments IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { read(&pending_pool, ids.org_id, ids.user_id, ids.project_id).await },
        );
    wait_for_blocked(&pool, holder).await;
    let mut contender = pool.begin().await.unwrap();
    let error = sqlx::query_scalar!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *contender)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    contender.rollback().await.unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    hold.rollback().await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(3),
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id
        )
        .execute(&pool),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        read(&pool, ids.org_id, ids.user_id, ids.project_id)
            .await
            .unwrap()
            .managers
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_observes_direct_deactivation_before_its_actor_lock(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { read(&pending_pool, ids.org_id, ids.user_id, ids.project_id).await },
        );
    wait_for_blocked(&pool, holder).await;
    hold.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProjectManagersError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn manager_reader_materializes_before_a_waiting_actor_deactivation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE project_management_assignments IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { read(&pending_pool, ids.org_id, ids.user_id, ids.project_id).await },
        );
    wait_for_blocked(&pool, holder).await;
    let reader_pid=sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",holder).fetch_one(&pool).await.unwrap().unwrap();
    let revoke_pool = pool.clone();
    let revoke = tokio::spawn(async move {
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&revoke_pool)
            .await
            .unwrap();
    });
    wait_for_blocked(&pool, reader_pid).await;
    hold.rollback().await.unwrap();
    assert!(pending.await.unwrap().unwrap().managers.is_empty());
    revoke.await.unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, ids.project_id).await,
        Err(ProjectManagersError::Forbidden)
    ));
}
