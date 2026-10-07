//! Weekly submission must remain bound to the page that initiated it.

use super::*;
use crate::server_fns::test_seed::{seed, time_entry};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = seed(pool, OrgRole::Member).await;
    let entry = time_entry(pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET oidc_subject=id::text WHERE id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let other = user(pool, ids.org_id, OrgRole::Member).await;
    let cookie = api.cookie(ids.user_id).await;
    let other_cookie = api.cookie(other).await;
    let body = json!({
        "week_start":"2026-09-07",
        "context": {
            "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id},
            "subject_id":ids.user_id,"expected_policy":"legacy_own"
        }
    });
    assert_eq!(
        api.call("submit_week", body.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call("submit_week", body.clone(), Some(&other_cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    for (field, value) in [
        ("subject_id", json!(other)),
        ("expected_policy", json!("scoped")),
        (
            "expected_requester",
            json!({"org_id":Uuid::now_v7(),"user_id":ids.user_id}),
        ),
    ] {
        let mut invalid = body.clone();
        invalid["context"][field] = value;
        assert_eq!(
            api.call("submit_week", invalid, Some(&cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN,
            "{field}"
        );
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call("submit_week", body.clone(), Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let stored = sqlx::query!(
        "SELECT state::text,rounded_minutes FROM time_entries WHERE id=$1",
        entry
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        (stored.state.as_deref(), stored.rounded_minutes),
        (Some("open"), None)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM approvals WHERE org_id=$1", ids.org_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        Some(0)
    );
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let result = api.json("submit_week", body, &cookie).await;
    assert_eq!(result["user_id"], json!(ids.user_id));
    assert_eq!(result["state"], json!("submitted"));
}
