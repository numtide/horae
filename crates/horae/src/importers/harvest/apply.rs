//! FK-safe, per-record application of a source row (FR-004, FR-020, FR-026).
//!
//! Each [`SourceRow`] is applied as an all-or-nothing unit inside a database
//! savepoint: client → project → task (+ `project_tasks` link) → time entry, with
//! the provenance rows written in the same unit. If any step fails the savepoint
//! rolls back — leaving no partial fragment — and the row is reported as an error
//! so the run continues (FR-018). Only on a clean commit are the row's newly
//! resolved parents, users and project-task links promoted into the run cache.

use horae_core::importers::harvest::types::{
    EntityType, EntryErrorKind, PersonErrorKind, PersonOutcome, ReportPerson, RowOutcome,
    SourceKind, SourceRow,
};
use horae_core::importers::harvest::{convert, keys};
use sqlx::{Acquire, Postgres, Transaction};
use uuid::Uuid;

use super::resolve::{self, OrgDefaults, PendingCache, RowFailure, RunCache};

/// The per-entity outcomes of applying one row, ready to fold into the summary.
pub struct RowResult {
    pub outcomes: Vec<(EntityType, RowOutcome)>,
    /// The person a committed CSV row resolved for the first time in the run.
    pub person: Option<ReportPerson>,
}

/// Apply one source row within its own savepoint. On success returns the entity
/// outcomes to count; on a per-record failure returns a single `Errored` outcome
/// tagged with the entity level that failed, having rolled back cleanly.
pub async fn apply_row(
    outer: &mut Transaction<'_, Postgres>,
    cache: &mut RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
    source: SourceKind,
) -> RowResult {
    // No provenance is written, so the next sync picks the entry up again.
    if row.running {
        return errored(
            EntityType::TimeEntry,
            row,
            RowFailure::entry(
                EntryErrorKind::TimerRunning,
                "timer still running in Harvest",
            ),
        );
    }
    for (entity, id) in [
        (EntityType::Client, row.harvest_client_id),
        (EntityType::Project, row.harvest_project_id),
        (EntityType::Task, row.harvest_task_id),
    ] {
        if let Some(id) = id
            && cache.parent_failed(entity, id)
        {
            return errored(
                EntityType::TimeEntry,
                row,
                RowFailure::new(format!("{} {id} failed to import", entity.as_str())),
            );
        }
    }
    // An entry never falls back to email matching for a person who failed,
    // which would attach two Harvest people sharing an email to one account.
    if let Some(id) = row.harvest_user_id
        && cache.parent_failed(EntityType::Person, id)
    {
        return errored(
            EntityType::TimeEntry,
            row,
            RowFailure::person(
                format!("person {id} failed to import"),
                PersonErrorKind::PersonNotImported,
                row.user_email.as_deref(),
                row.user_name.as_deref(),
            ),
        );
    }
    let mut sp = match outer.begin().await {
        Ok(sp) => sp,
        Err(e) => {
            return errored(
                EntityType::TimeEntry,
                row,
                RowFailure::new(format!("cannot open savepoint: {e}")),
            );
        }
    };

    match apply_within(&mut sp, cache, org, row, source).await {
        Ok((outcomes, pending, person)) => match sp.commit().await {
            Ok(()) => {
                cache.merge(pending);
                RowResult { outcomes, person }
            }
            Err(e) => errored(
                EntityType::TimeEntry,
                row,
                RowFailure::new(format!("commit failed: {e}")),
            ),
        },
        Err((entity, failure)) => {
            let _ = sp.rollback().await;
            errored(entity, row, failure)
        }
    }
}

type Applied = (
    Vec<(EntityType, RowOutcome)>,
    PendingCache,
    Option<ReportPerson>,
);

