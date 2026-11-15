//! The own-permission explanation, the version gate and the profile save,
//! each reaching only the session person's records.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::Recorder;
use crate::ops::{Caller, begin, begin_read};
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};

/// `get_my_permissions` past its session lookup.
async fn read(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<OwnPermissions, OpError> {
    let mut read = begin_read(pool, Caller::session(org_id, user_id)).await?;
    let result = permissions(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

/// `get_server_version` past its session lookup.
async fn version(pool: &PgPool, ids: &SeedIds) -> Result<String, OpError> {
    let read = begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = server_version(&read.actor);
    read.finish(result).await
}

/// `update_own_profile` past its session lookup.
async fn save(pool: &PgPool, ids: &SeedIds, name: &str, zone: Option<&str>) -> Result<(), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = update(&mut write.tx, &write.actor, name, zone).await;
    write.commit(result, &Recorder::default()).await
}

fn forbidden(message: &str) -> Result<OwnPermissions, OpError> {
    Err(OpError::forbidden(message))
}

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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn own_permissions_answer_from_the_stored_state(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let own = read(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert!(own.is_administrator);
    assert_eq!(own.profile.as_deref(), Some("Administrator"));
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn server_version_follows_the_stored_administrator_identity(pool: PgPool) {
    let admin = with_state(&pool, BuiltInProfile::Administrator, true).await;
    assert_eq!(version(&pool, &admin).await, Ok(crate::version::current()));
    // Every grant without the identity is not enough.
    let member = with_state(&pool, BuiltInProfile::Administrator, false).await;
    assert_eq!(
        version(&pool, &member).await,
        Err(OpError::forbidden("Administrator authority is required"))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn server_version_refuses_a_person_deactivated_mid_session(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        version(&pool, &ids).await,
        Err(OpError::forbidden(IDENTITY_REQUIRED))
    );
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
        assert_eq!(read(&pool, org, actor).await, forbidden(IDENTITY_REQUIRED));
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id).await,
        forbidden(IDENTITY_REQUIRED)
    );
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    // Invalid or missing state fails closed, as `Actor::load` does everywhere.
    sqlx::query!("UPDATE person_permission_states SET grants = ARRAY['private_invalid_grant'] WHERE user_id = $1", ids.user_id).execute(&pool).await.unwrap();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id).await,
        forbidden("Permission state is unavailable")
    );
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        read(&pool, ids.org_id, ids.user_id).await,
        forbidden("Permission state is unavailable")
    );
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
        Err(OpError::Forbidden(_))
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
            assert!(matches!(result, Err(OpError::Forbidden(_))));
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
        Err(OpError::Forbidden(_))
    ));
}

async fn profile(pool: &PgPool, user_id: Uuid) -> (String, Option<String>) {
    let row = sqlx::query!("SELECT name, timezone FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    (row.name, row.timezone)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_saves_their_name_and_zone(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    save(&pool, &ids, "  Ana Ruiz ", Some("Europe/Madrid"))
        .await
        .unwrap();
    assert_eq!(
        profile(&pool, ids.user_id).await,
        ("Ana Ruiz".into(), Some("Europe/Madrid".into()))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn clearing_the_zone_follows_the_workspace_again(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    save(&pool, &ids, "Ana", Some("Asia/Tokyo")).await.unwrap();
    save(&pool, &ids, "Ana", None).await.unwrap();
    assert_eq!(profile(&pool, ids.user_id).await.1, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unknown_zone_is_rejected_and_nothing_changes(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    for bad in ["Mars/Olympus_Mons", "europe/madrid", "+02:00", ""] {
        assert_eq!(
            save(&pool, &ids, "Ana", Some(bad)).await,
            Err(OpError::invalid(
                "timezone",
                "Choose a time zone from the list"
            ))
        );
    }
    assert_eq!(
        profile(&pool, ids.user_id).await,
        ("Test User".into(), None)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_blank_or_overlong_name_is_rejected(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    for bad in ["   ".to_owned(), "x".repeat(201), "a\0b".to_owned()] {
        assert_eq!(
            save(&pool, &ids, &bad, None).await,
            Err(OpError::invalid(
                "name",
                "Enter a name of 1 to 200 characters"
            ))
        );
    }
    assert_eq!(profile(&pool, ids.user_id).await.0, "Test User");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_profile_save_leaves_colleagues_alone(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let other = person(&pool, ids.org_id).await;
    save(&pool, &ids, "Ana", Some("Asia/Tokyo")).await.unwrap();
    assert_eq!(profile(&pool, other).await, ("Private person".into(), None));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_deactivated_person_cannot_save(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        save(&pool, &ids, "Ana", None).await,
        Err(OpError::forbidden("Account is not active"))
    );
    assert_eq!(profile(&pool, ids.user_id).await.0, "Test User");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_personal_zone_overrides_the_workspace_zone(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    sqlx::query!(
        "UPDATE organizations SET timezone = 'Pacific/Kiritimati' WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        crate::timezone::person_zone(&pool, ids.user_id)
            .await
            .unwrap(),
        "Pacific/Kiritimati"
    );
    save(&pool, &ids, "Ana", Some("Etc/GMT+12")).await.unwrap();
    assert_eq!(
        crate::timezone::person_zone(&pool, ids.user_id)
            .await
            .unwrap(),
        "Etc/GMT+12"
    );
}
