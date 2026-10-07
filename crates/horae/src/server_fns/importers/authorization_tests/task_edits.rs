//! Explicit task-rate intent through the registered authenticated endpoint.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let mut failures = Vec::new();
    for (case, role, grants, rate, expected_status, expected_rate) in [
        (
            "member_preserves",
            OrgRole::Member,
            vec![Permission::TaskWriteAll],
            json!({"action":"preserve"}),
            StatusCode::OK,
            Some(8000),
        ),
        (
            "legacy_admin_has_no_authority",
            OrgRole::Admin,
            vec![],
            json!({"action":"preserve"}),
            StatusCode::FORBIDDEN,
            Some(8000),
        ),
        (
            "name_editor_preserves_hidden_rate",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            json!({"action":"preserve"}),
            StatusCode::OK,
            Some(8000),
        ),
        (
            "name_editor_cannot_clear",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            json!({"action":"clear"}),
            StatusCode::FORBIDDEN,
            Some(8000),
        ),
        (
            "equal_rate_still_requires_authority",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            json!({"action":"set","amount_cents":8000,"currency":"EUR"}),
            StatusCode::FORBIDDEN,
            Some(8000),
        ),
        (
            "managed_rate_does_not_write_global_default",
            OrgRole::Admin,
            vec![
                Permission::TaskWriteAll,
                Permission::BillableRateWriteManaged,
            ],
            json!({"action":"set","amount_cents":0,"currency":"EUR"}),
            StatusCode::FORBIDDEN,
            Some(8000),
        ),
        (
            "rate_writer_clears",
            OrgRole::Member,
            vec![Permission::TaskWriteAll, Permission::BillableRateWriteAll],
            json!({"action":"clear"}),
            StatusCode::OK,
            None,
        ),
        (
            "rate_writer_sets_zero",
            OrgRole::Member,
            vec![Permission::TaskWriteAll, Permission::BillableRateWriteAll],
            json!({"action":"set","amount_cents":0,"currency":"EUR"}),
            StatusCode::OK,
            Some(0),
        ),
        (
            "rate_grant_without_task_write",
            OrgRole::Admin,
            vec![Permission::BillableRateWriteAll],
            json!({"action":"set","amount_cents":0,"currency":"EUR"}),
            StatusCode::FORBIDDEN,
            Some(8000),
        ),
    ] {
        let ids = crate::server_fns::test_seed::seed(pool, role).await;
        let grants = PermissionSelection::new(&grants);
        let may_read_rate = grants.contains(Permission::BillableRateReadAll);
        let stored: Vec<String> = serde_json::from_value(json!(grants)).unwrap();
        sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
            VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &stored)
            .execute(pool).await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
            ids.org_id
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE tasks SET default_rate_cents=8000,default_rate_currency='EUR' WHERE id=$1",
            ids.task_id
        )
        .execute(pool)
        .await
        .unwrap();
        let cookie = api.cookie(ids.user_id).await;
        let request = json!({"task_id":ids.task_id,"name":"Renamed", "billable_default":true,
            "rate":rate,"expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id}});
        if case == "member_preserves" {
            assert_eq!(
                api.call("update_task", request.clone(), None, false)
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            let mut mismatched = request.clone();
            mismatched["expected_requester"]["user_id"] = json!(Uuid::now_v7());
            assert_eq!(
                api.call("update_task", mismatched, Some(&cookie), false)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
        let response = api.call("update_task", request, Some(&cookie), false).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        if status != expected_status {
            failures.push(format!(
                "{case}: expected {expected_status}, got {status}: {body}"
            ));
        }
        if status == StatusCode::OK && !may_read_rate {
            let value: Value = serde_json::from_str(&body).unwrap();
            if value.get("default_rate_cents").is_some() {
                failures.push(format!("{case}: protected rate returned: {body}"));
            }
        }
        let after = sqlx::query!(
            "SELECT name,default_rate_cents,default_rate_currency FROM tasks WHERE id=$1",
            ids.task_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let expected_name = if expected_status == StatusCode::OK {
            "Renamed"
        } else {
            "Dev"
        };
        let expected_currency = expected_rate.map(|_| "EUR");
        if (
            after.name.as_str(),
            after.default_rate_cents,
            after.default_rate_currency.as_deref(),
        ) != (expected_name, expected_rate, expected_currency)
        {
            failures.push(format!("{case}: unexpected stored state {after:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "Task edit contract failures:\n{}",
        failures.join("\n")
    );
}
