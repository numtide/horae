use super::profiles::{ProfileAction, ProfileCommand, ProfileCommandError, execute};
use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use std::time::Duration;

#[path = "directory.rs"]
mod directory_tests;

#[path = "time_entries.rs"]
mod time_entries_tests;

#[path = "time_reports.rs"]
mod time_reports_tests;

#[path = "time_report_groups.rs"]
mod time_report_groups_tests;

#[path = "timesheet_people.rs"]
mod timesheet_people_tests;

#[path = "timesheet_context.rs"]
mod timesheet_context_tests;

async fn save_state(
    pool: &PgPool,
    org: Uuid,
    user: Uuid,
    admin: bool,
    grants: &PermissionSelection,
) {
    let ids: Vec<String> = serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, $5, 'individual')",
        Uuid::now_v7(),
        org,
        user,
        &ids,
        admin
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
    save_state(
        pool,
        ids.org_id,
        ids.user_id,
        true,
        &BuiltInProfile::Administrator.selection(),
    )
    .await;
    ids
}

async fn person(pool: &PgPool, org: Uuid, admin: bool, profile: BuiltInProfile) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Person')",
        id,
        org,
        format!("{id}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    save_state(pool, org, id, admin, &profile.selection()).await;
    id
}

fn apply(user_id: Uuid, profile: BuiltInProfile) -> ProfileCommand {
    ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        user_id,
        expected_person_revision: 0,
        action: ProfileAction::BuiltIn { profile },
        grants: profile.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    }
}

