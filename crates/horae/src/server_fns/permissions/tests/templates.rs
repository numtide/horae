use super::templates::{TemplateAction, TemplateCommand, TemplateCommandError, execute};
use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use std::time::Duration;

async fn administrator(pool: &PgPool) -> SeedIds {
    // Explicit identity, not the old role or equality with Administrator grants.
    let ids = seed(pool, OrgRole::Member).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(
            BuiltInProfile::Member
                .selection()
                .iter()
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, true, 'individual')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

fn create(name: &str, expected_access_revision: i64) -> TemplateCommand {
    TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision,
        action: TemplateAction::Create {
            name: name.to_owned(),
            grants: BuiltInProfile::Member.selection().iter().collect(),
        },
    }
}

fn delete(id: Uuid, expected_access_revision: i64) -> TemplateCommand {
    TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision,
        action: TemplateAction::Delete {
            id,
            expected_revision: 0,
        },
    }
}

async fn revision_and_count(pool: &PgPool, org: Uuid) -> (i64, i64) {
    let row = sqlx::query!(
        "SELECT access_revision, (SELECT count(*) FROM permission_templates WHERE org_id = $1) AS count
         FROM organizations WHERE id = $1", org
    ).fetch_one(pool).await.unwrap();
    (row.access_revision, row.count.unwrap())
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn template_command_waits_for_direct_actor_deactivation_on_replay(pool: PgPool) {
    let ids = administrator(&pool).await;
    let request = create("Team", 0);
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
        Err(TemplateCommandError::Forbidden)
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn template_command_overrides_readonly_defaults_and_keeps_stricter_timeouts(pool: PgPool) {
    let ids = administrator(&pool).await;
    let restricted = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET statement_timeout = '250ms'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let request = create("Team", 0);
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        execute(&restricted, ids.org_id, ids.user_id, &request),
    )
    .await
    .unwrap()
    .unwrap_err();
    let TemplateCommandError::Database(sqlx::Error::Database(error)) = error else {
        panic!("Expected statement timeout: {error}")
    };
    assert_eq!(error.code().as_deref(), Some("57014"));
    hold.rollback().await.unwrap();
    let defaults = sqlx::query!("SELECT current_setting('statement_timeout') AS timeout, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&restricted).await.unwrap();
    assert_eq!(defaults.timeout.as_deref(), Some("250ms"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (0, 0));
    execute(&restricted, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    restricted.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn create_canonical_retry_returns_one_historical_change(pool: PgPool) {
    let ids = administrator(&pool).await;
    let mut command = create(" Equipo ", 0);
    let first = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    if let TemplateAction::Create { name, grants } = &mut command.action {
        *name = "Equipo".to_owned();
        grants.reverse();
    }
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap(),
        first
    );
    let stored = load_permission_template(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        first.template_id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!((stored.name.as_str(), stored.revision), ("Equipo", 0));
    assert_eq!(stored.grants, BuiltInProfile::Member.selection());
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
    command.expected_access_revision = 1;
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &command).await,
        Err(TemplateCommandError::RequestConflict)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn equivalent_names_and_invalid_confirmed_selections_roll_back(pool: PgPool) {
    let ids = administrator(&pool).await;
    execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &create(" equipo ", 1)).await,
        Err(TemplateCommandError::NameConflict)
    ));
    for name in [" ".to_owned(), "x".repeat(101)] {
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &create(&name, 1)).await,
            Err(TemplateCommandError::Name(_))
        ));
    }
    let floor: Vec<_> = BuiltInProfile::Member.selection().iter().collect();
    let mut duplicate = floor.clone();
    duplicate.push(Permission::TimeReadOwn);
    let mut missing_prerequisite = floor;
    missing_prerequisite.push(Permission::BillingWrite);
    for grants in [vec![], duplicate, missing_prerequisite] {
        let mut command = create("Valid", 1);
        command.action = TemplateAction::Create {
            name: "Valid".to_owned(),
            grants,
        };
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &command).await,
            Err(TemplateCommandError::Selection(_))
        ));
    }
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn delete_replay_does_not_touch_a_same_name_replacement(pool: PgPool) {
    let ids = administrator(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    let command = delete(created.template_id, 1);
    let removed = execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let replacement = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 2))
        .await
        .unwrap();
    assert_eq!(
        execute(&pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap(),
        removed
    );
    assert_ne!(created.template_id, replacement.template_id);
    assert!(
        load_permission_template(
            &mut pool.acquire().await.unwrap(),
            ids.org_id,
            replacement.template_id
        )
        .await
        .unwrap()
        .is_some()
    );
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (3, 1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_future_missing_foreign_and_non_admin_authority_deny(pool: PgPool) {
    let ids = administrator(&pool).await;
    let other = administrator(&pool).await;
    for actor in [other.user_id, Uuid::now_v7()] {
        assert!(matches!(
            execute(&pool, ids.org_id, actor, &create("Denied", 0)).await,
            Err(TemplateCommandError::Forbidden)
        ));
    }
    for mode in [0, 2] {
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version = $2 WHERE id = $1",
            ids.org_id,
            mode
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &create("Denied", 0)).await,
            Err(TemplateCommandError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(
            BuiltInProfile::Administrator
                .selection()
                .iter()
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    sqlx::query!("UPDATE person_permission_states SET is_administrator = false, grants = $2 WHERE user_id = $1", ids.user_id, &grants).execute(&pool).await.unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &create("Denied", 0)).await,
        Err(TemplateCommandError::Forbidden)
    ));
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &create("Denied", 0)).await,
        Err(TemplateCommandError::Forbidden)
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (0, 0));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn actor_share_lock_does_not_block_template_command(pool: PgPool) {
    let ids = administrator(&pool).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR SHARE", ids.user_id)
        .fetch_one(&mut *held)
        .await
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(5),
        execute(&pool, ids.org_id, ids.user_id, &create("Allowed", 0)),
    )
    .await
    .expect("no incompatible user lock")
    .unwrap();
    held.rollback().await.unwrap();
}

