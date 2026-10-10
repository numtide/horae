//! Explicit project-link lifecycle changes within the existing editor transaction.

use super::*;
use crate::models::project_creation::ProjectTaskActivity;

pub(super) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    before: &EditableProject,
    form: &ProjectForm,
    edits: &[ProjectTaskActivity],
) -> Result<(), ServerFnError> {
    // The editor already holds current project-write authority, the organization
    // access gate and the parent project lock. Financial fields are independent.
    let mut edits: Vec<_> = edits.iter().collect();
    edits.sort_by_key(|edit| edit.task_id);
    for edit in edits {
        let source = TaskSource::Existing {
            task_id: edit.task_id,
        };
        if !before.form.tasks.iter().any(|task| task.source == source)
            || !form.tasks.iter().any(|task| task.source == source)
        {
            return Err(err(
                BAD_REQUEST,
                "Activity changes require a retained project task",
            ));
        }
        let task_active = sqlx::query_scalar!(
            "SELECT active FROM tasks WHERE id=$1 AND org_id=$2 FOR SHARE",
            edit.task_id,
            org_id,
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage_error)?
        .ok_or_else(|| not_found("Task not found"))?;
        if edit.active && !task_active {
            return Err(conflict(
                "Restore the task in the task catalog before restoring it on this project",
            ));
        }
        if !edit.active {
            let running = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM time_entries
                   WHERE org_id=$1 AND project_id=$2 AND task_id=$3 AND is_running) AS "running!""#,
                org_id,
                before.id,
                edit.task_id,
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(storage_error)?;
            if running {
                return Err(conflict(
                    "A running timer prevents archiving this project task",
                ));
            }
        }
        sqlx::query!(
            "UPDATE project_tasks SET active=$3 WHERE project_id=$1 AND task_id=$2 AND active IS DISTINCT FROM $3",
            before.id, edit.task_id, edit.active,
        ).execute(&mut **tx).await.map_err(storage_error)?;
    }
    Ok(())
}
