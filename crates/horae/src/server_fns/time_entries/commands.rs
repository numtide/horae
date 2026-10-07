//! Person-bound Timesheet operations; legacy shell commands remain independent.

use chrono::{NaiveDate, Timelike, Utc};
use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

use super::*;
use crate::models::scoped_time::{
    TimesheetCommand, TimesheetEntryInput, TimesheetPolicy, TimesheetTrackingOption,
    TimesheetWriteContext,
};

pub(super) fn public_error(error: ServerFnError) -> ServerFnError {
    match error {
        error @ ServerFnError::ServerError {
            code: BAD_REQUEST | FORBIDDEN | CONFLICT | UNAUTHORIZED,
            ..
        } => error,
        error => {
            tracing::error!(%error,"Timesheet operation failed");
            server_err("Timesheet is unavailable")
        }
    }
}

struct Authority {
    org: Uuid,
    subject: Uuid,
    owner: bool,
    legacy: bool,
    whole_subject: bool,
    projects: Vec<Uuid>,
}

impl Authority {
    fn permits(&self, project: Uuid) -> bool {
        self.whole_subject || self.projects.contains(&project)
    }
}

async fn begin<'a>(
    pool: &'a PgPool,
    org: Uuid,
    actor: Uuid,
    context: &TimesheetWriteContext,
) -> Result<(Transaction<'a, Postgres>, Authority), ServerFnError> {
    if context.expected_requester.org_id != org || context.expected_requester.user_id != actor {
        return Err(forbidden("Timesheet identity changed; refresh the page"));
    }
    let mut tx = pool.begin().await.map_err(server_err)?;
    permissions::configure_administration(&mut tx)
        .await
        .map_err(server_err)?;
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
        org
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    let legacy = match (policy, context.expected_policy) {
        (Some(0), TimesheetPolicy::LegacyOwn) if actor == context.subject_id => true,
        (Some(1), TimesheetPolicy::Scoped) => false,
        _ => return Err(forbidden("Timesheet policy changed; refresh the page")),
    };
    let users = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id=$1 AND id=ANY($2) AND active ORDER BY id FOR SHARE",
        org,
        &[actor, context.subject_id]
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    if !users.contains(&actor) || !users.contains(&context.subject_id) {
        return Err(forbidden("Active same-organization accounts required"));
    }
    let mut authority = Authority {
        org,
        subject: context.subject_id,
        owner: actor == context.subject_id,
        legacy,
        whole_subject: legacy,
        projects: Vec::new(),
    };
    if !legacy {
        let state = permissions::load_person_permissions(&mut tx, org, actor)
            .await
            .map_err(server_err)?
            .ok_or_else(|| forbidden("Current permissions unavailable"))?;
        authority.whole_subject = state.grants.contains(Permission::TimeWriteAll)
            || (authority.owner && state.grants.contains(Permission::TimeWriteOwn));
        if state.grants.contains(Permission::TimeWriteManaged) {
            let managed = sqlx::query_scalar!(
                "SELECT managed_user_id FROM person_management_assignments WHERE org_id=$1 AND manager_id=$2 AND managed_user_id=$3 FOR SHARE",
                org, actor, context.subject_id
            ).fetch_optional(&mut *tx).await.map_err(server_err)?;
            authority.whole_subject |= managed.is_some();
            authority.projects = sqlx::query_scalar!(
                "SELECT project_id FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2 ORDER BY project_id FOR SHARE",
                org, actor
            ).fetch_all(&mut *tx).await.map_err(server_err)?;
        }
    }
    Ok((tx, authority))
}

async fn choices(
    tx: &mut Transaction<'_, Postgres>,
    authority: &Authority,
    pair: Option<(Uuid, Uuid)>,
) -> Result<Vec<TimesheetTrackingOption>, ServerFnError> {
    sqlx::query_as!(TimesheetTrackingOption,
        r#"SELECT p.id AS project_id,p.name AS project_name,t.id AS task_id,t.name AS task_name,
                  (p.project_type <> 'non_billable' AND pt.billable) AS "billable!"
           FROM users u JOIN projects p ON p.org_id=u.org_id
           JOIN clients c ON c.id=p.client_id AND c.org_id=u.org_id
           JOIN project_tasks pt ON pt.project_id=p.id
           JOIN tasks t ON t.id=pt.task_id AND t.org_id=u.org_id
           WHERE u.org_id=$1 AND u.id=$2 AND u.active AND p.active AND c.active AND t.active
             AND ($5 OR pt.active)
             AND ($3 OR p.id=ANY($4))
             AND (($5 AND u.org_role='admin') OR EXISTS (
                 SELECT 1 FROM assignments a WHERE a.user_id=u.id AND a.project_id=p.id))
             AND NOT EXISTS (SELECT 1 FROM project_task_settings s
                 WHERE s.org_id=u.org_id AND s.project_id=p.id AND s.task_id=t.id AND s.restricted
                 AND NOT EXISTS (SELECT 1 FROM project_task_members m
                     WHERE m.org_id=u.org_id AND m.project_id=p.id AND m.task_id=t.id AND m.user_id=u.id))
             AND ($6::uuid IS NULL OR p.id=$6) AND ($7::uuid IS NULL OR t.id=$7)
           ORDER BY p.name,p.id,t.name,t.id FOR SHARE OF p,c,pt,t"#,
        authority.org,authority.subject,authority.whole_subject,&authority.projects,
        authority.legacy,pair.map(|p| p.0),pair.map(|p| p.1)
    ).fetch_all(&mut **tx).await.map_err(server_err)
}

