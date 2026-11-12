use super::*;
use crate::ops::testing::{Recorder, set_grants};
use crate::server_fns::test_seed::{SeedIds, built_in_profile, seed, wait_for_blocked};
use horae_core::permissions::catalog::Permission;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn project_editor_and_user_revocation_commit_in_gate_order(pool: PgPool) {
    use crate::models::project_creation::ProjectEditRequest;
    use crate::server_fns::project_creation::editing::{
        load_editable_project, save_editable_project,
    };

    for editor_first in [true, false] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        let manager = create(&pool, &ids, BuiltInProfile::ExecutiveManager)
            .await
            .unwrap();
        let original = load_editable_project(&pool, manager.id, ids.org_id, ids.project_id)
            .await
            .unwrap();
        let mut request = ProjectEditRequest {
            task_activity: Vec::new(),
            id: Uuid::now_v7(),
            project_id: ids.project_id,
            expected_revision: original.revision,
            expected_requester: original.access.as_ref().map(|access| access.requester),
            managers: original
                .access
                .as_ref()
                .map(|access| (&access.managers).into()),
            form: original.form,
            unchanged: Vec::new(),
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
                    attempt(&db, ids.org_id, ids.user_id, manager.id, Change::Archive).await
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
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        }) | Err(ServerFnError::ServerError { code: CONFLICT, .. })
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
    Archive,
}

async fn create(
    pool: &PgPool,
    ids: &SeedIds,
    profile: BuiltInProfile,
) -> Result<User, ServerFnError> {
    insert_user(
        pool,
        &Recorder::default(),
        Caller::session(ids.org_id, ids.user_id),
        &format!("{}@test.com", Uuid::now_v7()),
        "Person",
        profile,
    )
    .await
}

async fn attempt(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    target: Uuid,
    change: Change,
) -> Result<(), ServerFnError> {
    let caller = Caller::session(org, actor);
    match change {
        Change::Create => insert_user(
            pool,
            &Recorder::default(),
            caller,
            &format!("{}@test.com", Uuid::now_v7()),
            "New member",
            BuiltInProfile::Member,
        )
        .await
        .map(|_| ()),
        Change::Archive => change_user_active(pool, &Recorder::default(), caller, target, false)
            .await
            .map(|_| ()),
    }
}

/// An active Member of `org`.
async fn target(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Target')",
        id,
        org,
        format!("{id}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    provisioning::provision(&mut connection, org, id, BuiltInProfile::Member)
        .await
        .unwrap();
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
    let active = sqlx::query_scalar!("SELECT active FROM users WHERE id = $1", target)
        .fetch_one(pool)
        .await
        .unwrap();
    assert!(active, "unauthorized request archived the target");
}

fn assert_forbidden(result: Result<(), ServerFnError>, context: impl std::fmt::Debug) {
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{context:?}: {result:?}"
    );
}

