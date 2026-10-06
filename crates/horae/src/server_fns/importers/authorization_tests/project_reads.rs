//! Project read scope and protected payloads through real session endpoints.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let outsider = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let restricted = user(pool, ids.org_id, OrgRole::Admin).await;
    let teammate = user(pool, ids.org_id, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,rate_cents) VALUES ($1,$2,$3,76543)",
        Uuid::now_v7(),
        ids.project_id,
        teammate,
    )
    .execute(pool)
    .await
    .unwrap();
    // The legacy fee child must not use a different authenticated account than
    // the detail page that mounted it. This does not activate canonical fee scope.
    let legacy_cookie = api.cookie(restricted).await;
    let other_cookie = api.cookie(outsider.user_id).await;
    let fee_request = json!({
        "project_id": ids.project_id,
        "period_from": "2026-10-01", "period_to": "2026-10-31",
        "expected_requester": {"org_id": ids.org_id, "user_id": restricted},
    });
    assert_eq!(
        api.json(
            "get_project_fee_balances",
            fee_request.clone(),
            &legacy_cookie
        )
        .await,
        json!([])
    );
    assert_eq!(
        api.call("get_project_fee_balances", fee_request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "get_project_fee_balances",
            fee_request.clone(),
            Some(&other_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let mut wrong_organization = fee_request;
    wrong_organization["expected_requester"]["org_id"] = json!(outsider.org_id);
    assert_eq!(
        api.call(
            "get_project_fee_balances",
            wrong_organization,
            Some(&legacy_cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for (actor, permissions) in [
        (
            ids.user_id,
            vec![Permission::ProjectReadAll, Permission::BillableRateReadAll],
        ),
        (restricted, vec![]),
    ] {
        let grants: Vec<String> =
            serde_json::from_value(json!(PermissionSelection::new(&permissions))).unwrap();
        sqlx::query!(
            "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
             VALUES ($1,$2,$3,1,$4,false,'individual')",
            Uuid::now_v7(), ids.org_id, actor, &grants,
        ).execute(pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents=12345,budget_kind='amount',budget_amount_cents=86753 WHERE id=$1",
        ids.project_id,
    ).execute(pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode)
         VALUES ($1,$2,$3,$4,'project')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ($1,$2,$3,'Private launch plan')",
        Uuid::now_v7(), ids.org_id, ids.project_id,
    ).execute(pool).await.unwrap();
    let tag_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,'Launch')",
        tag_id,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        tag_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let restricted_cookie = api.cookie(restricted).await;
    let foreign_cookie = api.cookie(outsider.user_id).await;
    let list = json!({"client_id":null,"include_inactive":true});
    let detail = json!({"project_id":ids.project_id});
    let requester = json!({"org_id": ids.org_id, "user_id": ids.user_id});
    let bound = json!({"expected_requester": requester});
    for (endpoint, body) in [
        ("get_project_overview", json!({"expected_requester": null})),
        ("list_projects", list.clone()),
        ("get_project_details", detail.clone()),
        ("get_project_detail_view", detail.clone()),
        ("list_assignments", detail.clone()),
        ("list_project_tags", json!({})),
        ("list_project_spend", json!({})),
        ("list_project_budget_progress", json!({})),
    ] {
        assert_eq!(
            api.call(endpoint, body, None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let projects = api.json("list_projects", list.clone(), &cookie).await;
    assert_eq!(projects.as_array().unwrap().len(), 1);
    assert_eq!(projects[0]["id"], json!(ids.project_id));
    assert_eq!(projects[0]["rate_cents"], json!(12345));
    assert_eq!(projects[0]["budget_amount_cents"], json!(86753));
    let overview = api
        .json("get_project_overview", bound.clone(), &cookie)
        .await;
    assert_eq!(overview["requester"], requester);
    assert_eq!(overview["canonical_permissions"], json!(true));
    assert_eq!(overview["projects"].as_array().unwrap().len(), 1);
    assert_eq!(overview["projects"][0]["project"], projects[0]);
    assert_eq!(overview["projects"][0]["can_edit"], json!(false));
    assert_eq!(
        overview["projects"][0]["client"],
        json!({"id": ids.client_id, "name":"Acme", "active":true})
    );
    for changed_cookie in [&restricted_cookie, &foreign_cookie] {
        assert_eq!(
            api.call(
                "get_project_overview",
                bound.clone(),
                Some(changed_cookie),
                false
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
    }
    let restricted_overview = api
        .json(
            "get_project_overview",
            json!({
                "expected_requester": null, "org_id":ids.org_id, "viewer_id":ids.user_id,
            }),
            &restricted_cookie,
        )
        .await;
    assert_eq!(
        restricted_overview["requester"]["user_id"],
        json!(restricted)
    );
    assert_eq!(restricted_overview["projects"], json!([]));
    for endpoint in [
        "list_project_tags",
        "list_project_spend",
        "list_project_budget_progress",
    ] {
        let unbound = api.json(endpoint, json!({}), &cookie).await;
        assert_eq!(api.json(endpoint, bound.clone(), &cookie).await, unbound);
        for changed_cookie in [&restricted_cookie, &foreign_cookie] {
            assert_eq!(
                api.call(endpoint, bound.clone(), Some(changed_cookie), false)
                    .await
                    .status(),
                StatusCode::FORBIDDEN,
                "{endpoint} must not substitute another authenticated requester",
            );
        }
    }
    let bound_detail = json!({"project_id":ids.project_id,"expected_requester":requester});
    let detail_view = api
        .json("get_project_detail_view", bound_detail.clone(), &cookie)
        .await;
    assert_eq!(detail_view["requester"], requester);
    assert_eq!(detail_view["canonical_permissions"], json!(true));
    assert_eq!(detail_view["can_edit"], json!(false));
    assert_eq!(
        detail_view["project"],
        api.json("get_project_details", bound_detail.clone(), &cookie)
            .await
    );
    assert_eq!(detail_view["team"].as_array().unwrap().len(), 1);
    assert_eq!(detail_view["team"][0]["id"], json!(teammate));
    assert_eq!(detail_view["team"][0].as_object().unwrap().len(), 2);
    let team = api
        .json("list_assignments", bound_detail.clone(), &cookie)
        .await;
    assert_eq!(team.as_array().unwrap().len(), 1);
    assert_eq!(team[0]["user_id"], json!(teammate));
    assert_eq!(team[0]["rate_cents"], json!(76543));
    assert_eq!(
        api.json("list_assignments", detail.clone(), &cookie).await,
        team
    );
    assert_eq!(
        api.json("get_project_details", bound_detail.clone(), &cookie)
            .await,
        api.json("get_project_details", detail.clone(), &cookie)
            .await,
    );
    for changed_cookie in [&restricted_cookie, &foreign_cookie] {
        assert_eq!(
            api.call(
                "get_project_detail_view",
                bound_detail.clone(),
                Some(changed_cookie),
                false
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            api.call(
                "list_assignments",
                bound_detail.clone(),
                Some(changed_cookie),
                false
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
        assert_eq!(
            api.call(
                "get_project_details",
                bound_detail.clone(),
                Some(changed_cookie),
                false
            )
            .await
            .status(),
            StatusCode::FORBIDDEN,
        );
        for (endpoint, body) in [
            (
                "set_project_active",
                json!({"project_id":ids.project_id,"active":false,"expected_requester":requester}),
            ),
            (
                "set_projects_active",
                json!({"project_ids":[ids.project_id],"active":false,"expected_requester":requester}),
            ),
        ] {
            assert_eq!(
                api.call(endpoint, body, Some(changed_cookie), false)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    assert!(
        sqlx::query_scalar!("SELECT active FROM projects WHERE id=$1", ids.project_id)
            .fetch_one(pool)
            .await
            .unwrap()
    );
    let spend = api.json("list_project_spend", json!({}), &cookie).await;
    assert_eq!(spend.as_array().unwrap().len(), 1);
    assert_eq!(spend[0]["project_id"], json!(ids.project_id));
    assert_eq!(spend[0]["spent_cents"], json!(0));
    let budget = api
        .json("list_project_budget_progress", json!({}), &cookie)
        .await;
    assert_eq!(budget.as_array().unwrap().len(), 1);
    assert_eq!(budget[0]["project_id"], json!(ids.project_id));
    assert_eq!(budget[0]["budget"], json!(86753));
    assert_eq!(budget[0]["consumed"], json!(0));
    let details = api
        .json("get_project_details", detail.clone(), &cookie)
        .await;
    assert_eq!(details["id"], json!(ids.project_id));
    assert!(details.get("admin_notes").is_none());
    assert!(details.get("task_rate_currency").is_none());
    let tags = api.json("list_project_tags", json!({}), &cookie).await;
    assert_eq!(tags.as_array().unwrap().len(), 1);
    assert_eq!(tags[0]["project_id"], json!(ids.project_id));
    assert_eq!(tags[0]["tag_id"], json!(tag_id));
    let spoofed = json!({"client_id":null,"include_inactive":true,
        "org_id":ids.org_id,"viewer_id":ids.user_id,"user_id":ids.user_id});
    assert_eq!(
        api.json("list_projects", spoofed, &restricted_cookie).await,
        json!([])
    );
    assert_eq!(
        api.json("list_project_tags", json!({}), &restricted_cookie)
            .await,
        json!([])
    );
    for denied_cookie in [&restricted_cookie, &foreign_cookie] {
        assert_eq!(
            api.call(
                "get_project_detail_view",
                detail.clone(),
                Some(denied_cookie),
                false
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            api.json(
                "list_assignments",
                json!({"project_id":ids.project_id,"viewer_id":ids.user_id,"org_id":ids.org_id}),
                denied_cookie
            )
            .await,
            json!([]),
        );
        assert_eq!(
            api.json(
                "list_project_budget_progress",
                json!({"viewer_id": ids.user_id, "org_id": ids.org_id}),
                denied_cookie
            )
            .await,
            json!([])
        );
        assert_eq!(
            api.call(
                "get_project_details",
                detail.clone(),
                Some(denied_cookie),
                false
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    // The same authenticated session must lose financial fields immediately,
    // even if its old role is promoted independently of canonical grants.
    sqlx::query!("UPDATE users SET org_role='admin' WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    for permissions in [vec![Permission::ProjectReadAll], vec![]] {
        let can_read = !permissions.is_empty();
        let grants: Vec<String> =
            serde_json::from_value(json!(PermissionSelection::new(&permissions))).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &grants
        )
        .execute(pool)
        .await
        .unwrap();
        let projects = api.json("list_projects", list.clone(), &cookie).await;
        let overview = api
            .json("get_project_overview", bound.clone(), &cookie)
            .await;
        assert_eq!(overview["requester"], requester);
        assert_eq!(
            overview["projects"].as_array().unwrap().len(),
            usize::from(can_read)
        );
        if can_read {
            assert_eq!(overview["projects"][0]["project"], projects[0]);
            assert_eq!(overview["projects"][0]["can_edit"], json!(false));
        }
        let tags = api.json("list_project_tags", json!({}), &cookie).await;
        let team = api
            .json("list_assignments", bound_detail.clone(), &cookie)
            .await;
        assert_eq!(team.as_array().unwrap().len(), usize::from(can_read));
        if can_read {
            assert_eq!(team[0]["user_id"], json!(teammate));
            assert!(team[0].get("rate_cents").is_none(), "{team}");
        }
        let spend = api.json("list_project_spend", json!({}), &cookie).await;
        let budget = api
            .json("list_project_budget_progress", json!({}), &cookie)
            .await;
        assert_eq!(budget.as_array().unwrap().len(), usize::from(can_read));
        assert_eq!(spend.as_array().unwrap().len(), usize::from(can_read));
        assert_eq!(projects.as_array().unwrap().len(), usize::from(can_read));
        assert_eq!(tags.as_array().unwrap().len(), usize::from(can_read));
        if can_read {
            let view = api
                .json("get_project_detail_view", bound_detail.clone(), &cookie)
                .await;
            assert_eq!(view["can_edit"], json!(false));
            assert_eq!(view["team"][0].as_object().unwrap().len(), 2);
            assert!(budget[0].get("budget").is_none(), "{budget}");
            assert!(budget[0].get("consumed").is_none(), "{budget}");
            assert_eq!(budget[0]["breakdown"], json!([]));
            assert!(spend[0].get("spent_cents").is_none(), "{spend}");
            assert_eq!(spend[0]["spent_minutes"], json!(0));
            for field in ["rate_cents", "budget_amount_cents"] {
                assert!(projects[0].get(field).is_none(), "{projects}");
            }
            let details = api
                .json("get_project_details", detail.clone(), &cookie)
                .await;
            assert!(details.get("admin_notes").is_none());
            assert!(details.get("task_rate_currency").is_none());
        } else {
            assert_eq!(
                api.call(
                    "get_project_detail_view",
                    bound_detail.clone(),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::NOT_FOUND
            );
            assert_eq!(
                api.call("get_project_details", detail.clone(), Some(&cookie), false)
                    .await
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    for (endpoint, body) in [
        ("get_project_overview", bound),
        ("list_projects", list),
        ("get_project_details", detail),
        ("get_project_detail_view", bound_detail.clone()),
        ("list_assignments", bound_detail),
        ("list_project_tags", json!({})),
        ("list_project_spend", json!({})),
        ("list_project_budget_progress", json!({})),
    ] {
        assert_eq!(
            api.call(endpoint, body, Some(&cookie), false)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
