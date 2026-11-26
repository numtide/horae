//! The organization's task catalog, a page at a time, for task readers; and
//! [`visible_page`] and [`visible_get`], the tasks anyone may see, for
//! `/api/v1`.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::Permission;
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use super::REQUESTER_CHANGED;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::TimesheetWriteContext;
use crate::models::task::{TaskActivity, TaskCatalogEntry, TaskCatalogPage, TaskCursor};

const UNAVAILABLE: &str = "Task catalog access is unavailable";
const NOT_FOUND: &str = "Task not found";

/// A task and when it last changed, without rates.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRecord {
    pub id: Uuid,
    pub name: String,
    pub billable_default: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which tasks [`visible_page`] lists, and where its page starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskQuery {
    /// Only the tasks of this project.
    pub project_id: Option<Uuid>,
    /// Active tasks, or archived ones.
    pub active: bool,
    /// Only tasks whose `updated_at` is at or after it.
    pub updated_since: Option<DateTime<Utc>>,
    /// The last id of the previous page.
    pub after: Option<Uuid>,
    pub limit: u32,
}

/// The tasks the actor may see, in `id` order: up to `limit + 1` rows, the
/// extra one saying that another page follows. `TaskReadAll` sees the
/// catalog, narrowed to a project by the rule of the project pickers; anyone
/// else the tasks they may track ([`crate::ops::time::tracking`]), so never an
/// archived one.
pub async fn visible_page(
    tx: &mut Tx,
    actor: &Actor,
    query: &TaskQuery,
) -> Result<Vec<TaskRecord>, OpError> {
    visible(tx, actor, None, Some(query)).await
}

/// One task of [`visible_page`], whether active or archived. A task the actor
/// may not see is not found.
pub async fn visible_get(tx: &mut Tx, actor: &Actor, task_id: Uuid) -> Result<TaskRecord, OpError> {
    visible(tx, actor, Some(task_id), None)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| OpError::not_found(NOT_FOUND))
}

async fn visible(
    tx: &mut Tx,
    actor: &Actor,
    task_id: Option<Uuid>,
    query: Option<&TaskQuery>,
) -> Result<Vec<TaskRecord>, OpError> {
    let catalog = actor.has(Permission::TaskReadAll);
    let project_id = query.and_then(|query| query.project_id);
    let trackable: Vec<Uuid> = if catalog {
        Vec::new()
    } else {
        let context = TimesheetWriteContext {
            expected_requester: actor.requester(),
            subject_id: actor.id(),
        };
        let mut ids: Vec<Uuid> = crate::ops::time::tracking(tx, actor, &context)
            .await?
            .into_iter()
            .filter(|pair| project_id.is_none_or(|project| pair.project_id == project))
            .map(|pair| pair.task_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    Ok(sqlx::query_as!(
        TaskRecord,
        r#"SELECT t.id, t.name, t.billable_default, t.active,
                  t.created_at as "created_at: DateTime<Utc>",
                  t.updated_at as "updated_at: DateTime<Utc>"
           FROM tasks t
           WHERE t.org_id = $1 AND ($3 OR t.id = ANY($4))
             AND ($5::uuid IS NULL OR NOT $3 OR EXISTS (
               SELECT 1 FROM project_tasks pt
               JOIN projects p ON p.id = pt.project_id AND p.org_id = t.org_id
               LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
                 AND management.project_id = p.id AND management.manager_id = $2
               LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
               WHERE pt.task_id = t.id AND p.id = $5
                 AND ($6 OR ($7 AND management.id IS NOT NULL) OR member.id IS NOT NULL)
             ))
             AND ($8::uuid IS NULL OR t.id = $8) AND ($9::bool IS NULL OR t.active = $9)
             AND ($10::timestamptz IS NULL OR t.updated_at >= $10)
             AND t.id > $11
           ORDER BY t.id LIMIT $12"#,
        actor.org_id(),
        actor.id(),
        catalog,
        &trackable,
        project_id,
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        task_id,
        query.map(|query| query.active),
        query.and_then(|query| query.updated_since) as Option<DateTime<Utc>>,
        query.and_then(|query| query.after).unwrap_or(Uuid::nil()),
        query.map_or(1, |query| i64::from(query.limit) + 1),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Up to 100 tasks after `after`, common tasks first, with what the actor may
/// do there. Needs `TaskReadAll`; rates need `BillableRateReadAll`.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    activity: TaskActivity,
    after: Option<&TaskCursor>,
    expected: Option<PermissionRequester>,
) -> Result<TaskCatalogPage, OpError> {
    actor.expect_requester(expected, REQUESTER_CHANGED)?;
    if !actor.has(Permission::TaskReadAll) {
        return Err(OpError::forbidden(UNAVAILABLE));
    }
    if after.is_some_and(|cursor| cursor.name.contains('\0')) {
        return Err(OpError::invalid("after", "Invalid task catalog cursor"));
    }
    let org_id = actor.org_id();
    let can_edit = actor.has(Permission::TaskWriteAll);
    let can_read_rates = actor.has(Permission::BillableRateReadAll);
    let can_edit_rates = can_edit && actor.has(Permission::BillableRateWriteAll);
    let (active_only, archived_only) = (
        activity == TaskActivity::Active,
        activity == TaskActivity::Archived,
    );
    // Common tasks first, so the page splits into its two groups in order.
    let mut tasks = sqlx::query_as!(
        TaskCatalogEntry,
        "SELECT id,name,billable_default,is_default,active,
                CASE WHEN $2 THEN default_rate_cents END AS default_rate_cents
         FROM tasks
         WHERE org_id=$1 AND (NOT $3 OR active) AND (NOT $4 OR NOT active)
           AND ($5::text IS NULL OR (NOT is_default,name,id) > (NOT $7::boolean,$5,$6::uuid))
         ORDER BY NOT is_default,name,id LIMIT 101",
        org_id,
        can_read_rates,
        active_only,
        archived_only,
        after.map(|cursor| cursor.name.as_str()),
        after.map(|cursor| cursor.id),
        after.map(|cursor| cursor.is_default),
    )
    .fetch_all(&mut **tx)
    .await?;
    let next_after = if tasks.len() > 100 {
        tasks.truncate(100);
        tasks.last().map(|task| TaskCursor {
            is_default: task.is_default,
            name: task.name.clone(),
            id: task.id,
        })
    } else {
        None
    };
    Ok(TaskCatalogPage {
        requester: actor.requester(),
        can_edit,
        can_read_rates,
        can_edit_rates,
        tasks,
        next_after,
    })
}

#[cfg(test)]
mod tests;
