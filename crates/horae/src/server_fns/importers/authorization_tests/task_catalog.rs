//! The management catalog binds both rows and affordances to its requester.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let mut failures = Vec::new();
    for (grants, status, read_rates, edit, edit_rates) in [
        (vec![], StatusCode::FORBIDDEN, false, false, false),
        (
            vec![Permission::BillableRateReadAll],
            StatusCode::FORBIDDEN,
            false,
            false,
            false,
        ),
        (
            vec![Permission::TaskReadAll],
            StatusCode::OK,
            false,
            false,
            false,
        ),
        (
            vec![Permission::TaskWriteAll],
            StatusCode::OK,
            false,
            true,
            false,
        ),
        (
            vec![Permission::TaskReadAll, Permission::BillableRateReadAll],
            StatusCode::OK,
            true,
            false,
            false,
        ),
        (
            vec![Permission::TaskWriteAll, Permission::BillableRateWriteAll],
            StatusCode::OK,
            true,
            true,
            true,
        ),
    ] {
        let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
        let stored: Vec<String> =
            serde_json::from_value(json!(PermissionSelection::new(&grants))).unwrap();
        sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &stored)
            .execute(pool).await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
            ids.org_id
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!("UPDATE tasks SET active=false,default_rate_cents=8000,default_rate_currency='USD' WHERE id=$1", ids.task_id)
            .execute(pool).await.unwrap();
        let cookie = api.cookie(ids.user_id).await;
        let body = json!({"activity":"archived","after":null,"expected_requester":null});
        let response = api
            .call("load_task_catalog", body.clone(), Some(&cookie), false)
            .await;
        if response.status() != status {
            failures.push(format!(
                "{grants:?}: expected {status}, got {}",
                response.status()
            ));
            continue;
        }
        if status != StatusCode::OK {
            continue;
        }
        let page: Value = response.json().await.unwrap();
        assert_eq!(
            page["requester"],
            json!({"org_id":ids.org_id,"user_id":ids.user_id})
        );
        assert_eq!(page["can_edit"], edit);
        assert_eq!(page["can_read_rates"], read_rates);
        assert_eq!(page["can_edit_rates"], edit_rates);
        assert_eq!(page["tasks"].as_array().unwrap().len(), 1);
        let task = &page["tasks"][0];
        assert_eq!(task["id"], json!(ids.task_id));
        assert_eq!(task["active"], false);
        assert_eq!(
            task.get("default_rate_cents"),
            read_rates.then_some(&json!(8000))
        );
        assert_eq!(
            task.get("default_rate_currency"),
            read_rates.then_some(&json!("USD"))
        );
        assert_eq!(
            page.get("rate_currency"),
            edit_rates.then_some(&json!("EUR"))
        );
        let mut switched = body.clone();
        switched["expected_requester"] = json!({"org_id":ids.org_id,"user_id":Uuid::now_v7()});
        assert_eq!(
            api.call("load_task_catalog", switched, Some(&cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            api.call("load_task_catalog", body.clone(), None, false)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let floor: Vec<String> =
            serde_json::from_value(json!(PermissionSelection::new(&[]))).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &floor
        )
        .execute(pool)
        .await
        .unwrap();
        assert_eq!(
            api.call("load_task_catalog", body, Some(&cookie), false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert!(
        failures.is_empty(),
        "Task catalog delivery failures:\n{}",
        failures.join("\n")
    );
}
