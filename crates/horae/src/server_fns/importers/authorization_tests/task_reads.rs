//! Task catalog authority through registered functions and real session cookies.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let restricted = user(pool, ids.org_id, OrgRole::Admin).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    for (actor, permissions) in [
        (
            ids.user_id,
            vec![Permission::TaskReadAll, Permission::BillableRateReadAll],
        ),
        (restricted, vec![]),
    ] {
        let grants: Vec<String> =
            serde_json::from_value(json!(PermissionSelection::new(&permissions))).unwrap();
        sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
            VALUES ($1,$2,$3,1,$4,false,'individual')",
            Uuid::now_v7(), ids.org_id, actor, &grants).execute(pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let restricted_cookie = api.cookie(restricted).await;
    let foreign_cookie = api.cookie(foreign.user_id).await;
    for (endpoint, body) in [
        ("list_tasks", json!({})),
        ("list_tracking_tasks", json!({})),
        ("list_project_tasks", json!({"project_id":ids.project_id})),
    ] {
        assert_eq!(
            api.call(endpoint, body, None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let catalog = api.json("list_tasks", json!({}), &cookie).await;
    assert_eq!(catalog.as_array().unwrap().len(), 1);
    assert_eq!(catalog[0]["id"], json!(ids.task_id));
    assert_eq!(catalog[0]["default_rate_cents"], 999);
    let tracking = api.json("list_tracking_tasks", json!({}), &cookie).await;
    assert_eq!(tracking[0]["id"], json!(ids.task_id));
    assert!(tracking[0].get("default_rate_cents").is_none());
    assert_eq!(
        api.json("list_tasks", json!({}), &restricted_cookie).await,
        json!([])
    );
    let other = api.json("list_tasks", json!({}), &foreign_cookie).await;
    assert_eq!(other[0]["id"], json!(foreign.task_id));
    assert_eq!(
        api.json(
            "list_project_tasks",
            json!({"project_id":ids.project_id}),
            &foreign_cookie
        )
        .await,
        json!([])
    );

    let grants: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(&[Permission::TaskReadAll])))
            .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
    let catalog = api.json("list_tasks", json!({}), &cookie).await;
    assert_eq!(catalog[0]["id"], json!(ids.task_id));
    assert!(catalog[0].get("default_rate_cents").is_none());
    let floor: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &floor
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(api.json("list_tasks", json!({}), &cookie).await, json!([]));
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call("list_tasks", json!({}), Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