/// The resolve → create/skip pipeline for one row inside its savepoint. Returns
/// the outcomes plus the cache entries to promote only if the caller commits.
async fn apply_within(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
    source: SourceKind,
) -> Result<Applied, (EntityType, RowFailure)> {
    let mut outcomes = Vec::new();
    let mut pending = PendingCache::default();

    let client = resolve::resolve_client(sp, cache, org, &row.into())
        .await
        .map_err(|e| (EntityType::Client, e))?;
    let client_id = client.id;
    fold(&mut outcomes, &mut pending, EntityType::Client, client);

    let project = resolve::resolve_project(sp, cache, org, client_id, &row.into())
        .await
        .map_err(|e| (EntityType::Project, e))?;
    let project_id = project.id;
    fold(&mut outcomes, &mut pending, EntityType::Project, project);

    let task = resolve::resolve_task(sp, cache, org, &row.into())
        .await
        .map_err(|e| (EntityType::Task, e))?;
    let task_id = task.id;
    fold(&mut outcomes, &mut pending, EntityType::Task, task);

    resolve::ensure_project_task(sp, cache, project_id, task_id, row)
        .await
        .map_err(|e| (EntityType::Task, e))?;
    pending.project_task = Some((project_id, task_id));

    let (user_key, user_id, person_outcome) =
        resolve::resolve_user(sp, cache, org.org_id, row, source)
            .await
            .map_err(|e| (EntityType::TimeEntry, e))?;
    pending.user = Some((user_key, user_id));
    let person_outcome = person_outcome.filter(|_| !cache.person_counted(user_id));
    if person_outcome.is_some() {
        pending.person = Some(user_id);
    }
    let person = person_outcome.map(|outcome| {
        outcomes.push((
            EntityType::Person,
            match outcome {
                PersonOutcome::Created => RowOutcome::Created,
                PersonOutcome::Matched => RowOutcome::Skipped,
            },
        ));
        let email = row
            .user_email
            .as_deref()
            .map(keys::trim_ws)
            .filter(|e| !e.is_empty());
        let name = row
            .user_name
            .as_deref()
            .map(keys::trim_ws)
            .filter(|n| !n.is_empty());
        ReportPerson {
            harvest_id: row.harvest_user_id,
            horae_id: user_id,
            email: email.map(str::to_owned),
            name: name.or(email).unwrap_or_default().to_owned(),
            outcome,
            harvest_active: None,
        }
    });

    // Time entry — the record proper.
    let (te_outcome, entry_slot) =
        apply_time_entry(sp, cache, org, project_id, task_id, user_id, row)
            .await
            .map_err(|e| (EntityType::TimeEntry, e))?;
    outcomes.push((EntityType::TimeEntry, te_outcome));
    pending.entry_slot = entry_slot;

    Ok((outcomes, pending, person))
}

