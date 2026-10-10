use super::*;
use crate::server_fns::test_seed::wait_for_blocked;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_api_refreshes_scope_and_money_after_authority_wait(pool: PgPool) {
    for detail in [false, true] {
        for financial_only in [false, true] {
            let (ids, app) = canonical(
                &pool,
                OrgRole::Admin,
                &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
            )
            .await;
            let uri = if detail {
                format!("/harvest/v2/projects/{}", ids.project_id)
            } else {
                "/harvest/v2/projects".to_owned()
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
            wait_for_blocked(&pool, blocker).await;
            let permissions = if financial_only {
                vec![Permission::ProjectReadAll]
            } else {
                vec![]
            };
            let grants: Vec<String> = serde_json::from_value(
                serde_json::to_value(PermissionSelection::new(&permissions)).unwrap(),
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
                assert!(body["budget"].is_null());
            } else {
                assert_eq!(body["total_entries"], usize::from(financial_only));
                assert_eq!(
                    body["projects"].as_array().unwrap().len(),
                    usize::from(financial_only)
                );
                if financial_only {
                    assert!(body["projects"][0]["budget"].is_null());
                }
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_api_rejects_actor_deactivated_after_session_lookup(pool: PgPool) {
    for detail in [false, true] {
        let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
        let uri = if detail {
            format!("/harvest/v2/projects/{}", ids.project_id)
        } else {
            "/harvest/v2/projects".to_owned()
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
        wait_for_blocked(&pool, blocker).await;
        hold.commit().await.unwrap();
        assert_eq!(pending.await.unwrap().0, StatusCode::FORBIDDEN);
    }
}
