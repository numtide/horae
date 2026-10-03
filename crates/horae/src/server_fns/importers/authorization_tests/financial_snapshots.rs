//! Snapshot readers derive authority from the actual session, not JSON IDs.

use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Manager).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET project_type='fixed_fee', starts_on='2026-09-01' WHERE id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,fee_mode,fee_amount_cents) VALUES ($1,$2,$3,$4,'person','single',12500)",
        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id
    ).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let member = api
        .cookie(user(pool, ids.org_id, OrgRole::Member).await)
        .await;
    for name in ["prepare_invoice", "get_project_fee_balances"] {
        let body = json!({
            "client_id": ids.client_id, "project_id": ids.project_id,
            "period_from": "2026-09-01", "period_to": "2026-09-30",
            "project_ids": null, "overrides": null, "fees": null,
            "actor_id": foreign.user_id, "org_id": foreign.org_id,
            "org_role": "admin"
        });
        assert_eq!(
            api.call(name, body.clone(), None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            api.call(name, body.clone(), Some(&member), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        let result = api.json(name, body.clone(), &cookie).await;
        let amount = if name == "prepare_invoice" {
            &result["subtotal_cents"]
        } else {
            &result[0]["balance"]["remaining_cents"]
        };
        assert_eq!(amount, &json!(12500), "{name}: {result}");
        let mut foreign_target = body.clone();
        foreign_target["client_id"] = json!(foreign.client_id);
        foreign_target["project_id"] = json!(foreign.project_id);
        let response = api.call(name, foreign_target, Some(&cookie), false).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let error = response.text().await.unwrap();
        for field in ["subtotal_cents", "remaining_cents", "fee_amount_cents"] {
            assert!(!error.contains(field), "{name}: {error}");
        }
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(pool)
            .await
            .unwrap();
        assert_eq!(
            api.call(name, body, Some(&cookie), false).await.status(),
            StatusCode::UNAUTHORIZED
        );
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
            .execute(pool)
            .await
            .unwrap();
    }
}
