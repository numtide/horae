//! FK-safe, per-record application of a source row (FR-004, FR-020, FR-026).
//!
//! Each [`SourceRow`] is applied as an all-or-nothing unit inside a database
//! savepoint: client → project → task (+ `project_tasks` link) → time entry, with
//! the provenance rows written in the same unit. If any step fails the savepoint
//! rolls back — leaving no partial fragment — and the row is reported as an error
//! so the run continues (FR-018). Only on a clean commit are the row's newly
//! resolved parents promoted into the run cache.

use horae_core::importers::harvest::types::{EntityType, RowOutcome, SourceRow};
use horae_core::importers::harvest::{convert, keys};
use sqlx::{Acquire, Postgres, Transaction};
use uuid::Uuid;

use super::resolve::{self, OrgDefaults, ParentKind, RowFailure, RunCache};

/// The per-entity outcomes of applying one row, ready to fold into the summary.
pub struct RowResult {
    pub outcomes: Vec<(EntityType, RowOutcome)>,
}

/// Apply one source row within its own savepoint. On success returns the entity
/// outcomes to count; on a per-record failure returns a single `Errored` outcome
/// tagged with the entity level that failed, having rolled back cleanly.
pub async fn apply_row(
    outer: &mut Transaction<'_, Postgres>,
    cache: &mut RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
) -> RowResult {
    let mut sp = match outer.begin().await {
        Ok(sp) => sp,
        Err(e) => {
            return errored(
                EntityType::TimeEntry,
                row,
                format!("cannot open savepoint: {e}"),
            );
        }
    };

    match apply_within(&mut sp, cache, org, row).await {
        Ok((outcomes, pending)) => match sp.commit().await {
            Ok(()) => {
                cache.merge(pending);
                RowResult { outcomes }
            }
            Err(e) => errored(EntityType::TimeEntry, row, format!("commit failed: {e}")),
        },
        Err((entity, failure)) => {
            let _ = sp.rollback().await;
            errored(entity, row, failure.reason)
        }
    }
}

/// The resolve → create/skip pipeline for one row inside its savepoint. Returns
/// the outcomes plus the cache entries to promote only if the caller commits.
#[allow(clippy::type_complexity)]
async fn apply_within(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
) -> Result<
    (
        Vec<(EntityType, RowOutcome)>,
        Vec<(ParentKind, String, Uuid)>,
    ),
    (EntityType, RowFailure),
> {
    let mut outcomes = Vec::new();
    let mut pending = Vec::new();

    let client = resolve::resolve_client(sp, cache, org, row)
        .await
        .map_err(|e| (EntityType::Client, e))?;
    let client_id = client.id;
    fold(&mut outcomes, &mut pending, EntityType::Client, client);

    let project = resolve::resolve_project(sp, cache, org, client_id, row)
        .await
        .map_err(|e| (EntityType::Project, e))?;
    let project_id = project.id;
    fold(&mut outcomes, &mut pending, EntityType::Project, project);

    let task = resolve::resolve_task(sp, cache, org, row)
        .await
        .map_err(|e| (EntityType::Task, e))?;
    let task_id = task.id;
    fold(&mut outcomes, &mut pending, EntityType::Task, task);

    resolve::ensure_project_task(sp, project_id, task_id, row)
        .await
        .map_err(|e| (EntityType::Task, e))?;

    // Time entry — the record proper.
    let te_outcome = apply_time_entry(sp, org, project_id, task_id, row)
        .await
        .map_err(|e| (EntityType::TimeEntry, e))?;
    outcomes.push((EntityType::TimeEntry, te_outcome));

    Ok((outcomes, pending))
}

/// Insert (or skip) the time entry for a row, matched provenance-first then by
/// composite natural key.
async fn apply_time_entry(
    sp: &mut Transaction<'_, Postgres>,
    org: OrgDefaults<'_>,
    project_id: Uuid,
    task_id: Uuid,
    row: &SourceRow,
) -> Result<RowOutcome, RowFailure> {
    let user_id = resolve::resolve_user(sp, org.org_id, row).await?;

    let minutes_i64 = convert::hours_to_minutes(&row.hours)?;
    let minutes = i32::try_from(minutes_i64)
        .map_err(|_| RowFailure::new(format!("duration {minutes_i64} minutes out of range")))?;
    // Notes are stored trimmed with the shared whitespace set, so an exact
    // re-import matches without any SQL-side trimming (both sides are trim_ws).
    let notes = row
        .notes
        .as_deref()
        .map(keys::trim_ws)
        .filter(|n| !n.is_empty());

    // Provenance first.
    if let Some(hid) = row.harvest_time_entry_id
        && let Some(id) =
            super::provenance::lookup(&mut **sp, org.org_id, EntityType::TimeEntry, hid).await?
    {
        // Confirm/refresh the mapping and skip (idempotent, edit-robust).
        super::provenance::upsert(
            &mut **sp,
            org.org_id,
            EntityType::TimeEntry,
            hid,
            id,
            row.harvest_updated_at,
        )
        .await?;
        return Ok(RowOutcome::Skipped);
    }

    // Natural key: (user, project, task, spent_date, minutes, notes).
    let existing = sqlx::query_scalar!(
        "SELECT id FROM time_entries
         WHERE org_id = $1 AND user_id = $2 AND project_id = $3 AND task_id = $4
           AND spent_date = $5 AND minutes = $6
           AND COALESCE(notes, '') = COALESCE($7, '')",
        org.org_id,
        user_id,
        project_id,
        task_id,
        row.spent_date as chrono::NaiveDate,
        minutes,
        notes,
    )
    .fetch_optional(&mut **sp)
    .await?;
    if let Some(id) = existing {
        if let Some(hid) = row.harvest_time_entry_id {
            super::provenance::upsert(
                &mut **sp,
                org.org_id,
                EntityType::TimeEntry,
                hid,
                id,
                row.harvest_updated_at,
            )
            .await?;
        }
        return Ok(RowOutcome::Skipped);
    }

    // Create. State defaults to `open`; never `invoiced` from Harvest (FR-016).
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        id,
        org.org_id,
        user_id,
        project_id,
        task_id,
        row.spent_date as chrono::NaiveDate,
        minutes,
        notes,
        row.billable,
    )
    .execute(&mut **sp)
    .await?;
    if let Some(hid) = row.harvest_time_entry_id {
        super::provenance::upsert(
            &mut **sp,
            org.org_id,
            EntityType::TimeEntry,
            hid,
            id,
            row.harvest_updated_at,
        )
        .await?;
    }
    Ok(RowOutcome::Created)
}

/// Push a parent's outcome (when it was actually touched) and queue its cache
/// entry for promotion on commit.
fn fold(
    outcomes: &mut Vec<(EntityType, RowOutcome)>,
    pending: &mut Vec<(ParentKind, String, Uuid)>,
    entity: EntityType,
    resolved: resolve::Resolved,
) {
    if let Some(o) = resolved.outcome {
        outcomes.push((entity, o));
    }
    if let Some(e) = resolved.cache_entry {
        pending.push(e);
    }
}

fn errored(entity: EntityType, row: &SourceRow, reason: String) -> RowResult {
    RowResult {
        outcomes: vec![(
            entity,
            RowOutcome::Errored {
                source_location: row.source_location.clone(),
                reason,
            },
        )],
    }
}
