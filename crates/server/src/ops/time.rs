//! Time entries: the timer, the project/task picker and the Timesheet
//! commands, which act on one selected person's time.

use std::collections::BTreeSet;

use chrono::{NaiveDate, Timelike, Utc};
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::oauth::Scope;
use horae_core::types::EntryState;
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::TimeEntry;
use crate::models::scoped_time::{
    TimesheetCommand, TimesheetEntryInput, TimesheetTrackingOption, TimesheetWriteContext,
};
use crate::models::time_entry::TimeEntryContext;
use crate::plugin::AppEvent;
use crate::plugin::event::time_entry_payload;

const ACTIVE_ACCOUNT_REQUIRED: &str = "Active account required";
const NO_RUNNING_TIMER: &str = "No running timer found for this entry";
const TIME_WRITE_REQUIRED: &str = "Current time-write scope required";

fn require_active(actor: &Actor) -> Result<(), OpError> {
    if actor.is_active() {
        Ok(())
    } else {
        Err(OpError::forbidden(ACTIVE_ACCOUNT_REQUIRED))
    }
}

/// Holds the person's time-write lock, which a timesheet submission takes
/// exclusively, so new time cannot slip into a week being submitted.
async fn begin_own_time_write(tx: &mut Tx, actor: &Actor) -> Result<(), OpError> {
    require_active(actor)?;
    crate::db::lock_time_entry_write(tx, actor.id()).await?;
    Ok(())
}

/// A person's total minutes per day, read and locked ahead of a write so
/// [`ensure_daily_limit`] can tell whether the write grew them.
async fn day_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    dates: &[NaiveDate],
) -> Result<Vec<(NaiveDate, i64)>, OpError> {
    Ok(crate::db::daily_minutes_totals(tx, user_id, dates).await?)
}

/// Refuse a write that grows the person's day past 240 hours, as Harvest does
/// (409 "Daily hours cannot exceed 240."). `before` is [`day_totals`] from
/// ahead of the write; a day already over the limit may still be lowered.
async fn ensure_daily_limit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    before: &[(NaiveDate, i64)],
) -> Result<(), OpError> {
    let dates: Vec<_> = before.iter().map(|(day, _)| *day).collect();
    let after = day_totals(tx, user_id, &dates).await?;
    for ((_, before), (_, after)) in before.iter().zip(&after) {
        horae_core::duration::check_daily_change(*before, *after)
            .map_err(|error| OpError::conflict(error.to_string()))?;
    }
    Ok(())
}

/// What a stopping timer records.
struct Stopped {
    minutes: i32,
    start_minute: Option<i32>,
    notice: Option<String>,
}

/// Stops a timer that started at `started_at` on an entry holding `minutes`
/// and `start_minute`. The elapsed time is added to the minutes, so a restarted
/// entry keeps what it had, and clamped to what is left of the day, so a stop
/// never fails and the timer is never trapped running; the notice says what
/// was cut. A fresh timer starts at the clock time it started (D9, in UTC like
/// `spent_date`); a restarted one keeps its start. An entry that would cross
/// midnight stays untimed.
async fn stop_running(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    spent_date: NaiveDate,
    minutes: i32,
    start_minute: Option<i32>,
    started_at: chrono::DateTime<Utc>,
) -> Result<Stopped, OpError> {
    use horae_core::duration::{clamp_to_daily_allowance, format_hhmm, minutes_between};
    use horae_core::time_of_day::{DAY_MINUTES, SNAP_STEP, snap};
    // Floored to the minute, with no artificial minimum, so totals stay exact.
    let elapsed = minutes_between(started_at, Utc::now());
    let [(_, total)] = day_totals(tx, user_id, &[spent_date]).await?[..] else {
        return Err(OpError::Internal("Day total unavailable".into()));
    };
    let kept = clamp_to_daily_allowance(elapsed, total);
    // The day, this entry included, stays within MAX_DAILY_MINUTES, so it fits.
    let stopped = minutes + kept as i32;
    let notice = (kept < elapsed).then(|| {
        format!(
            "Daily hours cannot exceed 240. The timer was stopped at {}; {} over the limit was not recorded.",
            format_hhmm(stopped.into()),
            format_hhmm((elapsed - kept).into()),
        )
    });
    let start = if minutes == 0 {
        let clock = started_at.hour() as i32 * 60 + started_at.minute() as i32;
        Some(snap(clock, SNAP_STEP).min(i32::from(DAY_MINUTES) - 1))
    } else {
        start_minute
    };
    Ok(Stopped {
        minutes: stopped,
        start_minute: start.filter(|start| start + stopped <= i32::from(DAY_MINUTES)),
        notice,
    })
}

