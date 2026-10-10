use super::project_permissions::{canonical, selection};
use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_allows_read_only_grants_without_legacy_management(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ClientReadAll]).await;
    walk(&app, "clients", "is_active=true", &[ids.client_id]).await;
    let detail = page(&app, &format!("/harvest/v2/clients/{}", ids.client_id)).await;
    assert_eq!(detail["id"], ids.client_id.to_string());
    assert_eq!(detail["name"], "Acme");
    for field in [
        "tax_id",
        "default_rate_cents",
        "default_hourly_rate",
        "projects",
        "invoices",
    ] {
        assert!(detail.get(field).is_none(), "Unexpected field: {field}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_does_not_inherit_legacy_role_or_related_workflow_authority(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    for permissions in [
        vec![],
        vec![Permission::ProjectReadAll],
        vec![Permission::InvoiceReadAll],
        vec![Permission::BillableRateReadAll],
    ] {
        selection(&pool, ids.user_id, &permissions).await;
        for query in ["", "?per_page=1&page=2", "?is_active=true"] {
            let body = page(&app, &format!("/harvest/v2/clients{query}")).await;
            assert_eq!(body["total_entries"], 0);
            assert!(body["clients"].as_array().unwrap().is_empty());
        }
        assert_eq!(
            request(&app, &format!("/harvest/v2/clients/{}", ids.client_id))
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_preserves_filters_counts_and_foreign_id_denial(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ClientReadAll]).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!("UPDATE clients SET active=false WHERE id=$1", ids.client_id)
        .execute(&pool)
        .await
        .unwrap();
    walk(&app, "clients", "is_active=false", &[ids.client_id]).await;
    for query in ["is_active=true", "updated_since=2100-01-01T00%3A00%3A00Z"] {
        assert_eq!(
            page(&app, &format!("/harvest/v2/clients?{query}")).await["total_entries"],
            0
        );
    }
    let beyond = page(
        &app,
        "/harvest/v2/clients?is_active=false&per_page=1&page=2",
    )
    .await;
    assert_eq!(beyond["total_entries"], 1);
    assert!(beyond["clients"].as_array().unwrap().is_empty());
    for id in [foreign.client_id, Uuid::now_v7()] {
        assert_eq!(
            request(&app, &format!("/harvest/v2/clients/{id}")).await.0,
            StatusCode::NOT_FOUND
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_rejects_unavailable_policy_and_inactive_sessions(pool: PgPool) {
    for state in ["unknown_policy", "missing", "malformed", "inactive"] {
        let (ids, app) = canonical(&pool, OrgRole::Admin, &[Permission::ClientReadAll]).await;
        match state {
            "unknown_policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version=99 WHERE id=$1",
                    ids.org_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "malformed" => {
                sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown_permission'] WHERE user_id=$1", ids.user_id)
                    .execute(&pool).await.unwrap();
            }
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        for uri in [
            "/harvest/v2/clients?is_active=false".to_owned(),
            format!("/harvest/v2/clients/{}", ids.client_id),
        ] {
            let (status, body) = request(&app, &uri).await;
            assert_eq!(
                status,
                if state == "inactive" {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::FORBIDDEN
                },
                "{state}: {body}"
            );
            assert!(!body.contains("Acme"));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_observes_revocation_after_waiting_for_current_authority(pool: PgPool) {
    for detail in [false, true] {
        let (ids, app) = canonical(&pool, OrgRole::Admin, &[Permission::ClientReadAll]).await;
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
        let uri = if detail {
            format!("/harvest/v2/clients/{}", ids.client_id)
        } else {
            "/harvest/v2/clients".to_owned()
        };
        let pending = tokio::spawn(async move { request(&app, &uri).await });
        crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
        let grants: Vec<String> =
            serde_json::from_value(serde_json::json!(PermissionSelection::new(&[]))).unwrap();
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
        assert_eq!(
            status,
            if detail {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            },
            "{body}"
        );
        assert!(!body.contains("Acme"));
        if !detail {
            let body: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(body["total_entries"], 0);
            assert!(body["clients"].as_array().unwrap().is_empty());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn client_api_rechecks_activity_after_session_authentication(pool: PgPool) {
    for detail in [false, true] {
        let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ClientReadAll]).await;
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *hold)
            .await
            .unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *hold)
            .await
            .unwrap();
        let uri = if detail {
            format!("/harvest/v2/clients/{}", ids.client_id)
        } else {
            "/harvest/v2/clients".to_owned()
        };
        let pending = tokio::spawn(async move { request(&app, &uri).await });
        crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
        hold.commit().await.unwrap();
        let (status, body) = pending.await.unwrap();
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(!body.contains("Acme"));
    }
}
