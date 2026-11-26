//! Writing the organization's task catalog: adding a task ([`create`]),
//! changing its details ([`update`]) and archiving or restoring it
//! ([`set_active`]). Projects link tasks through the project editor, as
//! Harvest links them through task assignments.
//!
//! Each needs `TaskWriteAll`, and is bound to the requester the page was
//! loaded for. Run [`create`] and [`update`] in `ops::begin_gated` with the
//! `Shared` gate, and [`set_active`] with `AccessChange`: tracking writers
//! hold the shared gate until their entry is inserted.

use horae_core::money::RateTemplate;
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::rates::RateEdit;
use uuid::Uuid;

use crate::models::Task;
use crate::models::permission_editor::PermissionRequester;
use crate::models::task::TaskRateEdit;
use crate::ops::{Actor, Done, OpError, Tx};
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, active_transition, task_payload};

const REQUESTER_CHANGED: &str = "Task requester has changed";
const EDITING_REQUIRED: &str = "Current task editing authority is required";
const NOT_FOUND: &str = "Task not found";

/// A task's details as its form submits them.
pub struct TaskDetails {
    pub name: String,
    pub billable_default: bool,
    pub is_default: bool,
    pub rate: TaskRateEdit,
}

impl TaskDetails {
    /// The trimmed name, which may not be empty.
    fn name(&self) -> Result<&str, OpError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(OpError::conflict("Task name cannot be empty"));
        }
        Ok(name)
    }
}

/// Adds a task to the catalog. Events: `task_created`, with the rate the
/// response withholds from someone without `BillableRateReadAll`.
pub async fn create(
    tx: &mut Tx,
    actor: &Actor,
    expected: PermissionRequester,
    details: &TaskDetails,
) -> Result<Done<Task>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    let name = details.name()?;
    authorize(actor)?;
    let org_id = actor.org_id();
    let amount = authorize_rate(actor, &details.rate)?;
    let task = sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, is_default)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        Uuid::now_v7(),
        org_id,
        name,
        details.billable_default,
        amount,
        details.is_default,
    )
    .fetch_one(&mut **tx)
    .await?;
    let event = AppEvent::TaskCreated {
        occurred_at: chrono::Utc::now(),
        org_id,
        task: task_payload(&task),
    };
    Ok(Done::new(redact_rate(actor, task)).with(event))
}

/// Changes a task's details. Events: `task_updated` when something changed.
pub async fn update(
    tx: &mut Tx,
    actor: &Actor,
    expected: PermissionRequester,
    task_id: Uuid,
    details: &TaskDetails,
) -> Result<Done<Task>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    let name = details.name()?;
    authorize(actor)?;
    let org_id = actor.org_id();
    let amount = authorize_rate(actor, &details.rate)?;
    let preserve = matches!(details.rate, TaskRateEdit::Preserve {});
    let before = lock(tx, org_id, task_id).await?;
    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks
            SET name = $3, billable_default = $4, is_default = $7,
                default_rate_cents = CASE WHEN $5 THEN default_rate_cents ELSE $6 END
          WHERE id = $1 AND org_id = $2
            AND (name, billable_default, is_default, default_rate_cents)
                IS DISTINCT FROM ($3, $4, $7,
                    CASE WHEN $5 THEN default_rate_cents ELSE $6 END)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        name,
        details.billable_default,
        preserve,
        amount,
        details.is_default,
    )
    .fetch_optional(&mut **tx)
    .await?;
    // The event carries the committed rate; the response must not inherit
    // that authority over a hidden global rate.
    let event = task.as_ref().map(|task| AppEvent::TaskUpdated {
        occurred_at: chrono::Utc::now(),
        org_id,
        task: task_payload(task),
    });
    let mut done = Done::new(redact_rate(actor, task.unwrap_or(before)));
    done.events.extend(event);
    Ok(done)
}

