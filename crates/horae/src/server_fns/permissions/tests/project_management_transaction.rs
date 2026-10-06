use super::*;
use crate::server_fns::permissions::project_management::execute_in_transaction;

async fn composed_edit(pool: &PgPool, commit: bool) {
    let ids = fixture(pool).await;
    let target = person(pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let original = sqlx::query_scalar!("SELECT name FROM projects WHERE id = $1", ids.project_id)
        .fetch_one(pool)
        .await
        .unwrap();
    let command = request(ids.project_id, &[target]);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .unwrap();
    crate::db::lock_organization(
        &mut tx,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    // Match the editor's organization gate and project UPDATE lock.
    sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
        ids.project_id
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let outcome = execute_in_transaction(&mut tx, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.access_revision, 1);
    sqlx::query!(
        "UPDATE projects SET name = $2 WHERE id = $1",
        ids.project_id,
        "Atomic project edit"
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let stored = sqlx::query_scalar!(
        "SELECT manager_id FROM project_management_assignments
        WHERE org_id = $1 AND project_id = $2 ORDER BY manager_id",
        ids.org_id,
        ids.project_id
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(stored, vec![target]);
    // Neither relationship nor receipt may escape the enclosing transaction.
    assert!(managers(pool, ids.org_id, ids.project_id).await.is_empty());
    if commit {
        tx.commit().await.unwrap();
    } else {
        tx.rollback().await.unwrap();
    }
    assert_eq!(revision(pool, ids.org_id).await, i64::from(commit));
    assert_eq!(
        managers(pool, ids.org_id, ids.project_id).await,
        if commit { vec![target] } else { vec![] }
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT name FROM projects WHERE id = $1", ids.project_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        if commit {
            "Atomic project edit".into()
        } else {
            original
        }
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE request_id = $1",
            command.request_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(i64::from(commit))
    );
    // A committed receipt replays without another change; a rollback leaves the
    // original intent available for a fresh, successful attempt.
    assert_eq!(
        execute(pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap(),
        outcome
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_and_manager_changes_commit_with_the_same_receipt(pool: PgPool) {
    composed_edit(&pool, true).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_rollback_discards_managers_revision_and_receipt(pool: PgPool) {
    composed_edit(&pool, false).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn delegation_reuses_editor_gate_without_upgrading_past_foreign_keys(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    crate::db::lock_organization(
        &mut tx,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    sqlx::query!("SET LOCAL lock_timeout = '500ms'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
        ids.project_id
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    // This is the organization lock taken by an unrelated child foreign key.
    // It can coexist with the editor gate, and must stay compatible throughout.
    let mut foreign_key = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR KEY SHARE",
        ids.org_id
    )
    .fetch_one(&mut *foreign_key)
    .await
    .unwrap();
    let result = execute_in_transaction(
        &mut tx,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[ids.user_id]),
    )
    .await;
    foreign_key.rollback().await.unwrap();
    tx.rollback().await.unwrap();
    assert!(
        result.is_ok(),
        "delegation upgraded the editor gate: {result:?}"
    );
}
