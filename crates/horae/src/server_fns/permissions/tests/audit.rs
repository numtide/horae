use super::audit::{AuditPrincipal, AuditReadError, decode, read};
use super::templates::{TemplateAction, TemplateCommand};
use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::OrgRole;
use serde_json::json;
use sqlx::PgPool;
use std::time::Duration;

async fn fixture(pool: &PgPool, administrator: bool) -> SeedIds {
    let ids = seed(pool, OrgRole::Admin).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(BuiltInProfile::Administrator.selection()).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
        VALUES ($1, $2, $3, 1, $4, $5, 'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id, &grants, administrator).execute(pool).await.unwrap();
    ids
}

async fn receipt_id(pool: &PgPool, request: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM permission_change_receipts WHERE request_id = $1",
        request
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn person(pool: &PgPool, org: Uuid, administrator: bool, profile: BuiltInProfile) -> Uuid {
    let user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Audit person')",
        user,
        org,
        format!("{user}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(profile.selection()).unwrap()).unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
        VALUES ($1, $2, $3, 1, $4, $5, 'individual')",
        Uuid::now_v7(), org, user, &grants, administrator).execute(pool).await.unwrap();
    user
}

async fn create_template(pool: &PgPool, ids: &SeedIds) -> (Uuid, Uuid) {
    let request = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        action: TemplateAction::Create {
            name: "Readers".into(),
            grants: BuiltInProfile::Member.selection().iter().collect(),
        },
    };
    let outcome = super::templates::execute(pool, ids.org_id, ids.user_id, &request)
        .await
        .unwrap();
    (
        receipt_id(pool, request.request_id).await,
        outcome.template_id,
    )
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn administrator_reads_historical_template_without_intent_or_replay_result(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, template) = create_template(&pool, &ids).await;
    let record = read(&pool, ids.org_id, ids.user_id, receipt)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.id, receipt);
    assert_eq!(
        record.actor,
        AuditPrincipal::User {
            user_id: ids.user_id
        }
    );
    let view = serde_json::to_value(&record).unwrap();
    assert!(view.get("intent").is_none());
    assert!(view.get("result").is_none());
    assert!(view.get("request_id").is_none());
    assert_eq!(view["audit"]["kind"], "template");
    assert_eq!(
        view["audit"]["details"]["after"]["id"],
        template.to_string()
    );
    let deletion = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        action: TemplateAction::Delete {
            id: template,
            expected_revision: 0,
        },
    };
    super::templates::execute(&pool, ids.org_id, ids.user_id, &deletion)
        .await
        .unwrap();
    let historical = read(&pool, ids.org_id, ids.user_id, receipt)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_value(historical).unwrap(), view);
    let removed = read(
        &pool,
        ids.org_id,
        ids.user_id,
        receipt_id(&pool, deletion.request_id).await,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::to_value(removed).unwrap()["audit"]["details"]["before"]["id"],
        template.to_string()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_admin_and_all_grants_without_explicit_identity_cannot_read_audit(pool: PgPool) {
    let ids = fixture(&pool, false).await;
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, Uuid::now_v7()).await,
        Err(AuditReadError::Forbidden)
    ));
}

