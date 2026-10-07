use super::canonical_fields::{canonical_actor, keep_protected};
use super::*;
use crate::models::project_creation::ProjectTaskActivity;
use crate::server_fns::test_seed::{SeedIds, time_entry};
use horae_core::permissions::catalog::{Permission, PermissionSelection};

async fn fixture(pool: &PgPool) -> (SeedIds, EditableProject) {
    let (owner, project) = configured_fixture(pool).await;
    canonical_actor(
        pool,
        &owner,
        &PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let project = load_editable_project(pool, owner.user_id, owner.org_id, project.id)
        .await
        .unwrap();
    (owner, project)
}

fn request(project: EditableProject, task: Uuid, active: bool) -> ProjectEditRequest {
    let mut request = edit_request(project);
    request.unchanged = keep_protected(&request.form);
    request.task_activity.push(ProjectTaskActivity {
        task_id: task,
        active,
    });
    request
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_task_archive_restore_preserves_hidden_configuration_history_and_replays(
    pool: PgPool,
) {
    let (owner, original) = fixture(&pool).await;
    let history = time_entry(
        &pool,
        &SeedIds {
            project_id: original.id,
            ..owner
        },
        EntryState::Open,
    )
    .await;
    sqlx::query!("INSERT INTO project_task_members (id,org_id,project_id,task_id,user_id) VALUES ($1,$2,$3,$4,$5)", Uuid::now_v7(), owner.org_id, original.id, owner.task_id, owner.user_id)
        .execute(&pool).await.unwrap();
    for (budget_minutes, budget_cents) in [(Some(120), None), (None, Some(3000))] {
        sqlx::query!("UPDATE project_task_settings SET budget_minutes=$3,budget_cents=$4,restricted=true WHERE project_id=$1 AND task_id=$2", original.id, owner.task_id, budget_minutes, budget_cents)
        .execute(&pool).await.unwrap();
        for active in [false, true] {
            let project = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
                .await
                .unwrap();
            assert!(
                project.form.tasks[0].rate.is_empty(),
                "rate is withheld from this editor"
            );
            let request = request(project, owner.task_id, active);
            for changed in [true, false] {
                let (_, actual) =
                    save_editable_project(&pool, owner.user_id, owner.org_id, &request, false)
                        .await
                        .unwrap();
                assert_eq!(actual, changed);
            }
            let after = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
                .await
                .unwrap();
            assert_eq!(after.archived_task_ids.contains(&owner.task_id), !active);
            assert!(after.inactive_task_ids.is_empty());
            let saved = sqlx::query!("SELECT pt.active,pt.billable,pt.rate_cents,s.budget_minutes,s.budget_cents,s.restricted
            FROM project_tasks pt JOIN project_task_settings s USING(project_id,task_id) WHERE pt.project_id=$1 AND pt.task_id=$2", original.id, owner.task_id)
            .fetch_one(&pool).await.unwrap();
            assert_eq!(
                (
                    saved.active,
                    saved.billable,
                    saved.rate_cents,
                    saved.budget_minutes,
                    saved.budget_cents,
                    saved.restricted
                ),
                (active, true, Some(2500), budget_minutes, budget_cents, true)
            );
            assert_eq!(
                sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", history)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                60
            );
            assert_eq!(
                sqlx::query_scalar!(
                    "SELECT count(*) FROM project_task_members WHERE project_id=$1 AND task_id=$2",
                    original.id,
                    owner.task_id
                )
                .fetch_one(&pool)
                .await
                .unwrap(),
                Some(1)
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_settings_save_without_activity_intent_does_not_restore_archived_links(
    pool: PgPool,
) {
    let (owner, project) = fixture(&pool).await;
    let archive = request(project, owner.task_id, false);
    save_editable_project(&pool, owner.user_id, owner.org_id, &archive, false)
        .await
        .unwrap();
    let project = load_editable_project(&pool, owner.user_id, owner.org_id, archive.project_id)
        .await
        .unwrap();
    let mut rename = edit_request(project);
    rename.form.name = "Unrelated project edit".into();
    rename.unchanged = keep_protected(&rename.form);
    let encoded = serde_json::to_value(&rename).unwrap();
    assert!(encoded.get("task_activity").is_none());
    let decoded: ProjectEditRequest = serde_json::from_value(encoded).unwrap();
    save_editable_project(&pool, owner.user_id, owner.org_id, &decoded, false)
        .await
        .unwrap();
    assert!(
        !sqlx::query_scalar!(
            "SELECT active FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            rename.project_id,
            owner.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invalid_task_activity_targets_and_global_restore_order_rollback_the_whole_edit(
    pool: PgPool,
) {
    let (owner, project) = fixture(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for case in [
        "foreign",
        "removed",
        "duplicate",
        "global_archived",
        "running",
    ] {
        let current = load_editable_project(&pool, owner.user_id, owner.org_id, project.id)
            .await
            .unwrap();
        let mut edit = request(current, owner.task_id, case == "global_archived");
        edit.form.name = "Must roll back".into();
        match case {
            "foreign" => edit.task_activity[0].task_id = foreign.task_id,
            "removed" => {
                edit.form.tasks.clear();
                edit.unchanged = keep_protected(&edit.form);
            }
            "duplicate" => edit.task_activity.push(edit.task_activity[0].clone()),
            "global_archived" => {
                sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", owner.task_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "running" => {
                sqlx::query!("UPDATE tasks SET active=true WHERE id=$1", owner.task_id)
                    .execute(&pool)
                    .await
                    .unwrap();
                let entry = time_entry(
                    &pool,
                    &SeedIds {
                        project_id: project.id,
                        ..owner
                    },
                    EntryState::Open,
                )
                .await;
                sqlx::query!(
                    "UPDATE time_entries SET is_running=true,started_at=now() WHERE id=$1",
                    entry
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let error = save_editable_project(&pool, owner.user_id, owner.org_id, &edit, false)
            .await
            .unwrap_err();
        assert!(
            matches!(
                error,
                ServerFnError::ServerError {
                    code: BAD_REQUEST | CONFLICT,
                    ..
                }
            ),
            "{case}: {error:?}"
        );
        assert_eq!(
            sqlx::query_scalar!("SELECT name FROM projects WHERE id=$1", project.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            project.form.name
        );
        assert!(
            sqlx::query_scalar!(
                "SELECT active FROM project_tasks WHERE project_id=$1 AND task_id=$2",
                project.id,
                owner.task_id
            )
            .fetch_one(&pool)
            .await
            .unwrap()
        );
    }
}
