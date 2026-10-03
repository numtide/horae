//! Own-access delivery shares the process's existing registered-route harness.

use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let cookie = api.cookie(ids.user_id).await;
    let name = "get_my_permissions";
    assert_eq!(
        api.call(name, json!({}), None, false).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(api.json(name, json!({}), &cookie).await, Value::Null);
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(BuiltInProfile::Member.selection()).unwrap())
            .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id, org_id, user_id, catalog_version, grants, is_administrator, source)
        VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let expected = json!({
        "catalog_version": 1,
        "grants": BuiltInProfile::Member.selection(),
        "is_administrator": false,
        "access_revision": 0,
        "person_revision": 0,
        "managed_person_ids": [],
        "managed_project_ids": [],
    });
    assert_eq!(api.json(name, json!({}), &cookie).await, expected);
    assert_eq!(
        api.json(
            name,
            json!({"user_id":foreign.user_id,"org_id":foreign.org_id,"is_administrator":true}),
            &cookie
        )
        .await,
        expected
    );
    sqlx::query!(
        "UPDATE person_permission_states SET catalog_version=2 WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let response = api.call(name, json!({}), Some(&cookie), false).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let catalog_error = response.text().await.unwrap();
    assert!(catalog_error.contains("Permission state is unavailable"));
    for private in ["catalog", "person_permission_states", "sqlx", "SELECT"] {
        assert!(!catalog_error.contains(private), "{catalog_error}");
    }
    sqlx::query!(
        "UPDATE person_permission_states SET catalog_version=1 WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(api.json(name, json!({}), &cookie).await, expected);
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['secret_invalid_grant'] WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let response = api.call(name, json!({}), Some(&cookie), false).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text().await.unwrap();
    assert!(body.contains("Permission state is unavailable"));
    for private in [
        "secret_invalid_grant",
        "person_permission_states",
        "sqlx",
        "SELECT",
    ] {
        assert!(!body.contains(private), "{body}");
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call(name, json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let expired = api.cookie(foreign.user_id).await;
    assert_eq!(
        api.client
            .post(format!("{}/test/expire", api.base))
            .header("cookie", &expired)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        api.call(name, json!({}), Some(&expired), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
