//! Task creation through registered server functions and real session cookies.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    check_initial_rate(pool, api).await;
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let restricted = user(pool, ids.org_id, OrgRole::Admin).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    for (actor, permissions) in [
        (
            ids.user_id,
            vec![Permission::TaskWriteAll, Permission::ProjectWriteAll],
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
    let cookie = api.cookie(ids.user_id).await;
    let restricted_cookie = api.cookie(restricted).await;
    let requester = json!({"org_id":ids.org_id,"user_id":ids.user_id});
    let body = json!({"name":"  Created by current permissions  ","billable_default":true,"project_id":null,
        "rate":{"action":"preserve"},"expected_requester":requester});
    assert_eq!(
        api.call("create_task", body.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "create_task",
            {
                let mut restricted_body = body.clone();
                restricted_body["expected_requester"]["user_id"] = json!(restricted);
                restricted_body
            },
            Some(&restricted_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let task = api.json("create_task", body.clone(), &cookie).await;
    assert_eq!(task["org_id"], json!(ids.org_id));
    assert_eq!(task["name"], "Created by current permissions");
    assert!(task.get("default_rate_cents").is_none());

    let linked = api
        .json(
            "create_task",
            json!({"name":"New linked task", "billable_default":true,
        "project_id":ids.project_id,"rate":{"action":"preserve"},"expected_requester":requester}),
            &cookie,
        )
        .await;
    let linked_id: Uuid = serde_json::from_value(linked["id"].clone()).unwrap();
    assert!(linked.get("default_rate_cents").is_none());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM project_tasks WHERE project_id=$1 AND task_id=$2)",
            ids.project_id,
            linked_id
        )
        .fetch_one(pool)
        .await
        .unwrap(),
        Some(true)
    );
    assert_eq!(
        api.call(
            "create_task",
            json!({"name":"Foreign destination", "billable_default":true,
        "project_id":foreign.project_id,"rate":{"action":"preserve"},"expected_requester":requester}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let grants: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(&[Permission::TaskWriteAll])))
            .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(
            "create_task",
            json!({"name":"Revoked project editor", "billable_default":true,
        "project_id":ids.project_id,"rate":{"action":"preserve"},"expected_requester":requester}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let floor: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &floor
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call("create_task", body.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call("create_task", body, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", foreign.org_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        Some(1)
    );
}

async fn check_initial_rate(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let grants: Vec<String> =
        serde_json::from_value(json!(PermissionSelection::new(&[Permission::TaskWriteAll])))
            .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1,default_currency='EUR' WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let body = json!({
        "name":"Initial rate", "billable_default":true, "project_id":null,
        "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "rate":{"action":"set","amount_cents":0,"currency":"EUR"}
    });
    assert_eq!(
        api.call("create_task", body.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN,
        "Even an initial zero rate requires independent rate-write authority"
    );
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::TaskWriteAll,
        Permission::BillableRateWriteAll,
        Permission::ProjectWriteAll,
    ])))
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
    for (field, value, status) in [
        (
            "expected_requester",
            json!({"org_id":ids.org_id,"user_id":Uuid::now_v7()}),
            StatusCode::FORBIDDEN,
        ),
        (
            "expected_requester",
            json!({"org_id":Uuid::now_v7(),"user_id":ids.user_id}),
            StatusCode::FORBIDDEN,
        ),
        (
            "rate",
            json!({"action":"set","amount_cents":0,"currency":"USD"}),
            StatusCode::CONFLICT,
        ),
        (
            "rate",
            json!({"action":"set","amount_cents":-1,"currency":"EUR"}),
            StatusCode::BAD_REQUEST,
        ),
        ("project_id", json!(Uuid::now_v7()), StatusCode::NOT_FOUND),
    ] {
        let mut rejected = body.clone();
        rejected[field] = value;
        assert_eq!(
            api.call("create_task", rejected, Some(&cookie), false)
                .await
                .status(),
            status,
            "{field}"
        );
    }
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        Some(1),
        "Rejected creations leave no task"
    );

    let created = api.json("create_task", body.clone(), &cookie).await;
    assert_eq!(created["default_rate_cents"], json!(0));
    let id: Uuid = serde_json::from_value(created["id"].clone()).unwrap();
    let stored = sqlx::query!(
        "SELECT default_rate_cents,default_rate_currency FROM tasks WHERE id=$1",
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(stored.default_rate_cents, Some(0));
    assert_eq!(stored.default_rate_currency.as_deref(), Some("EUR"));

    let mut visible = body;
    visible["rate"]["amount_cents"] = json!(12345);
    let created = api.json("create_task", visible, &cookie).await;
    assert_eq!(created["default_rate_cents"], json!(12345));
}
