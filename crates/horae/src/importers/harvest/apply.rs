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

use super::resolve::{self, OrgDefaults, PendingCache, RowFailure, RunCache};

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
async fn apply_within(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
) -> Result<(Vec<(EntityType, RowOutcome)>, PendingCache), (EntityType, RowFailure)> {
    let mut outcomes = Vec::new();
    let mut pending = PendingCache::default();

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
    let (te_outcome, entry_slot) = apply_time_entry(sp, cache, org, project_id, task_id, row)
        .await
        .map_err(|e| (EntityType::TimeEntry, e))?;
    outcomes.push((EntityType::TimeEntry, te_outcome));
    pending.entry_slot = entry_slot;

    Ok((outcomes, pending))
}

/// Insert (or skip) the time entry for a row.
///
/// Identity depends on whether the row carries a Harvest id:
///
/// - **With an id** (API source): the id alone identifies the entry. Provenance
///   is checked first; on a miss the natural key may *adopt* an existing entry
///   not yet claimed by any Harvest id (e.g. from an earlier CSV import), but an
///   entry claimed by a different id is a different record — two identical
///   entries with distinct ids are never collapsed into one row.
/// - **Without an id** (CSV source): identity is the natural key plus its
///   occurrence number within the run. Harvest's Detailed export lists every
///   entry, so the Nth identical row is a real Nth entry: it matches the Nth
///   stored entry with that key (skip) or creates one — keeping duplicates on
///   first import while a re-import of the same file stays idempotent.
///
/// This split assumes a single run never mixes id-bearing and id-less rows for
/// the same natural key — true for both shipped sources (CSV rows never carry
/// ids, API rows always do); a hypothetical mixed source could conflate the two
/// matching schemes.
///
/// Also returns the natural-key slot an id-less row consumed, for the caller to
/// bump in the run cache once the savepoint commits.
async fn apply_time_entry(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    project_id: Uuid,
    task_id: Uuid,
    row: &SourceRow,
) -> Result<(RowOutcome, Option<String>), RowFailure> {
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
        return Ok((RowOutcome::Skipped, None));
    }

    // Natural key: (user, project, task, spent_date, minutes, notes).
    let (existing, entry_slot) = if row.harvest_time_entry_id.is_some() {
        // Unmapped Harvest id: adopt only an entry no id has claimed yet, so two
        // distinct ids with identical fields never collapse into one row.
        let found = sqlx::query_scalar!(
            "SELECT id FROM time_entries AS te
             WHERE org_id = $1 AND user_id = $2 AND project_id = $3 AND task_id = $4
               AND spent_date = $5 AND minutes = $6
               AND COALESCE(notes, '') = COALESCE($7, '')
               AND NOT EXISTS (
                 SELECT 1 FROM harvest_import_map AS m
                 WHERE m.org_id = $1
                   AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
                   AND m.horae_id = te.id)
             ORDER BY id
             LIMIT 1",
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
        (found, None)
    } else {
        // Id-less row: match the stored entry at this key's next occurrence slot
        // (ordered by id — UUID v7, so creation order — for determinism).
        let slot_key = entry_slot_key(user_id, project_id, task_id, row, minutes, notes);
        let offset = i64::try_from(cache.entry_slot_offset(&slot_key)).unwrap_or(i64::MAX);
        let found = sqlx::query_scalar!(
            "SELECT id FROM time_entries
             WHERE org_id = $1 AND user_id = $2 AND project_id = $3 AND task_id = $4
               AND spent_date = $5 AND minutes = $6
               AND COALESCE(notes, '') = COALESCE($7, '')
             ORDER BY id
             OFFSET $8
             LIMIT 1",
            org.org_id,
            user_id,
            project_id,
            task_id,
            row.spent_date as chrono::NaiveDate,
            minutes,
            notes,
            offset,
        )
        .fetch_optional(&mut **sp)
        .await?;
        (found, Some(slot_key))
    };
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
        return Ok((RowOutcome::Skipped, entry_slot));
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
    Ok((RowOutcome::Created, entry_slot))
}

/// The run-cache key for an id-less entry's natural-key slot: the resolved ids
/// plus exactly the fields the natural-key SQL compares (notes case-sensitive,
/// trimmed), so the in-run counter and the query can never disagree.
fn entry_slot_key(
    user_id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
    row: &SourceRow,
    minutes: i32,
    notes: Option<&str>,
) -> String {
    format!(
        "{user_id}\u{1f}{project_id}\u{1f}{task_id}\u{1f}{}\u{1f}{minutes}\u{1f}{}",
        row.spent_date,
        notes.unwrap_or(""),
    )
}

/// Push a parent's outcome (when it was actually touched) and queue its cache
/// entry for promotion on commit.
fn fold(
    outcomes: &mut Vec<(EntityType, RowOutcome)>,
    pending: &mut PendingCache,
    entity: EntityType,
    resolved: resolve::Resolved,
) {
    if let Some(o) = resolved.outcome {
        outcomes.push((entity, o));
    }
    if let Some(e) = resolved.cache_entry {
        pending.parents.push(e);
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