/// Preserve archived history while enforcing current task grants. Lock both
/// the restriction and a matching grant so revocation cannot race the write.
async fn lock_task_access(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
) -> Result<(), OpError> {
    let restricted = sqlx::query_scalar!(
        "SELECT s.restricted FROM project_task_settings s
         JOIN users u ON u.org_id = s.org_id AND u.id = $3
         WHERE s.project_id = $1 AND s.task_id = $2 FOR SHARE OF s",
        project_id,
        task_id,
        user_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or(false);
    if restricted {
        let grant = sqlx::query_scalar!(
            "SELECT id FROM project_task_members
             WHERE project_id = $1 AND task_id = $2 AND user_id = $3 FOR SHARE",
            project_id,
            task_id,
            user_id,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if grant.is_none() {
            return Err(OpError::conflict(
                "Task access is restricted. Refresh the timesheet and try again.",
            ));
        }
    }
    Ok(())
}

/// The time-entry lifecycle events the plugin bus announces. Exhaustive on
/// purpose: adding an `AppEvent::TimeEntry…` variant forces the mapping in
/// [`Change::event`] to be spelled out.
#[derive(Clone, Copy)]
enum Change {
    Created,
    Stopped,
    Updated,
    Deleted,
}

impl Change {
    fn event(self, entry: &TimeEntry) -> AppEvent {
        let occurred_at = Utc::now();
        let org_id = entry.org_id;
        let time_entry = time_entry_payload(entry);
        match self {
            Self::Created => AppEvent::TimeEntryCreated {
                occurred_at,
                org_id,
                time_entry,
            },
            Self::Stopped => AppEvent::TimeEntryStopped {
                occurred_at,
                org_id,
                time_entry,
            },
            Self::Updated => AppEvent::TimeEntryUpdated {
                occurred_at,
                org_id,
                time_entry,
            },
            Self::Deleted => AppEvent::TimeEntryDeleted {
                occurred_at,
                org_id,
                time_entry,
            },
        }
    }
}

/// The actor's active, assigned project/task combinations; no rates.
pub async fn contexts(tx: &mut Tx, actor: &Actor) -> Result<Vec<TimeEntryContext>, OpError> {
    require_active(actor)?;
    Ok(sqlx::query_as!(
        TimeEntryContext,
        r#"SELECT project_id as "project_id!", task_id as "task_id!", billable as "billable!"
           FROM time_entry_contexts WHERE user_id = $1 ORDER BY project_id, task_id"#,
        actor.id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// A started timer, and the one it stopped with its daily-limit notice.
#[derive(Debug)]
pub struct Started {
    pub entry: TimeEntry,
    pub stopped: Option<(TimeEntry, Option<String>)>,
}

/// Starts a timer on the actor's today, in their own zone, stopping the
/// actor's running timer first as [`stop_timer`] does (021 D-6). Only one timer
/// may run at a time per person, enforced by the `one_running_timer_per_user`
/// partial unique index; a start that loses a race is reported as a conflict.
///
/// Eligibility and effective billability are resolved in the insert's snapshot,
/// not in a pre-check separated from the write.
pub async fn start_timer(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    task_id: Uuid,
    notes: Option<&str>,
) -> Result<Done<Started>, OpError> {
    begin_own_time_write(tx, actor).await?;
    // As on the Timesheet: own time needs the own or the everyone time-write grant.
    if !(actor.has(Permission::TimeWriteOwn) || actor.has(Permission::TimeWriteAll)) {
        return Err(OpError::forbidden(TIME_WRITE_REQUIRED));
    }
    let user_id = actor.id();
    let running = sqlx::query_scalar!(
        "SELECT id FROM time_entries WHERE user_id = $1 AND is_running",
        user_id
    )
    .fetch_optional(&mut **tx)
    .await?;
    let mut events = Vec::new();
    let stopped = match running {
        Some(running) => match stop_timer(tx, actor, running).await {
            Ok(done) => {
                events = done.events;
                Some(done.value)
            }
            // Another write stopped it first; the insert below still guards.
            Err(OpError::NotFound(_)) => None,
            Err(error) => return Err(error),
        },
        None => None,
    };
    let spent_date = crate::timezone::today_for_person(&mut **tx, user_id).await?;
    lock_task_access(tx, user_id, project_id, task_id).await?;
    let day = day_totals(tx, user_id, &[spent_date]).await?;
    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes,
            billable, is_running, started_at, state)
           SELECT $1, org_id, user_id, project_id, task_id, $5, 0, $6,
                  billable, true, now(), 'open'
           FROM time_entry_contexts
           WHERE user_id = $2 AND project_id = $3 AND task_id = $4
           ON CONFLICT (user_id) WHERE is_running DO NOTHING
           RETURNING id, org_id, user_id, project_id, task_id,
                     spent_date as "spent_date: chrono::NaiveDate",
                     minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                     started_at as "started_at: chrono::DateTime<chrono::Utc>",
                     state as "state: EntryState", invoice_id,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>",
                     updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        Uuid::now_v7(), user_id, project_id, task_id, spent_date as chrono::NaiveDate, notes,
    ).fetch_optional(&mut **tx).await?
    .ok_or_else(|| OpError::conflict("Project/task is unavailable or not assigned, or a timer is already running."))?;
    ensure_daily_limit(tx, user_id, &day).await?;
    events.push(Change::Created.event(&entry));
    Ok(Done {
        value: Started { entry, stopped },
        events,
    })
}

/// Stops the actor's running timer and records the elapsed minutes, clamped to
/// what is left of the day; the notice, when present, says how much the daily
/// limit cut. The stopped entry carries the project whose budget the adapter
/// checks after the commit.
pub async fn stop_timer(
    tx: &mut Tx,
    actor: &Actor,
    entry_id: Uuid,
) -> Result<Done<(TimeEntry, Option<String>)>, OpError> {
    // Stopping an own running timer remains possible after task access is revoked.
    begin_own_time_write(tx, actor).await?;
    actor.require_scope(Scope::TimeWrite)?;
    let user_id = actor.id();

    // The row is locked before the day, as on every other write path, so
    // racing stops queue instead of deadlocking and the clamp reads current
    // minutes.
    let row = sqlx::query!(
        r#"SELECT started_at as "started_at: chrono::DateTime<chrono::Utc>",
                  state as "state: EntryState",
                  spent_date as "spent_date: chrono::NaiveDate", minutes, start_minute
               FROM time_entries
               WHERE id = $1 AND user_id = $2 AND is_running = true FOR UPDATE"#,
        entry_id,
        user_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    // A locked (submitted/approved/invoiced) entry must not receive minutes:
    // its rounded_minutes were persisted at lock time, so a write here would
    // put time on the books that billing never sees.
    if row.state != EntryState::Open {
        return Err(OpError::conflict(
            "This entry was locked while its timer ran and can no longer be stopped. \
             Ask a manager to reject the submission first.",
        ));
    }

    let started_at = row
        .started_at
        .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    let Stopped {
        minutes,
        start_minute,
        notice,
    } = stop_running(
        tx,
        user_id,
        row.spent_date,
        row.minutes,
        row.start_minute,
        started_at,
    )
    .await?;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET is_running = false,
             minutes = $3,
             start_minute = $4,
             started_at = NULL,
             notified_long_running_at = NULL
         WHERE id = $1 AND user_id = $2 AND is_running = true AND state = $5
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        minutes,
        start_minute,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    let event = Change::Stopped.event(&entry);
    Ok(Done::new((entry, notice)).with(event))
}

/// The actor's running timer, if any.
pub async fn current_timer(tx: &mut Tx, actor: &Actor) -> Result<Option<TimeEntry>, OpError> {
    require_active(actor)?;
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
         FROM time_entries
         WHERE user_id = $1 AND is_running = true
         LIMIT 1"#,
        actor.id(),
    )
    .fetch_optional(&mut **tx)
    .await?)
}

// ── Timesheet commands ──────────────────────────────────────────────────────

const ACTIVE_ACCOUNTS_REQUIRED: &str = "Active same-organization accounts required";
const TIMESHEET_UNAVAILABLE: &str = "Timesheet is unavailable";

/// What a Timesheet command changed besides its events: the projects whose
/// budgets the adapter checks after the commit, and a notice for the person,
/// e.g. when the daily limit clamped a stopped timer.
pub struct TimesheetChange {
    pub projects: BTreeSet<Uuid>,
    pub notice: Option<String>,
    /// The entry the command created or changed, as saved; `None` for a
    /// deletion or a reorder.
    pub entry: Option<TimeEntry>,
}

/// Changes to one entry: a `None` field keeps its value, and `Some(None)`
/// clears an optional one.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryPatch {
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub spent_date: Option<NaiveDate>,
    pub minutes: Option<i32>,
    pub notes: Option<Option<String>>,
    pub billable: Option<bool>,
    pub start_minute: Option<Option<i32>>,
}

impl EntryPatch {
    /// The entry's values with these changes, as an Update would send them.
    fn merge(self, entry: &TimeEntry) -> TimesheetEntryInput {
        TimesheetEntryInput {
            project_id: self.project_id.unwrap_or(entry.project_id),
            task_id: self.task_id.unwrap_or(entry.task_id),
            spent_date: self.spent_date.unwrap_or(entry.spent_date),
            minutes: self.minutes.unwrap_or(entry.minutes),
            notes: self.notes.unwrap_or_else(|| entry.notes.clone()),
            billable: self.billable.unwrap_or(entry.billable),
            start_minute: self.start_minute.unwrap_or(entry.start_minute),
        }
    }
}

/// What [`write`] applies: a Timesheet command, or a patch merged into the
/// entry once it is locked.
enum Write {
    Command(TimesheetCommand),
    Patch { entry_id: Uuid, patch: EntryPatch },
}

/// What the actor may write of the selected person's time.
struct Authority {
    org: Uuid,
    subject: Uuid,
    owner: bool,
    whole_subject: bool,
    projects: Vec<Uuid>,
    /// The actor's own Administrator identity when they are the subject, so
    /// an OAuth actor, which never has it, does not track on every project;
    /// `None` reads another subject's stored identity.
    subject_administrator: Option<bool>,
}

impl Authority {
    fn permits(&self, project: Uuid) -> bool {
        self.whole_subject || self.projects.contains(&project)
    }
}

/// The actor's write scope over the context's person, from the stored grants
/// and the management assignments, locked for the rest of the transaction.
async fn authorize(
    tx: &mut Tx,
    actor: &Actor,
    context: &TimesheetWriteContext,
) -> Result<Authority, OpError> {
    actor.expect_requester(
        Some(context.expected_requester),
        "Timesheet identity changed; refresh the page",
    )?;
    let org = actor.org_id();
    let owner = actor.id() == context.subject_id;
    // An active actor's own row is already held: `Actor::load` locked it.
    if !actor.is_active() || (!owner && !subject_is_active(tx, org, context.subject_id).await?) {
        return Err(OpError::forbidden(ACTIVE_ACCOUNTS_REQUIRED));
    }
    let mut authority = Authority {
        org,
        subject: context.subject_id,
        owner,
        whole_subject: actor.has(Permission::TimeWriteAll)
            || (owner && actor.has(Permission::TimeWriteOwn)),
        projects: Vec::new(),
        subject_administrator: owner.then(|| actor.administrator()),
    };
    if actor.has(Permission::TimeWriteManaged) {
        let managed = sqlx::query_scalar!(
            "SELECT managed_user_id FROM person_management_assignments WHERE org_id=$1 AND manager_id=$2 AND managed_user_id=$3 FOR SHARE",
            org, actor.id(), context.subject_id
        ).fetch_optional(&mut **tx).await?;
        authority.whole_subject |= managed.is_some();
        authority.projects = sqlx::query_scalar!(
            "SELECT project_id FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2 ORDER BY project_id FOR SHARE",
            org, actor.id()
        ).fetch_all(&mut **tx).await?;
    }
    Ok(authority)
}

/// Locks the selected person's activity for the rest of the transaction.
async fn subject_is_active(tx: &mut Tx, org: Uuid, subject: Uuid) -> Result<bool, OpError> {
    let found = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id=$1 AND id=$2 AND active FOR SHARE",
        org,
        subject
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(found.is_some())
}

/// The subject's trackable pairs, as `time_entry_contexts` defines them: an
/// assignment, or the subject's Administrator identity, admits a project.
async fn choices(
    tx: &mut Tx,
    authority: &Authority,
    pair: Option<(Uuid, Uuid)>,
) -> Result<Vec<TimesheetTrackingOption>, OpError> {
    Ok(sqlx::query_as!(TimesheetTrackingOption,
        r#"SELECT p.id AS project_id,p.name AS project_name,t.id AS task_id,t.name AS task_name,
                  (p.project_type <> 'non_billable' AND pt.billable) AS "billable!"
           FROM users u JOIN projects p ON p.org_id=u.org_id
           JOIN clients c ON c.id=p.client_id AND c.org_id=u.org_id
           JOIN project_tasks pt ON pt.project_id=p.id
           JOIN tasks t ON t.id=pt.task_id AND t.org_id=u.org_id
           WHERE u.org_id=$1 AND u.id=$2 AND u.active AND p.active AND c.active AND t.active
             AND pt.active
             AND ($3 OR p.id=ANY($4))
             AND (EXISTS (SELECT 1 FROM assignments a WHERE a.user_id=u.id AND a.project_id=p.id)
                 OR COALESCE($7, EXISTS (SELECT 1 FROM person_permission_states state
                     WHERE state.org_id=u.org_id AND state.user_id=u.id AND state.is_administrator)))
             AND NOT EXISTS (SELECT 1 FROM project_task_settings s
                 WHERE s.org_id=u.org_id AND s.project_id=p.id AND s.task_id=t.id AND s.restricted
                 AND NOT EXISTS (SELECT 1 FROM project_task_members m
                     WHERE m.org_id=u.org_id AND m.project_id=p.id AND m.task_id=t.id AND m.user_id=u.id))
             AND ($5::uuid IS NULL OR p.id=$5) AND ($6::uuid IS NULL OR t.id=$6)
           ORDER BY p.name,p.id,t.name,t.id FOR SHARE OF p,c,pt,t"#,
        authority.org,authority.subject,authority.whole_subject,&authority.projects,
        pair.map(|p| p.0),pair.map(|p| p.1),authority.subject_administrator
    ).fetch_all(&mut **tx).await?)
}

/// The project/task pairs the actor may track for the context's person: ones
/// the person can track and the actor may write. A read: for their own time
/// it follows [`Actor::may_track_own_time`], so an OAuth token without
/// `time:write` still lists them; writes check through [`authorize`] alone.
pub async fn tracking(
    tx: &mut Tx,
    actor: &Actor,
    context: &TimesheetWriteContext,
) -> Result<Vec<TimesheetTrackingOption>, OpError> {
    let mut authority = authorize(tx, actor, context).await?;
    authority.whole_subject |= authority.owner && actor.may_track_own_time();
    choices(tx, &authority, None).await
}

/// Whether the subject can track `project`/`task` under the actor's scope, and
/// if so the pair's effective billability.
async fn eligible(
    tx: &mut Tx,
    authority: &Authority,
    project: Uuid,
    task: Uuid,
) -> Result<bool, OpError> {
    if !authority.permits(project) {
        return Err(OpError::forbidden(TIME_WRITE_REQUIRED));
    }
    sqlx::query_scalar!(
        "SELECT id FROM assignments WHERE user_id=$1 AND project_id=$2 FOR SHARE",
        authority.subject,
        project
    )
    .fetch_optional(&mut **tx)
    .await?;
    lock_task_access(tx, authority.subject, project, task).await?;
    choices(tx, authority, Some((project, task)))
        .await?
        .first()
        .map(|option| option.billable)
        .ok_or_else(|| {
            OpError::conflict("The selected person cannot currently track this project/task")
        })
}

async fn entries(
    tx: &mut Tx,
    authority: &Authority,
    ids: &[Uuid],
) -> Result<Vec<TimeEntry>, OpError> {
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT id,org_id,user_id,project_id,task_id,
            spent_date AS "spent_date: chrono::NaiveDate",minutes,start_minute,sort_order,
            rounded_minutes,notes,billable,is_running,
            started_at AS "started_at: chrono::DateTime<chrono::Utc>",
            state AS "state: EntryState",invoice_id,
            created_at AS "created_at: chrono::DateTime<chrono::Utc>",
            updated_at AS "updated_at: chrono::DateTime<chrono::Utc>"
           FROM time_entries WHERE org_id=$1 AND user_id=$2 AND id=ANY($3)
           ORDER BY id FOR UPDATE"#,
        authority.org,
        authority.subject,
        ids
    )
    .fetch_all(&mut **tx)
    .await?)
}

