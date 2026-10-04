//! Person-bound Timesheet operations; legacy shell commands remain independent.

use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::models::scoped_time::{
    TimesheetCommand, TimesheetTrackingOption, TimesheetWriteContext,
};

pub(super) async fn tracking(
    _pool: &PgPool,
    _org_id: Uuid,
    _actor_id: Uuid,
    _context: &TimesheetWriteContext,
) -> Result<Vec<TimesheetTrackingOption>, ServerFnError> {
    Err(forbidden("Timesheet command integration is unavailable"))
}

pub(super) async fn apply(
    _pool: &PgPool,
    _org_id: Uuid,
    _actor_id: Uuid,
    _context: &TimesheetWriteContext,
    _command: TimesheetCommand,
) -> Result<(), ServerFnError> {
    Err(forbidden("Timesheet command integration is unavailable"))
}
