use super::own::{OwnPermissionsError, read};
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::{
    permissions::catalog::{BuiltInProfile, Permission},
    types::OrgRole,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn state(pool: &PgPool, ids: &SeedIds, profile: BuiltInProfile, administrator: bool) {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(profile.selection()).unwrap()).unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source, revision)
        VALUES ($1, $2, $3, 1, $4, $5, 'individual', 3)", Uuid::now_v7(), ids.org_id, ids.user_id, &grants, administrator).execute(pool).await.unwrap();
}

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Member).await;
    state(pool, &ids, BuiltInProfile::Member, false).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1, access_revision = 7 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_legacy_mode_never_discloses_staging(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    assert_eq!(read(&pool, ids.org_id, ids.user_id).await.unwrap(), None);
    state(&pool, &ids, BuiltInProfile::Administrator, true).await;
    assert_eq!(read(&pool, ids.org_id, ids.user_id).await.unwrap(), None);
    sqlx::query!("UPDATE person_permission_states SET grants = ARRAY['private_invalid_grant'] WHERE user_id = $1", ids.user_id).execute(&pool).await.unwrap();
    assert_eq!(read(&pool, ids.org_id, ids.user_id).await.unwrap(), None);
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_preserve_exact_grants_and_independent_identity(pool: PgPool) {
    for profile in BuiltInProfile::ALL {
        let ids = fixture(&pool).await;
        let mut selection = profile.selection();
        selection.add(Permission::InvoiceReadManaged);
        let grants: Vec<String> =
            serde_json::from_value(serde_json::to_value(&selection).unwrap()).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
            ids.user_id,
            &grants
        )
        .execute(&pool)
        .await
        .unwrap();
        for administrator in [false, true] {
            sqlx::query!(
                "UPDATE person_permission_states SET is_administrator = $2 WHERE user_id = $1",
                ids.user_id,
                administrator
            )
            .execute(&pool)
            .await
            .unwrap();
            let result = read(&pool, ids.org_id, ids.user_id).await.unwrap().unwrap();
            assert_eq!(result.grants, selection.iter().collect::<Vec<_>>());
            assert_eq!(result.is_administrator, administrator);
            assert_eq!(
                (
                    result.catalog_version,
                    result.access_revision,
                    result.person_revision
                ),
                (1, 7, 3)
            );
            assert!(result.managed_person_ids.is_empty());
            assert!(result.managed_project_ids.is_empty());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_deny_invalid_identity_policy_and_state(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    for (org, actor) in [
        (ids.org_id, foreign.user_id),
        (ids.org_id, Uuid::now_v7()),
        (Uuid::now_v7(), ids.user_id),
    ] {
        assert!(matches!(
            read(&pool, org, actor).await,
            Err(OwnPermissionsError::Forbidden)
        ));
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Forbidden)
    ));
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 2 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Unavailable)
    ));
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE person_permission_states SET grants = ARRAY['private_invalid_grant'] WHERE user_id = $1", ids.user_id).execute(&pool).await.unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Storage(_))
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Unavailable)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_wait_for_current_actor_deactivation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut writer = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&mut *writer)
        .await
        .unwrap();
    let pending = {
        let pool = pool.clone();
        tokio::spawn(async move { read(&pool, ids.org_id, ids.user_id).await })
    };
    wait_for_blocked(&pool, pid).await;
    writer.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap(),
        Err(OwnPermissionsError::Forbidden)
    ));
}