const NEGATIVE_MINUTES: &str = "Minutes cannot be negative";

/// Validate an optional start time (minutes since midnight, 0..=1439) and clamp
/// the duration so the entry never crosses midnight (Constitution: exactness).
/// Returns the possibly-clamped minutes and the validated start.
fn normalize_start(minutes: i32, start_minute: Option<i32>) -> Result<(i32, Option<i32>), OpError> {
    // One entry past the day's limit can never fit; refuse it before the
    // database trigger turns it into an opaque error.
    horae_core::duration::check_daily_total(minutes.into())
        .map_err(|error| OpError::conflict(error.to_string()))?;
    match start_minute {
        None => Ok((minutes, None)),
        Some(sm) if (0..=1439).contains(&sm) => {
            // Snap the start to the grid so every write path (drag-move/resize,
            // typed times) lands on a tidy boundary (FR-008); snap first, then
            // clamp the duration against the snapped start.
            let snapped = horae_core::time_of_day::snap(sm, horae_core::time_of_day::SNAP_STEP)
                .clamp(0, 1439);
            let clamped =
                horae_core::time_of_day::clamp_to_day(snapped as u16, minutes.max(0) as u32) as i32;
            Ok((clamped, Some(snapped)))
        }
        Some(_) => Err(OpError::invalid(
            "start_minute",
            "start time must be within the day (0..=1439)",
        )),
    }
}

