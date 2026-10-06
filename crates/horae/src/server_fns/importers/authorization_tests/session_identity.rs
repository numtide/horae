//! The account menu's identity response is not a financial or provider profile.

use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let manager = user(pool, ids.org_id, OrgRole::Manager).await;
    let member = user(pool, ids.org_id, OrgRole::Member).await;
    sqlx::query!(
        "UPDATE users SET oidc_subject = id::text, cost_rate_cents = 6000,
            billable_rate_cents = 10000 WHERE org_id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();

    assert_eq!(
        api.call("get_me", json!({}), None, false).await.status(),
        StatusCode::UNAUTHORIZED
    );
    for (actor, role) in [
        (ids.user_id, OrgRole::Admin),
        (manager, OrgRole::Manager),
        (member, OrgRole::Member),
    ] {
        let cookie = api.cookie(actor).await;
        let expected = json!({
            "id":actor,"org_id":ids.org_id,"name":"Test User",
            "email":format!("{actor}@test.com"),"org_role":role
        });
        for request in [
            json!({}),
            json!({"user_id":foreign.user_id,"org_id":foreign.org_id}),
        ] {
            assert_eq!(api.json("get_me", request, &cookie).await, expected);
        }
    }

    let cookie = api.cookie(ids.user_id).await;
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.json("get_me", json!({}), &cookie).await["org_role"],
        json!(OrgRole::Member)
    );
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call("get_me", json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call("logout", json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::OK
    );

    let cookie = api.cookie(member).await;
    assert_eq!(
        api.call("logout", json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        api.call("get_me", json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let missing = api.cookie(Uuid::now_v7()).await;
    assert_eq!(
        api.call("get_me", json!({}), Some(&missing), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
