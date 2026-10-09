use super::*;
use crate::models::permission_editor::PermissionRequester;

fn requester(viewer: &User) -> PermissionRequester {
    PermissionRequester {
        org_id: viewer.org_id,
        user_id: viewer.id,
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn overview_resolves_only_visible_client_identity_without_directory_or_rates(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    designate(&pool, &ids).await;
    sqlx::query!(
        "UPDATE clients SET active=false, default_rate_cents=98765 WHERE id=$1",
        ids.client_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let other_client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id,org_id,name,currency) VALUES ($1,$2,'Hidden client','EUR')",
        other_client,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Hidden project','EUR')",
        Uuid::now_v7(), ids.org_id, other_client,
    ).execute(&pool).await.unwrap();

    let overview = fetch_project_overview(&pool, &viewer, None).await.unwrap();
    assert_eq!(overview.requester, requester(&viewer));
    assert!(overview.canonical_permissions);
    assert_eq!(overview.projects.len(), 1);
    let row = &overview.projects[0];
    assert_eq!(row.project.id, ids.project_id);
    let client = row.client.as_ref().unwrap();
    assert_eq!(
        (client.id, client.name.as_str(), client.active),
        (ids.client_id, "Acme", false)
    );
    assert!(!row.can_edit);
    let payload = serde_json::to_value(&overview).unwrap();
    let client_payload = payload["projects"][0]["client"].as_object().unwrap();
    assert_eq!(
        client_payload.len(),
        3,
        "only id, name and active are workflow identity"
    );
    assert!(!payload.to_string().contains("Hidden client"));
    assert!(!payload.to_string().contains("98765"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn overview_rejects_changed_requester_including_empty_results(pool: PgPool) {
    let (_, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    let initial = fetch_project_overview(&pool, &viewer, None).await.unwrap();
    assert!(initial.projects.is_empty());
    let same = fetch_project_overview(&pool, &viewer, Some(initial.requester))
        .await
        .unwrap();
    assert_eq!(same.requester, initial.requester);
    for expected in [
        PermissionRequester {
            user_id: Uuid::now_v7(),
            ..initial.requester
        },
        PermissionRequester {
            org_id: Uuid::now_v7(),
            ..initial.requester
        },
    ] {
        let error = fetch_project_overview(&pool, &viewer, Some(expected))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            }
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn overview_edit_affordance_requires_current_project_write_scope(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    for (grants, designated, editable) in [
        (vec![Permission::ProjectReadAll], false, false),
        (
            vec![Permission::ProjectReadAll, Permission::ProjectWriteManaged],
            false,
            false,
        ),
        (
            vec![Permission::ProjectReadAll, Permission::ProjectWriteManaged],
            true,
            true,
        ),
        (vec![Permission::ProjectReadAll], true, false),
        (vec![Permission::ProjectWriteAll], false, true),
    ] {
        replace_grants(&pool, &ids, &grants).await;
        sqlx::query!(
            "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
            ids.org_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        if designated {
            designate(&pool, &ids).await;
        }
        let overview = fetch_project_overview(&pool, &viewer, None).await.unwrap();
        assert_eq!(overview.projects.len(), 1);
        assert_eq!(
            overview.projects[0].can_edit, editable,
            "grants={grants:?}, designated={designated}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn overview_denies_unavailable_actor_instead_of_binding_an_empty_success(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let error = fetch_project_overview(&pool, &viewer, Some(requester(&viewer)))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_overview_uses_current_role_not_the_captured_user_role(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let viewer = User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: "reader@test.invalid".into(),
        name: "Reader".into(),
        oidc_subject: None,
        org_role: OrgRole::Manager,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    };
    let before = fetch_project_overview(&pool, &viewer, None).await.unwrap();
    assert!(!before.canonical_permissions);
    assert!(before.projects[0].can_edit);
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let after = fetch_project_overview(&pool, &viewer, Some(before.requester))
        .await
        .unwrap();
    assert_eq!(after.projects.len(), 1);
    assert!(!after.projects[0].can_edit);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn overview_observes_edit_revocation_after_waiting_for_organization(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let before = fetch_project_overview(&pool, &viewer, None).await.unwrap();
    assert!(before.projects[0].can_edit);
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        ids.org_id
    )
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
        fetch_project_overview(&pending_pool, &viewer, Some(before.requester)).await
    });
    crate::server_fns::test_seed::wait_for_blocked(&pool, holder).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::ProjectReadAll])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    hold.commit().await.unwrap();
    let after = pending.await.unwrap().unwrap();
    assert_eq!(after.projects.len(), 1);
    assert!(!after.projects[0].can_edit);
}
