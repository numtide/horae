//! The Projects page's budget progress, read through `ops`.

use chrono::NaiveDate;
use uuid::Uuid;

use super::ServerFnError;
use crate::models::ProjectBudgetOverview;

/// Batch overview projection. Authorization is checked against the current
/// database actor, not a role cached in the session or supplied by the caller.
pub(super) async fn progress_for_viewer(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    viewer_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<ProjectBudgetOverview>, ServerFnError> {
    use crate::ops::{self, Caller};
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = ops::projects::budget_progress(&mut read.tx, &read.actor, date).await;
    Ok(read.finish(result).await?)
}
