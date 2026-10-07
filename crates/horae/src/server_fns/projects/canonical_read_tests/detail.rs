use super::*;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_contains_only_scoped_workflow_labels_and_current_edit_authority(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
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
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None)
        .await
        .unwrap();
    assert_eq!(view.project.id, ids.project_id);
    assert_eq!(view.requester.user_id, ids.user_id);
    assert!(view.canonical_permissions);
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
        !detail_view::fetch(&pool, &viewer, ids.project_id, Some(view.requester))
            .await
            .unwrap()
            .can_edit
    );
    designate(&pool, &ids).await;
    assert!(
        detail_view::fetch(&pool, &viewer, ids.project_id, Some(view.requester))
            .await
            .unwrap()
            .can_edit
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    assert!(
        !detail_view::fetch(&pool, &viewer, ids.project_id, Some(view.requester))
            .await
            .unwrap()
            .can_edit
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_shared_member_gets_own_identity_not_other_people_or_legacy_actions(
    pool: PgPool,
) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
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
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None)
        .await
        .unwrap();
    assert!(!view.can_edit);
    assert_eq!(
        view.team.iter().map(|person| person.id).collect::<Vec<_>>(),
        [ids.user_id]
    );
    assert!(view.project.task_rate_currency.is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_view_rejects_changed_requester_and_current_resource_denial(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None)
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
            detail_view::fetch(&pool, &viewer, ids.project_id, Some(expected)).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
    let foreign = seed(&pool, OrgRole::Admin).await;
    assert!(matches!(
        detail_view::fetch(&pool, &viewer, foreign.project_id, Some(view.requester)).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    replace_grants(&pool, &ids, &[]).await;
    assert!(matches!(
        detail_view::fetch(&pool, &viewer, ids.project_id, Some(view.requester)).await,
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
        detail_view::fetch(&pool, &viewer, ids.project_id, Some(view.requester)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}
