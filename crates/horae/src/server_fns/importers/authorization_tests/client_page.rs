//! Canonical Clients page context through registered functions and real session cookies.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let restricted = user(pool, ids.org_id, OrgRole::Admin).await;
    for (actor, permissions) in [
        (ids.user_id, vec![Permission::ClientReadAll]),
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
    let unbound_detail = json!({"client_id": ids.client_id, "expected_requester": null});
    for (endpoint, body) in [
        ("client_overview", json!({"expected_requester": null})),
        ("client_detail_view", unbound_detail.clone()),
    ] {
        assert_eq!(
            api.call(endpoint, body, None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    let page = api
        .json("client_overview", json!({"expected_requester": null}), &cookie)
        .await;
    let requester = json!({"org_id": ids.org_id, "user_id": ids.user_id});
    assert_eq!(page["requester"], requester);
    assert_eq!(page["can_read"], true);
    assert_eq!(page["can_manage"], false);
    assert_eq!(page["clients"][0]["client"]["id"], json!(ids.client_id));
    let bound = json!({"expected_requester": requester});
    assert_eq!(
        api.json("client_overview", bound.clone(), &cookie).await,
        page
    );
    let bound_detail = json!({"client_id": ids.client_id, "expected_requester": requester});
    let view = api
        .json("client_detail_view", bound_detail.clone(), &cookie)
        .await;
    assert_eq!(view["details"]["client"]["id"], json!(ids.client_id));
    assert_eq!(view["can_manage"], false);

    for (endpoint, body) in [
        ("client_overview", bound),
        ("client_detail_view", bound_detail),
    ] {
        assert_eq!(
            api.call(endpoint, body, Some(&restricted_cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN,
            "{endpoint} must not substitute another authenticated requester",
        );
    }
    let withheld = api
        .json(
            "client_overview",
            json!({"expected_requester": null}),
            &restricted_cookie,
        )
        .await;
    assert_eq!(withheld["can_read"], false);
    assert_eq!(withheld["clients"], json!([]));
    assert_eq!(
        api.call("client_detail_view", unbound_detail, Some(&restricted_cookie), false)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}