async fn receipts(pool: &PgPool, org: Uuid) -> Vec<serde_json::Value> {
    sqlx::query_scalar!(
        "SELECT audit FROM permission_change_receipts WHERE org_id = $1 ORDER BY created_at, id",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn business_snapshot(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        "SELECT jsonb_build_object('users', (SELECT jsonb_agg(u ORDER BY id) FROM users u),
         'assignments', (SELECT jsonb_agg(a ORDER BY id) FROM assignments a),
         'clients', (SELECT jsonb_agg(c ORDER BY id) FROM clients c),
         'projects', (SELECT jsonb_agg(p ORDER BY id) FROM projects p),
         'tasks', (SELECT jsonb_agg(t ORDER BY id) FROM tasks t))"
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn delete_preserves_adjusted_grants_identity_and_business_rows(pool: PgPool) {
    let ids = administrator(&pool).await;
    let other = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'other@example.test', 'Other')", other, ids.org_id).execute(&pool).await.unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(
            BuiltInProfile::Accounting
                .selection()
                .iter()
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source, revision)
         VALUES ($1, $2, $3, 1, $4, false, 'individual', 7)", Uuid::now_v7(), ids.org_id, other, &grants
    ).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', 12345)", Uuid::now_v7(), ids.project_id, other).execute(&pool).await.unwrap();
    let snapshot = business_snapshot(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    // Fixture represents previously confirmed applications with independent edits.
    sqlx::query!("UPDATE person_permission_states SET source = 'template', template_id = $2, applied_template_revision = 0 WHERE org_id = $1", ids.org_id, created.template_id).execute(&pool).await.unwrap();
    let removed = execute(
        &pool,
        ids.org_id,
        ids.user_id,
        &delete(created.template_id, 1),
    )
    .await
    .unwrap();
    assert_eq!(removed.detached_people, 2);
    for (user, selection, administrator, revision) in [
        (ids.user_id, BuiltInProfile::Member, true, 1),
        (other, BuiltInProfile::Accounting, false, 8),
    ] {
        let state = load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, user)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            state,
            PersonPermissions {
                grants: selection.selection(),
                is_administrator: administrator,
                source: PermissionSource::Individual,
                revision
            }
        );
    }
    assert_eq!(business_snapshot(&pool).await, snapshot);
    let audit = receipts(&pool, ids.org_id).await;
    assert_eq!(audit.len(), 2);
    assert_eq!(
        audit[1]["before"]["id"],
        serde_json::json!(created.template_id)
    );
    assert!(audit[1]["after"].is_null());
    let people = audit[1]["detached_people"].as_array().unwrap();
    assert_eq!(people.len(), 2);
    for person in people {
        let user: Uuid = serde_json::from_value(person["user_id"].clone()).unwrap();
        let (selection, admin, previous, revision) = if user == ids.user_id {
            (BuiltInProfile::Member, true, 0, 1)
        } else {
            assert_eq!(user, other);
            (BuiltInProfile::Accounting, false, 7, 8)
        };
        assert_eq!(
            person["grants"],
            serde_json::to_value(selection.selection().iter().collect::<Vec<_>>()).unwrap()
        );
        assert_eq!(person["is_administrator"], admin);
        assert_eq!(
            person["previous_template_id"],
            serde_json::json!(created.template_id)
        );
        assert_eq!(person["previous_applied_revision"], 0);
        assert_eq!(person["previous_revision"], previous);
        assert_eq!(person["revision"], revision);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_creators_never_exceed_fifty_profiles(pool: PgPool) {
    let ids = administrator(&pool).await;
    for index in 0..49 {
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &create(&format!("Profile {index}"), index),
        )
        .await
        .unwrap();
    }
    let first = create("First", 49);
    let second = create("Second", 49);
    let results = tokio::join!(
        execute(&pool, ids.org_id, ids.user_id, &first),
        execute(&pool, ids.org_id, ids.user_id, &second)
    );
    assert!(matches!(
        results,
        (Ok(_), Err(TemplateCommandError::Stale)) | (Err(TemplateCommandError::Stale), Ok(_))
    ));
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &create("Too many", 50)).await,
        Err(TemplateCommandError::Limit)
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (50, 50));
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 50);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn stale_foreign_and_exhausted_revisions_never_partially_delete(pool: PgPool) {
    let ids = administrator(&pool).await;
    let other = administrator(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &create("Stale", 0)).await,
        Err(TemplateCommandError::Stale)
    ));
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &delete(created.template_id, 0)
        )
        .await,
        Err(TemplateCommandError::Stale)
    ));
    assert!(matches!(
        execute(
            &pool,
            other.org_id,
            other.user_id,
            &delete(created.template_id, 0)
        )
        .await,
        Err(TemplateCommandError::NotFound)
    ));
    let mut stale = delete(created.template_id, 1);
    stale.action = TemplateAction::Delete {
        id: created.template_id,
        expected_revision: 2,
    };
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &stale).await,
        Err(TemplateCommandError::Stale)
    ));
    sqlx::query!("UPDATE person_permission_states SET source = 'template', template_id = $2, applied_template_revision = 0, revision = $3 WHERE user_id = $1", ids.user_id, created.template_id, i64::MAX).execute(&pool).await.unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &delete(created.template_id, 1)
        )
        .await,
        Err(TemplateCommandError::RevisionExhausted)
    ));
    let state =
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        state.source,
        PermissionSource::Template {
            id: created.template_id,
            applied_revision: 0
        }
    );
    assert_eq!(state.revision, i64::MAX);
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        ids.org_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &create("Overflow", i64::MAX)
        )
        .await,
        Err(TemplateCommandError::RevisionExhausted)
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (i64::MAX, 1));
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn revocation_winning_org_gate_denies_pending_replay_and_new_command(pool: PgPool) {
    for deactivate in [false, true] {
        for replay in [false, true] {
            let ids = administrator(&pool).await;
            let request = create("Equipo", 0);
            execute(&pool, ids.org_id, ids.user_id, &request)
                .await
                .unwrap();
            let request = if replay { request } else { create("New", 1) };
            let mut held = pool.begin().await.unwrap();
            let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *held)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *held)
            .await
            .unwrap();
            let command_pool = pool.clone();
            let pending = tokio::spawn(async move {
                execute(&command_pool, ids.org_id, ids.user_id, &request).await
            });
            wait_for_blocked(&pool, pid).await;
            if deactivate {
                sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                    .execute(&mut *held)
                    .await
                    .unwrap();
            } else {
                sqlx::query!("UPDATE person_permission_states SET is_administrator = false WHERE user_id = $1", ids.user_id).execute(&mut *held).await.unwrap();
            }
            held.commit().await.unwrap();
            assert!(matches!(
                pending.await.unwrap(),
                Err(TemplateCommandError::Forbidden)
            ));
            assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
            assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invalid_stored_authority_and_template_grants_fail_closed(pool: PgPool) {
    let ids = administrator(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    for grants in [vec!["unknown".to_owned()], vec![]] {
        sqlx::query!(
            "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
            ids.user_id,
            &grants
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &create("Denied", 1)).await,
            Err(TemplateCommandError::Storage(_))
        ));
    }
    // Restore the actor, then corrupt the independently loaded template selection.
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(
            BuiltInProfile::Member
                .selection()
                .iter()
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants = $2 WHERE user_id = $1",
        ids.user_id,
        &grants
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE permission_templates SET catalog_version = 2 WHERE id = $1",
        created.template_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &delete(created.template_id, 1)
        )
        .await,
        Err(TemplateCommandError::Storage(_))
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn failed_audit_insert_rolls_back_creation_and_detachment(pool: PgPool) {
    let ids = administrator(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    sqlx::query!("UPDATE person_permission_states SET source = 'template', template_id = $2, applied_template_revision = 0 WHERE user_id = $1", ids.user_id, created.template_id).execute(&pool).await.unwrap();
    sqlx::query!("ALTER TABLE permission_change_receipts ADD CONSTRAINT injected_failure CHECK (false) NOT VALID").execute(&pool).await.unwrap();
    for request in [create("Rolled back", 1), delete(created.template_id, 1)] {
        assert!(matches!(
            execute(&pool, ids.org_id, ids.user_id, &request).await,
            Err(TemplateCommandError::Database(_))
        ));
        assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
        assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
    }
    let state =
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        (state.revision, state.source),
        (
            0,
            PermissionSource::Template {
                id: created.template_id,
                applied_revision: 0
            }
        )
    );
}

#[test]
fn unknown_grants_and_authority_fields_cannot_deserialize_as_commands() {
    let request = create("Equipo", 0);
    let mut value = serde_json::to_value(&request).unwrap();
    value["action"]["grants"] = serde_json::json!(["unknown"]);
    assert!(serde_json::from_value::<TemplateCommand>(value).is_err());
    let mut value = serde_json::to_value(request).unwrap();
    value["is_administrator"] = serde_json::json!(true);
    assert!(serde_json::from_value::<TemplateCommand>(value).is_err());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn malformed_assignee_aborts_all_detachments(pool: PgPool) {
    let ids = administrator(&pool).await;
    let created = execute(&pool, ids.org_id, ids.user_id, &create("Equipo", 0))
        .await
        .unwrap();
    let person = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'invalid@example.test', 'Invalid')", person, ids.org_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id, org_id, user_id, catalog_version,
         grants, is_administrator, source, template_id, applied_template_revision)
         VALUES ($1, $2, $3, 1, ARRAY['unknown'], false, 'template', $4, 0)",
        Uuid::now_v7(),
        ids.org_id,
        person,
        created.template_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE person_permission_states SET source = 'template', template_id = $2, applied_template_revision = 0 WHERE user_id = $1", ids.user_id, created.template_id).execute(&pool).await.unwrap();
    let before = sqlx::query_scalar!(
        "SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &pool,
            ids.org_id,
            ids.user_id,
            &delete(created.template_id, 1)
        )
        .await,
        Err(TemplateCommandError::Storage(
            PermissionStorageError::Identifier(_)
        ))
    ));
    let after = sqlx::query_scalar!(
        "SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, before);
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_exact_retry_creates_one_receipt_without_changing_people(pool: PgPool) {
    let ids = administrator(&pool).await;
    let before = sqlx::query_scalar!(
        "SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let request = create(" Equipo ", 0);
    let (first, second) = tokio::join!(
        execute(&pool, ids.org_id, ids.user_id, &request),
        execute(&pool, ids.org_id, ids.user_id, &request)
    );
    let outcome = first.unwrap();
    assert_eq!(second.unwrap(), outcome);
    let after = sqlx::query_scalar!(
        "SELECT jsonb_agg(s ORDER BY user_id) FROM person_permission_states s WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, before);
    let audits = receipts(&pool, ids.org_id).await;
    assert_eq!(
        audits,
        vec![serde_json::json!({
            "previous_access_revision": 0, "access_revision": 1,
            "before": null, "after": { "id": outcome.template_id, "name": "Equipo",
                "catalog_version": 1, "revision": 0,
                "grants": BuiltInProfile::Member.selection().iter().collect::<Vec<_>>() },
            "detached_people": []
        })]
    );
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
}

async fn insert_receipt(
    pool: &PgPool,
    org: Uuid,
    principal: (Option<Uuid>, Option<&str>, Option<&str>),
    request: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO permission_change_receipts
         (id, org_id, actor_user_id, operator_id, operator_command, request_id, format_version, intent, result, audit)
         VALUES ($1, $2, $3, $4, $5, $6, 1, '{}', '{}', '{}')",
        Uuid::now_v7(), org, principal.0, principal.1, principal.2, request
    ).execute(pool).await?;
    Ok(())
}

fn sqlstate(error: sqlx::Error, code: &str) {
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some(code)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn receipts_enforce_tenant_exclusive_principal_and_request_uniqueness(pool: PgPool) {
    let ids = administrator(&pool).await;
    let other = administrator(&pool).await;
    for principal in [
        (None, None, None),
        (Some(ids.user_id), Some("operator"), Some("command")),
        (Some(ids.user_id), None, Some("command")),
        (None, Some("operator"), None),
        (None, Some(" "), Some("command")),
        (None, Some("operator"), Some(" ")),
    ] {
        sqlstate(
            insert_receipt(&pool, ids.org_id, principal, Uuid::now_v7())
                .await
                .unwrap_err(),
            "23514",
        );
    }
    sqlstate(
        insert_receipt(
            &pool,
            ids.org_id,
            (Some(other.user_id), None, None),
            Uuid::now_v7(),
        )
        .await
        .unwrap_err(),
        "23503",
    );
    let request = Uuid::now_v7();
    let user = (Some(ids.user_id), None, None);
    let operator = (None, Some("operator"), Some("reconcile"));
    insert_receipt(&pool, ids.org_id, user, request)
        .await
        .unwrap();
    insert_receipt(&pool, ids.org_id, operator, request)
        .await
        .unwrap();
    sqlstate(
        insert_receipt(&pool, ids.org_id, user, request)
            .await
            .unwrap_err(),
        "23505",
    );
    sqlstate(
        insert_receipt(&pool, ids.org_id, operator, request)
            .await
            .unwrap_err(),
        "23505",
    );
    insert_receipt(
        &pool,
        other.org_id,
        (Some(other.user_id), None, None),
        request,
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn command_receipts_are_not_shared_between_administrators(pool: PgPool) {
    let ids = administrator(&pool).await;
    let mut request = create("Equipo", 0);
    let first = execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    let second_actor = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'admin2@example.test', 'Other admin')", second_actor, ids.org_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         SELECT $1, org_id, $2, catalog_version, grants, true, 'individual'
         FROM person_permission_states WHERE user_id = $3",
        Uuid::now_v7(), second_actor, ids.user_id
    ).execute(&pool).await.unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, second_actor, &request).await,
        Err(TemplateCommandError::Stale)
    ));
    request.expected_access_revision = 1;
    request.action = create("Other", 1).action;
    let second = execute(&pool, ids.org_id, second_actor, &request)
        .await
        .unwrap();
    assert_ne!(first.template_id, second.template_id);
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unsupported_receipt_version_fails_without_repeating_the_change(pool: PgPool) {
    let ids = administrator(&pool).await;
    let request = create("Equipo", 0);
    execute(&pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version = 2 WHERE org_id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        execute(&pool, ids.org_id, ids.user_id, &request).await,
        Err(TemplateCommandError::ReceiptVersion)
    ));
    assert_eq!(revision_and_count(&pool, ids.org_id).await, (1, 1));
    assert_eq!(receipts(&pool, ids.org_id).await.len(), 1);
}
