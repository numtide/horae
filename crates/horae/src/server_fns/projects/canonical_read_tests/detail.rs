use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn detail_view_contains_only_scoped_workflow_labels_and_current_edit_authority(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let member = team::teammate(&pool, &ids).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert_eq!(view.project.id, ids.project_id);
    assert_eq!(view.requester.user_id, ids.user_id);
    assert!(!view.can_edit);
    assert_eq!(
        serde_json::to_value(&view.team).unwrap(),
        serde_json::json!([{"id":member,"name":"Project teammate"}])
    );
    assert_eq!(
        serde_json::to_value(&view.tasks).unwrap(),
        serde_json::json!([{"id":ids.task_id,"name":"Dev"}])
    );
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::ProjectWriteManaged],
    )
    .await;
    assert!(
        !detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today()
        )
        .await
        .unwrap()
        .can_edit
    );
    designate(&pool, &ids).await;
    assert!(
        detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today()
        )
        .await
        .unwrap()
        .can_edit
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    assert!(
        !detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today()
        )
        .await
        .unwrap()
        .can_edit
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn detail_view_shared_member_gets_own_identity_not_other_people_or_legacy_actions(
    pool: PgPool,
) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[]),
    )
    .await;
    team::teammate(&pool, &ids).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert!(!view.can_edit);
    assert_eq!(
        view.team.iter().map(|person| person.id).collect::<Vec<_>>(),
        [ids.user_id]
    );
    assert!(view.project.task_rate_currency.is_none());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn detail_view_rejects_changed_requester_and_current_resource_denial(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    for expected in [
        PermissionRequester {
            user_id: Uuid::now_v7(),
            ..view.requester
        },
        PermissionRequester {
            org_id: Uuid::now_v7(),
            ..view.requester
        },
    ] {
        assert!(matches!(
            detail_view::fetch(&pool, &viewer, ids.project_id, Some(expected), today()).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    assert!(matches!(
        detail_view::fetch(
            &pool,
            &viewer,
            foreign.project_id,
            Some(view.requester),
            today()
        )
        .await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    replace_grants(&pool, &ids, &[]).await;
    assert!(matches!(
        detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today()
        )
        .await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today()
        )
        .await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}

/// The seeded person as the session loads them.
pub(super) fn session_user(ids: &SeedIds) -> User {
    User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Test User".into(),
        oidc_subject: None,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn detail_view_identity_and_lifecycle_affordances_follow_the_profile(pool: PgPool) {
    // Archiving needs `ProjectWriteAll` here (nobody manages the project), and
    // the client link needs `ClientReadAll`; a Member has neither.
    for (profile, can_change_status) in [
        (BuiltInProfile::Administrator, true),
        (BuiltInProfile::ExecutiveManager, true),
        (BuiltInProfile::Member, false),
    ] {
        let ids = seed(&pool, profile).await;
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
            Uuid::now_v7(),
            ids.project_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let viewer = session_user(&ids);
        let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
            .await
            .unwrap();
        assert_eq!(view.client_id, ids.client_id, "{profile:?}");
        assert_eq!(
            view.project_type,
            ProjectType::TimeAndMaterials,
            "{profile:?}"
        );
        assert!(view.active, "{profile:?}");
        assert_eq!(view.can_open_client, can_change_status, "{profile:?}");
        assert_eq!(view.can_change_status, can_change_status, "{profile:?}");

        sqlx::query!(
            "UPDATE projects SET active=false, project_type='fixed_fee' WHERE id=$1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let archived = detail_view::fetch(
            &pool,
            &viewer,
            ids.project_id,
            Some(view.requester),
            today(),
        )
        .await
        .unwrap();
        assert!(!archived.active, "{profile:?}");
        assert_eq!(archived.project_type, ProjectType::FixedFee, "{profile:?}");
        assert_eq!(archived.can_change_status, can_change_status, "{profile:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn detail_view_status_follows_project_write_and_scopes_client_link(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::ExecutiveManager,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::ProjectWriteAll]),
    )
    .await;
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert!(view.can_edit);
    assert!(view.can_change_status);
    assert!(!view.can_open_client);

    // Managed-project writing covers only projects the viewer manages.
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::ProjectWriteManaged],
    )
    .await;
    let unmanaged = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert!(!unmanaged.can_edit && !unmanaged.can_change_status);
    super::designate(&pool, &ids).await;
    let managed = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert!(managed.can_edit && managed.can_change_status);
    assert_eq!(view.project.client_name, "Acme");
    assert_eq!(view.client_id, ids.client_id);

    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::ClientReadAll],
    )
    .await;
    let view = detail_view::fetch(
        &pool,
        &viewer,
        ids.project_id,
        Some(view.requester),
        today(),
    )
    .await
    .unwrap();
    assert!(view.can_open_client);
    assert!(!view.can_change_status && !view.can_edit);
}
