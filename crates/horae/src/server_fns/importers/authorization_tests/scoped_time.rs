//! Scoped entry payloads through the real session-authenticated route.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::types::EntryState;

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    let target_ids = crate::server_fns::test_seed::SeedIds {
        user_id: target,
        ..ids
    };
    let entry = crate::server_fns::test_seed::time_entry(pool, &target_ids, EntryState::Open).await;
    let invoice = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'PRIVATE-INVOICE','2026-09-07','2026-10-07','EUR',45000)",
        invoice,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET notes='Visible work notes', rounded_minutes=60, invoice_id=$2,
        created_at='2026-09-07 12:00:00+00' WHERE id=$1",
        entry,
        invoice
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET oidc_subject=id::text, cost_rate_cents=6000, billable_rate_cents=10000 WHERE org_id=$1", ids.org_id).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    for role in [OrgRole::Member, OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            target,
            role as OrgRole
        )
        .execute(pool)
        .await
        .unwrap();
        let cookie = api.cookie(target).await;
        let response = api
            .json(
                "list_time_entries",
                json!({
                    "_user_id":foreign.user_id,"project_id":null,
                    "date_from":"2026-09-01","date_to":"2026-09-30","limit":null
                }),
                &cookie,
            )
            .await;
        for row in response.as_array().unwrap() {
            assert!(
                row.get("invoice_id").is_none(),
                "time read disclosed invoice identity: {row}"
            );
            assert_eq!(row["user_id"], json!(target));
        }
        assert_eq!(response.as_array().unwrap().len(), 1);
        assert_eq!(response[0]["id"], json!(entry));
        let stored =
            sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", entry)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(stored, Some(invoice));
    }
    let request = json!({"query":{"date_from":"2026-09-01","date_to":"2026-09-30","user_id":null,"project_id":null,"after":null}});
    assert_eq!(
        api.call("list_visible_time_entries", request.clone(), None, false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        api.call(
            "list_visible_time_entries",
            request.clone(),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::TimeReadManaged
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
    let empty = json!({"requester":{"org_id":ids.org_id,"user_id":ids.user_id},"entries":[],"next_after":null});
    assert_eq!(
        api.json("list_visible_time_entries", request.clone(), &cookie)
            .await,
        empty
    );
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(pool).await.unwrap();
    let expected = json!({
        "requester":{"org_id":ids.org_id,"user_id":ids.user_id},
        "entries":[{"id":entry,"user_id":target,"user_name":"Test User", "project_id":ids.project_id,"project_name":"Widget",
            "task_id":ids.task_id,"task_name":"Dev","client_id":ids.client_id,"client_name":"Acme",
            "spent_date":"2026-09-07","minutes":60,"rounded_minutes":60,"notes":"Visible work notes",
            "billable":true,"is_running":false,"started_at":null,"start_minute":null,"sort_order":0,
            "state":"open","created_at":"2026-09-07T12:00:00Z"}],
        "next_after":null
    });
    assert_eq!(
        api.json("list_visible_time_entries", request.clone(), &cookie)
            .await,
        expected
    );
    let mut forged = request.clone();
    forged["org_id"] = json!(foreign.org_id);
    forged["user_id"] = json!(foreign.user_id);
    forged["is_administrator"] = json!(true);
    assert_eq!(
        api.json("list_visible_time_entries", forged, &cookie).await,
        expected
    );
    let mut filtered = request.clone();
    filtered["query"]["user_id"] = json!(foreign.user_id);
    assert_eq!(
        api.json("list_visible_time_entries", filtered, &cookie)
            .await,
        empty
    );
    let mut invalid = request.clone();
    invalid["query"]["date_to"] = json!("2026-08-31");
    assert_eq!(
        api.call("list_visible_time_entries", invalid, Some(&cookie), false)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE manager_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.json("list_visible_time_entries", request.clone(), &cookie)
            .await,
        empty
    );
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1", ids.user_id).execute(pool).await.unwrap();
    let response = api
        .call(
            "list_visible_time_entries",
            request.clone(),
            Some(&cookie),
            false,
        )
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text().await.unwrap();
    assert!(body.contains("Time entries are unavailable"));
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
        api.call("list_visible_time_entries", request, Some(&cookie), false)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