/// Archives or restores a task. Archiving also deactivates its project
/// links, and is refused while a timer runs on it. Events: `task_deactivated`
/// or `task_reactivated` when the status changed.
pub async fn set_active(
    tx: &mut Tx,
    actor: &Actor,
    expected: PermissionRequester,
    task_id: Uuid,
    active: bool,
) -> Result<Done<Task>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    authorize(actor)?;
    let org_id = actor.org_id();
    if !active {
        // Link updates invalidate editors through their parent project trigger.
        // Take every parent before the task/link locks, in stable order.
        sqlx::query_scalar!(
            "SELECT p.id FROM projects p JOIN project_tasks pt ON pt.project_id=p.id
             JOIN tasks t ON t.id=pt.task_id AND t.org_id=p.org_id
             WHERE p.org_id=$1 AND t.id=$2 ORDER BY p.id FOR NO KEY UPDATE OF p",
            org_id,
            task_id,
        )
        .fetch_all(&mut **tx)
        .await?;
    }
    let before = lock(tx, org_id, task_id).await?;
    if !active {
        let running = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM time_entries
                WHERE org_id=$1 AND task_id=$2 AND is_running) AS "running!""#,
            org_id,
            task_id,
        )
        .fetch_one(&mut **tx)
        .await?;
        if running {
            // Do not reveal an entry or its owner to a task-only administrator.
            return Err(OpError::conflict(
                "A running timer prevents archiving this task",
            ));
        }
    }
    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks SET active = $3
          WHERE id = $1 AND org_id = $2
            AND active IS DISTINCT FROM $3
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        active,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let transition = task
        .as_ref()
        .and_then(|updated| active_transition(Some(before.active), updated.active));
    if !active {
        sqlx::query!(
            "UPDATE project_tasks pt SET active=false FROM projects p
             WHERE pt.project_id=p.id AND p.org_id=$1 AND pt.task_id=$2 AND pt.active",
            org_id,
            task_id,
        )
        .execute(&mut **tx)
        .await?;
    }
    let task = task.unwrap_or(before);
    let event = transition.map(|transition| {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        match transition {
            ActiveTransition::Reactivated => AppEvent::TaskReactivated {
                occurred_at,
                org_id,
                task,
            },
            ActiveTransition::Deactivated => AppEvent::TaskDeactivated {
                occurred_at,
                org_id,
                task,
            },
        }
    });
    let mut done = Done::new(redact_rate(actor, task));
    done.events.extend(event);
    Ok(done)
}

fn authorize(actor: &Actor) -> Result<(), OpError> {
    if actor.has(Permission::TaskWriteAll) {
        Ok(())
    } else {
        Err(OpError::forbidden(EDITING_REQUIRED))
    }
}

/// Hides the global rate from someone without `BillableRateReadAll`.
fn redact_rate(actor: &Actor, mut task: Task) -> Task {
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    task
}

/// The rate and currency to store, refused without `BillableRateWriteAll`
/// unless the rate is left as it is. A new rate is in the organization's
/// currency, which must still be the one the form showed.
/// The template to store: `None` for preserve, clear and a zero, which is no
/// rate.
fn authorize_rate(actor: &Actor, rate: &TaskRateEdit) -> Result<Option<i64>, OpError> {
    let intent = match rate {
        TaskRateEdit::Preserve {} => RateEdit::Unchanged,
        TaskRateEdit::Clear {} => RateEdit::Reset,
        TaskRateEdit::Set { amount_cents } => RateEdit::Set(*amount_cents),
    };
    intent
        .authorize(actor.has(Permission::BillableRateWriteAll))
        .map_err(|_| {
            OpError::forbidden("Current global billable-rate editing authority is required")
        })?;
    let TaskRateEdit::Set { amount_cents } = rate else {
        return Ok(None);
    };
    if *amount_cents < 0 {
        return Err(OpError::bad_input("Task rate must not be negative"));
    }
    Ok(RateTemplate::from_hundredths(*amount_cents).map(RateTemplate::hundredths))
}

/// Locks the task, shared between detail edits and status changes, including
/// no-ops.
async fn lock(tx: &mut Tx, org_id: Uuid, task_id: Uuid) -> Result<Task, OpError> {
    sqlx::query_as!(
        Task,
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks WHERE id = $1 AND org_id = $2 FOR UPDATE",
        task_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))
}