/// People Admin and Executive Manager manage people like an Administrator;
/// the other profiles cannot.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn managing_people_follows_the_people_write_grant(pool: PgPool) {
    for &profile in BuiltInProfile::ALL {
        let ids = seed(&pool, profile).await;
        let manages = matches!(
            profile,
            BuiltInProfile::Administrator
                | BuiltInProfile::ExecutiveManager
                | BuiltInProfile::PeopleAdmin
        );
        let target = target(&pool, ids.org_id).await;
        for change in [Change::Create, Change::Archive] {
            let result = attempt(&pool, ids.org_id, ids.user_id, target, change).await;
            if manages {
                result.unwrap();
            } else {
                assert_forbidden(result, (profile, change));
                assert_unchanged(&pool, ids.org_id, target).await;
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn revoked_or_archived_managers_cannot_add_or_archive(pool: PgPool) {
    for revocation in ["grant", "archived"] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        let target = target(&pool, ids.org_id).await;
        match revocation {
            "grant" => set_grants(
                &mut pool.acquire().await.unwrap(),
                &ids,
                &[Permission::PeopleReadAll],
            )
            .await
            .unwrap(),
            _ => {
                sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }
        for change in [Change::Create, Change::Archive] {
            let result = attempt(&pool, ids.org_id, ids.user_id, target, change).await;
            assert_forbidden(result, (revocation, change));
            assert_unchanged(&pool, ids.org_id, target).await;
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn foreign_and_unknown_actors_cannot_change_local_user_access(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let target = target(&pool, ids.org_id).await;
    for actor in [foreign.user_id, Uuid::now_v7()] {
        for change in [Change::Create, Change::Archive] {
            let result = attempt(&pool, ids.org_id, actor, target, change).await;
            assert_forbidden(result, change);
            assert_unchanged(&pool, ids.org_id, target).await;
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn user_mutations_wait_for_actor_revocation_and_recheck_after_commit(pool: PgPool) {
    for change in [Change::Create, Change::Archive] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        let target = target(&pool, ids.org_id).await;
        let mut revocation = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        set_grants(&mut revocation, &ids, &[]).await.unwrap();
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
        assert_forbidden(result, change);
        assert_unchanged(&pool, ids.org_id, target).await;
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn user_mutations_recheck_revocation_after_waiting_for_organization_lock(pool: PgPool) {
    for change in [Change::Create, Change::Archive] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
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
        assert_forbidden(result, change);
        assert_unchanged(&pool, ids.org_id, target).await;
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn active_administrator_can_create_and_archive(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let events = Recorder::default();
    let caller = Caller::session(ids.org_id, ids.user_id);
    let created = insert_user(
        &pool,
        &events,
        caller,
        "new@test.com",
        "New",
        BuiltInProfile::Member,
    )
    .await
    .unwrap();
    assert_eq!(created.org_id, ids.org_id);
    assert!(matches!(
        events.take().as_slice(),
        [crate::plugin::AppEvent::UserCreated { user, .. }] if user.id == created.id
    ));
    let updated = change_user_active(&pool, &events, caller, created.id, false)
        .await
        .unwrap();
    assert!(!updated.active);
    assert!(matches!(
        events.take().as_slice(),
        [crate::plugin::AppEvent::UserDeactivated { user, .. }] if user.id == created.id
    ));
    // Archiving again changes nothing and announces nothing.
    change_user_active(&pool, &events, caller, created.id, false)
        .await
        .unwrap();
    assert!(events.take().is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn duplicate_creation_rolls_back_and_leaves_access_changes_usable(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let target = target(&pool, ids.org_id).await;
    let events = Recorder::default();
    let result = insert_user(
        &pool,
        &events,
        Caller::session(ids.org_id, ids.user_id),
        &format!("{target}@test.com"),
        "Duplicate",
        BuiltInProfile::Member,
    )
    .await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    assert!(events.take().is_empty());
    assert_unchanged(&pool, ids.org_id, target).await;
    let archived = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        attempt(&pool, ids.org_id, ids.user_id, target, Change::Archive),
    )
    .await
    .unwrap();
    archived.unwrap();
}

/// Only an administrator chooses a profile above Member, and nobody becomes
/// an administrator by being added.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn profiles_above_member_need_an_administrator(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let people_admin = seed(&pool, BuiltInProfile::PeopleAdmin).await;
    for &profile in BuiltInProfile::ALL {
        for ids in [&admin, &people_admin] {
            let result = create(&pool, ids, profile).await;
            let code = match &result {
                Ok(user) => {
                    assert_eq!(
                        built_in_profile(&pool, ids.org_id, user.id).await,
                        Some((profile, false)),
                    );
                    continue;
                }
                Err(ServerFnError::ServerError { code, .. }) => *code,
                Err(error) => panic!("{error}"),
            };
            let expected = if profile == BuiltInProfile::Administrator {
                BAD_REQUEST
            } else {
                FORBIDDEN
            };
            assert_eq!(code, expected, "{profile:?}");
            assert!(
                profile == BuiltInProfile::Administrator || ids.user_id == people_admin.user_id,
                "{profile:?}"
            );
        }
    }
}

/// An administrator's account is archived or restored only by another
/// administrator.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn only_an_administrator_archives_or_restores_an_administrator(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let people_admin = create(&pool, &ids, BuiltInProfile::PeopleAdmin)
        .await
        .unwrap()
        .id;
    let second = target(&pool, ids.org_id).await;
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = true WHERE user_id = $1",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    let change = |actor, user, active| {
        let pool = pool.clone();
        async move {
            change_user_active(
                &pool,
                &Recorder::default(),
                Caller::session(ids.org_id, actor),
                user,
                active,
            )
            .await
            .map(|_| ())
        }
    };
    assert_forbidden(change(people_admin, second, false).await, "archive");
    change(ids.user_id, second, false).await.unwrap();
    assert_forbidden(change(people_admin, second, true).await, "restore");
    change(ids.user_id, second, true).await.unwrap();
    // Other people stay within the People Admin's reach.
    let member = target(&pool, ids.org_id).await;
    change(people_admin, member, false).await.unwrap();
}
