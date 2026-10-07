//! Current task lifecycle authority through the registered session endpoint.

use super::*;
use crate::models::project_creation::{
    EditableProject, ProjectEditRequest, ProjectTaskActivity, ProtectedProjectField,
};
use horae_core::permissions::catalog::{Permission, PermissionSelection};

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let mut failures = Vec::new();
    for (case, role, grants, running, expected) in [
        (
            "member_archives",
            OrgRole::Member,
            vec![Permission::TaskWriteAll],
            false,
            StatusCode::OK,
        ),
        (
            "legacy_admin_is_not_authority",
            OrgRole::Admin,
            vec![],
            false,
            StatusCode::FORBIDDEN,
        ),
        (
            "archive_hides_rate",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            false,
            StatusCode::OK,
        ),
        (
            "running_timer_blocks_archive",
            OrgRole::Admin,
            vec![Permission::TaskWriteAll],
            true,
            StatusCode::CONFLICT,
        ),
        (
            "rate_reader_may_see_rate",
            OrgRole::Member,
            vec![Permission::TaskWriteAll, Permission::BillableRateReadAll],
            false,
            StatusCode::OK,
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
        if running {
            let entry = crate::server_fns::test_seed::time_entry(
                pool,
                &ids,
                horae_core::types::EntryState::Open,
            )
            .await;
            sqlx::query!(
                "UPDATE time_entries SET is_running=true,started_at=now() WHERE id=$1",
                entry
            )
            .execute(pool)
            .await
            .unwrap();
        }
        let cookie = api.cookie(ids.user_id).await;
        let request = json!({"task_id":ids.task_id,"active":false,
            "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id}});
        if case == "member_archives" {
            assert_eq!(
                api.call("set_task_active", request.clone(), None, false)
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            let mut mismatch = request.clone();
            mismatch["expected_requester"]["user_id"] = json!(Uuid::now_v7());
            assert_eq!(
                api.call("set_task_active", mismatch, Some(&cookie), false)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
            let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
            let mut foreign_request = request.clone();
            foreign_request["task_id"] = json!(foreign.task_id);
            assert_eq!(
                api.call("set_task_active", foreign_request, Some(&cookie), false)
                    .await
                    .status(),
                StatusCode::NOT_FOUND
            );

            // Exercise real timer delivery on both sides of successful archival.
            sqlx::query!(
                "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
                ids.project_id,
                ids.task_id
            )
            .execute(pool)
            .await
            .unwrap();
            sqlx::query!(
                "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id
            )
            .execute(pool)
            .await
            .unwrap();
            let timer = api
                .call(
                    "start_timer",
                    json!({"project_id":ids.project_id,"task_id":ids.task_id,"notes":null}),
                    Some(&cookie),
                    false,
                )
                .await;
            assert_eq!(timer.status(), StatusCode::OK);
            let timer: Value = timer.json().await.unwrap();
            assert_eq!(
                api.call("set_task_active", request.clone(), Some(&cookie), false)
                    .await
                    .status(),
                StatusCode::CONFLICT
            );
            assert_eq!(
                api.call(
                    "stop_timer",
                    json!({"entry_id":timer["id"]}),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::OK
            );
        }
        let response = api
            .call("set_task_active", request.clone(), Some(&cookie), false)
            .await;
        let status = response.status();
        let body = response.text().await.unwrap();
        if status != expected {
            failures.push(format!("{case}: expected {expected}, got {status}: {body}"));
        }
        if status == StatusCode::OK {
            let value: Value = serde_json::from_str(&body).unwrap();
            let expected_rate = if may_read_rate {
                Some(json!(8000))
            } else {
                None
            };
            if value.get("default_rate_cents").cloned() != expected_rate {
                failures.push(format!("{case}: wrong rate projection: {body}"));
            }
        }
        let active = sqlx::query_scalar!("SELECT active FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(pool)
            .await
            .unwrap();
        if active != (expected != StatusCode::OK) {
            failures.push(format!("{case}: unexpected stored activity {active}"));
        }
        if case == "member_archives" {
            assert_eq!(
                api.call(
                    "start_timer",
                    json!({"project_id":ids.project_id,"task_id":ids.task_id,"notes":null}),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::CONFLICT
            );
            for active in [false, true, true] {
                let mut activity_request = request.clone();
                activity_request["active"] = json!(active);
                let response = api
                    .call("set_task_active", activity_request, Some(&cookie), false)
                    .await;
                assert_eq!(response.status(), StatusCode::OK);
                let value: Value = response.json().await.unwrap();
                assert_eq!(value["active"], json!(active));
                assert!(value.get("default_rate_cents").is_none());
            }
            assert_eq!(
                api.call(
                    "start_timer",
                    json!({"project_id":ids.project_id,"task_id":ids.task_id,"notes":null}),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::CONFLICT
            );
            assert_eq!(
                api.call(
                    "load_project_editor",
                    json!({"project_id":ids.project_id}),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::FORBIDDEN
            );
            let stored: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
                Permission::ProjectWriteAll
            ])))
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &stored
            )
            .execute(pool)
            .await
            .unwrap();
            let editor: EditableProject = serde_json::from_value(
                api.json(
                    "load_project_editor",
                    json!({"project_id":ids.project_id}),
                    &cookie,
                )
                .await,
            )
            .unwrap();
            assert_eq!(editor.archived_task_ids, [ids.task_id]);
            let mut unchanged = vec![
                ProtectedProjectField::ProjectRate,
                ProtectedProjectField::Budget,
                ProtectedProjectField::Fees,
                ProtectedProjectField::InvoiceDefaults,
                ProtectedProjectField::PrivateNotes,
            ];
            unchanged.extend(
                editor
                    .form
                    .tasks
                    .iter()
                    .map(|task| ProtectedProjectField::TaskRate(task.id)),
            );
            for person in &editor.form.team {
                unchanged.extend([
                    ProtectedProjectField::PersonRate(person.user_id),
                    ProtectedProjectField::CostRate(person.user_id),
                ]);
            }
            let restore = ProjectEditRequest {
                id: Uuid::now_v7(),
                project_id: editor.id,
                expected_revision: editor.revision,
                expected_requester: editor.access.as_ref().map(|access| access.requester),
                managers: editor
                    .access
                    .as_ref()
                    .map(|access| (&access.managers).into()),
                form: editor.form,
                unchanged,
                task_activity: vec![ProjectTaskActivity {
                    task_id: ids.task_id,
                    active: true,
                }],
            };
            for _ in 0..2 {
                assert_eq!(
                    api.call(
                        "save_project_editor",
                        json!({"request":restore}),
                        Some(&cookie),
                        false
                    )
                    .await
                    .status(),
                    StatusCode::OK
                );
            }
            let timer = api
                .call(
                    "start_timer",
                    json!({"project_id":ids.project_id,"task_id":ids.task_id,"notes":null}),
                    Some(&cookie),
                    false,
                )
                .await;
            assert_eq!(timer.status(), StatusCode::OK);
            let timer: Value = timer.json().await.unwrap();
            assert_eq!(
                api.call(
                    "stop_timer",
                    json!({"entry_id":timer["id"]}),
                    Some(&cookie),
                    false
                )
                .await
                .status(),
                StatusCode::OK
            );
        }
    }
    assert!(
        failures.is_empty(),
        "Task activity contract failures:\n{}",
        failures.join("\n")
    );
}
