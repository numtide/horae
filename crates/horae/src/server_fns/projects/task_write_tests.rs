use super::canonical_read_tests::fixture;
use super::*;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;
use uuid::Uuid;

async fn task_count(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn linked_task_creation_requires_both_current_grants_and_managed_scope(pool: PgPool) {
    for (project_grant, designated, allowed) in [
        (Permission::ProjectWriteAll, false, true),
        (Permission::ProjectWriteManaged, false, false),
        (Permission::ProjectWriteManaged, true, true),
    ] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[Permission::TaskWriteAll, project_grant]),
        )
        .await;
        if designated {
            sqlx::query!(
                "INSERT INTO project_management_assignments (id,org_id,project_id,manager_id)
                VALUES ($1,$2,$3,$4)",
                Uuid::now_v7(),
                ids.org_id,
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let result = create_task_for_project(
            &pool,
            ids.org_id,
            ids.user_id,
            "Scoped task",
            true,
            Some(ids.project_id),
        )
        .await;
        if allowed {
            let task = result.unwrap();
            let link = sqlx::query!(
                "SELECT billable,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
                ids.project_id,
                task.id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert!(link.billable);
            assert_eq!(link.rate_cents, None);
            assert_eq!(task.default_rate_cents, None);
        } else {
            assert!(matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ));
        }
        assert_eq!(
            task_count(&pool, ids.org_id).await,
            if allowed { 2 } else { 1 }
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_edit_authority_does_not_create_global_tasks(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let result = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "Unauthorized catalog task",
        true,
        Some(ids.project_id),
    )
    .await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(task_count(&pool, ids.org_id).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn new_task_link_obeys_nonbillable_project_without_changing_catalog_default(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE projects SET project_type='non_billable' WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let task = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "Internal work",
        true,
        Some(ids.project_id),
    )
    .await
    .unwrap();
    assert!(task.billable_default);
    let link = sqlx::query!(
        "SELECT billable,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
        ids.project_id,
        task.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!link.billable);
    assert_eq!(link.rate_cents, None);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unavailable_project_rolls_back_new_catalog_task(pool: PgPool) {
    for unavailable in [
        "archived_project",
        "archived_client",
        "foreign_project",
        "foreign_client",
        "missing",
    ] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
        )
        .await;
        let foreign = seed(&pool, OrgRole::Admin).await;
        let mut project_id = ids.project_id;
        match unavailable {
            "archived_project" => {
                sqlx::query!(
                    "UPDATE projects SET active=false WHERE id=$1",
                    ids.project_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "archived_client" => {
                sqlx::query!("UPDATE clients SET active=false WHERE id=$1", ids.client_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "foreign_project" => project_id = foreign.project_id,
            "foreign_client" => {
                sqlx::query!(
                    "UPDATE projects SET client_id=$2 WHERE id=$1",
                    ids.project_id,
                    foreign.client_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "missing" => project_id = Uuid::now_v7(),
            _ => unreachable!(),
        }
        let result = create_task_for_project(
            &pool,
            ids.org_id,
            ids.user_id,
            "Must roll back",
            true,
            Some(project_id),
        )
        .await;
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: NOT_FOUND,
                    ..
                })
            ),
            "{unavailable}: {result:?}"
        );
        assert_eq!(task_count(&pool, ids.org_id).await, 1, "{unavailable}");
        assert_eq!(task_count(&pool, foreign.org_id).await, 1, "{unavailable}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_creation_fails_closed_for_unavailable_actor_permissions(pool: PgPool) {
    for invalid in ["inactive", "missing", "unknown_grant", "unknown_catalog"] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
        )
        .await;
        match invalid {
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "unknown_grant" => {
                sqlx::query!("UPDATE person_permission_states SET grants=array_append(grants,'unknown.grant') WHERE user_id=$1", ids.user_id)
                .execute(&pool).await.unwrap();
            }
            "unknown_catalog" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        for project_id in [None, Some(ids.project_id)] {
            let result = create_task_for_project(
                &pool,
                ids.org_id,
                ids.user_id,
                "Unavailable actor",
                true,
                project_id,
            )
            .await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{invalid}: {result:?}"
            );
            assert_eq!(task_count(&pool, ids.org_id).await, 1, "{invalid}");
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_creation_rechecks_revoked_grants_after_organization_wait(pool: PgPool) {
    for linked in [false, true] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
        )
        .await;
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
        let pending = tokio::spawn(async move {
            create_task_for_project(
                &pending_pool,
                ids.org_id,
                ids.user_id,
                "Revoked task",
                true,
                linked.then_some(ids.project_id),
            )
            .await
        });
        wait_for_blocked(&pool, holder).await;
        let grants: Vec<String> = serde_json::from_value(
            serde_json::to_value(PermissionSelection::new(&[Permission::ProjectWriteAll])).unwrap(),
        )
        .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &grants
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        hold.commit().await.unwrap();
        let result = pending.await.unwrap();
        assert!(matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
        assert_eq!(task_count(&pool, ids.org_id).await, 1);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_creation_rechecks_revoked_project_designation_after_wait(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteManaged]),
    )
    .await;
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,project_id,manager_id)
        VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
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
    let pending = tokio::spawn(async move {
        create_task_for_project(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            "Revoked manager task",
            true,
            Some(ids.project_id),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    sqlx::query!("DELETE FROM project_management_assignments WHERE org_id=$1 AND project_id=$2 AND manager_id=$3",
        ids.org_id, ids.project_id, ids.user_id).execute(&mut *hold).await.unwrap();
    hold.commit().await.unwrap();
    let result = pending.await.unwrap();
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(task_count(&pool, ids.org_id).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn global_task_creation_accepts_current_grant_without_legacy_manager(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let task = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "  Canonical task  ",
        true,
        None,
    )
    .await
    .unwrap();
    assert_eq!(task.name, "Canonical task");
    assert_eq!(task.org_id, ids.org_id);
    assert!(task.default_rate_cents.is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn global_task_creation_does_not_inherit_legacy_administrator_authority(pool: PgPool) {
    let (ids, _) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    let result =
        create_task_for_project(&pool, ids.org_id, ids.user_id, "Forbidden task", true, None).await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1),
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn global_task_creation_rejects_unknown_policy_without_inserting(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 99 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "Unknown-policy task",
        true,
        None,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1),
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_creation_cannot_link_with_global_task_authority_alone(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let result = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "Unscoped project task",
        true,
        Some(ids.project_id),
    )
    .await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1),
    );
}
