//! Identity-only project choices through the registered session endpoint.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    let cookie = api.cookie(ids.user_id).await;
    let request =
        json!({"context":{"kind":"create"},"query":{"kind":"search","query":"","after":null}});
    assert_eq!(
        api.call("project_people", request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call("project_people", request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::ProjectCreateAll
    ])))
    .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET oidc_subject=id::text, cost_rate_cents=6000, billable_rate_cents=10000 WHERE org_id=$1", ids.org_id).execute(pool).await.unwrap();
    let expected = json!({
        "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "people":[{"id":ids.user_id,"name":"Test User"},{"id":target,"name":"Test User"}],
        "next_after":null
    });
    assert_eq!(
        api.json("project_people", request.clone(), &cookie).await,
        expected
    );
    let mut forged = request.clone();
    forged["org_id"] = json!(foreign.org_id);
    forged["user_id"] = json!(foreign.user_id);
    forged["is_administrator"] = json!(true);
    assert_eq!(api.json("project_people", forged, &cookie).await, expected);
    let resolve = json!({"context":{"kind":"create"},"query":{"kind":"resolve","ids":[target,foreign.user_id,target]}});
    assert_eq!(
        api.json("project_people", resolve.clone(), &cookie).await["people"],
        json!([{"id":target,"name":"Test User"}])
    );
    for project in [ids.project_id, foreign.project_id] {
        assert_eq!(api.call("project_people", json!({"context":{"kind":"edit","project_id":project},"query":{"kind":"search","query":"","after":null}}), Some(&cookie), false).await.status(), StatusCode::FORBIDDEN);
    }
    assert_eq!(api.call("project_people", json!({"context":{"kind":"create"},"query":{"kind":"search","query":"\0","after":null}}), Some(&cookie), false).await.status(), StatusCode::BAD_REQUEST);
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.json("project_people", resolve.clone(), &cookie).await["people"],
        json!([])
    );
    let revoked: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &revoked
    )
    .execute(pool)
    .await
    .unwrap();
    for query in [&request, &resolve] {
        assert_eq!(
            api.call("project_people", query.clone(), Some(&cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1", ids.user_id).execute(pool).await.unwrap();
    let response = api
        .call("project_people", request.clone(), Some(&cookie), false)
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text().await.unwrap();
    assert!(body.contains("Project people are unavailable"));
    for private in [
        "private_invalid_grant",
        "person_permission_states",
        "SELECT",
        "sqlx",
    ] {
        assert!(!body.contains(private), "{body}");
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call("project_people", request, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
