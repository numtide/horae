//! Session-bound report payloads, including populated private source fields.

use super::*;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let other = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let entry = crate::server_fns::test_seed::time_entry(pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE users SET oidc_subject=id::text,billable_rate_cents=123456,cost_rate_cents=987654 WHERE id=$1", ids.user_id).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let request = json!({"query": {
        "date_from":"2026-09-01", "date_to":"2026-09-30",
        "client_ids":[], "project_ids":[], "user_ids":[], "task_ids":[], "tag_ids":[],
        "after":null, "expected_requester":null
    }});
    const ENDPOINT: &str = "list_visible_time_report_entries";
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
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
        VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    let expected = json!({
        "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "entries":[{"id":entry,"spent_date":"2026-09-07","project_name":"Widget",
            "task_name":"Dev","user_name":"Test User","minutes":60,"rounded_minutes":60,"billable":true,"notes":null}],
        "next_after":null
    });
    assert_eq!(api.json(ENDPOINT, request.clone(), &cookie).await, expected);
    let mut forged = request.clone();
    forged["org_id"] = json!(other.org_id);
    forged["user_id"] = json!(other.user_id);
    assert_eq!(api.json(ENDPOINT, forged, &cookie).await, expected);
    let mut filtered = request.clone();
    filtered["query"]["user_ids"] = json!([other.user_id]);
    assert_eq!(
        api.json(ENDPOINT, filtered, &cookie).await["entries"],
        json!([])
    );
    let mut invalid = request.clone();
    invalid["query"]["date_to"] = json!("2026-08-01");
    assert_eq!(
        api.call(ENDPOINT, invalid, Some(&cookie), false)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut changed = request.clone();
    changed["query"]["expected_requester"] = json!({"org_id":ids.org_id,"user_id":other.user_id});
    assert_eq!(
        api.call(ENDPOINT, changed, Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
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
