use super::project_management::{ProjectManagersCommand, ProjectManagersError, execute};
use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use std::time::Duration;

#[path = "project_management_read.rs"]
mod reader_tests;

#[path = "project_management_transaction.rs"]
mod transaction_tests;

async fn permissions(pool: &PgPool, org: Uuid, user: Uuid, grants: &[Permission]) {
    let ids: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(grants)).unwrap())
            .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, false, 'individual')
         ON CONFLICT (org_id, user_id) DO UPDATE SET grants = EXCLUDED.grants",
        Uuid::now_v7(), org, user, &ids
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Member).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    permissions(
        pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectWriteAll],
    )
    .await;
    ids
}

async fn person(pool: &PgPool, org: Uuid, grants: &[Permission]) -> Uuid {
    let user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Manager')",
        user,
        org,
        format!("{user}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    permissions(pool, org, user, grants).await;
    user
}

fn request(project_id: Uuid, managers: &[Uuid]) -> ProjectManagersCommand {
    ProjectManagersCommand {
        kind: crate::models::project_managers::ProjectManagersCommandKind::ReplaceProjectManagers,
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        project_id,
        manager_ids: managers.to_vec(),
    }
}

async fn managers(pool: &PgPool, org: Uuid, project: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar!(
        "SELECT manager_id FROM project_management_assignments
        WHERE org_id = $1 AND project_id = $2 ORDER BY manager_id",
        org,
        project
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn revision(pool: &PgPool, org: Uuid) -> i64 {
    sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1",
        org
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[derive(Clone, Copy)]
enum ActivitySubject {
    Actor,
    AddedManager,
}

async fn retains_activity_until_commit(pool: &PgPool, subject: ActivitySubject) {
    let ids = fixture(pool).await;
    let target = person(pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let user = match subject {
        ActivitySubject::Actor => ids.user_id,
        ActivitySubject::AddedManager => target,
    };
    let mut hold = pool.begin().await.unwrap();
    // Pause after all decisions and relationship writes, before receipt commit.
    sqlx::query!("LOCK TABLE permission_change_receipts IN SHARE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let command = request(ids.project_id, &[target]);
    let pending =
        tokio::spawn(
            async move { execute(&pending_pool, ids.org_id, ids.user_id, &command).await },
        );
    wait_for_blocked(pool, holder).await;
    let command_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder)
        .fetch_one(pool).await.unwrap().unwrap();
    let deactivate_pool = pool.clone();
    let mut deactivate = tokio::spawn(async move {
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
            .execute(&deactivate_pool)
            .await
    });
    let protected = tokio::select! {
        result = &mut deactivate => {
            result.unwrap().unwrap();
            false
        },
        () = wait_for_blocked(pool, command_pid) => true,
    };
    hold.commit().await.unwrap();
    assert!(pending.await.unwrap().unwrap().changed);
    if protected {
        deactivate.await.unwrap().unwrap();
    }
    assert!(
        protected,
        "deactivation must wait until the delegation commits"
    );
    assert_eq!(
        managers(pool, ids.org_id, ids.project_id).await,
        vec![target]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_delegation_retains_actor_activity_until_commit(pool: PgPool) {
    retains_activity_until_commit(&pool, ActivitySubject::Actor).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_delegation_retains_added_manager_activity_until_commit(pool: PgPool) {
    retains_activity_until_commit(&pool, ActivitySubject::AddedManager).await;
}

async fn deactivation_before_check_denies_delegation(pool: &PgPool, subject: ActivitySubject) {
    let ids = fixture(pool).await;
    let target = person(pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let user = match subject {
        ActivitySubject::Actor => ids.user_id,
        ActivitySubject::AddedManager => target,
    };
    let mut deactivate = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
        .execute(&mut *deactivate)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *deactivate)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let command = request(ids.project_id, &[target]);
    let pending =
        tokio::spawn(
            async move { execute(&pending_pool, ids.org_id, ids.user_id, &command).await },
        );
    wait_for_blocked(pool, holder).await;
    deactivate.commit().await.unwrap();
    let result = pending.await.unwrap();
    match subject {
        ActivitySubject::Actor => assert!(matches!(result, Err(ProjectManagersError::Forbidden))),
        ActivitySubject::AddedManager => {
            assert!(matches!(result, Err(ProjectManagersError::Ineligible)))
        }
    }
    assert_eq!(revision(pool, ids.org_id).await, 0);
    assert!(managers(pool, ids.org_id, ids.project_id).await.is_empty());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_delegation_denies_actor_deactivated_before_check(pool: PgPool) {
    deactivation_before_check_denies_delegation(&pool, ActivitySubject::Actor).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_delegation_denies_manager_deactivated_before_check(pool: PgPool) {
    deactivation_before_check_denies_delegation(&pool, ActivitySubject::AddedManager).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancelled_project_delegation_rolls_back_and_releases_single_connection(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let connection = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE permission_change_receipts IN SHARE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = connection.clone();
    let command = request(ids.project_id, &[target]);
    let retry = command.clone();
    let pending =
        tokio::spawn(
            async move { execute(&pending_pool, ids.org_id, ids.user_id, &command).await },
        );
    wait_for_blocked(&pool, holder).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    hold.rollback().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), revision(&connection, ids.org_id))
            .await
            .unwrap(),
        0
    );
    assert!(
        managers(&connection, ids.org_id, ids.project_id)
            .await
            .is_empty()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(&connection)
        .await
        .unwrap(),
        Some(0)
    );
    assert!(
        execute(&connection, ids.org_id, ids.user_id, &retry)
            .await
            .unwrap()
            .changed
    );
    connection.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_delegation_overrides_readonly_defaults_locally(pool: PgPool) {
    let ids = fixture(&pool).await;
    let restricted = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    execute(
        &restricted,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[ids.user_id]),
    )
    .await
    .unwrap();
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&restricted).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    restricted.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_editor_delegates_existing_read_grants_without_promotion(pool: PgPool) {
    let ids = fixture(&pool).await;
    let first = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let second = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    let before = load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, first)
        .await
        .unwrap()
        .unwrap();
    let mut command = request(ids.project_id, &[second, first]);
    let outcome = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.access_revision, 1);
    assert_eq!(
        managers(&pool, ids.org_id, ids.project_id).await,
        vec![first, second]
    );
    assert_eq!(
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, first)
            .await
            .unwrap()
            .unwrap(),
        before
    );
    let membership = sqlx::query_scalar!(
        "SELECT count(*) FROM assignments WHERE project_id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(membership, Some(0));
    command.manager_ids.reverse();
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap(),
        outcome
    );
    let json = serde_json::to_value(outcome).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 3);
    assert!(json.get("grants").is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn designation_is_required_before_managed_editor_can_delegate(pool: PgPool) {
    let ids = fixture(&pool).await;
    let editor = person(&pool, ids.org_id, &[Permission::ProjectWriteManaged]).await;
    let command = request(ids.project_id, &[editor]);
    assert!(matches!(
        execute(&pool, ids.org_id, editor, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
    execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let mut remove_self = request(ids.project_id, &[]);
    remove_self.expected_access_revision = 1;
    assert!(
        execute(&pool, ids.org_id, editor, &remove_self)
            .await
            .unwrap()
            .changed
    );
    assert!(matches!(
        execute(&pool, ids.org_id, editor, &remove_self).await,
        Err(ProjectManagersError::Forbidden)
    ));
    assert!(managers(&pool, ids.org_id, ids.project_id).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn mixed_invalid_addition_preserves_entire_previous_set(pool: PgPool) {
    let ids = fixture(&pool).await;
    let valid = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let invalid = person(&pool, ids.org_id, &[Permission::TimeReadAll]).await;
    let mut initial = request(ids.project_id, &[ids.user_id]);
    execute(&pool, ids.org_id, ids.user_id, &initial)
        .await
        .unwrap();
    initial.request_id = Uuid::now_v7();
    initial.expected_access_revision = 1;
    initial.manager_ids = vec![valid, invalid];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &initial).await,
        Err(ProjectManagersError::Ineligible)
    ));
    assert_eq!(
        managers(&pool, ids.org_id, ids.project_id).await,
        vec![ids.user_id]
    );
    assert_eq!(revision(&pool, ids.org_id).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn retained_ineligible_managers_and_archived_projects_allow_noop_and_removal(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[target]),
    )
    .await
    .unwrap();
    let before = sqlx::query!(
        "SELECT id, revision FROM project_management_assignments WHERE manager_id = $1",
        target
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", target)
        .execute(&pool)
        .await
        .unwrap();
    permissions(&pool, ids.org_id, target, &[]).await;
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut noop = request(ids.project_id, &[target]);
    noop.expected_access_revision = 1;
    assert!(
        !execute(&pool, ids.org_id, ids.user_id, &noop)
            .await
            .unwrap()
            .changed
    );
    let after = sqlx::query!(
        "SELECT id, revision FROM project_management_assignments WHERE manager_id = $1",
        target
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((before.id, before.revision), (after.id, after.revision));
    let audit = sqlx::query_scalar!(
        "SELECT audit FROM permission_change_receipts WHERE request_id = $1",
        noop.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audit["change"].is_null());
    let mut remove = request(ids.project_id, &[]);
    remove.expected_access_revision = 1;
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &remove)
            .await
            .unwrap()
            .access_revision,
        2
    );
    assert!(managers(&pool, ids.org_id, ids.project_id).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_actor_grants_activity_tenant_and_policy_are_required(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let command = request(ids.project_id, &[]);
    let reader = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = true WHERE user_id = $1",
        reader
    )
    .execute(&pool)
    .await
    .unwrap();
    for actor in [reader, foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            execute(&pool, ids.org_id, actor, &command).await,
            Err(ProjectManagersError::Forbidden)
        ));
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 0 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
    assert_eq!(revision(&pool, ids.org_id).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn foreign_missing_inactive_and_malformed_additions_fail_without_disclosure(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let inactive = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", inactive)
        .execute(&pool)
        .await
        .unwrap();
    for target in [foreign.user_id, inactive, Uuid::now_v7()] {
        assert!(matches!(
            execute(
                &pool,
                ids.org_id,
                ids.user_id,
                &request(ids.project_id, &[target])
            )
            .await,
            Err(ProjectManagersError::Ineligible)
        ));
    }
    let malformed = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    sqlx::query!(
        "UPDATE person_permission_states SET grants = ARRAY['not_a_grant'] WHERE user_id = $1",
        malformed
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &request(ids.project_id, &[malformed])
        )
        .await,
        Err(ProjectManagersError::Storage(_))
    ));
    for project in [foreign.project_id, Uuid::now_v7()] {
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &request(project, &[])).await,
            Err(ProjectManagersError::NotFound)
        ));
    }
    assert_eq!(revision(&pool, ids.org_id).await, 0);
    assert!(managers(&pool, ids.org_id, ids.project_id).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn stale_duplicate_and_exhausted_revisions_never_replace_the_set(pool: PgPool) {
    let ids = fixture(&pool).await;
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &request(ids.project_id, &[ids.user_id, ids.user_id])
        )
        .await,
        Err(ProjectManagersError::Duplicate)
    ));
    let mut command = request(ids.project_id, &[ids.user_id]);
    command.expected_access_revision = 1;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::Stale)
    ));
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        ids.org_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    command.expected_access_revision = i64::MAX;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::RevisionExhausted)
    ));
    assert!(managers(&pool, ids.org_id, ids.project_id).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn receipt_conflicts_precede_decoding_and_replay_requires_current_authority(pool: PgPool) {
    let ids = fixture(&pool).await;
    let command = request(ids.project_id, &[ids.user_id]);
    let result = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let mut changed = command.clone();
    changed.manager_ids.clear();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &changed).await,
        Err(ProjectManagersError::RequestConflict)
    ));
    sqlx::query!("UPDATE permission_change_receipts SET intent = '{}'::jsonb, result = '{}'::jsonb WHERE request_id = $1", command.request_id)
        .execute(&pool).await.unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::RequestConflict)
    ));
    permissions(
        &pool,
        ids.org_id,
        ids.user_id,
        &[Permission::ProjectReadAll],
    )
    .await;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
    assert_eq!(revision(&pool, ids.org_id).await, result.access_revision);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn audit_failure_rolls_back_removal_addition_and_revision(pool: PgPool) {
    let ids = fixture(&pool).await;
    execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[ids.user_id]),
    )
    .await
    .unwrap();
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    let mut command = request(ids.project_id, &[target]);
    command.expected_access_revision = 1;
    sqlx::query!("ALTER TABLE permission_change_receipts ADD CONSTRAINT reject_receipt CHECK (false) NOT VALID")
        .execute(&pool).await.unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::Database(_))
    ));
    assert_eq!(revision(&pool, ids.org_id).await, 1);
    assert_eq!(
        managers(&pool, ids.org_id, ids.project_id).await,
        vec![ids.user_id]
    );
    let receipts = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_change_receipts WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(receipts, Some(1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_replacements_commit_only_one_current_revision(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadAll]).await;
    let first = request(ids.project_id, &[ids.user_id]);
    let second = request(ids.project_id, &[target]);
    let (a, b) = tokio::join!(
        execute(&pool, ids.org_id, ids.user_id, &first),
        execute(&pool, ids.org_id, ids.user_id, &second)
    );
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(ProjectManagersError::Stale)) | (Err(ProjectManagersError::Stale), Ok(_))
    ));
    assert_eq!(revision(&pool, ids.org_id).await, 1);
    assert_eq!(managers(&pool, ids.org_id, ids.project_id).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn revocation_winning_gate_denies_waiting_delegation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *revoke)
    .await
    .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let org = ids.org_id;
    let actor = ids.user_id;
    let command = request(ids.project_id, &[actor]);
    let worker_pool = pool.clone();
    let pending = tokio::spawn(async move { execute(&worker_pool, org, actor, &command).await });
    wait_for_blocked(&pool, pid).await;
    let floor: Vec<String> =
        serde_json::from_value(serde_json::to_value(BuiltInProfile::Member.selection()).unwrap())
            .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
        actor,
        &floor
    )
    .execute(&mut *revoke)
    .await
    .unwrap();
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProjectManagersError::Forbidden)
    ));
    assert!(managers(&pool, org, ids.project_id).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_project_lock_returns_busy_instead_of_forming_an_org_fk_cycle(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut editor = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
        ids.project_id
    )
    .fetch_one(&mut *editor)
    .await
    .unwrap();
    let mut gate = pool.begin().await.unwrap();
    // Pause the command after it owns the organization gate, before authorization.
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    let gate_pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap()
        .unwrap();
    let org = ids.org_id;
    let actor = ids.user_id;
    let command = request(ids.project_id, &[actor]);
    let retry = command.clone();
    let worker_pool = pool.clone();
    let pending = tokio::spawn(async move { execute(&worker_pool, org, actor, &command).await });
    wait_for_blocked(&pool, gate_pid).await;
    let command_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))", gate_pid)
        .fetch_one(&pool).await.unwrap().unwrap();
    let editor_wait = tokio::spawn(async move {
        sqlx::query!("SELECT id FROM organizations WHERE id = $1 FOR SHARE", org)
            .fetch_one(&mut *editor)
            .await
            .unwrap();
        editor.commit().await.unwrap();
    });
    // The editor must still hold project UPDATE when the command tries NOWAIT.
    wait_for_blocked(&pool, command_pid).await;
    gate.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(result, Err(ProjectManagersError::Busy)),
        "{result:?}"
    );
    tokio::time::timeout(Duration::from_secs(5), editor_wait)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revision(&pool, org).await, 0);
    assert!(managers(&pool, org, ids.project_id).await.is_empty());
    let receipts = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_change_receipts WHERE request_id = $1",
        retry.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(receipts, Some(0));
    assert_eq!(
        execute(&pool, org, actor, &retry)
            .await
            .unwrap()
            .access_revision,
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn replacement_audits_exact_delta_and_preserves_membership_history_and_other_scopes(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let member_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, rate_cents) VALUES ($1, $2, $3, 12345)",
        member_id,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let cost_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_member_costs (id, org_id, project_id, user_id, cost_rate_cents) VALUES ($1, $2, $3, $4, 6789)",
        cost_id, ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    let entry =
        crate::server_fns::test_seed::time_entry(&pool, &ids, horae_core::types::EntryState::Open)
            .await;
    let other_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Other', 'EUR')",
        other_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let other_link = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
        other_link, ids.org_id, ids.user_id, other_project).execute(&pool).await.unwrap();
    let people_link = Uuid::now_v7();
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        people_link, ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &request(ids.project_id, &[ids.user_id]),
    )
    .await
    .unwrap();
    let removed_id = sqlx::query_scalar!(
        "SELECT id FROM project_management_assignments WHERE project_id = $1 AND manager_id = $2",
        ids.project_id,
        ids.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut replace = request(ids.project_id, &[target]);
    replace.expected_access_revision = 1;
    execute(&pool, ids.org_id, ids.user_id, &replace)
        .await
        .unwrap();
    let audit = sqlx::query_scalar!(
        "SELECT audit FROM permission_change_receipts WHERE request_id = $1",
        replace.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit["previous_access_revision"], 1);
    assert_eq!(audit["access_revision"], 2);
    assert_eq!(audit["change"]["removed"][0]["id"], removed_id.to_string());
    assert_eq!(
        audit["change"]["removed"][0]["manager_id"],
        ids.user_id.to_string()
    );
    assert_eq!(
        audit["change"]["added"][0]["manager_id"],
        target.to_string()
    );
    assert_eq!(audit["change"]["added"].as_array().unwrap().len(), 1);
    assert_eq!(audit["change"]["removed"].as_array().unwrap().len(), 1);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT rate_cents FROM assignments WHERE id = $1",
            member_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(12345)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT cost_rate_cents FROM project_member_costs WHERE id = $1",
            cost_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        6789
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id = $1", entry)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT id FROM project_management_assignments WHERE id = $1",
            other_link
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        other_link
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT id FROM person_management_assignments WHERE id = $1",
            people_link
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        people_link
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn historical_replay_does_not_reapply_removed_or_now_ineligible_designations(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let command = request(ids.project_id, &[target]);
    let outcome = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let mut removal = request(ids.project_id, &[]);
    removal.expected_access_revision = 1;
    execute(&pool, ids.org_id, ids.user_id, &removal)
        .await
        .unwrap();
    permissions(&pool, ids.org_id, target, &[]).await;
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap(),
        outcome
    );
    assert!(managers(&pool, ids.org_id, ids.project_id).await.is_empty());
    assert_eq!(revision(&pool, ids.org_id).await, 2);
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version = 2 WHERE request_id = $1",
        command.request_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(ProjectManagersError::ReceiptVersion)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn managed_scope_never_uses_another_project_or_legacy_membership(pool: PgPool) {
    let ids = fixture(&pool).await;
    let editor = person(&pool, ids.org_id, &[Permission::ProjectWriteManaged]).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        ids.project_id,
        editor
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut command = request(ids.project_id, &[editor]);
    assert!(matches!(
        execute(&pool, ids.org_id, editor, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
    execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let other_manager = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    command.request_id = Uuid::now_v7();
    command.expected_access_revision = 1;
    command.manager_ids.push(other_manager);
    execute(&pool, ids.org_id, editor, &command).await.unwrap();
    assert_eq!(
        managers(&pool, ids.org_id, ids.project_id).await,
        vec![editor, other_manager]
    );
    let other_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Unrelated', 'EUR')",
        other_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    command.project_id = other_project;
    assert!(matches!(
        execute(&pool, ids.org_id, editor, &command).await,
        Err(ProjectManagersError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_user_share_locks_allow_manager_and_receipt_foreign_keys(pool: PgPool) {
    let ids = fixture(&pool).await;
    let target = person(&pool, ids.org_id, &[Permission::ProjectReadManaged]).await;
    let mut legacy = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE id = ANY($1) ORDER BY id FOR SHARE",
        &[ids.user_id, target]
    )
    .fetch_all(&mut *legacy)
    .await
    .unwrap();
    let command = request(ids.project_id, &[target]);
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        execute(&pool, ids.org_id, ids.user_id, &command),
    )
    .await;
    legacy.rollback().await.unwrap();
    assert!(result.unwrap().unwrap().changed);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn actual_template_command_receipt_cannot_be_reused_for_project_delegation(pool: PgPool) {
    use super::templates::{TemplateAction, TemplateCommand, TemplateCommandError};
    let ids = fixture(&pool).await;
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = true WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let command = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        action: TemplateAction::Create {
            name: "Reader".into(),
            grants: BuiltInProfile::Member.selection().iter().collect(),
        },
    };
    super::templates::execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let mut delegation = request(ids.project_id, &[ids.user_id]);
    delegation.expected_access_revision = 1;
    delegation.request_id = command.request_id;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &delegation).await,
        Err(ProjectManagersError::RequestConflict)
    ));
    delegation.request_id = Uuid::now_v7();
    execute(&pool, ids.org_id, ids.user_id, &delegation)
        .await
        .unwrap();
    let mut other = command;
    other.request_id = delegation.request_id;
    assert!(matches!(
        super::templates::execute(&pool, ids.org_id, ids.user_id, &other).await,
        Err(TemplateCommandError::RequestConflict)
    ));
}