fn normalize(input: &mut TimesheetEntryInput) -> Result<(), OpError> {
    if input.minutes < 0 {
        return Err(OpError::invalid("minutes", NEGATIVE_MINUTES));
    }
    (input.minutes, input.start_minute) = normalize_start(input.minutes, input.start_minute)?;
    Ok(())
}

/// Applies one Timesheet command to the context's person, atomically over
/// every entry it names. Client expectations never grant authority: the
/// actor's current write scope is checked against each entry's source and
/// destination project.
pub async fn apply(
    tx: &mut Tx,
    actor: &Actor,
    context: &TimesheetWriteContext,
    command: TimesheetCommand,
) -> Result<Done<TimesheetChange>, OpError> {
    write(tx, actor, context, Write::Command(command)).await
}

/// Changes the named fields of one of the context's person's entries. The
/// changes are merged into the entry as it is once locked, then checked and
/// saved as the Timesheet's Update, so a concurrent edit of another field is
/// kept.
pub async fn patch(
    tx: &mut Tx,
    actor: &Actor,
    context: &TimesheetWriteContext,
    entry_id: Uuid,
    patch: EntryPatch,
) -> Result<Done<TimesheetChange>, OpError> {
    write(tx, actor, context, Write::Patch { entry_id, patch }).await
}

