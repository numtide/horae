use super::canonical_fields::canonical_actor;
use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns::test_seed::SeedIds;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

fn context(ids: &SeedIds) -> ProjectEditorContext {
    ProjectEditorContext {
        project_id: ids.project_id,
        requester: PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        },
    }
}

async fn read(
    pool: &PgPool,
    ids: &SeedIds,
    context: ProjectEditorContext,
) -> Result<ProjectEditorCatalog, ServerFnError> {
    super::super::catalog::read(
        pool,
        ids.user_id,
        ids.org_id,
        context,
        &ProjectEditorCatalogSearch::default(),
        false,
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_withholds_defaults_without_their_financial_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE clients SET default_rate_cents=987654 WHERE id=$1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=876543, default_rate_currency='EUR' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let catalog = read(&pool, &ids, context(&ids)).await.unwrap();
    assert_eq!(catalog.context, context(&ids));
    assert_eq!(catalog.clients[0].id, ids.client_id);
    assert_eq!(catalog.tasks[0].id, ids.task_id);
    assert_eq!(catalog.clients[0].default_rate_cents, None);
    assert_eq!(catalog.tasks[0].default_rate_cents, None);
    assert_eq!(catalog.tasks[0].default_rate_currency, None);
    let payload = serde_json::to_string(&catalog).unwrap();
    assert!(!payload.contains("987654") && !payload.contains("876543"));
    assert!(!payload.contains("people") && !payload.contains("previous_code"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_requires_operation_authority_not_legacy_role_or_rate_grant(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[Permission::ProjectWriteAll, Permission::BillableRateReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=876543, default_rate_currency='EUR' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let catalog = read(&pool, &ids, context(&ids)).await.unwrap();
    assert_eq!(catalog.tasks[0].default_rate_cents, Some(876543));
    assert_eq!(
        catalog.tasks[0].default_rate_currency.as_deref(),
        Some("EUR")
    );
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::BillableRateReadAll])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, &ids, context(&ids)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_rejects_switched_requesters_and_out_of_scope_projects(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    for requester in [
        PermissionRequester {
            user_id: Uuid::now_v7(),
            ..context(&ids).requester
        },
        PermissionRequester {
            org_id: foreign.org_id,
            ..context(&ids).requester
        },
    ] {
        assert!(matches!(
            read(
                &pool,
                &ids,
                ProjectEditorContext {
                    requester,
                    ..context(&ids)
                }
            )
            .await,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ));
    }
    for project_id in [foreign.project_id, Uuid::now_v7()] {
        assert!(matches!(
            read(
                &pool,
                &ids,
                ProjectEditorContext {
                    project_id,
                    ..context(&ids)
                }
            )
            .await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
    assert!(read(&pool, &ids, context(&ids)).await.is_ok());
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        read(&pool, &ids, context(&ids)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_has_no_implicit_legacy_policy_fallback(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    assert!(matches!(
        read(&pool, &ids, context(&ids)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert!(
        load_creation_options(
            &pool,
            ids.user_id,
            ids.org_id,
            &CreationSearch::default(),
            false
        )
        .await
        .is_ok()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_managed_scope_needs_designation_but_never_opens_global_rates(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[
            Permission::ProjectWriteManaged,
            Permission::BillableRateReadManaged,
        ]),
    )
    .await;
    assert!(matches!(
        read(&pool, &ids, context(&ids)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id)
        .execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=876543, default_rate_currency='EUR' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let catalog = read(&pool, &ids, context(&ids)).await.unwrap();
    assert_eq!(catalog.tasks[0].default_rate_cents, None);
    assert_eq!(catalog.tasks[0].default_rate_currency, None);
    assert!(matches!(
        read(
            &pool,
            &ids,
            ProjectEditorContext {
                project_id: foreign.project_id,
                ..context(&ids)
            }
        )
        .await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE manager_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        read(&pool, &ids, context(&ids)).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_filters_candidates_before_bounded_literal_search_pages(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let mut expected = Vec::new();
    for _ in 0..51 {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO clients (id,org_id,name,currency) VALUES ($1,$2,'Équipe %_ 王','EUR')",
            id,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Équipe %_ 王')",
            id,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        expected.push(id);
    }
    for (client, task, active) in [
        (ids.client_id, ids.task_id, false),
        (foreign.client_id, foreign.task_id, true),
    ] {
        sqlx::query!(
            "UPDATE clients SET name='Équipe %_ 王', active=$2 WHERE id=$1",
            client,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE tasks SET name='Équipe %_ 王', active=$2 WHERE id=$1",
            task,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    expected.sort_unstable();
    let mut search = ProjectEditorCatalogSearch {
        clients: crate::models::project_creation::CatalogSearch {
            query: " %_ 王 ".into(),
            offset: 0,
        },
        tasks: crate::models::project_creation::CatalogSearch {
            query: " %_ 王 ".into(),
            offset: 0,
        },
    };
    let first = super::super::catalog::read(
        &pool,
        ids.user_id,
        ids.org_id,
        context(&ids),
        &search,
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        first
            .clients
            .iter()
            .map(|client| client.id)
            .collect::<Vec<_>>(),
        expected[..50]
    );
    assert_eq!(
        first.tasks.iter().map(|task| task.id).collect::<Vec<_>>(),
        expected[..50]
    );
    assert!(first.more_clients && first.more_tasks);
    search.clients.offset = 50;
    search.tasks.offset = 50;
    let last = super::super::catalog::read(
        &pool,
        ids.user_id,
        ids.org_id,
        context(&ids),
        &search,
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        last.clients
            .iter()
            .map(|client| client.id)
            .collect::<Vec<_>>(),
        expected[50..]
    );
    assert_eq!(
        last.tasks.iter().map(|task| task.id).collect::<Vec<_>>(),
        expected[50..]
    );
    assert!(!last.more_clients && !last.more_tasks);
    for invalid in [
        crate::models::project_creation::CatalogSearch {
            query: "\0".into(),
            offset: 0,
        },
        crate::models::project_creation::CatalogSearch {
            query: "王".repeat(101),
            offset: 0,
        },
        crate::models::project_creation::CatalogSearch {
            query: String::new(),
            offset: 10001,
        },
    ] {
        search.clients = invalid;
        assert!(matches!(
            super::super::catalog::read(
                &pool,
                ids.user_id,
                ids.org_id,
                context(&ids),
                &search,
                false
            )
            .await,
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_catalog_rechecks_revocation_after_waiting_for_the_organization(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    canonical_actor(
        &pool,
        &ids,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let mut revoke = pool.begin().await.unwrap();
    lock_organization(&mut revoke, ids.org_id, OrganizationLock::AccessChange)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(&[])).unwrap())
            .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&mut *revoke)
    .await
    .unwrap();
    let pending = {
        let pool = pool.clone();
        let context = context(&ids);
        tokio::spawn(async move {
            super::super::catalog::read(
                &pool,
                context.requester.user_id,
                context.requester.org_id,
                context,
                &ProjectEditorCatalogSearch::default(),
                false,
            )
            .await
        })
    };
    crate::server_fns::test_seed::wait_for_blocked(&pool, pid).await;
    revoke.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}
