//! Directory response boundaries through the registered session endpoint.

use super::*;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    check_scoped(pool, api).await;
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

async fn check_scoped(pool: &PgPool, api: &Api) {
    use horae_core::permissions::catalog::{BuiltInProfile, Permission, PermissionSelection};

    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    let cookie = api.cookie(ids.user_id).await;
    let request = json!({"activity":"all","after":null});
    assert_eq!(
        api.call("list_people", request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call("list_people", request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::PeopleReadManaged
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
    sqlx::query!(
        "INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id)
        VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        target
    )
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
    let expected = json!({
        "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "people":[{"id":target,"name":"Test User","email":format!("{target}@test.com"),"active":true}],
        "next_after":null
    });
    assert_eq!(
        api.json("list_people", request.clone(), &cookie).await,
        expected
    );
    // Caller-supplied authority must not select a foreign account or broaden scope.
    assert_eq!(
        api.json(
            "list_people",
            json!({"activity":"all","after":null,
        "org_id":foreign.org_id,"user_id":foreign.user_id,"is_administrator":true}),
            &cookie
        )
        .await,
        expected
    );
    assert_eq!(
        api.call(
            "list_people",
            json!({"activity":"all","after":{"name":"bad\0name","id":target}}),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", target)
        .execute(pool)
        .await
        .unwrap();
    let archived = api
        .json(
            "list_people",
            json!({"activity":"archived","after":null}),
            &cookie,
        )
        .await;
    assert_eq!(archived["people"][0]["active"], false);
    assert_eq!(archived["people"][0]["id"], json!(target));
    assert_eq!(
        api.json(
            "list_people",
            json!({"activity":"active","after":null}),
            &cookie
        )
        .await["people"],
        json!([])
    );

    let revoked: Vec<String> =
        serde_json::from_value(json!(BuiltInProfile::Member.selection())).unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &revoked
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call("list_people", request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1", ids.user_id).execute(pool).await.unwrap();
    let response = api
        .call("list_people", request.clone(), Some(&cookie), false)
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text().await.unwrap();
    assert!(body.contains("People directory is unavailable"));
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
        api.call("list_people", request, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