async fn write(
    tx: &mut Tx,
    actor: &Actor,
    context: &TimesheetWriteContext,
    write: Write,
) -> Result<Done<TimesheetChange>, OpError> {
    let authority = authorize(tx, actor, context).await?;
    let org = authority.org;
    crate::db::lock_time_entry_write(tx, authority.subject).await?;
    let mut ids = match &write {
        Write::Patch { entry_id, .. }
        | Write::Command(
            TimesheetCommand::Update { entry_id, .. }
            | TimesheetCommand::StopTimer { entry_id }
            | TimesheetCommand::RestartTimer { entry_id }
            | TimesheetCommand::Reschedule { entry_id, .. },
        ) => vec![*entry_id],
        Write::Command(
            TimesheetCommand::Delete { entry_ids } | TimesheetCommand::Reorder { entry_ids, .. },
        ) => entry_ids.clone(),
        Write::Command(_) => Vec::new(),
    };
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(OpError::invalid("entry_ids", "Duplicate entry identifiers"));
    }
    // Starting or restarting stops the subject's running timer first (021 D-6),
    // so that entry is locked and checked with the others.
    let starts = matches!(
        write,
        Write::Command(TimesheetCommand::StartTimer { .. } | TimesheetCommand::RestartTimer { .. })
    );
    if starts {
        let running = sqlx::query_scalar!(
            "SELECT id FROM time_entries WHERE org_id=$1 AND user_id=$2 AND is_running",
            org,
            authority.subject
        )
        .fetch_optional(&mut **tx)
        .await?;
        ids.extend(running.filter(|running| !ids.contains(running)));
    }
    // Approval writers acquire coverage before entries. Keep that order, and
    // recheck source dates after locking rows to detect a concurrent move.
    let mut dates = sqlx::query_scalar!(
        r#"SELECT spent_date AS "spent_date: chrono::NaiveDate" FROM time_entries
           WHERE org_id=$1 AND user_id=$2 AND id=ANY($3)"#,
        org,
        authority.subject,
        &ids
    )
    .fetch_all(&mut **tx)
    .await?;
    // A timer is filed on the subject's own today, not the actor's.
    let today = crate::timezone::today_for_person(&mut **tx, authority.subject).await?;
    match &write {
        Write::Command(
            TimesheetCommand::Create { entry } | TimesheetCommand::Update { entry, .. },
        ) => dates.push(entry.spent_date),
        Write::Command(TimesheetCommand::StartTimer { .. }) => dates.push(today),
        Write::Command(
            TimesheetCommand::Reschedule { spent_date, .. }
            | TimesheetCommand::Reorder { spent_date, .. },
        ) => dates.push(*spent_date),
        Write::Patch { patch, .. } => dates.extend(patch.spent_date),
        Write::Command(_) => {}
    }
    let coverage = sqlx::query_scalar!(
        "SELECT id FROM approvals WHERE org_id=$1 AND user_id=$2
         AND EXISTS (SELECT 1 FROM unnest($3::date[]) d WHERE d BETWEEN period_start AND period_end)
         ORDER BY id FOR SHARE",
        org,
        authority.subject,
        &dates as &[NaiveDate]
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut before = entries(tx, &authority, &ids).await?;
    if before.len() != ids.len()
        || before
            .iter()
            .any(|entry| !dates.contains(&entry.spent_date))
    {
        return Err(OpError::conflict(
            "Entries changed or are unavailable; refresh the timesheet",
        ));
    }
    // Temporary integration boundary: FR-019 requires submitted editing, but
    // weekly coverage and submit-time rounding must be reconciled first.
    if !coverage.is_empty()
        || before
            .iter()
            .any(|entry| entry.state != EntryState::Open || entry.invoice_id.is_some())
    {
        return Err(OpError::conflict(
            "Approval or billing coverage requires the approval editing integration",
        ));
    }
    let command = match write {
        Write::Command(command) => command,
        // The one entry is locked: merge into what it holds now.
        Write::Patch { entry_id, patch } => {
            let entry = before
                .first()
                .ok_or_else(|| OpError::conflict("Entry set changed"))?;
            TimesheetCommand::Update {
                entry_id,
                entry: patch.merge(entry),
            }
        }
    };
    if let TimesheetCommand::RestartTimer { entry_id } = &command {
        let entry = before
            .iter()
            .find(|entry| entry.id == *entry_id)
            .ok_or_else(|| OpError::conflict("Entry set changed"))?;
        horae_core::time_entry_timer::check_restart(
            entry.is_running,
            entry.state,
            entry.spent_date,
            today,
        )
        .map_err(|refusal| OpError::conflict(refusal.to_string()))?;
    }
    for entry in &before {
        if !authority.permits(entry.project_id) {
            return Err(OpError::forbidden(TIME_WRITE_REQUIRED));
        }
        // An owner may always stop their own timer, even after losing access.
        let stopping =
            matches!(command, TimesheetCommand::StopTimer { .. }) || (starts && entry.is_running);
        if !(stopping && authority.owner) {
            eligible(tx, &authority, entry.project_id, entry.task_id).await?;
        }
    }
    let mut events = Vec::new();
    let mut projects = BTreeSet::new();
    let mut notice = None;
    let mut changed = None;
    let target = match &command {
        TimesheetCommand::Update { entry_id, .. }
        | TimesheetCommand::StopTimer { entry_id }
        | TimesheetCommand::RestartTimer { entry_id }
        | TimesheetCommand::Reschedule { entry_id, .. } => Some(*entry_id),
        _ => None,
    };
    let day = day_totals(tx, authority.subject, &dates).await?;
    if starts {
        for row in before.iter_mut().filter(|row| row.is_running) {
            notice = stop(tx, authority.subject, row).await?;
            save(tx, row).await?;
            projects.insert(row.project_id);
            events.push(Change::Stopped.event(row));
        }
    }
    match command {
        TimesheetCommand::Create { entry } => {
            let created = create(tx, &authority, entry, false).await?;
            projects.insert(created.project_id);
            events.push(Change::Created.event(&created));
            changed = Some(created);
        }
        TimesheetCommand::StartTimer {
            project_id,
            task_id,
            notes,
        } => {
            let created = create(
                tx,
                &authority,
                TimesheetEntryInput {
                    project_id,
                    task_id,
                    spent_date: today,
                    minutes: 0,
                    notes,
                    billable: true,
                    start_minute: None,
                },
                true,
            )
            .await?;
            events.push(Change::Created.event(&created));
            changed = Some(created);
        }
        TimesheetCommand::Delete { .. } => {
            sqlx::query!(
                "DELETE FROM time_entries WHERE org_id=$1 AND user_id=$2 AND id=ANY($3)",
                org,
                authority.subject,
                &ids
            )
            .execute(&mut **tx)
            .await?;
            projects.extend(before.iter().map(|entry| entry.project_id));
            events.extend(before.iter().map(|entry| Change::Deleted.event(entry)));
        }
        command => {
            for row in &mut before {
                let original = row.clone();
                let mut change = Change::Updated;
                match &command {
                    TimesheetCommand::Update { entry, .. } => {
                        let mut input = entry.clone();
                        normalize(&mut input)?;
                        let billable =
                            eligible(tx, &authority, input.project_id, input.task_id).await?;
                        row.billable = billable && input.billable;
                        row.project_id = input.project_id;
                        row.task_id = input.task_id;
                        row.spent_date = input.spent_date;
                        row.minutes = input.minutes;
                        row.start_minute = input.start_minute;
                        row.notes = input.notes;
                    }
                    TimesheetCommand::Reschedule {
                        spent_date,
                        start_minute,
                        minutes,
                        ..
                    } => {
                        if *minutes < 0 {
                            return Err(OpError::invalid("minutes", NEGATIVE_MINUTES));
                        }
                        (row.minutes, row.start_minute) =
                            normalize_start(*minutes, Some(*start_minute))?;
                        row.spent_date = *spent_date;
                    }
                    TimesheetCommand::Reorder {
                        spent_date,
                        entry_ids,
                    } => {
                        if row.start_minute.is_some()
                            || (row.is_running && row.spent_date != *spent_date)
                        {
                            return Err(OpError::conflict(
                                "Only untimed entries can be reordered; running timers cannot change day",
                            ));
                        }
                        row.spent_date = *spent_date;
                        row.sort_order = i32::try_from(
                            entry_ids
                                .iter()
                                .position(|id| *id == row.id)
                                .ok_or_else(|| OpError::conflict("Entry set changed"))?,
                        )
                        .map_err(|_| OpError::invalid("entry_ids", "Too many entries"))?;
                    }
                    TimesheetCommand::StopTimer { .. } => {
                        if !row.is_running {
                            return Err(OpError::conflict("The timer is no longer running"));
                        }
                        notice = stop(tx, authority.subject, row).await?;
                        change = Change::Stopped;
                    }
                    TimesheetCommand::RestartTimer { entry_id } => {
                        // The running timer stopped above is not the target.
                        if row.id != *entry_id {
                            continue;
                        }
                        row.is_running = true;
                        row.started_at = Some(Utc::now());
                    }
                    _ => unreachable!("creation and deletion handled above"),
                }
                if *row != original {
                    save(tx, row).await?;
                    projects.insert(original.project_id);
                    projects.insert(row.project_id);
                    events.push(change.event(row));
                }
            }
            changed = target.and_then(|id| before.into_iter().find(|row| row.id == id));
        }
    }
    ensure_daily_limit(tx, authority.subject, &day).await?;
    Ok(Done {
        value: TimesheetChange {
            projects,
            notice,
            entry: changed,
        },
        events,
    })
}

