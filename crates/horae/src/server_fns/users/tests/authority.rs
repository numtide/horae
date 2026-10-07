use super::*;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_editor_and_user_revocation_commit_in_gate_order(pool: PgPool) {
    use crate::models::project_creation::ProjectEditRequest;
    use crate::server_fns::project_creation::editing::{
        load_editable_project, save_editable_project,
    };

    for editor_first in [true, false] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let manager = insert_user(
            &pool,
            ids.org_id,
            ids.user_id,
            &format!("{}@test.com", Uuid::now_v7()),
            "Manager",
            OrgRole::Manager,
        )
        .await
        .unwrap();
        let original = load_editable_project(&pool, manager.id, ids.org_id, ids.project_id)
            .await
            .unwrap();
        let mut request = ProjectEditRequest {
            id: Uuid::now_v7(),
            project_id: ids.project_id,
            expected_revision: original.revision,
            form: original.form,
        };
        request.form.name = "Committed before revocation".into();
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *barrier)
            .await
            .unwrap();
        if editor_first {
            sqlx::query!(
                "SELECT id FROM projects WHERE id=$1 FOR UPDATE",
                ids.project_id
            )
            .fetch_one(&mut *barrier)
            .await
            .unwrap();
        } else {
            sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE", manager.id)
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
        }
        let mut operations = tokio::task::JoinSet::new();
        for edit in [editor_first, !editor_first] {
            let db = pool.clone();
            let request = request.clone();
            operations.spawn(async move {
                let result = if edit {
                    save_editable_project(&db, manager.id, ids.org_id, &request, false)
                        .await
                        .map(|_| ())
                } else {
                    change_user_role(&db, ids.org_id, ids.user_id, manager.id, OrgRole::Member)
                        .await
                        .map(|_| ())
                };
                (edit, result)
            });
            if edit == editor_first {
                wait_for_blocked(&pool, blocker).await;
            }
        }
        let first_pid = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            blocker,
        ).fetch_one(&pool).await.unwrap();
        wait_for_blocked(&pool, first_pid).await;
        barrier.commit().await.unwrap();
        while let Some(outcome) =
            tokio::time::timeout(std::time::Duration::from_secs(10), operations.join_next())
                .await
                .unwrap()
        {
            let (edit, result) = outcome.unwrap();
            if edit && !editor_first {
                assert!(
                    matches!(
                        result,
                        Err(ServerFnError::ServerError { code: CONFLICT, .. })
                    ),
                    "{result:?}"
                );
            } else {
                result.unwrap();
            }
        }
        let name = sqlx::query_scalar!("SELECT name FROM projects WHERE id=$1", ids.project_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            name,
            if editor_first {
                "Committed before revocation"
            } else {
                "Widget"
            }
        );
        assert!(matches!(
            save_editable_project(&pool, manager.id, ids.org_id, &request, false).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
}

#[derive(Clone, Copy, Debug)]
enum Change {
    Create,
    Role,
    Active,
}

async fn attempt(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    target: Uuid,
    change: Change,
) -> Result<(), ServerFnError> {
    match change {
        Change::Create => insert_user(
            pool,
            org,
            actor,
            &format!("{}@test.com", Uuid::now_v7()),
            "New member",
            OrgRole::Member,
        )
        .await
        .map(|_| ()),
        Change::Role => change_user_role(pool, org, actor, target, OrgRole::Manager)
            .await
            .map(|_| ()),
        Change::Active => change_user_active(pool, org, actor, target, false)
            .await
            .map(|_| ()),
    }
}

async fn target(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $2, $3, 'Target', 'member')", id, org, format!("{id}@test.com"))
        .execute(pool).await.unwrap();
    id
}

