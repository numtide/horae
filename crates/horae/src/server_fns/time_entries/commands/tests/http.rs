//! Person-bound commands through real registered routes and session cookies.

use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Manager).await;
    let target = user(pool, ids.org_id, OrgRole::Member).await;
    sqlx::query!(
        "UPDATE users SET oidc_subject=id::text WHERE id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        target
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)",Uuid::now_v7(),ids.org_id,ids.user_id,target).execute(pool).await.unwrap();
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::TimeWriteManaged
    ])))
    .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')",Uuid::now_v7(),ids.org_id,ids.user_id,&grants).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let context = json!({"expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id},"subject_id":target,"expected_policy":"scoped"});
    let tracking = json!({"context":context});
    let create = json!({"context":context,"command":{"operation":"create","entry":{
        "project_id":ids.project_id,"task_id":ids.task_id,"spent_date":"2026-09-07",
        "minutes":45,"notes":"Delegated","billable":true,"start_minute":null
    }}});
    for (name, body) in [
        ("load_timesheet_tracking", tracking.clone()),
        ("apply_timesheet_command", create.clone()),
    ] {
        assert_eq!(
            api.call(name, body, None, false).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let options = api
        .json("load_timesheet_tracking", tracking.clone(), &cookie)
        .await;
    assert_eq!(options.as_array().unwrap().len(), 1);
    assert_eq!(options[0].as_object().unwrap().len(), 5);
    assert_eq!(
        api.json("apply_timesheet_command", create.clone(), &cookie)
            .await,
        Value::Null
    );
    let row = sqlx::query!(
        "SELECT id,user_id,minutes FROM time_entries WHERE org_id=$1",
        ids.org_id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!((row.user_id, row.minutes), (target, 45));
    let mut invalid = create.clone();
    invalid["context"]["expected_requester"]["user_id"] = json!(target);
    assert_eq!(
        api.call("apply_timesheet_command", invalid, Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut invalid = create.clone();
    invalid["command"]["entry"]["minutes"] = json!(-1);
    assert_eq!(
        api.call("apply_timesheet_command", invalid, Some(&cookie), false)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    // Reopening uses the real existing workflow, not a fictional persisted
    // rejected state. Its removed coverage must permit ordinary writes again.
    let approval = Uuid::now_v7();
    sqlx::query!("INSERT INTO approvals (id,org_id,user_id,period_start,period_end,state) VALUES ($1,$2,$3,'2026-09-07','2026-09-13','submitted')",approval,ids.org_id,target).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE time_entries SET state='submitted',rounded_minutes=60 WHERE id=$1",
        row.id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.call(
            "apply_timesheet_command",
            create.clone(),
            Some(&cookie),
            false
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    api.json(
        "reject_submission",
        json!({"approval_id":approval}),
        &cookie,
    )
    .await;
    api.json("apply_timesheet_command", create.clone(), &cookie)
        .await;
    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['private_invalid_grant'] WHERE user_id=$1",ids.user_id).execute(pool).await.unwrap();
    for (name, body) in [
        ("load_timesheet_tracking", tracking.clone()),
        ("apply_timesheet_command", create.clone()),
    ] {
        let response = api.call(name, body, Some(&cookie), false).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response.text().await.unwrap();
        assert!(body.contains("Timesheet is unavailable"), "{body}");
        for secret in [
            "private_invalid_grant",
            "person_permission_states",
            "SELECT",
            "catalog_version",
        ] {
            assert!(!body.contains(secret), "{body}");
        }
    }
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE manager_id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    assert_eq!(
        api.json("load_timesheet_tracking", tracking, &cookie).await,
        json!([])
    );
    assert_eq!(
        api.call("apply_timesheet_command", create, Some(&cookie), false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
}