async fn person(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Private person')",
        id,
        org,
        format!("{id}@example.test")
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_scope_is_sorted_and_excludes_membership_and_other_managers(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id).await;
    let first = person(&pool, ids.org_id).await;
    let second = person(&pool, ids.org_id).await;
    for target in [second, first] {
        sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    }
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, other, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role) VALUES ($1,$2,$3,'lead')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, other, ids.project_id).execute(&pool).await.unwrap();
    let foreign = fixture(&pool).await;
    sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), foreign.org_id, foreign.user_id, foreign.project_id).execute(&pool).await.unwrap();
    let result = read(&pool, ids.org_id, ids.user_id).await.unwrap().unwrap();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(result.managed_person_ids, expected);
    assert!(result.managed_project_ids.is_empty());
    let second_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1,$2,$3,'Second project','EUR')", second_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let mut expected_projects = vec![ids.project_id, second_project];
    expected_projects.sort();
    for project in expected_projects.iter().rev() {
        sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, *project).execute(&pool).await.unwrap();
    }
    let before = sqlx::query_scalar!("SELECT jsonb_build_object('org',(SELECT to_jsonb(o) FROM organizations o WHERE id=$1),'state',(SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id=$1),'people',(SELECT jsonb_agg(p ORDER BY id) FROM person_management_assignments p WHERE org_id=$1),'projects',(SELECT jsonb_agg(p ORDER BY id) FROM project_management_assignments p WHERE org_id=$1),'receipts',(SELECT count(*) FROM permission_change_receipts WHERE org_id=$1))", ids.org_id).fetch_one(&pool).await.unwrap();
    let result = read(&pool, ids.org_id, ids.user_id).await.unwrap().unwrap();
    assert_eq!(result.managed_project_ids, expected_projects);
    let after = sqlx::query_scalar!("SELECT jsonb_build_object('org',(SELECT to_jsonb(o) FROM organizations o WHERE id=$1),'state',(SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id=$1),'people',(SELECT jsonb_agg(p ORDER BY id) FROM person_management_assignments p WHERE org_id=$1),'projects',(SELECT jsonb_agg(p ORDER BY id) FROM project_management_assignments p WHERE org_id=$1),'receipts',(SELECT count(*) FROM permission_change_receipts WHERE org_id=$1))", ids.org_id).fetch_one(&pool).await.unwrap();
    assert_eq!(after, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_reload_after_winning_gate_under_repeatable_read_defaults(pool: PgPool) {
    let ids = fixture(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for deactivate in [false, true] {
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
        let pending = {
            let pool = reader_pool.clone();
            tokio::spawn(async move { read(&pool, ids.org_id, ids.user_id).await })
        };
        wait_for_blocked(&pool, pid).await;
        if deactivate {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&mut *writer)
                .await
                .unwrap();
        } else {
            let grants: Vec<String> = serde_json::from_value(
                serde_json::to_value(BuiltInProfile::Accounting.selection()).unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2, revision=4 WHERE user_id=$1",
                ids.user_id,
                &grants
            )
            .execute(&mut *writer)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE organizations SET access_revision=8 WHERE id=$1",
                ids.org_id
            )
            .execute(&mut *writer)
            .await
            .unwrap();
            sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&mut *writer).await.unwrap();
        }
        writer.commit().await.unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap();
        if deactivate {
            assert!(matches!(result, Err(OwnPermissionsError::Forbidden)));
        } else {
            let result = result.unwrap().unwrap();
            assert_eq!(
                result.grants,
                BuiltInProfile::Accounting
                    .selection()
                    .iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!((result.access_revision, result.person_revision), (8, 4));
            assert_eq!(result.managed_project_ids, vec![ids.project_id]);
        }
    }
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn own_permissions_reader_retains_actor_activity_until_snapshot_is_loaded(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE person_permission_states IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let blocker_pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap()
        .unwrap();
    let pending = {
        let pool = pool.clone();
        tokio::spawn(async move { read(&pool, ids.org_id, ids.user_id).await })
    };
    wait_for_blocked(&pool, blocker_pid).await;
    let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker_pid).fetch_one(&pool).await.unwrap().unwrap();
    let revoke = {
        let pool = pool.clone();
        tokio::spawn(async move {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
        })
    };
    wait_for_blocked(&pool, reader_pid).await;
    blocker.commit().await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .is_some()
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), revoke)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id).await,
        Err(OwnPermissionsError::Forbidden)
    ));
}
