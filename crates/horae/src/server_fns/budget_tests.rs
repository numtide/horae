//! The budget reader with the adapter's errors, for the project tests that
//! compare status codes. The budget tests themselves live in
//! `ops::projects` and `notifications`.

use sqlx::PgPool;
use uuid::Uuid;

/// `list_project_budget_progress` on a pinned `date`.
pub(crate) async fn progress_for_viewer(
    pool: &PgPool,
    org_id: Uuid,
    viewer_id: Uuid,
    date: chrono::NaiveDate,
    expected_requester: Option<crate::models::permission_editor::PermissionRequester>,
) -> Result<Vec<crate::models::ProjectBudgetOverview>, super::ServerFnError> {
    use super::op_error;
    use crate::ops::{self, Caller};
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id))
        .await
        .map_err(op_error)?;
    let result =
        ops::projects::budget_progress_on(&mut read.tx, &read.actor, date, expected_requester)
            .await;
    read.finish(result).await.map_err(op_error)
}