async fn create(
    tx: &mut Tx,
    authority: &Authority,
    mut input: TimesheetEntryInput,
    running: bool,
) -> Result<TimeEntry, OpError> {
    normalize(&mut input)?;
    let billable =
        eligible(tx, authority, input.project_id, input.task_id).await? && input.billable;
    let id = Uuid::now_v7();
    let inserted=sqlx::query_scalar!(
        "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,notes,billable,start_minute,is_running,started_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,CASE WHEN $11 THEN now() END)
         ON CONFLICT (user_id) WHERE is_running DO NOTHING RETURNING id",
        id,authority.org,authority.subject,input.project_id,input.task_id,input.spent_date as NaiveDate,
        input.minutes,input.notes,billable,input.start_minute,running
    ).fetch_optional(&mut **tx).await?;
    if inserted.is_none() {
        return Err(OpError::conflict("This person already has a running timer"));
    }
    entries(tx, authority, &[id]).await?.pop().ok_or_else(|| {
        tracing::error!(%id, "Created Timesheet entry unavailable");
        OpError::Internal(TIMESHEET_UNAVAILABLE.into())
    })
}

/// Stops a locked, running row in memory; the caller saves it. Returns the
/// daily-limit notice, if the limit cut the elapsed time.
async fn stop(tx: &mut Tx, subject: Uuid, row: &mut TimeEntry) -> Result<Option<String>, OpError> {
    let started = row
        .started_at
        .ok_or_else(|| OpError::conflict("Timer start is unavailable"))?;
    let stopped = stop_running(
        tx,
        subject,
        row.spent_date,
        row.minutes,
        row.start_minute,
        started,
    )
    .await?;
    row.minutes = stopped.minutes;
    row.start_minute = stopped.start_minute;
    row.started_at = None;
    row.is_running = false;
    Ok(stopped.notice)
}

async fn save(tx: &mut Tx, entry: &mut TimeEntry) -> Result<(), OpError> {
    let updated = sqlx::query_scalar!(
        r#"UPDATE time_entries SET project_id=$2,task_id=$3,spent_date=$4,minutes=$5,
            notes=$6,billable=$7,start_minute=$8,sort_order=$9,is_running=$10,started_at=$11,
            notified_long_running_at=CASE WHEN $10 THEN notified_long_running_at ELSE NULL END
           WHERE id=$1 RETURNING updated_at AS "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry.id,
        entry.project_id,
        entry.task_id,
        entry.spent_date as NaiveDate,
        entry.minutes,
        entry.notes,
        entry.billable,
        entry.start_minute,
        entry.sort_order,
        entry.is_running,
        entry.started_at as Option<chrono::DateTime<Utc>>
    )
    .fetch_one(&mut **tx)
    .await?;
    entry.updated_at = updated;
    Ok(())
}

#[cfg(test)]
mod tests;
