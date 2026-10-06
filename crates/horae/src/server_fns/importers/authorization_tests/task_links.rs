//! Existing-task association uses project authority, not global task authority.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let mut failures = Vec::new();
    for (case, role, grants, designated, rate, status) in [
        (
            "project_editor",
            OrgRole::Member,
            vec![Permission::ProjectWriteAll],
            false,
            None,
            StatusCode::OK,
        ),
        (
            "legacy_admin",
            OrgRole::Admin,
            vec![],
            false,
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            "catalog_only",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            false,
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            "managed_editor",
            OrgRole::Member,
            vec![Permission::ProjectWriteManaged],
            true,
            None,
            StatusCode::OK,
        ),
        (
            "undesignated_editor",
            OrgRole::Admin,
            vec![Permission::ProjectWriteManaged],
            false,
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            "hidden_rate",
            OrgRole::Admin,
            vec![Permission::ProjectWriteAll],
            false,
            Some("0"),
            StatusCode::FORBIDDEN,
        ),
        (
            "managed_rate",
            OrgRole::Member,
            vec![
                Permission::ProjectWriteAll,
                Permission::BillableRateWriteManaged,
            ],
            true,
            Some("0"),
            StatusCode::OK,
        ),
        (
            "undesignated_rate",
            OrgRole::Admin,
            vec![
                Permission::ProjectWriteAll,
                Permission::BillableRateWriteManaged,
            ],
            false,
            Some("0"),
            StatusCode::FORBIDDEN,
        ),
        (
            "all_rate",
            OrgRole::Member,
            vec![
                Permission::ProjectWriteAll,
                Permission::BillableRateWriteAll,
            ],
            false,
            Some("12.34"),
            StatusCode::OK,
        ),
    ] {
        let ids = crate::server_fns::test_seed::seed(pool, role).await;
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
        sqlx::query!(
            "UPDATE tasks SET default_rate_cents=8000,default_rate_currency='EUR' WHERE id=$1",
            ids.task_id
        )
        .execute(pool)
        .await
        .unwrap();
        if designated {
            sqlx::query!("INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
                .execute(pool).await.unwrap();
        }
        let cookie = api.cookie(ids.user_id).await;
        let body = json!({
            "project_id":ids.project_id,"task_id":ids.task_id,
            "rate":rate.map(|amount| json!({"amount":amount,"currency":"EUR"})),
            "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id}
        });
        for _ in 0..2 {
            let response = api
                .call("link_project_task", body.clone(), Some(&cookie), false)
                .await;
            if response.status() != status {
                failures.push(format!(
                    "{case}: expected {status}, got {}",
                    response.status()
                ));
            }
        }
        let link = sqlx::query!(
            "SELECT active,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            ids.project_id,
            ids.task_id
        )
        .fetch_optional(pool)
        .await
        .unwrap();
        if status == StatusCode::OK {
            if let Some(link) = link {
                assert!(link.active);
                assert_eq!(
                    link.rate_cents,
                    Some(match rate {
                        Some("0") => 0,
                        Some(_) => 1234,
                        None => 8000,
                    })
                );
            } else {
                failures.push(format!("{case}: authorized association missing"));
            }
        } else if link.is_some() {
            failures.push(format!("{case}: denied association persisted"));
        }
        if case == "project_editor" {
            assert_eq!(
                api.call("link_project_task", body.clone(), None, false)
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            let mut switched = body.clone();
            switched["expected_requester"]["user_id"] = json!(Uuid::now_v7());
            let response = api
                .call("link_project_task", switched, Some(&cookie), false)
                .await;
            if response.status() != StatusCode::FORBIDDEN {
                failures.push(format!("switched requester: {}", response.status()));
            }
            let mut explicit = body.clone();
            explicit["rate"] = json!({"amount":"80","currency":"EUR"});
            let response = api
                .call("link_project_task", explicit, Some(&cookie), false)
                .await;
            if response.status() != StatusCode::FORBIDDEN {
                failures.push(format!(
                    "equal rate on existing link: {}",
                    response.status()
                ));
            }
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
            let response = api
                .call("link_project_task", body, Some(&cookie), false)
                .await;
            if response.status() != StatusCode::FORBIDDEN {
                failures.push(format!("revoked project editor: {}", response.status()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Task linking failures:\n{}",
        failures.join("\n")
    );
}
