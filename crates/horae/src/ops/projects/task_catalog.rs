//! The organization's task catalog, a page at a time, for task readers.

use horae_core::permissions::catalog::Permission;

use super::super::{Actor, OpError, Tx};
use crate::models::permission_editor::PermissionRequester;
use crate::models::task::{TaskActivity, TaskCatalogEntry, TaskCatalogPage, TaskCursor};

const UNAVAILABLE: &str = "Task catalog access is unavailable";

/// Up to 100 tasks after `after`, common tasks first, with what the actor may
/// do there. Needs `TaskReadAll`; rates need `BillableRateReadAll`.
pub(crate) async fn page(
    tx: &mut Tx,
    actor: &Actor,
    activity: TaskActivity,
    after: Option<&TaskCursor>,
) -> Result<TaskCatalogPage, OpError> {
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
    let rate_currency = if can_edit_rates {
        Some(sqlx::query_scalar!(
            "SELECT upper(btrim(default_currency)) AS \"currency!\" FROM organizations WHERE id=$1",
            org_id,
        ).fetch_one(&mut **tx).await?)
    } else {
        None
    };
    let (active_only, archived_only) = (
        activity == TaskActivity::Active,
        activity == TaskActivity::Archived,
    );
    // Common tasks first, so the page splits into its two groups in order.
    let mut tasks = sqlx::query_as!(
        TaskCatalogEntry,
        "SELECT id,name,billable_default,is_default,active,
                CASE WHEN $2 THEN default_rate_cents END AS default_rate_cents,
                CASE WHEN $2 AND default_rate_cents IS NOT NULL
                     THEN default_rate_currency END AS default_rate_currency
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
        requester: PermissionRequester {
            org_id,
            user_id: actor.id(),
        },
        can_edit,
        can_read_rates,
        can_edit_rates,
        rate_currency,
        tasks,
        next_after,
    })
}
