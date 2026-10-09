//! Exercise grouped reports through the registered route and real sessions.

use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    const ENDPOINT: &str = "list_visible_time_report_groups";
    let ids = seed(pool, OrgRole::Member).await;
    let foreign = seed(pool, OrgRole::Admin).await;
    time_entry(pool, &ids, EntryState::Open).await;
    time_entry(pool, &foreign, EntryState::Open).await;
    let hidden = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Hidden colleague')",
        hidden,
        ids.org_id,
        format!("{hidden}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    time_entry(
        pool,
        &SeedIds {
            user_id: hidden,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!("UPDATE users SET oidc_subject=id::text,billable_rate_cents=123456,cost_rate_cents=987654 WHERE id=$1", ids.user_id)
        .execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let other_cookie = api.cookie(foreign.user_id).await;
    let request = json!({"query": {
        "date_from":"2026-09-01", "date_to":"2026-09-30",
        "client_ids":[], "project_ids":[], "user_ids":[], "task_ids":[], "tag_ids":[],
        "group_by":"project", "after":null,
        "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id}
    }});
    assert_eq!(
        api.call(ENDPOINT, request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let grants: Vec<String> =
        serde_json::from_value(json!(BuiltInProfile::Member.selection())).unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants)
        .execute(pool).await.unwrap();
    let totals =
        json!({"entry_count":1,"total_minutes":60,"rounded_minutes":60,"billable_minutes":60});
    for (dimension, id, name) in [
        ("client", ids.client_id, "Acme"),
        ("project", ids.project_id, "Widget"),
        ("task", ids.task_id, "Dev"),
        ("person", ids.user_id, "Test User"),
    ] {
        let mut body = request.clone();
        body["query"]["group_by"] = json!(dimension);
        assert_eq!(
            api.json(ENDPOINT, body, &cookie).await,
            json!({
                "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
                "groups":[{"id":id,"name":name,"totals":totals}],
                "next_after":null,"totals":totals
            })
        );
    }
    // A selected foreign or inaccessible entity cannot widen current scope.
    for (key, value) in [
        ("project_ids", json!([foreign.project_id])),
        ("user_ids", json!([hidden, foreign.user_id])),
    ] {
        let mut body = request.clone();
        body["query"][key] = value;
        let result = api.json(ENDPOINT, body, &cookie).await;
        assert_eq!(result["groups"], json!([]));
        assert_eq!(
            result["totals"],
            json!({"entry_count":0,"total_minutes":0,"rounded_minutes":0,"billable_minutes":0})
        );
    }
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&other_cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    // Dioxus 0.7.9 maps argument decoding failures to 500 before the handler.
    // Assert rejection without report data separately from domain validation.
    for (key, value) in [
        ("group_by", json!("profitability")),
        ("actor_id", json!(foreign.user_id)),
        ("project_ids", json!(["not-a-uuid"])),
    ] {
        let mut body = request.clone();
        body["query"][key] = value;
        let response = api.call(ENDPOINT, body, Some(&cookie), false).await;
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{key}"
        );
        let error = response.text().await.unwrap();
        assert!(!error.contains("\"groups\"") && !error.contains("\"totals\""));
    }
    for (key, value) in [
        ("date_to", json!("2026-08-31")),
        (
            "after",
            json!({"group_by":"task","name":"Widget","id":ids.project_id}),
        ),
        (
            "after",
            json!({"group_by":"project","name":"bad\u{0000}name","id":ids.project_id}),
        ),
    ] {
        let mut body = request.clone();
        body["query"][key] = value;
        assert_eq!(
            api.call(ENDPOINT, body, Some(&cookie), false)
                .await
                .status(),
            StatusCode::BAD_REQUEST,
            "{key}"
        );
    }
    sqlx::query!(
        "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(ENDPOINT, request.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        api.call(ENDPOINT, request, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