pub(super) async fn tracking(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    context: &TimesheetWriteContext,
) -> Result<Vec<TimesheetTrackingOption>, ServerFnError> {
    let (mut tx, authority) = begin(pool, org, actor, context).await?;
    let result = choices(&mut tx, &authority, None).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

async fn eligible(
    tx: &mut Transaction<'_, Postgres>,
    authority: &Authority,
    project: Uuid,
    task: Uuid,
) -> Result<bool, ServerFnError> {
    if !authority.permits(project) {
        return Err(forbidden("Current time-write scope required"));
    }
    sqlx::query_scalar!(
        "SELECT id FROM assignments WHERE user_id=$1 AND project_id=$2 FOR SHARE",
        authority.subject,
        project
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?;
    lock_task_access(tx, authority.subject, project, task).await?;
    choices(tx, authority, Some((project, task)))
        .await?
        .first()
        .map(|option| option.billable)
        .ok_or_else(|| conflict("The selected person cannot currently track this project/task"))
}

/// Legacy own historical edits retain their existing task-grant and billability
/// rules. This path cannot admit a new context or a canonical-policy operation.
async fn legacy_billable(
    tx: &mut Transaction<'_, Postgres>,
    authority: &Authority,
    project: Uuid,
    task: Uuid,
) -> Result<bool, ServerFnError> {
    lock_task_access(tx, authority.subject, project, task).await?;
    sqlx::query_scalar!(
        r#"SELECT (p.project_type <> 'non_billable' AND COALESCE(pt.billable,t.billable_default)) AS "billable!"
           FROM projects p JOIN tasks t ON t.id=$3 AND t.org_id=p.org_id
           LEFT JOIN project_tasks pt ON pt.project_id=p.id AND pt.task_id=t.id
           WHERE p.org_id=$1 AND p.id=$2 FOR SHARE OF p,t"#,
        authority.org,project,task
    ).fetch_optional(&mut **tx).await.map_err(server_err)?
        .ok_or_else(|| conflict("Historical project/task unavailable"))
}

/// Events and budget checks are released only after the complete command commits.
pub(super) struct Effects {
    pub entries: Vec<(TimeEntry, TimeEntryEvent)>,
    pub projects: BTreeSet<Uuid>,
}

async fn entries(
    tx: &mut Transaction<'_, Postgres>,
    authority: &Authority,
    ids: &[Uuid],
) -> Result<Vec<TimeEntry>, ServerFnError> {
    sqlx::query_as!(
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
    .await
    .map_err(server_err)
}

fn normalize(input: &mut TimesheetEntryInput) -> Result<(), ServerFnError> {
    if input.minutes < 0 {
        return Err(err(BAD_REQUEST, "Minutes cannot be negative"));
    }
    (input.minutes, input.start_minute) = normalize_start(input.minutes, input.start_minute)?;
    Ok(())
}

pub(super) async fn apply(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    context: &TimesheetWriteContext,
    command: TimesheetCommand,
) -> Result<Effects, ServerFnError> {
    let (mut tx, authority) = begin(pool, org, actor, context).await?;
    crate::db::lock_time_entry_write(&mut tx, authority.subject)
        .await
        .map_err(server_err)?;
    let ids = match &command {
        TimesheetCommand::Update { entry_id, .. }
        | TimesheetCommand::StopTimer { entry_id }
        | TimesheetCommand::Reschedule { entry_id, .. } => vec![*entry_id],
        TimesheetCommand::Delete { entry_ids } | TimesheetCommand::Reorder { entry_ids, .. } => {
            entry_ids.clone()
        }
        _ => Vec::new(),
    };
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err(err(BAD_REQUEST, "Duplicate entry identifiers"));
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
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    let today = Utc::now().date_naive();
    match &command {
        TimesheetCommand::Create { entry } | TimesheetCommand::Update { entry, .. } => {
            dates.push(entry.spent_date)
        }
        TimesheetCommand::StartTimer { .. } => dates.push(today),
        TimesheetCommand::Reschedule { spent_date, .. }
        | TimesheetCommand::Reorder { spent_date, .. } => dates.push(*spent_date),
        _ => {}
    }
    let coverage = sqlx::query_scalar!(
        "SELECT id FROM approvals WHERE org_id=$1 AND user_id=$2
         AND EXISTS (SELECT 1 FROM unnest($3::date[]) d WHERE d BETWEEN period_start AND period_end)
         ORDER BY id FOR SHARE",
        org,
        authority.subject,
        &dates as &[NaiveDate]
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    let mut before = entries(&mut tx, &authority, &ids).await?;
    if before.len() != ids.len()
        || before
            .iter()
            .any(|entry| !dates.contains(&entry.spent_date))
    {
        return Err(conflict(
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
        return Err(conflict(
            "Approval or billing coverage requires the approval editing integration",
        ));
    }
    for entry in &before {
        if !authority.permits(entry.project_id) {
            return Err(forbidden("Current time-write scope required"));
        }
        if !(matches!(command, TimesheetCommand::StopTimer { .. }) && authority.owner) {
            if authority.legacy {
                legacy_billable(&mut tx, &authority, entry.project_id, entry.task_id).await?;
            } else {
                eligible(&mut tx, &authority, entry.project_id, entry.task_id).await?;
            }
        }
    }
    let mut effects = Effects {
        entries: Vec::new(),
        projects: BTreeSet::new(),
    };
    match command {
        TimesheetCommand::Create { entry } => {
            let created = create(&mut tx, &authority, entry, false).await?;
            effects.projects.insert(created.project_id);
            effects.entries.push((created, TimeEntryEvent::Created));
        }
        TimesheetCommand::StartTimer {
            project_id,
            task_id,
            notes,
        } => {
            let created = create(
                &mut tx,
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
            effects.entries.push((created, TimeEntryEvent::Created));
        }
        TimesheetCommand::Delete { .. } => {
            sqlx::query!(
                "DELETE FROM time_entries WHERE org_id=$1 AND user_id=$2 AND id=ANY($3)",
                org,
                authority.subject,
                &ids
            )
            .execute(&mut *tx)
            .await
            .map_err(server_err)?;
            effects
                .projects
                .extend(before.iter().map(|entry| entry.project_id));
            effects.entries.extend(
                before
                    .into_iter()
                    .map(|entry| (entry, TimeEntryEvent::Deleted)),
            );
        }
        command => {
            for row in &mut before {
                let original = row.clone();
                let mut event = TimeEntryEvent::Updated;
                match &command {
                    TimesheetCommand::Update { entry, .. } => {
                        let mut input = entry.clone();
                        normalize(&mut input)?;
                        let billable = if authority.legacy
                            && (row.project_id, row.task_id) == (input.project_id, input.task_id)
                        {
                            legacy_billable(&mut tx, &authority, input.project_id, input.task_id)
                                .await?
                        } else {
                            eligible(&mut tx, &authority, input.project_id, input.task_id).await?
                        };
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
                            return Err(err(BAD_REQUEST, "Minutes cannot be negative"));
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
                            return Err(conflict(
                                "Only untimed entries can be reordered; running timers cannot change day",
                            ));
                        }
                        row.spent_date = *spent_date;
                        row.sort_order = i32::try_from(
                            entry_ids
                                .iter()
                                .position(|id| *id == row.id)
                                .ok_or_else(|| conflict("Entry set changed"))?,
                        )
                        .map_err(|_| err(BAD_REQUEST, "Too many entries"))?;
                    }
                    TimesheetCommand::StopTimer { .. } => {
                        if !row.is_running {
                            return Err(conflict("The timer is no longer running"));
                        }
                        let started = row
                            .started_at
                            .ok_or_else(|| conflict("Timer start is unavailable"))?;
                        row.minutes = i32::try_from(horae_core::duration::minutes_between(
                            started,
                            Utc::now(),
                        ))
                        .map_err(|_| conflict("Timer duration exceeds supported minutes"))?;
                        let start = horae_core::time_of_day::snap(
                            started.hour() as i32 * 60 + started.minute() as i32,
                            horae_core::time_of_day::SNAP_STEP,
                        )
                        .min(1439);
                        row.start_minute =
                            (i64::from(start) + i64::from(row.minutes) <= 1440).then_some(start);
                        row.started_at = None;
                        row.is_running = false;
                        event = TimeEntryEvent::Stopped;
                    }
                    _ => unreachable!("creation and deletion handled above"),
                }
                if *row != original {
                    save(&mut tx, row).await?;
                    effects.projects.insert(original.project_id);
                    effects.projects.insert(row.project_id);
                    effects.entries.push((row.clone(), event));
                }
            }
        }
    }
    tx.commit().await.map_err(server_err)?;
    Ok(effects)
}

async fn create(
    tx: &mut Transaction<'_, Postgres>,
    authority: &Authority,
    mut input: TimesheetEntryInput,
    running: bool,
) -> Result<TimeEntry, ServerFnError> {
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
    ).fetch_optional(&mut **tx).await.map_err(server_err)?;
    if inserted.is_none() {
        return Err(conflict("This person already has a running timer"));
    }
    entries(tx, authority, &[id])
        .await?
        .pop()
        .ok_or_else(|| server_err("Created entry unavailable"))
}

async fn save(
    tx: &mut Transaction<'_, Postgres>,
    entry: &mut TimeEntry,
) -> Result<(), ServerFnError> {
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
    .await
    .map_err(server_err)?;
    entry.updated_at = updated;
    Ok(())
}

#[cfg(test)]
mod tests;