async fn assert_unchanged(pool: &PgPool, org: Uuid, target: Uuid) {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"count!\" FROM users WHERE org_id = $1",
        org
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(count, 2, "unauthorized request created a user");
    let state = user_active_role(&mut pool.acquire().await.unwrap(), target, org)
        .await
        .unwrap();
    assert_eq!(
        state,
        Some((true, OrgRole::Member)),
        "unauthorized request modified the target"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn revoked_administrator_cannot_create_change_roles_or_deactivate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = target(&pool, ids.org_id).await;
    for (role, active) in [
        (OrgRole::Member, true),
        (OrgRole::Manager, true),
        (OrgRole::Admin, false),
    ] {
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            ids.user_id,
            role as OrgRole,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        for change in [Change::Create, Change::Role, Change::Active] {
            let result = attempt(&pool, ids.org_id, ids.user_id, target, change).await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{role:?}/{active}/{change:?}: {result:?}"
            );
            assert_unchanged(&pool, ids.org_id, target).await;
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn foreign_and_unknown_actors_cannot_change_local_user_access(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let target = target(&pool, ids.org_id).await;
    for actor in [foreign.user_id, Uuid::now_v7()] {
        for change in [Change::Create, Change::Role, Change::Active] {
            let result = attempt(&pool, ids.org_id, actor, target, change).await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{change:?}: {result:?}"
            );
            assert_unchanged(&pool, ids.org_id, target).await;
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn user_mutations_wait_for_actor_revocation_and_recheck_after_commit(pool: PgPool) {
    for change in [Change::Create, Change::Role, Change::Active] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let target = target(&pool, ids.org_id).await;
        let mut revocation = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role = 'member' WHERE id = $1",
            ids.user_id
        )
        .execute(&mut *revocation)
        .await
        .unwrap();
        let db = pool.clone();
        let mut request =
            tokio::spawn(
                async move { attempt(&db, ids.org_id, ids.user_id, target, change).await },
            );
        let result = tokio::select! {
            result = &mut request => {
                revocation.commit().await.unwrap();
                result.unwrap()
            },
            () = wait_for_blocked(&pool, blocker) => {
                revocation.commit().await.unwrap();
                tokio::time::timeout(std::time::Duration::from_secs(10), request).await.unwrap().unwrap()
            },
        };
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{change:?}: {result:?}"
        );
        assert_unchanged(&pool, ids.org_id, target).await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn user_mutations_recheck_revocation_after_waiting_for_organization_lock(pool: PgPool) {
    for change in [Change::Create, Change::Role, Change::Active] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let target = target(&pool, ids.org_id).await;
        let mut revocation = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *revocation)
        .await
        .unwrap();
        let db = pool.clone();
        let mut request =
            tokio::spawn(
                async move { attempt(&db, ids.org_id, ids.user_id, target, change).await },
            );
        let early = tokio::select! {
            result = &mut request => Some(result.unwrap()),
            () = wait_for_blocked(&pool, blocker) => None,
        };
        sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
            .execute(&mut *revocation)
            .await
            .unwrap();
        revocation.commit().await.unwrap();
        let result = match early {
            Some(result) => result,
            None => tokio::time::timeout(std::time::Duration::from_secs(10), request)
                .await
                .unwrap()
                .unwrap(),
        };
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{change:?}: {result:?}"
        );
        assert_unchanged(&pool, ids.org_id, target).await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn active_administrator_can_create_change_roles_and_deactivate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let created = insert_user(
        &pool,
        ids.org_id,
        ids.user_id,
        "new@test.com",
        "New",
        OrgRole::Member,
    )
    .await
    .unwrap();
    assert_eq!(created.org_id, ids.org_id);
    assert_eq!(created.org_role, OrgRole::Member);
    let (updated, previous) =
        change_user_role(&pool, ids.org_id, ids.user_id, created.id, OrgRole::Manager)
            .await
            .unwrap();
    assert_eq!(previous, Some(OrgRole::Member));
    assert_eq!(updated.org_role, OrgRole::Manager);
    let (updated, previous) = change_user_active(&pool, ids.org_id, ids.user_id, created.id, false)
        .await
        .unwrap();
    assert_eq!(previous, Some(true));
    assert!(!updated.active);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn duplicate_creation_rolls_back_and_leaves_access_changes_usable(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = target(&pool, ids.org_id).await;
    let result = insert_user(
        &pool,
        ids.org_id,
        ids.user_id,
        &format!("{target}@test.com"),
        "Duplicate",
        OrgRole::Admin,
    )
    .await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    assert_unchanged(&pool, ids.org_id, target).await;
    let (updated, _) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        change_user_role(&pool, ids.org_id, ids.user_id, target, OrgRole::Manager),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.org_role, OrgRole::Manager);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn access_transaction_holds_actor_authority_until_commit(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut access = begin_user_access_change(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *access)
        .await
        .unwrap();
    let db = pool.clone();
    let mut revoke = tokio::spawn(async move {
        sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
            .execute(&db)
            .await
            .unwrap();
    });
    tokio::select! {
        result = &mut revoke => panic!("revocation completed before access transaction committed: {result:?}"),
        () = wait_for_blocked(&pool, blocker) => {},
    }
    access.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), revoke)
        .await
        .unwrap()
        .unwrap();
    let result = begin_user_access_change(&pool, ids.org_id, ids.user_id).await;
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
}