async fn state(pool: &PgPool, org: Uuid, user: Uuid) -> PersonPermissions {
    load_person_permissions(&mut pool.acquire().await.unwrap(), org, user)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_command_waits_for_direct_actor_deactivation_even_on_replay(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let request = apply(user, BuiltInProfile::PeopleAdmin);
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&mut *revoke)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { execute(&pending_pool, ids.org_id, ids.user_id, &request).await },
        );
    wait_for_blocked(&pool, pid).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProfileCommandError::Forbidden)
    ));
    assert_eq!(state(&pool, ids.org_id, user).await.revision, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_command_rechecks_remaining_administrator_after_deactivation_wait(pool: PgPool) {
    let ids = fixture(&pool).await;
    let survivor = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    let request = apply(ids.user_id, BuiltInProfile::Member);
    let mut revoke = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", survivor)
        .execute(&mut *revoke)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending =
        tokio::spawn(
            async move { execute(&pending_pool, ids.org_id, ids.user_id, &request).await },
        );
    wait_for_blocked(&pool, pid).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ProfileCommandError::LastAdministrator)
    ));
    assert!(state(&pool, ids.org_id, ids.user_id).await.is_administrator);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_command_overrides_readonly_defaults_locally(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
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
        &apply(user, BuiltInProfile::PeopleAdmin),
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
async fn profile_command_retains_target_activity_through_receipt_insert(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut hold = pool.begin().await.unwrap();
    // SHARE permits receipt lookup and blocks the later insert after evaluation.
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
    let pending = tokio::spawn(async move {
        execute(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            &apply(user, BuiltInProfile::PeopleAdmin),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    let command_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder)
        .fetch_one(&pool).await.unwrap().unwrap();
    let deactivate_pool = pool.clone();
    let deactivate = tokio::spawn(async move {
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
            .execute(&deactivate_pool)
            .await
            .unwrap();
    });
    wait_for_blocked(&pool, command_pid).await;
    hold.commit().await.unwrap();
    assert!(pending.await.unwrap().unwrap().changed);
    deactivate.await.unwrap();
    assert_eq!(state(&pool, ids.org_id, user).await.revision, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_command_rechecks_target_activation_before_demotion(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
        .execute(&pool)
        .await
        .unwrap();
    let mut activate = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=true WHERE id=$1", user)
        .execute(&mut *activate)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *activate)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        execute(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            &apply(user, BuiltInProfile::Member),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    activate.commit().await.unwrap();
    assert!(pending.await.unwrap().unwrap().changed);
    assert!(state(&pool, ids.org_id, ids.user_id).await.is_administrator);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_demotion_retains_surviving_administrator_until_commit(pool: PgPool) {
    let ids = fixture(&pool).await;
    let survivor = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
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
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        execute(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            &apply(ids.user_id, BuiltInProfile::Member),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    let command_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))", holder)
        .fetch_one(&pool).await.unwrap().unwrap();
    let deactivate_pool = pool.clone();
    let deactivate = tokio::spawn(async move {
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", survivor)
            .execute(&deactivate_pool)
            .await
            .unwrap();
    });
    wait_for_blocked(&pool, command_pid).await;
    hold.commit().await.unwrap();
    assert!(pending.await.unwrap().unwrap().changed);
    deactivate.await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancelled_profile_save_rolls_back_effects_and_releases_single_connection(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::PeopleAdmin).await;
    let connection = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let request = apply(user, BuiltInProfile::Member);
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
    let pending_request = request.clone();
    let pending = tokio::spawn(async move {
        execute(&pending_pool, ids.org_id, ids.user_id, &pending_request).await
    });
    wait_for_blocked(&pool, holder).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    hold.rollback().await.unwrap();
    let unchanged =
        tokio::time::timeout(Duration::from_secs(5), state(&connection, ids.org_id, user))
            .await
            .unwrap();
    assert_eq!(unchanged.revision, 0);
    assert_eq!(unchanged.grants, BuiltInProfile::PeopleAdmin.selection());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id=$1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM permission_change_receipts WHERE org_id=$1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    assert!(
        execute(&connection, ids.org_id, ids.user_id, &request)
            .await
            .unwrap()
            .changed
    );
    connection.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn explicit_application_and_adjustments_preserve_confirmed_grants(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut request = apply(user, BuiltInProfile::ProjectManager);
    let final_grants = PermissionSelection::new(&[Permission::ProjectReadManaged]);
    request.grants = final_grants.iter().collect();
    let result = execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert!(result.changed);
    assert_eq!(
        state(&pool, ids.org_id, user).await,
        PersonPermissions {
            grants: final_grants,
            is_administrator: false,
            source: PermissionSource::BuiltIn(BuiltInProfile::ProjectManager),
            revision: 1,
        }
    );
    request.grants.reverse();
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &request)
            .await
            .unwrap(),
        result
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn last_active_administrator_cannot_be_demoted_or_replaced_by_equivalent_grants(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    let inactive = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", inactive)
        .execute(&pool)
        .await
        .unwrap();
    let request = apply(ids.user_id, BuiltInProfile::Member);
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::LastAdministrator)
    ));
    assert_eq!(state(&pool, ids.org_id, ids.user_id).await.revision, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unchanged_edit_preserves_identity_source_timestamp_and_revisions(pool: PgPool) {
    let ids = fixture(&pool).await;
    let mut request = apply(ids.user_id, BuiltInProfile::Administrator);
    request.action = ProfileAction::Edit;
    let before = sqlx::query_scalar!(
        "SELECT updated_at FROM person_permission_states WHERE user_id = $1",
        ids.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let outcome = execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert!(!outcome.changed);
    assert_eq!((outcome.access_revision, outcome.person_revision), (0, 0));
    let row = sqlx::query!("SELECT updated_at, source, is_administrator FROM person_permission_states WHERE user_id = $1", ids.user_id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        (row.updated_at, row.source.as_str(), row.is_administrator),
        (before, "individual", true)
    );
    let audit = sqlx::query_scalar!(
        "SELECT audit FROM permission_change_receipts WHERE request_id = $1",
        request.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audit.get("change").unwrap().is_null());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn explicit_identity_changes_and_inactive_targets_do_not_change_activation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", user)
        .execute(&pool)
        .await
        .unwrap();
    let mut request = apply(user, BuiltInProfile::Administrator);
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert!(state(&pool, ids.org_id, user).await.is_administrator);
    request = apply(user, BuiltInProfile::Member);
    request.action = ProfileAction::Edit;
    request.expected_person_revision = 1;
    request.expected_access_revision = 1;
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert!(!state(&pool, ids.org_id, user).await.is_administrator);
    assert!(
        !sqlx::query_scalar!("SELECT active FROM users WHERE id = $1", user)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reduced_administrator_and_stale_person_proposals_are_rejected(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut request = apply(user, BuiltInProfile::Administrator);
    request.grants = BuiltInProfile::Member.selection().iter().collect();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::AdministratorSelection)
    ));
    request = apply(user, BuiltInProfile::ProjectManager);
    request.expected_person_revision = 1;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Stale)
    ));
    assert_eq!(state(&pool, ids.org_id, user).await.revision, 0);
}

async fn relationships(pool: &PgPool, ids: &SeedIds, manager: Uuid) -> (Uuid, Uuid, Uuid) {
    let project_link = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
        project_link, ids.org_id, manager, ids.project_id).execute(pool).await.unwrap();
    let person_link = Uuid::now_v7();
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        person_link, ids.org_id, manager, ids.user_id).execute(pool).await.unwrap();
    let incoming = Uuid::now_v7();
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        incoming, ids.org_id, ids.user_id, manager).execute(pool).await.unwrap();
    (project_link, person_link, incoming)
}

