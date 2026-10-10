use super::project_permissions::{canonical, selection};
use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_uses_canonical_catalog_scope_for_count_list_and_direct_id(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::TaskReadAll]).await;
    walk(&app, "tasks", "is_active=true", &[ids.task_id]).await;
    let detail = page(&app, &format!("/harvest/v2/tasks/{}", ids.task_id)).await;
    assert_eq!(detail["id"], ids.task_id.to_string());
    assert!(detail.get("default_hourly_rate").is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_does_not_inherit_legacy_admin_or_tracking_identity(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    for query in ["", "?per_page=1&page=2", "?is_active=true"] {
        let body = page(&app, &format!("/harvest/v2/tasks{query}")).await;
        assert_eq!(body["total_entries"], 0);
        assert!(body["tasks"].as_array().unwrap().is_empty());
    }
    assert_eq!(
        request(&app, &format!("/harvest/v2/tasks/{}", ids.task_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_rechecks_global_rate_access_independently_of_catalog(pool: PgPool) {
    let (ids, app) = canonical(
        &pool,
        OrgRole::Admin,
        &[Permission::TaskReadAll, Permission::BillableRateReadAll],
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let uri = format!("/harvest/v2/tasks/{}", ids.task_id);
    assert_eq!(page(&app, &uri).await["default_hourly_rate"], 9.99);
    selection(
        &pool,
        ids.user_id,
        &[Permission::TaskReadAll, Permission::BillableRateReadManaged],
    )
    .await;
    assert!(page(&app, &uri).await.get("default_hourly_rate").is_none());
    let body = page(&app, "/harvest/v2/tasks").await;
    assert_eq!(body["total_entries"], 1);
    assert!(body["tasks"][0].get("default_hourly_rate").is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_filters_archived_tasks_and_preserves_total_past_last_page(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::TaskReadAll]).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    walk(&app, "tasks", "is_active=false", &[ids.task_id]).await;
    let empty = page(&app, "/harvest/v2/tasks?is_active=true").await;
    assert_eq!(empty["total_entries"], 0);
    let beyond = page(&app, "/harvest/v2/tasks?is_active=false&per_page=1&page=2").await;
    assert_eq!(beyond["total_entries"], 1);
    assert!(beyond["tasks"].as_array().unwrap().is_empty());
    assert_eq!(
        request(&app, &format!("/harvest/v2/tasks/{}", foreign.task_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_rejects_unknown_policy_before_empty_or_direct_results(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 99 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for uri in [
        "/harvest/v2/tasks?is_active=false".to_owned(),
        format!("/harvest/v2/tasks/{}", ids.task_id),
    ] {
        assert_eq!(request(&app, &uri).await.0, StatusCode::FORBIDDEN);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_rechecks_catalog_and_financial_grants_after_authority_wait(pool: PgPool) {
    for detail in [false, true] {
        for financial_only in [false, true] {
            let (ids, app) = canonical(
                &pool,
                OrgRole::Admin,
                &[Permission::TaskReadAll, Permission::BillableRateReadAll],
            )
            .await;
            sqlx::query!(
                "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
                ids.task_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let uri = if detail {
                format!("/harvest/v2/tasks/{}", ids.task_id)
            } else {
                "/harvest/v2/tasks".to_owned()
            };
            let mut hold = pool.begin().await.unwrap();
            sqlx::query!(
                "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
                ids.org_id
            )
            .execute(&mut *hold)
            .await
            .unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *hold)
                .await
                .unwrap();
            let pending = tokio::spawn(async move { request(&app, &uri).await });
            crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
            let remaining = if financial_only {
                vec![Permission::TaskReadAll]
            } else {
                vec![]
            };
            let grants: Vec<String> = serde_json::from_value(
                serde_json::to_value(PermissionSelection::new(&remaining)).unwrap(),
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
            let (status, body) = pending.await.unwrap();
            if detail && !financial_only {
                assert_eq!(status, StatusCode::NOT_FOUND);
                continue;
            }
            assert_eq!(status, StatusCode::OK, "{body}");
            let body: Value = serde_json::from_str(&body).unwrap();
            if detail {
                assert!(body.get("default_hourly_rate").is_none());
            } else {
                assert_eq!(body["total_entries"], usize::from(financial_only));
                assert_eq!(
                    body["tasks"].as_array().unwrap().len(),
                    usize::from(financial_only)
                );
                if financial_only {
                    assert!(body["tasks"][0].get("default_hourly_rate").is_none());
                }
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_rechecks_deactivation_after_session_lookup(pool: PgPool) {
    for detail in [false, true] {
        let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::TaskReadAll]).await;
        let uri = if detail {
            format!("/harvest/v2/tasks/{}", ids.task_id)
        } else {
            "/harvest/v2/tasks".to_owned()
        };
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *hold)
            .await
            .unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *hold)
            .await
            .unwrap();
        let pending = tokio::spawn(async move { request(&app, &uri).await });
        crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
        hold.commit().await.unwrap();
        assert_eq!(pending.await.unwrap().0, StatusCode::FORBIDDEN);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_api_rejects_malformed_canonical_state_without_legacy_fallback(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[Permission::TaskReadAll]).await;
    sqlx::query!(
        "UPDATE person_permission_states SET catalog_version=999 WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for uri in [
        "/harvest/v2/tasks?is_active=false".to_owned(),
        format!("/harvest/v2/tasks/{}", ids.task_id),
    ] {
        assert_eq!(request(&app, &uri).await.0, StatusCode::FORBIDDEN);
    }
}
