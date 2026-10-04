use super::*;
use crate::server_fns::test_seed::seed;
use horae_core::permissions::catalog::{
    BuiltInProfile, Permission, PermissionSelection, validate_template_name,
};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

fn grant_ids(selection: &PermissionSelection) -> Vec<String> {
    selection
        .iter()
        .map(|grant| {
            serde_json::to_value(grant)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

async fn template(pool: &PgPool, org: Uuid, name: &str) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    let grants = grant_ids(&BuiltInProfile::Member.selection());
    sqlx::query!(
        "INSERT INTO permission_templates (id, org_id, name, catalog_version, grants)
         VALUES ($1, $2, $3, 1, $4)",
        id,
        org,
        name,
        &grants
    )
    .execute(pool)
    .await?;
    Ok(id)
}

async fn person(pool: &PgPool, org: Uuid, user: Uuid) -> Result<(), sqlx::Error> {
    let grants = grant_ids(&BuiltInProfile::Member.selection());
    sqlx::query!(
        "INSERT INTO person_permission_states
         (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, 1, $4, false, 'individual')",
        Uuid::now_v7(),
        org,
        user,
        &grants
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn constraint(error: sqlx::Error, expected: &str) {
    let sqlx::Error::Database(error) = error else {
        panic!("unexpected error: {error}")
    };
    assert_eq!(error.code().as_deref(), Some(expected));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn storage_does_not_activate_or_infer_legacy_roles(pool: PgPool) {
    for role in [OrgRole::Admin, OrgRole::Manager, OrgRole::Member] {
        let ids = seed(&pool, role).await;
        let row = sqlx::query!(
            "SELECT permission_policy_version, access_revision FROM organizations WHERE id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((row.permission_policy_version, row.access_revision), (0, 0));
        assert!(
            load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn additive_migration_preserves_populated_legacy_records(pool: PgPool) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|migration| migration.version < 42)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Manager).await;
    let membership = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role, rate_cents)
         VALUES ($1, $2, $3, 'lead', 12345)",
        membership,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = sqlx::query_scalar!(
        "SELECT jsonb_build_object('users', (SELECT jsonb_agg(u) FROM users u),
         'assignments', (SELECT jsonb_agg(a) FROM assignments a),
         'clients', (SELECT jsonb_agg(c) FROM clients c),
         'projects', (SELECT jsonb_agg(p) FROM projects p),
         'tasks', (SELECT jsonb_agg(t) FROM tasks t))"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let after = sqlx::query_scalar!(
        "SELECT jsonb_build_object('users', (SELECT jsonb_agg(u) FROM users u),
         'assignments', (SELECT jsonb_agg(a) FROM assignments a),
         'clients', (SELECT jsonb_agg(c) FROM clients c),
         'projects', (SELECT jsonb_agg(p) FROM projects p),
         'tasks', (SELECT jsonb_agg(t) FROM tasks t))"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, before);
    let row = sqlx::query!(
        "SELECT permission_policy_version, access_revision,
         (SELECT count(*) FROM permission_templates) AS templates,
         (SELECT count(*) FROM person_permission_states) AS people
         FROM organizations WHERE id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            row.permission_policy_version,
            row.access_revision,
            row.templates,
            row.people
        ),
        (0, 0, Some(0), Some(0))
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_equivalent_names_create_only_one_template(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let (first, second) = tokio::join!(
        template(&pool, ids.org_id, "Equipo"),
        template(
            &pool,
            ids.org_id,
            validate_template_name(" equipo ").unwrap()
        )
    );
    match (first, second) {
        (Ok(_), Err(error)) | (Err(error), Ok(_)) => constraint(error, "23505"),
        results => panic!("exactly one insert must succeed: {results:?}"),
    }
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_templates WHERE org_id = $1",
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, Some(1));
    assert!(
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn person_grants_and_identity_round_trip_independently_of_source(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    person(&pool, ids.org_id, ids.user_id).await.unwrap();
    for profile in BuiltInProfile::ALL {
        let mut adjusted = profile.selection();
        adjusted.add(Permission::InvoiceWriteAll);
        for selection in [profile.selection(), adjusted] {
            let grants = grant_ids(&selection);
            let key = serde_json::to_value(profile)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            for administrator in [false, true] {
                sqlx::query!(
                    "UPDATE person_permission_states SET grants = $1, is_administrator = $2,
                 source = 'built_in', built_in_profile = $3 WHERE user_id = $4",
                    &grants,
                    administrator,
                    key,
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
                let state = load_person_permissions(
                    &mut pool.acquire().await.unwrap(),
                    ids.org_id,
                    ids.user_id,
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(state.grants, selection);
                assert_eq!(state.is_administrator, administrator);
                assert_eq!(state.source, PermissionSource::BuiltIn(*profile));
                assert_eq!(state.revision, 0);
            }
        }
    }
}

async fn legacy_template_storage(pool: &PgPool) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|migration| migration.version < 47)
            .cloned()
            .collect(),
    );
    previous.run(pool).await.unwrap();
    // Reproduce C-locale installations even when the test cluster uses Unicode.
    sqlx::query!("ALTER TABLE permission_templates ALTER COLUMN name TYPE text COLLATE \"C\"")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn unicode_name_migration_preserves_existing_templates(pool: PgPool) {
    legacy_template_storage(&pool).await;
    let ids = seed(&pool, OrgRole::Member).await;
    let id = template(&pool, ids.org_id, "Ágil").await.unwrap();
    let before = load_permission_template(&mut pool.acquire().await.unwrap(), ids.org_id, id)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let after = load_permission_template(&mut pool.acquire().await.unwrap(), ids.org_id, id)
        .await
        .unwrap();
    assert_eq!(after, before);
    constraint(
        template(&pool, ids.org_id, "ágil").await.unwrap_err(),
        "23505",
    );
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn unicode_name_collision_rolls_back_migration_without_changing_profiles(pool: PgPool) {
    legacy_template_storage(&pool).await;
    let ids = seed(&pool, OrgRole::Member).await;
    let first = template(&pool, ids.org_id, "Ágil").await.unwrap();
    let second = template(&pool, ids.org_id, "ágil").await.unwrap();
    let migrator = sqlx::migrate!("./migrations");
    let error = migrator.run(&pool).await.unwrap_err();
    let sqlx::migrate::MigrateError::ExecuteMigration(error, 47) = error else {
        panic!("unexpected migration error: {error}");
    };
    constraint(error, "23505");
    for (id, name) in [(first, "Ágil"), (second, "ágil")] {
        let row = load_permission_template(&mut pool.acquire().await.unwrap(), ids.org_id, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.name, name);
        assert_eq!(row.grants, BuiltInProfile::Member.selection());
        assert_eq!(row.revision, 0);
    }
    // The old unique index must still protect the table after rollback.
    template(&pool, ids.org_id, "Equipo").await.unwrap();
    constraint(
        template(&pool, ids.org_id, "equipo").await.unwrap_err(),
        "23505",
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn template_names_are_case_insensitive_and_tenant_local(pool: PgPool) {
    let local = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Member).await;
    for (first, second) in [("Equipo", "equipo"), ("Ágil", "ágil"), ("Роль", "роль")] {
        let id = template(&pool, local.org_id, first).await.unwrap();
        constraint(
            template(&pool, local.org_id, second).await.unwrap_err(),
            "23505",
        );
        template(&pool, foreign.org_id, second).await.unwrap();
        let row = load_permission_template(&mut pool.acquire().await.unwrap(), local.org_id, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.name, first);
        assert_eq!(row.revision, 0);
        assert_eq!(row.grants, BuiltInProfile::Member.selection());
        assert!(
            load_permission_template(&mut pool.acquire().await.unwrap(), foreign.org_id, id)
                .await
                .unwrap()
                .is_none()
        );
    }
    // Only case is folded: accents and Unicode normalization forms remain labels.
    template(&pool, local.org_id, "Agil").await.unwrap();
    template(&pool, local.org_id, "A\u{301}gil").await.unwrap();
    for invalid in [String::new(), "x".repeat(101)] {
        constraint(
            template(&pool, local.org_id, &invalid).await.unwrap_err(),
            "23514",
        );
    }
    template(&pool, local.org_id, &"á".repeat(100))
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn person_state_rejects_duplicates_and_foreign_users_and_templates(pool: PgPool) {
    let local = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Member).await;
    person(&pool, local.org_id, local.user_id).await.unwrap();
    constraint(
        person(&pool, local.org_id, local.user_id)
            .await
            .unwrap_err(),
        "23505",
    );
    constraint(
        person(&pool, local.org_id, foreign.user_id)
            .await
            .unwrap_err(),
        "23503",
    );
    let id = template(&pool, foreign.org_id, "Other").await.unwrap();
    let error = sqlx::query!(
        "UPDATE person_permission_states SET source = 'template', template_id = $1,
         applied_template_revision = 0 WHERE user_id = $2",
        id,
        local.user_id
    )
    .execute(&pool)
    .await
    .unwrap_err();
    constraint(error, "23503");
    assert!(
        load_person_permissions(
            &mut pool.acquire().await.unwrap(),
            foreign.org_id,
            local.user_id
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn malformed_stored_grants_fail_without_repair(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    person(&pool, ids.org_id, ids.user_id).await.unwrap();
    let template_id = template(&pool, ids.org_id, "Team").await.unwrap();
    let valid = grant_ids(&BuiltInProfile::Member.selection());
    let mut unknown = valid.clone();
    unknown.push("unknown_grant".into());
    let mut duplicate = valid.clone();
    duplicate.push(valid[0].clone());
    let mut incomplete = valid.clone();
    incomplete.push("invoice_write_all".into());
    for (version, grants) in [
        (1, unknown),
        (1, duplicate),
        (1, incomplete),
        (1, vec![]),
        (2, valid),
    ] {
        sqlx::query!("UPDATE person_permission_states SET catalog_version = $1, grants = $2 WHERE user_id = $3", version, &grants, ids.user_id).execute(&pool).await.unwrap();
        sqlx::query!(
            "UPDATE permission_templates SET catalog_version = $1, grants = $2 WHERE id = $3",
            version,
            &grants,
            template_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
                .await
                .is_err()
        );
        assert!(
            load_permission_template(&mut pool.acquire().await.unwrap(), ids.org_id, template_id)
                .await
                .is_err()
        );
        let persisted = sqlx::query_scalar!(
            "SELECT grants FROM person_permission_states WHERE user_id = $1",
            ids.user_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(persisted, grants);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn source_shape_and_revision_constraints_reject_invalid_rows(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    person(&pool, ids.org_id, ids.user_id).await.unwrap();
    for (source, builtin, template, applied, revision) in [
        ("unknown", None, None, None, 0_i64),
        ("built_in", None, None, None, 0),
        ("built_in", Some("unknown"), None, None, 0),
        ("individual", Some("member"), None, None, 0),
        ("individual", None, None, Some(0_i64), 0),
        ("template", None, None, Some(0), 0),
        ("template", None, Some(Uuid::now_v7()), None, 0),
        ("individual", None, None, None, -1),
    ] {
        let error = sqlx::query!(
            "UPDATE person_permission_states SET source = $1, built_in_profile = $2,
             template_id = $3, applied_template_revision = $4, revision = $5 WHERE user_id = $6",
            source,
            builtin,
            template,
            applied,
            revision,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap_err();
        constraint(error, "23514");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn referenced_template_deletion_requires_detaching_without_grant_or_identity_loss(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Member).await;
    person(&pool, ids.org_id, ids.user_id).await.unwrap();
    let id = template(&pool, ids.org_id, "Administrator").await.unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET source = 'template', template_id = $1,
         applied_template_revision = 0, is_administrator = true WHERE user_id = $2",
        id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before =
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        before.source,
        PermissionSource::Template {
            id,
            applied_revision: 0
        }
    );
    constraint(
        sqlx::query!("DELETE FROM permission_templates WHERE id = $1", id)
            .execute(&pool)
            .await
            .unwrap_err(),
        "23503",
    );
    // Fixture-only detachment tests storage preservation, not an authorized command or its audit.
    sqlx::query!(
        "UPDATE person_permission_states SET source = 'individual', template_id = NULL,
         applied_template_revision = NULL WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("DELETE FROM permission_templates WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();
    let replacement = template(&pool, ids.org_id, "Administrator").await.unwrap();
    assert_ne!(replacement, id);
    let after =
        load_person_permissions(&mut pool.acquire().await.unwrap(), ids.org_id, ids.user_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(after.source, PermissionSource::Individual);
    assert_eq!(after.grants, before.grants);
    assert_eq!(after.is_administrator, before.is_administrator);
}