/// Insert, update or skip the time entry for a row.
///
/// A mapped entry whose Harvest `updated_at` moved forward is updated while it
/// is still `open` and unedited in Horae; otherwise the row is an error that
/// says why, and the Horae entry stays as it is.
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
    user_id: Uuid,
    row: &SourceRow,
) -> Result<(RowOutcome, Option<String>), RowFailure> {
    let minutes_i64 = convert::hours_to_minutes(&row.hours).map_err(|error| match error {
        convert::ConvertError::Negative(hours) => RowFailure::entry(
            EntryErrorKind::NegativeHours,
            format!("negative hours: {hours}"),
        ),
        other => other.into(),
    })?;
    let minutes = i32::try_from(minutes_i64)
        .map_err(|_| RowFailure::new(format!("duration {minutes_i64} minutes out of range")))?;
    // Notes are stored trimmed with the shared whitespace set, so an exact
    // re-import matches without any SQL-side trimming (both sides are trim_ws).
    let notes = row
        .notes
        .as_deref()
        .map(keys::trim_ws)
        .filter(|n| !n.is_empty());

    // Provenance first. `updated_at` is a transaction start time, so an edit
    // can be stamped earlier than the importer's last write: any difference
    // counts as an edit, not only a later one.
    if let Some(hid) = row.harvest_time_entry_id
        && let Some(mapped) = sqlx::query!(
            r#"SELECT m.horae_id,
                      m.harvest_updated_at AS "harvest_updated_at: chrono::DateTime<chrono::Utc>",
                      te.id AS "entry_id?",
                      te.state = 'open' AS "open?",
                      te.updated_at IS DISTINCT FROM COALESCE(m.horae_updated_at, te.created_at)
                        AS "edited?"
               FROM harvest_import_map AS m
               LEFT JOIN time_entries AS te ON te.org_id = m.org_id AND te.id = m.horae_id
               WHERE m.org_id = $1
                 AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
                 AND m.harvest_import_id = $2"#,
            org.org_id,
            hid,
        )
        .fetch_optional(&mut **sp)
        .await?
    {
        // A durable preview rolls back each page, so the version it simulated
        // writing on an earlier page (a create or an update) is newer than the
        // stored one. `simulated` alone marks an entry this preview created.
        let known = mapped.harvest_updated_at.max(cache.simulated_entry(hid));
        let simulated = cache
            .simulated_entry(hid)
            .filter(|_| mapped.harvest_updated_at.is_none());
        let changed = matches!(
            (row.harvest_updated_at, known),
            (Some(new), Some(old)) if new > old
        );
        // An unchanged entry, or one deleted in Horae, keeps today's behaviour:
        // confirm/refresh the mapping and skip.
        if !changed || mapped.entry_id.is_none() {
            super::provenance::upsert(
                &mut **sp,
                org.org_id,
                EntityType::TimeEntry,
                hid,
                mapped.horae_id,
                row.harvest_updated_at,
            )
            .await?;
            let outcome = if changed && simulated.is_some() {
                RowOutcome::Updated
            } else {
                RowOutcome::Skipped
            };
            return Ok((outcome, None));
        }
        if mapped.open != Some(true) {
            return Err(RowFailure::entry(
                EntryErrorKind::ChangedInHorae,
                "entry is no longer open in Horae; not updated from Harvest",
            ));
        }
        if mapped.edited != Some(false) {
            return Err(RowFailure::entry(
                EntryErrorKind::ChangedInHorae,
                "entry was changed in Horae (edited, reopened or voided) after import; not updated from Harvest",
            ));
        }
        update_time_entry(
            sp,
            mapped.horae_id,
            (project_id, task_id, user_id),
            row,
            minutes,
            notes,
        )
        .await?;
        record_entry(sp, org.org_id, hid, mapped.horae_id, row.harvest_updated_at).await?;
        return Ok((RowOutcome::Updated, None));
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
            record_entry(sp, org.org_id, hid, id, row.harvest_updated_at).await?;
        }
        return Ok((RowOutcome::Skipped, entry_slot));
    }

    // Create. State defaults to `open`; never `invoiced` from Harvest (FR-016).
    // A row past the day's limit on its own fails here, not on the trigger.
    horae_core::duration::check_daily_total(minutes.into())
        .map_err(|limit| RowFailure::new(limit.to_string()))?;
    crate::db::lock_time_entry_write(sp, user_id).await?;
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
    // An import spans thousands of person-days in one transaction, too many to
    // hold a day lock each, so it sums the day without one; the savepoint
    // drops the row if it grew the day past the limit.
    // shortcut: a web write committing to the same day between this sum and
    // the import's commit can still push the day past 240 hours; take the day
    // locks per batch, in date order, if that is ever seen.
    let after = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0)::bigint as "total!" FROM time_entries
           WHERE user_id = $1 AND spent_date = $2"#,
        user_id,
        row.spent_date as chrono::NaiveDate,
    )
    .fetch_one(&mut **sp)
    .await?;
    horae_core::duration::check_daily_change(after - i64::from(minutes), after)
        .map_err(|limit| RowFailure::new(limit.to_string()))?;
    if let Some(hid) = row.harvest_time_entry_id {
        record_entry(sp, org.org_id, hid, id, row.harvest_updated_at).await?;
    }
    Ok((RowOutcome::Created, entry_slot))
}

