use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use sqlx::PgPool;
use uuid::Uuid;

fn admin(ids: &SeedIds) -> User {
    User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Test administrator".into(),
        oidc_subject: None,
        org_role: OrgRole::Admin,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

async fn assignment_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar!("SELECT COUNT(*) AS \"count!: i64\" FROM assignments")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_membership_and_task_callers_wait_before_authorizing(pool: PgPool) {
    for operation in ["assign", "remove", "link", "create_link", "create_task"] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let actor = admin(&ids);
        let assignment = insert_assignment(
            &pool,
            &actor,
            ids.project_id,
            ids.user_id,
            ProjectRole::Lead,
        )
        .await
        .unwrap();
        let mut revocation = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        if operation == "create_task" {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        } else {
            // Mutations of project access must exclude existing SHARE readers.
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR SHARE",
                ids.org_id
            )
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        }
        let db = pool.clone();
        let actor_id = actor.id;
        let mut requests = tokio::task::JoinSet::new();
        requests.spawn(async move {
            match operation {
                "assign" => {
                    insert_assignment(&db, &actor, ids.project_id, ids.user_id, ProjectRole::Lead)
                        .await
                        .map(|_| ())
                }
                "remove" => remove_assignment(&db, &actor, assignment.id)
                    .await
                    .map(|_| ()),
                "link" => {
                    link_project_task_record(
                        &db,
                        ids.org_id,
                        actor.id,
                        ids.project_id,
                        ids.task_id,
                        None,
                    )
                    .await
                }
                "create_link" => create_task_for_project(
                    &db,
                    ids.org_id,
                    actor.id,
                    "Blocked task",
                    true,
                    Some(ids.project_id),
                    &TaskRateEdit::Preserve {},
                )
                .await
                .map(|_| ()),
                "create_task" => create_task_for_project(
                    &db,
                    ids.org_id,
                    actor.id,
                    "Blocked task",
                    true,
                    None,
                    &TaskRateEdit::Preserve {},
                )
                .await
                .map(|_| ()),
                _ => unreachable!(),
            }
        });
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            actor_id
        )
        .fetch_one(&mut *revocation)
        .await
        .expect("gate must precede actor");
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", actor_id)
            .execute(&mut *revocation)
            .await
            .unwrap();
        revocation.commit().await.unwrap();
        let result = requests.join_next().await.unwrap().unwrap();
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{operation}: {result:?}"
        );
        let counts = sqlx::query!(
            "SELECT (SELECT count(*) FROM assignments WHERE project_id=$1) AS assignments,
                    (SELECT count(*) FROM project_tasks WHERE project_id=$1) AS links,
                    (SELECT count(*) FROM tasks WHERE org_id=$2) AS tasks",
            ids.project_id,
            ids.org_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (counts.assignments, counts.links, counts.tasks),
            (Some(1), Some(0), Some(1)),
            "{operation}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn creation_rejects_foreign_and_unknown_project_or_person(pool: PgPool) {
    let local = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for (project_id, user_id) in [
        (local.project_id, foreign.user_id),
        (foreign.project_id, local.user_id),
        (foreign.project_id, foreign.user_id),
        (Uuid::now_v7(), local.user_id),
        (local.project_id, Uuid::now_v7()),
    ] {
        let result = insert_assignment(
            &pool,
            &admin(&local),
            project_id,
            user_id,
            ProjectRole::Lead,
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
            "{result:?}"
        );
        assert_eq!(assignment_count(&pool).await, 0);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn removal_preserves_foreign_and_malformed_assignments(pool: PgPool) {
    let local = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for (project_id, user_id) in [
        (foreign.project_id, foreign.user_id),
        (local.project_id, foreign.user_id),
        (foreign.project_id, local.user_id),
    ] {
        let id = Uuid::now_v7();
        // The legacy schema permits malformed links; neither endpoint may own
        // a link solely because one of its two resources belongs to the actor.
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            id,
            project_id,
            user_id,
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            remove_assignment(&pool, &admin(&local), id)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(assignment_count(&pool).await, 3);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_assignment_lifecycle_preserves_role_and_duplicate_behavior(pool: PgPool) {
    let local = seed(&pool, OrgRole::Admin).await;
    let actor = admin(&local);
    for role in [
        ProjectRole::Freelancer,
        ProjectRole::Lead,
        ProjectRole::Admin,
    ] {
        let assignment = insert_assignment(&pool, &actor, local.project_id, local.user_id, role)
            .await
            .unwrap();
        assert_eq!(
            (assignment.project_id, assignment.user_id, assignment.role),
            (local.project_id, local.user_id, role)
        );
        assert!(
            insert_assignment(&pool, &actor, local.project_id, local.user_id, role)
                .await
                .is_err()
        );
        assert_eq!(assignment_count(&pool).await, 1);
        let removed = remove_assignment(&pool, &actor, assignment.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(removed.id, assignment.id);
        assert!(
            remove_assignment(&pool, &actor, assignment.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            remove_assignment(&pool, &actor, Uuid::now_v7())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(assignment_count(&pool).await, 0);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn stale_administrator_cannot_create_or_remove_after_revocation(pool: PgPool) {
    let local = seed(&pool, OrgRole::Admin).await;
    let actor = admin(&local);
    let assignment = insert_assignment(
        &pool,
        &actor,
        local.project_id,
        local.user_id,
        ProjectRole::Lead,
    )
    .await
    .unwrap();
    for (role, active) in [
        (OrgRole::Manager, true),
        (OrgRole::Member, true),
        (OrgRole::Admin, false),
    ] {
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            actor.id,
            role as OrgRole,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        let created = insert_assignment(
            &pool,
            &actor,
            local.project_id,
            local.user_id,
            ProjectRole::Lead,
        )
        .await;
        assert!(
            matches!(
                created,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{created:?}"
        );
        let removed = remove_assignment(&pool, &actor, assignment.id).await;
        assert!(
            matches!(
                removed,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{removed:?}"
        );
        assert_eq!(assignment_count(&pool).await, 1);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn assignment_mutations_wait_for_revocation_and_recheck_authority(pool: PgPool) {
    for remove in [false, true] {
        let local = seed(&pool, OrgRole::Admin).await;
        let actor = admin(&local);
        let assignment = insert_assignment(
            &pool,
            &actor,
            local.project_id,
            local.user_id,
            ProjectRole::Lead,
        )
        .await
        .unwrap();
        let mut revocation = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role = 'member' WHERE id = $1",
            actor.id
        )
        .execute(&mut *revocation)
        .await
        .unwrap();
        let request_pool = pool.clone();
        let request = tokio::spawn(async move {
            if remove {
                remove_assignment(&request_pool, &actor, assignment.id)
                    .await
                    .map(|_| ())
            } else {
                insert_assignment(
                    &request_pool,
                    &actor,
                    local.project_id,
                    local.user_id,
                    ProjectRole::Lead,
                )
                .await
                .map(|_| ())
            }
        });
        wait_for_blocked(&pool, blocker).await;
        revocation.commit().await.unwrap();
        let result = request.await.unwrap();
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
    assert_eq!(assignment_count(&pool).await, 2);
}
