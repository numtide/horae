//! Directory response boundaries through the registered session endpoint.

use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let manager = user(pool, ids.org_id, OrgRole::Manager).await;
    let member = user(pool, ids.org_id, OrgRole::Member).await;
    let archived = user(pool, ids.org_id, OrgRole::Member).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", archived)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET oidc_subject = id::text, cost_rate_cents = 6000,
            billable_rate_cents = 10000 WHERE org_id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();

    for include_inactive in [false, true] {
        let request = json!({"include_inactive":include_inactive});
        assert_eq!(
            api.call("list_users", request.clone(), None, false)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        for actor in [ids.user_id, manager, member, archived] {
            let cookie = api.cookie(actor).await;
            let response = api
                .call("list_users", request.clone(), Some(&cookie), false)
                .await;
            if actor == archived {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            } else if include_inactive && actor != ids.user_id {
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
            } else {
                assert_eq!(response.status(), StatusCode::OK);
                let rows: Vec<Value> = response.json().await.unwrap();
                let mut expected_ids = vec![ids.user_id, manager, member];
                if include_inactive {
                    expected_ids.push(archived);
                }
                expected_ids.sort();
                let mut actual_ids = Vec::new();
                for row in rows {
                    let mut keys: Vec<_> = row
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(String::as_str)
                        .collect();
                    keys.sort();
                    assert_eq!(keys, ["active", "email", "id", "name", "org_role"]);
                    let id: Uuid = serde_json::from_value(row["id"].clone()).unwrap();
                    assert_ne!(id, foreign.user_id);
                    assert_eq!(row["active"], json!(id != archived));
                    assert_eq!(row["email"], json!(format!("{id}@test.com")));
                    assert_eq!(row["name"], "Test User");
                    let role = if id == ids.user_id {
                        OrgRole::Admin
                    } else if id == manager {
                        OrgRole::Manager
                    } else {
                        OrgRole::Member
                    };
                    assert_eq!(row["org_role"], json!(role));
                    actual_ids.push(id);
                }
                actual_ids.sort();
                assert_eq!(actual_ids, expected_ids);
            }
        }
    }

    // Reusing a session must not retain its earlier inactive-directory access.
    let cookie = api.cookie(ids.user_id).await;
    let own = api.json("get_me", json!({}), &cookie).await;
    assert_eq!(own["cost_rate_cents"], 6000);
    assert_eq!(own["billable_rate_cents"], 10000);
    assert_eq!(own["oidc_subject"], ids.user_id.to_string());
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(
            "list_users",
            json!({"include_inactive":true}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}