/// Rewrite an open entry with Harvest's newer version, under the same daily
/// limit as an insert.
async fn update_time_entry(
    sp: &mut Transaction<'_, Postgres>,
    id: Uuid,
    (project_id, task_id, user_id): (Uuid, Uuid, Uuid),
    row: &SourceRow,
    minutes: i32,
    notes: Option<&str>,
) -> Result<(), RowFailure> {
    horae_core::duration::check_daily_total(minutes.into())
        .map_err(|limit| RowFailure::new(limit.to_string()))?;
    let old_user = sqlx::query_scalar!("SELECT user_id FROM time_entries WHERE id = $1", id)
        .fetch_one(&mut **sp)
        .await?;
    crate::db::lock_time_entry_write(sp, user_id).await?;
    if old_user != user_id {
        crate::db::lock_time_entry_write(sp, old_user).await?;
    }
    // Like a web edit, never write into or out of a submitted or approved
    // period (reject deletes the approvals row).
    let covered = sqlx::query_scalar!(
        "SELECT a.id FROM approvals AS a, time_entries AS te
         WHERE te.id = $1 AND a.org_id = te.org_id
           AND ((a.user_id = te.user_id AND te.spent_date BETWEEN a.period_start AND a.period_end)
             OR (a.user_id = $2 AND $3 BETWEEN a.period_start AND a.period_end))
         ORDER BY a.id FOR SHARE OF a",
        id,
        user_id,
        row.spent_date as chrono::NaiveDate,
    )
    .fetch_all(&mut **sp)
    .await?;
    if !covered.is_empty() {
        return Err(RowFailure::entry(
            EntryErrorKind::ChangedInHorae,
            "entry's date is in a period submitted for approval in Horae; not updated from Harvest",
        ));
    }
    let before = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0)::bigint as "total!" FROM time_entries
           WHERE user_id = $1 AND spent_date = $2"#,
        user_id,
        row.spent_date as chrono::NaiveDate,
    )
    .fetch_one(&mut **sp)
    .await?;
    // The caller's checks ran before the lock: a submit or an edit that
    // committed while this waited must still stop the write. Postgres
    // re-evaluates this WHERE on the row version it waited for.
    let written = sqlx::query!(
        "UPDATE time_entries AS te
            SET user_id = $2, project_id = $3, task_id = $4, spent_date = $5,
                minutes = $6, notes = $7, billable = $8
          FROM harvest_import_map AS m
          WHERE te.id = $1 AND te.state = 'open'
            AND m.org_id = te.org_id
            AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
            AND m.horae_id = te.id
            AND te.updated_at = COALESCE(m.horae_updated_at, te.created_at)",
        id,
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
    if written.rows_affected() != 1 {
        return Err(RowFailure::entry(
            EntryErrorKind::ChangedInHorae,
            "entry changed in Horae during the import; not updated from Harvest",
        ));
    }
    let after = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0)::bigint as "total!" FROM time_entries
           WHERE user_id = $1 AND spent_date = $2"#,
        user_id,
        row.spent_date as chrono::NaiveDate,
    )
    .fetch_one(&mut **sp)
    .await?;
    horae_core::duration::check_daily_change(before, after)
        .map_err(|limit| RowFailure::new(limit.to_string()))
}

/// Map a Harvest entry to the Horae entry just written or adopted, remembering
/// that entry's `updated_at` so a later edit made in Horae can be told apart.
async fn record_entry(
    sp: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    harvest_id: i64,
    horae_id: Uuid,
    harvest_updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO harvest_import_map
             (org_id, harvest_entity_type, harvest_import_id, horae_id, harvest_updated_at, horae_updated_at)
           SELECT $1, 'time_entry'::harvest_entity_type, $2, te.id, $4, te.updated_at
           FROM time_entries AS te WHERE te.org_id = $1 AND te.id = $3
           ON CONFLICT (org_id, harvest_entity_type, harvest_import_id)
           DO UPDATE SET horae_id = EXCLUDED.horae_id,
                         harvest_updated_at = EXCLUDED.harvest_updated_at,
                         horae_updated_at = EXCLUDED.horae_updated_at"#,
        org_id,
        harvest_id,
        horae_id,
        harvest_updated_at as Option<chrono::DateTime<chrono::Utc>>,
    )
    .execute(&mut **sp)
    .await?;
    Ok(())
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
pub(super) fn fold(
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

fn errored(entity: EntityType, row: &SourceRow, failure: RowFailure) -> RowResult {
    RowResult {
        outcomes: vec![(
            entity,
            RowOutcome::Errored {
                source_location: row.source_location.clone(),
                reason: failure.reason,
                person: failure.person,
                entry_kind: failure.entry_kind,
            },
        )],
        person: None,
    }
}
