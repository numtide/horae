use super::own::{OwnPermissionsError, read};
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use crate::server_fns::{FORBIDDEN, server_version};
use dioxus::prelude::ServerFnError;
use horae_core::{
    permissions::catalog::{BuiltInProfile, Permission},
    types::OrgRole,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn state(pool: &PgPool, ids: &SeedIds, profile: BuiltInProfile, administrator: bool) {
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(profile.selection()).unwrap()).unwrap();
    crate::server_fns::test_seed::clear_state(pool, ids.user_id).await;
    sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source, revision)
        VALUES ($1, $2, $3, 1, $4, $5, 'individual', 3)", Uuid::now_v7(), ids.org_id, ids.user_id, &grants, administrator).execute(pool).await.unwrap();
}

async fn fixture(pool: &PgPool) -> SeedIds {
    with_state(pool, BuiltInProfile::Member, false).await
}

/// A person whose stored state holds `profile`'s grants individually.
async fn with_state(pool: &PgPool, profile: BuiltInProfile, administrator: bool) -> SeedIds {
    let ids = seed(pool, BuiltInProfile::Member).await;
    state(pool, &ids, profile, administrator).await;
    sqlx::query!(
        "UPDATE organizations SET access_revision = 7 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

/// The organization's permission policy no longer changes the answer.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn own_permissions_answer_from_the_stored_state_under_either_policy(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    for policy in [0, 1] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version = $2 WHERE id = $1",
            ids.org_id,
            policy
        )
        .execute(&pool)
        .await
        .unwrap();
        let own = read(&pool, ids.org_id, ids.user_id).await.unwrap();
        assert!(own.is_administrator, "policy {policy}");
        assert_eq!(own.profile.as_deref(), Some("Administrator"));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn own_permissions_name_the_profile_or_template(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ProjectManager).await;
    let own = read(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert_eq!(own.profile.as_deref(), Some("Project Manager"));

    let template = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO permission_templates (id, org_id, name, catalog_version, grants)
         SELECT $1, org_id, 'Night shift', catalog_version, grants
           FROM person_permission_states WHERE user_id = $2",
        template,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states
            SET source = 'template', built_in_profile = NULL, template_id = $2,
                applied_template_revision = 0
          WHERE user_id = $1",
        ids.user_id,
        template
    )
    .execute(&pool)
    .await
    .unwrap();
    let own = read(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert_eq!(own.profile.as_deref(), Some("Night shift"));

    state(&pool, &ids, BuiltInProfile::Member, false).await;
    let own = read(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert_eq!(own.profile, None, "individually chosen grants");
}

// `get_server_version` past its session lookup: the user as `require_user`
// would load it. The stored role is dead data the check must not read.
fn session_user(ids: &SeedIds) -> crate::models::User {
    crate::models::User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: String::new(),
        name: String::new(),
        oidc_subject: None,
        org_role: OrgRole::Admin,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

fn is_forbidden(result: Result<String, ServerFnError>) -> bool {
    matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    )
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn server_version_follows_the_stored_administrator_identity(pool: PgPool) {
    let admin = with_state(&pool, BuiltInProfile::Administrator, true).await;
    let version = server_version(&pool, &session_user(&admin)).await;
    assert_eq!(version.unwrap(), crate::version::current());
    // Every grant without the identity, and a stored `admin` role, are not enough.
    let member = with_state(&pool, BuiltInProfile::Administrator, false).await;
    let version = server_version(&pool, &session_user(&member)).await;
    assert!(is_forbidden(version));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn server_version_refuses_a_person_deactivated_mid_session(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let version = server_version(&pool, &session_user(&ids)).await;
    assert!(is_forbidden(version));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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
            let result = read(&pool, ids.org_id, ids.user_id).await.unwrap();
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn own_permissions_deny_invalid_identity_and_state(pool: PgPool) {
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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
    let result = read(&pool, ids.org_id, ids.user_id).await.unwrap();
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
    let result = read(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert_eq!(result.managed_project_ids, expected_projects);
    let after = sqlx::query_scalar!("SELECT jsonb_build_object('org',(SELECT to_jsonb(o) FROM organizations o WHERE id=$1),'state',(SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id=$1),'people',(SELECT jsonb_agg(p ORDER BY id) FROM person_management_assignments p WHERE org_id=$1),'projects',(SELECT jsonb_agg(p ORDER BY id) FROM project_management_assignments p WHERE org_id=$1),'receipts',(SELECT count(*) FROM permission_change_receipts WHERE org_id=$1))", ids.org_id).fetch_one(&pool).await.unwrap();
    assert_eq!(after, before);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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
            let result = result.unwrap();
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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
            .is_ok()
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