#[test]
fn historical_decoder_distinguishes_missing_change_from_explicit_noop() {
    let mut audit =
        json!({"project_id": Uuid::now_v7(), "previous_access_revision": 4, "access_revision": 4});
    assert!(matches!(
        decode(1, audit.clone()),
        Err(AuditReadError::InvalidDocument)
    ));
    audit["change"] = serde_json::Value::Null;
    assert!(decode(1, audit.clone()).is_ok());
    audit["access_revision"] = json!(5);
    assert!(matches!(
        decode(1, audit),
        Err(AuditReadError::InvalidDocument)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_and_project_history_decode_exact_changes_and_noops(pool: PgPool) {
    use super::profiles::{ProfileAction, ProfileCommand};
    use super::project_management::ProjectManagersCommand;
    let ids = fixture(&pool, true).await;
    let mut profile = ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        user_id: ids.user_id,
        expected_person_revision: 0,
        action: ProfileAction::BuiltIn {
            profile: BuiltInProfile::Administrator,
        },
        grants: BuiltInProfile::Administrator.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    };
    super::profiles::execute(&pool, ids.org_id, ids.user_id, &profile)
        .await
        .unwrap();
    let record = read(
        &pool,
        ids.org_id,
        ids.user_id,
        receipt_id(&pool, profile.request_id).await,
    )
    .await
    .unwrap()
    .unwrap();
    let audit = serde_json::to_value(record).unwrap();
    assert_eq!(audit["audit"]["kind"], "profile");
    assert_eq!(
        audit["audit"]["details"]["change"]["after"]["grants"],
        serde_json::to_value(&profile.grants).unwrap()
    );
    profile.request_id = Uuid::now_v7();
    profile.expected_access_revision = 1;
    profile.expected_person_revision = 1;
    profile.action = ProfileAction::Edit;
    super::profiles::execute(&pool, ids.org_id, ids.user_id, &profile)
        .await
        .unwrap();
    let noop = read(
        &pool,
        ids.org_id,
        ids.user_id,
        receipt_id(&pool, profile.request_id).await,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(serde_json::to_value(noop).unwrap()["audit"]["details"]["change"].is_null());
    let mut managers = ProjectManagersCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        project_id: ids.project_id,
        manager_ids: vec![ids.user_id],
    };
    super::project_management::execute(&pool, ids.org_id, ids.user_id, &managers)
        .await
        .unwrap();
    let added_id = receipt_id(&pool, managers.request_id).await;
    let added = serde_json::to_value(
        read(&pool, ids.org_id, ids.user_id, added_id)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(added["audit"]["kind"], "project_managers");
    assert_eq!(
        added["audit"]["details"]["change"]["added"][0]["manager_id"],
        ids.user_id.to_string()
    );
    managers.request_id = Uuid::now_v7();
    managers.expected_access_revision = 2;
    super::project_management::execute(&pool, ids.org_id, ids.user_id, &managers)
        .await
        .unwrap();
    let noop = read(
        &pool,
        ids.org_id,
        ids.user_id,
        receipt_id(&pool, managers.request_id).await,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(serde_json::to_value(noop).unwrap()["audit"]["details"]["change"].is_null());
    managers.request_id = Uuid::now_v7();
    managers.manager_ids.clear();
    super::project_management::execute(&pool, ids.org_id, ids.user_id, &managers)
        .await
        .unwrap();
    let removed = read(
        &pool,
        ids.org_id,
        ids.user_id,
        receipt_id(&pool, managers.request_id).await,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::to_value(removed).unwrap()["audit"]["details"]["change"]["removed"][0],
        added["audit"]["details"]["change"]["added"][0]
    );
    assert_eq!(
        serde_json::to_value(
            read(&pool, ids.org_id, ids.user_id, added_id)
                .await
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        added
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn tenant_activity_and_policy_checks_precede_receipt_existence_and_decode(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let foreign = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
    assert!(
        read(&pool, foreign.org_id, foreign.user_id, receipt)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        read(&pool, foreign.org_id, foreign.user_id, Uuid::now_v7())
            .await
            .unwrap()
            .is_none()
    );
    for actor in [foreign.user_id, Uuid::now_v7()] {
        assert!(matches!(
            read(&pool, ids.org_id, actor, receipt).await,
            Err(AuditReadError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version = 99 WHERE id = $1",
        receipt
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, receipt).await,
        Err(AuditReadError::Forbidden)
    ));
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
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
            read(&pool, ids.org_id, ids.user_id, receipt).await,
            Err(AuditReadError::Forbidden)
        ));
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, receipt).await,
        Err(AuditReadError::UnsupportedFormat)
    ));
    sqlx::query!(
        "UPDATE permission_change_receipts SET format_version = 1, audit = '{}'::jsonb WHERE id = $1",
        receipt
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, receipt).await,
        Err(AuditReadError::InvalidDocument)
    ));
    sqlx::query!(
        "UPDATE person_permission_states SET grants = ARRAY['not_a_permission'] WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, ids.user_id, receipt).await,
        Err(AuditReadError::Storage(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn operator_attribution_is_not_a_user_and_excludes_private_replay_payload(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
    let operator_receipt = Uuid::now_v7();
    sqlx::query!("INSERT INTO permission_change_receipts
        (id, org_id, operator_id, operator_command, request_id, format_version, intent, result, audit)
        SELECT $1, org_id, 'operator-invocation', 'permission.change', $2, format_version,
        '{\"private_input\":\"sentinel\"}'::jsonb, '{\"private_result\":\"sentinel\"}'::jsonb, audit
        FROM permission_change_receipts WHERE id = $3", operator_receipt, Uuid::now_v7(), receipt)
        .execute(&pool).await.unwrap();
    let record = read(&pool, ids.org_id, ids.user_id, operator_receipt)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        record.actor,
        AuditPrincipal::Operator {
            invocation_id: "operator-invocation".into(),
            command: "permission.change".into()
        }
    );
    let json = serde_json::to_string(&record).unwrap();
    assert!(!json.contains("sentinel"));
    assert!(!json.contains("private_input"));
    assert!(!json.contains("private_result"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn failed_mutation_has_no_success_audit_and_reads_do_not_write(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
    let request = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        action: TemplateAction::Create {
            name: "READERS".into(),
            grants: BuiltInProfile::Member.selection().iter().collect(),
        },
    };
    assert!(
        super::templates::execute(&pool, ids.org_id, ids.user_id, &request)
            .await
            .is_err()
    );
    read(&pool, ids.org_id, ids.user_id, receipt)
        .await
        .unwrap()
        .unwrap();
    read(&pool, ids.org_id, ids.user_id, Uuid::now_v7())
        .await
        .unwrap();
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_change_receipts WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, Some(1));
    let revision = sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(revision, 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn revocation_winning_gate_denies_waiting_historical_reader(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
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
    let worker_pool = pool.clone();
    let pending = tokio::spawn(async move { read(&worker_pool, org, actor, receipt).await });
    wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = false WHERE user_id = $1",
        actor
    )
    .execute(&mut *revoke)
    .await
    .unwrap();
    revoke.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap(),
        Err(AuditReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reader_winning_gate_finishes_before_revocation_and_later_reads_fail(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
    // Pause the real reader after authorization, without adding production hooks.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let org = ids.org_id;
    let actor = ids.user_id;
    let reader_pool = pool.clone();
    let reader = tokio::spawn(async move { read(&reader_pool, org, actor, receipt).await });
    wait_for_blocked(&pool, holder).await;
    let reader_pid = sqlx::query_scalar!(
        "SELECT pid FROM pg_stat_activity WHERE datname = current_database()
         AND $1 = ANY(pg_blocking_pids(pid))",
        holder
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let writer_pool = pool.clone();
    let writer = tokio::spawn(async move {
        let mut tx = writer_pool.begin().await.unwrap();
        sqlx::query!("SELECT id FROM organizations WHERE id = $1 FOR UPDATE", org)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET is_administrator = false WHERE user_id = $1",
            actor
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    });
    wait_for_blocked(&pool, reader_pid.unwrap()).await;
    hold.commit().await.unwrap();
    let record = tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(record.id, receipt);
    tokio::time::timeout(Duration::from_secs(5), writer)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        read(&pool, org, actor, receipt).await,
        Err(AuditReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn current_administrator_reads_inactive_authors_history_without_live_state(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let (receipt, _) = create_template(&pool, &ids).await;
    let admin = person(&pool, ids.org_id, true, BuiltInProfile::Administrator).await;
    let original = serde_json::to_value(
        read(&pool, ids.org_id, admin, receipt)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let historical = read(&pool, ids.org_id, admin, receipt)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_value(historical).unwrap(), original);
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        admin
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, ids.org_id, admin, receipt).await,
        Err(AuditReadError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn template_detachment_history_preserves_exact_grants_and_provenance(pool: PgPool) {
    use super::profiles::{ProfileAction, ProfileCommand};
    let ids = fixture(&pool, true).await;
    let user = person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
    let (_, template) = create_template(&pool, &ids).await;
    let application = ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 1,
        user_id: user,
        expected_person_revision: 0,
        action: ProfileAction::Template {
            id: template,
            expected_revision: 0,
        },
        grants: BuiltInProfile::ProjectManager.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    };
    super::profiles::execute(&pool, ids.org_id, ids.user_id, &application)
        .await
        .unwrap();
    let deletion = TemplateCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 2,
        action: TemplateAction::Delete {
            id: template,
            expected_revision: 0,
        },
    };
    super::templates::execute(&pool, ids.org_id, ids.user_id, &deletion)
        .await
        .unwrap();
    let stored = sqlx::query!(
        "SELECT id, audit FROM permission_change_receipts WHERE request_id = $1",
        deletion.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let historical = serde_json::to_value(
        read(&pool, ids.org_id, ids.user_id, stored.id)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(historical["audit"]["details"], stored.audit);
    let detached = &historical["audit"]["details"]["detached_people"][0];
    assert_eq!(detached["user_id"], json!(user));
    assert_eq!(detached["grants"], json!(application.grants));
    assert_eq!(detached["previous_template_id"], json!(template));
    assert_eq!(detached["previous_revision"], 1);
    assert_eq!(detached["revision"], 2);
    let application_history = serde_json::to_value(
        read(
            &pool,
            ids.org_id,
            ids.user_id,
            receipt_id(&pool, application.request_id).await,
        )
        .await
        .unwrap()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        application_history["audit"]["details"]["change"]["after"]["source"],
        json!({"kind": "template", "value": {"id": template, "applied_revision": 0}})
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn profile_history_keeps_removed_relationships_after_grants_change_again(pool: PgPool) {
    use super::profiles::{ProfileAction, ProfileCommand};
    let ids = fixture(&pool, true).await;
    let manager = person(&pool, ids.org_id, false, BuiltInProfile::ProjectManager).await;
    let project_link = Uuid::now_v7();
    let person_link = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, manager_id, project_id)
        VALUES ($1, $2, $3, $4)",
        project_link,
        ids.org_id,
        manager,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id)
        VALUES ($1, $2, $3, $4)",
        person_link,
        ids.org_id,
        manager,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut command = ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        user_id: manager,
        expected_person_revision: 0,
        action: ProfileAction::BuiltIn {
            profile: BuiltInProfile::Member,
        },
        grants: BuiltInProfile::Member.selection().iter().collect(),
        remove_projects: vec![project_link],
        remove_people: vec![person_link],
    };
    super::profiles::execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let stored = sqlx::query!(
        "SELECT id, audit FROM permission_change_receipts WHERE request_id = $1",
        command.request_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    command.request_id = Uuid::now_v7();
    command.expected_access_revision = 1;
    command.expected_person_revision = 1;
    command.action = ProfileAction::BuiltIn {
        profile: BuiltInProfile::ProjectManager,
    };
    command.grants = BuiltInProfile::ProjectManager.selection().iter().collect();
    command.remove_projects.clear();
    command.remove_people.clear();
    super::profiles::execute(&pool, ids.org_id, ids.user_id, &command)
        .await
        .unwrap();
    let historical = serde_json::to_value(
        read(&pool, ids.org_id, ids.user_id, stored.id)
            .await
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(historical["audit"]["details"], stored.audit);
    let change = &historical["audit"]["details"]["change"];
    assert_eq!(
        change["removed_projects"],
        json!([{"id": project_link, "subject_id": ids.project_id, "revision": 0}])
    );
    assert_eq!(
        change["removed_people"],
        json!([{"id": person_link, "subject_id": ids.user_id, "revision": 0}])
    );
    assert_eq!(
        change["after"]["grants"],
        json!(BuiltInProfile::Member.selection())
    );
}

#[test]
fn historical_decoder_rejects_unknown_mixed_fields_and_invalid_revision_steps() {
    let base = json!({"project_id": Uuid::now_v7(), "previous_access_revision": 0, "access_revision": 0, "change": null});
    assert!(matches!(
        decode(2, base.clone()),
        Err(AuditReadError::UnsupportedFormat)
    ));
    for (field, value) in [
        ("extra", json!(true)),
        ("user_id", json!(Uuid::now_v7())),
        ("previous_access_revision", json!(-1)),
        ("access_revision", json!(1)),
        ("change", json!({"added": [], "removed": []})),
    ] {
        let mut bad = base.clone();
        bad[field] = value;
        assert!(
            matches!(decode(1, bad), Err(AuditReadError::InvalidDocument)),
            "{field}"
        );
    }
}

#[test]
fn historical_decoder_rejects_missing_template_snapshots_and_invalid_grants() {
    let snapshot = json!({"id": Uuid::now_v7(), "name": "Readers", "catalog_version": 1,
        "grants": BuiltInProfile::Member.selection(), "revision": 0});
    let base = json!({"previous_access_revision": 0, "access_revision": 1, "before": null,
        "after": snapshot, "detached_people": []});
    assert!(decode(1, base.clone()).is_ok());
    for field in ["before", "after"] {
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(matches!(
            decode(1, missing),
            Err(AuditReadError::InvalidDocument)
        ));
    }
    for (field, value) in [
        ("catalog_version", json!(99)),
        ("revision", json!(-1)),
        ("grants", json!([])),
        ("grants", json!(["unknown_permission"])),
        ("grants", json!(["project_write_all"])),
    ] {
        let mut bad = base.clone();
        bad["after"][field] = value;
        assert!(
            matches!(decode(1, bad), Err(AuditReadError::InvalidDocument)),
            "{field}"
        );
    }
}

#[test]
fn historical_person_snapshot_validates_provenance_without_inferring_admin_identity() {
    let person = json!({"grants": BuiltInProfile::Member.selection(), "is_administrator": true,
        "source": {"kind": "individual"}, "revision": 0});
    let mut after = person.clone();
    after["revision"] = json!(1);
    let base = json!({"user_id": Uuid::now_v7(), "previous_access_revision": 0, "access_revision": 1,
        "change": {"catalog_version": 1, "before": person, "after": after,
        "removed_projects": [], "removed_people": []}});
    assert!(decode(1, base.clone()).is_ok());
    for source in [
        json!({"kind": "unknown"}),
        json!({"kind": "template", "value": {"id": Uuid::now_v7(), "applied_revision": -1}}),
        json!({"kind": "individual", "extra": "private"}),
    ] {
        let mut bad = base.clone();
        bad["change"]["after"]["source"] = source;
        assert!(matches!(
            decode(1, bad),
            Err(AuditReadError::InvalidDocument)
        ));
    }
}