async fn relationship_ids(pool: &PgPool, org: Uuid) -> (Vec<Uuid>, Vec<Uuid>) {
    let projects = sqlx::query_scalar!(
        "SELECT id FROM project_management_assignments WHERE org_id = $1 ORDER BY id",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let people = sqlx::query_scalar!(
        "SELECT id FROM person_management_assignments WHERE org_id = $1 ORDER BY id",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap();
    (projects, people)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn simultaneous_losses_require_exact_confirmation_preserve_history_and_never_restore_links(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let (project_link, person_link, incoming) = relationships(&pool, &ids, user).await;
    let membership = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, rate_cents) VALUES ($1, $2, $3, 12345)",
        membership,
        ids.project_id,
        user
    )
    .execute(&pool)
    .await
    .unwrap();
    let original_links = relationship_ids(&pool, ids.org_id).await;
    let mut request = apply(user, BuiltInProfile::Member);
    request.remove_projects = vec![project_link];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    assert_eq!(relationship_ids(&pool, ids.org_id).await, original_links);
    assert_eq!(state(&pool, ids.org_id, user).await.revision, 0);
    request.remove_people = vec![person_link, incoming];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    request.remove_people = vec![person_link];
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(
        relationship_ids(&pool, ids.org_id).await,
        (vec![], vec![incoming])
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT rate_cents FROM assignments WHERE id = $1",
            membership
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(12345)
    );
    let audit = sqlx::query_scalar!(
        "SELECT audit FROM permission_change_receipts WHERE request_id = $1",
        request.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        audit["change"]["removed_projects"][0]["subject_id"],
        serde_json::json!(ids.project_id)
    );
    assert_eq!(
        audit["change"]["removed_people"][0]["id"],
        serde_json::json!(person_link)
    );
    assert_eq!(audit["change"]["before"]["revision"], 0);
    assert_eq!(audit["change"]["after"]["revision"], 1);
    let mut restore = apply(user, BuiltInProfile::ProjectManager);
    restore.expected_person_revision = 1;
    restore.expected_access_revision = 1;
    execute(&pool, ids.org_id, ids.user_id, &restore)
        .await
        .unwrap();
    assert_eq!(
        relationship_ids(&pool, ids.org_id).await,
        (vec![], vec![incoming])
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn read_only_retention_and_explicit_keep_project_access_preserve_independent_person_losses(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let (project_link, person_link, incoming) = relationships(&pool, &ids, user).await;
    let mut request = apply(user, BuiltInProfile::Member);
    request.action = ProfileAction::Edit;
    request.grants =
        PermissionSelection::new(&[Permission::ProjectReadManaged, Permission::TimeReadManaged])
            .iter()
            .collect();
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert!(
        !state(&pool, ids.org_id, user)
            .await
            .grants
            .contains(Permission::ProjectWriteManaged)
    );
    assert_eq!(
        relationship_ids(&pool, ids.org_id).await.0,
        vec![project_link]
    );
    request.request_id = Uuid::now_v7();
    request.expected_access_revision = 1;
    request.expected_person_revision = 1;
    request.grants = PermissionSelection::new(&[Permission::ProjectWriteManaged])
        .iter()
        .collect();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    request.remove_people = vec![person_link];
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(
        relationship_ids(&pool, ids.org_id).await,
        (vec![project_link], vec![incoming])
    );
    assert_eq!(
        state(&pool, ids.org_id, user).await.grants,
        PermissionSelection::new(&[Permission::ProjectWriteManaged])
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn adjusted_template_reset_and_replay_after_deletion_use_explicit_intent(pool: PgPool) {
    use super::templates::{self, TemplateAction, TemplateCommand};
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let creation = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        action: TemplateAction::Create {
            name: "Team".into(),
            grants: BuiltInProfile::ProjectManager.selection().iter().collect(),
        },
    };
    let template = templates::execute(&pool, ids.org_id, ids.user_id, &creation)
        .await
        .unwrap();
    let mut request = apply(user, BuiltInProfile::Member);
    request.expected_access_revision = 1;
    request.action = ProfileAction::Template {
        id: template.template_id,
        expected_revision: 0,
    };
    let adjusted = execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(
        state(&pool, ids.org_id, user).await.grants,
        BuiltInProfile::Member.selection()
    );
    request.request_id = Uuid::now_v7();
    request.expected_access_revision = 2;
    request.expected_person_revision = 1;
    request.grants = BuiltInProfile::ProjectManager.selection().iter().collect();
    let reset = execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(
        state(&pool, ids.org_id, user).await.grants,
        BuiltInProfile::ProjectManager.selection()
    );
    assert_ne!(adjusted, reset);
    templates::execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: 3,
            action: TemplateAction::Delete {
                id: template.template_id,
                expected_revision: 0,
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &request)
            .await
            .unwrap(),
        reset
    );
    let saved = state(&pool, ids.org_id, user).await;
    assert_eq!(
        (saved.source, saved.revision),
        (PermissionSource::Individual, 3)
    );
    request.request_id = Uuid::now_v7();
    request.expected_access_revision = 4;
    request.expected_person_revision = 3;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn all_grant_template_cannot_confer_identity_and_cross_command_keys_conflict(pool: PgPool) {
    use super::templates::{self, TemplateAction, TemplateCommand, TemplateCommandError};
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    let mut creation = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        action: TemplateAction::Create {
            name: "All grants".into(),
            grants: BuiltInProfile::Administrator.selection().iter().collect(),
        },
    };
    let template = templates::execute(&pool, ids.org_id, ids.user_id, &creation)
        .await
        .unwrap();
    let mut request = apply(user, BuiltInProfile::Administrator);
    request.action = ProfileAction::Template {
        id: template.template_id,
        expected_revision: 0,
    };
    request.expected_access_revision = 1;
    request.request_id = creation.request_id;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::RequestConflict)
    ));
    request.request_id = Uuid::now_v7();
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    let saved = state(&pool, ids.org_id, user).await;
    assert!(!saved.is_administrator);
    assert_eq!(saved.grants, BuiltInProfile::Administrator.selection());
    creation.request_id = request.request_id;
    assert!(matches!(
        templates::execute(&pool, ids.org_id, ids.user_id, &creation).await,
        Err(TemplateCommandError::RequestConflict)
    ));
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            user,
            &apply(user, BuiltInProfile::Administrator)
        )
        .await,
        Err(ProfileCommandError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_self_demotions_leave_one_active_explicit_administrator(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    let a = apply(ids.user_id, BuiltInProfile::Member);
    let b = apply(other, BuiltInProfile::Member);
    let (first, second) = tokio::join!(
        execute(&pool, ids.org_id, ids.user_id, &a),
        execute(&pool, ids.org_id, other, &b)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let (remaining, mut retry) = if first.is_ok() {
        (other, b)
    } else {
        (ids.user_id, a)
    };
    retry.expected_access_revision = 1;
    assert!(matches!(
        execute(&pool, ids.org_id, remaining, &retry).await,
        Err(ProfileCommandError::LastAdministrator)
    ));
    let count = sqlx::query_scalar!("SELECT count(*) FROM person_permission_states p JOIN users u ON u.id = p.user_id WHERE p.org_id = $1 AND p.is_administrator AND u.active", ids.org_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, Some(1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn revocation_winning_gate_denies_new_and_replayed_changes(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let request = apply(user, BuiltInProfile::ProjectManager);
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let waiting_pool = pool.clone();
    let org = ids.org_id;
    let actor = ids.user_id;
    let waiting = tokio::spawn(async move { execute(&waiting_pool, org, actor, &request).await });
    wait_for_blocked(&pool, pid).await;
    sqlx::query!("UPDATE person_permission_states SET is_administrator = false WHERE org_id = $1 AND user_id = $2", org, actor).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        waiting.await.unwrap(),
        Err(ProfileCommandError::Forbidden)
    ));
    assert!(matches!(
        execute(&pool, org, actor, &apply(user, BuiltInProfile::Member)).await,
        Err(ProfileCommandError::Forbidden)
    ));
    assert_eq!(state(&pool, org, user).await.revision, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn audit_failure_rolls_back_profile_and_both_relationship_sets(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let (project, person, _) = relationships(&pool, &ids, user).await;
    let before = state(&pool, ids.org_id, user).await;
    let links = relationship_ids(&pool, ids.org_id).await;
    sqlx::query!("ALTER TABLE permission_change_receipts ADD CONSTRAINT test_reject_profile_audit CHECK (false)").execute(&pool).await.unwrap();
    let mut request = apply(user, BuiltInProfile::Member);
    request.remove_projects = vec![project];
    request.remove_people = vec![person];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Database(_))
    ));
    assert_eq!(state(&pool, ids.org_id, user).await, before);
    assert_eq!(relationship_ids(&pool, ids.org_id).await, links);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT access_revision FROM organizations WHERE id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn existing_user_share_lock_does_not_block_profile_command(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE org_id = $1 FOR SHARE",
        ids.org_id
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &apply(user, BuiltInProfile::Accounting),
        ),
    )
    .await
    .expect("compatible user SHARE locks must not block this command")
    .unwrap();
    tx.rollback().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn management_schema_rejects_foreign_parents_self_links_and_duplicate_pairs(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let manager = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    for (manager_id, project_id) in [
        (foreign.user_id, ids.project_id),
        (manager, foreign.project_id),
    ] {
        let error = sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(), ids.org_id, manager_id, project_id).execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23503")
        );
    }
    for (manager_id, managed_user, expected) in [
        (foreign.user_id, ids.user_id, "23503"),
        (manager, foreign.user_id, "23503"),
        (manager, manager, "23514"),
    ] {
        let error = sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
            Uuid::now_v7(), ids.org_id, manager_id, managed_user).execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some(expected)
        );
    }
    relationships(&pool, &ids, manager).await;
    let error = sqlx::query!("INSERT INTO project_management_assignments (id, org_id, manager_id, project_id) VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(), ids.org_id, manager, ids.project_id).execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23505")
    );
    let error = sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(), ids.org_id, manager, ids.user_id).execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23505")
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_actor_policy_tenant_and_target_checks_precede_mutation(pool: PgPool) {
    let ids = fixture(&pool).await;
    let foreign = fixture(&pool).await;
    let non_admin = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    let request = apply(non_admin, BuiltInProfile::Member);
    for actor in [non_admin, foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            execute(&pool, ids.org_id, actor, &request).await,
            Err(ProfileCommandError::Forbidden)
        ));
    }
    for version in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version = $2 WHERE id = $1",
            ids.org_id,
            version
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &request).await,
            Err(ProfileCommandError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for user in [foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            execute(
                &pool,
                ids.org_id,
                ids.user_id,
                &apply(user, BuiltInProfile::Member)
            )
            .await,
            Err(ProfileCommandError::NotFound)
        ));
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Forbidden)
    ));
    assert_eq!(state(&pool, ids.org_id, non_admin).await.revision, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn malformed_person_or_remaining_administrator_state_fails_closed(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE person_permission_states SET catalog_version = 2 WHERE org_id = $1 AND user_id = $2", ids.org_id, other).execute(&pool).await.unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &apply(other, BuiltInProfile::Member)
        )
        .await,
        Err(ProfileCommandError::Storage(_))
    ));
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &apply(ids.user_id, BuiltInProfile::Member)
        )
        .await,
        Err(ProfileCommandError::Storage(_))
    ));
    assert_eq!(state(&pool, ids.org_id, ids.user_id).await.revision, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn stale_template_and_org_revision_and_exhausted_revisions_roll_back(pool: PgPool) {
    use super::templates::{self, TemplateAction, TemplateCommand};
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let created = templates::execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: 0,
            action: TemplateAction::Create {
                name: "Team".into(),
                grants: BuiltInProfile::Member.selection().iter().collect(),
            },
        },
    )
    .await
    .unwrap();
    let mut request = apply(user, BuiltInProfile::Member);
    request.expected_access_revision = 1;
    request.action = ProfileAction::Template {
        id: created.template_id,
        expected_revision: 1,
    };
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Stale)
    ));
    request.action = ProfileAction::Template {
        id: created.template_id,
        expected_revision: 0,
    };
    sqlx::query!(
        "UPDATE organizations SET access_revision = 2 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Stale)
    ));
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        ids.org_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    request.expected_access_revision = i64::MAX;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::RevisionExhausted)
    ));
    sqlx::query!(
        "UPDATE organizations SET access_revision = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET revision = $3 WHERE org_id = $1 AND user_id = $2",
        ids.org_id,
        user,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    request.expected_access_revision = 1;
    request.expected_person_revision = i64::MAX;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::RevisionExhausted)
    ));
    assert_eq!(
        state(&pool, ids.org_id, user).await.source,
        PermissionSource::Individual
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invalid_confirmations_and_noncanonical_grants_are_not_silently_repaired(pool: PgPool) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let mut request = apply(user, BuiltInProfile::Member);
    for grants in [
        vec![],
        vec![Permission::ProjectWriteManaged],
        vec![Permission::TimeReadOwn, Permission::TimeReadOwn],
    ] {
        request.grants = grants;
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &request).await,
            Err(ProfileCommandError::Selection(_))
        ));
    }
    request.grants = BuiltInProfile::Member.selection().iter().collect();
    request.remove_people = vec![user, user];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["is_administrator"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProfileCommand>(unknown).is_err());
    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["grants"] = serde_json::json!(["invented_admin"]);
    assert!(serde_json::from_value::<ProfileCommand>(unknown).is_err());
    assert_eq!(state(&pool, ids.org_id, user).await.revision, 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unchanged_and_unrelated_edits_do_not_clean_up_preexisting_incompatible_links(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let (project, person, _) = relationships(&pool, &ids, user).await;
    let links = relationship_ids(&pool, ids.org_id).await;
    let mut request = apply(user, BuiltInProfile::Member);
    request.action = ProfileAction::Edit;
    assert!(
        !execute(&pool, ids.org_id, ids.user_id, &request)
            .await
            .unwrap()
            .changed
    );
    assert_eq!(relationship_ids(&pool, ids.org_id).await, links);
    request.request_id = Uuid::now_v7();
    request.grants = PermissionSelection::new(&[Permission::ClientReadAll])
        .iter()
        .collect();
    request.remove_projects = vec![project];
    request.remove_people = vec![person];
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    request.remove_projects.clear();
    request.remove_people.clear();
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    assert_eq!(relationship_ids(&pool, ids.org_id).await, links);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn recreated_relationship_invalidates_waiting_confirmation_without_partial_removal(
    pool: PgPool,
) {
    let ids = fixture(&pool).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let (project, old_person, incoming) = relationships(&pool, &ids, user).await;
    let mut request = apply(user, BuiltInProfile::Member);
    request.remove_projects = vec![project];
    request.remove_people = vec![old_person];
    let mut tx = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let waiting_pool = pool.clone();
    let org = ids.org_id;
    let actor = ids.user_id;
    let pending_request = request.clone();
    let waiting =
        tokio::spawn(async move { execute(&waiting_pool, org, actor, &pending_request).await });
    wait_for_blocked(&pool, pid).await;
    sqlx::query!("DELETE FROM person_management_assignments WHERE org_id = $1 AND manager_id = $2 AND id = ANY($3)",
        org, user, &[old_person]).execute(&mut *tx).await.unwrap();
    let replacement = Uuid::now_v7();
    sqlx::query!("INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id) VALUES ($1, $2, $3, $4)",
        replacement, org, user, actor).execute(&mut *tx).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        org,
        1_i64
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        waiting.await.unwrap(),
        Err(ProfileCommandError::Stale)
    ));
    request.expected_access_revision = 1;
    assert!(matches!(
        execute(&pool, org, actor, &request).await,
        Err(ProfileCommandError::Confirmation)
    ));
    assert_eq!(
        relationship_ids(&pool, org).await,
        (vec![project], vec![incoming, replacement])
    );
    assert_eq!(state(&pool, org, user).await.revision, 0);
}
