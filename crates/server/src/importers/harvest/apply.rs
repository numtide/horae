//! FK-safe, per-record application of a source row (FR-004, FR-020, FR-026).
//!
//! Each [`SourceRow`] is applied as an all-or-nothing unit inside a database
//! savepoint: client → project → task (+ `project_tasks` link) → time entry, with
//! the provenance rows written in the same unit. If any step fails the savepoint
//! rolls back — leaving no partial fragment — and the row is reported as an error
//! so the run continues (FR-018). Only on a clean commit are the row's newly
//! resolved parents, users and project-task links promoted into the run cache.

use horae_core::importers::harvest::link::{self, Billing, Current, LinkAction};
use horae_core::importers::harvest::types::{
    EntityType, EntryErrorKind, IdNotice, ImportMode, LinkConflict, LinkConflictKind,
    PersonErrorKind, PersonOutcome, ReportPerson, RowOutcome, SourceKind, SourceRow,
};
use horae_core::importers::harvest::{convert, keys};
use horae_core::types::EntryState;
use sqlx::{Acquire, Postgres, Transaction};
use uuid::Uuid;

use super::report::ImportReport;
use super::resolve::{self, InvoiceSlot, OrgDefaults, PendingCache, RowFailure, RunCache};

/// The per-entity outcomes of applying one row, ready to fold into the summary.
pub struct RowResult {
    pub outcomes: Vec<(EntityType, RowOutcome)>,
    /// The person a committed CSV row resolved for the first time in the run.
    pub person: Option<ReportPerson>,
    /// Records the committed row created without their Harvest id as API id.
    pub notices: Vec<IdNotice>,
    /// What the row did to its entry's billing (spec 025).
    pub link: Option<LinkNote>,
    /// The billing write the caller makes for the whole page, with
    /// [`write_links`].
    pub write: Option<LinkWrite>,
}

impl RowResult {
    /// Counts the row in the report and hands back its billing write.
    pub fn record_into(self, report: &mut ImportReport) -> Option<LinkWrite> {
        for (entity, outcome) in &self.outcomes {
            report.record(*entity, outcome);
        }
        report.people.extend(self.person);
        for notice in self.notices {
            report.note_id(notice);
        }
        let links = &mut report.summary.links;
        match self.link {
            None => {}
            Some(LinkNote::Linked) => links.linked += 1,
            Some(LinkNote::Unlinked) => links.unlinked += 1,
            Some(LinkNote::BilledWithoutInvoice) => links.billed_without_invoice += 1,
            Some(LinkNote::Conflict(conflict)) => report.note_conflict(conflict),
        }
        self.write
    }
}

/// An entry to bill on an imported invoice, bill with no invoice, or reopen.
pub struct LinkWrite {
    entry: Uuid,
    /// The Harvest entry, whose provenance remembers the new version.
    harvest_id: Option<i64>,
    invoice: Option<Uuid>,
    billed: bool,
    /// For the conflict reported when the write is skipped.
    source_location: String,
    harvest_invoice_id: Option<i64>,
}

impl LinkWrite {
    #[cfg(test)]
    pub fn link(entry: Uuid, invoice: Uuid, source_location: &str) -> Self {
        Self {
            entry,
            harvest_id: None,
            invoice: Some(invoice),
            billed: true,
            source_location: source_location.to_owned(),
            harvest_invoice_id: None,
        }
    }

    /// Take back the count `record_into` made for a write that did not happen.
    fn uncount(&self, report: &mut ImportReport) {
        let links = &mut report.summary.links;
        let count = match (self.billed, self.invoice) {
            (false, _) => &mut links.unlinked,
            (true, Some(_)) => &mut links.linked,
            (true, None) => &mut links.billed_without_invoice,
        };
        *count -= 1;
    }
}

/// Writes a page's billing in one statement, after its entries: billed
/// entries freeze their rounding as generation does, reopened ones clear it,
/// and provenance remembers each entry's new version, so the import's own
/// write never reads as an edit made in Horae.
///
/// Writing entry by entry, inside each entry's savepoint, made a first import
/// with 2,000 invoices most of two thirds slower (SC-006). An entry that was
/// submitted, or put on a Horae invoice, since the row read it is left alone,
/// and the report lists it as a conflict instead of counting the change.
pub async fn write_links(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    writes: &mut Vec<LinkWrite>,
    report: &mut ImportReport,
) -> sqlx::Result<()> {
    if writes.is_empty() {
        return Ok(());
    }
    let entries: Vec<Uuid> = writes.iter().map(|write| write.entry).collect();
    let invoices: Vec<Option<Uuid>> = writes.iter().map(|write| write.invoice).collect();
    let billed: Vec<bool> = writes.iter().map(|write| write.billed).collect();
    let harvest_ids: Vec<i64> = writes.iter().filter_map(|write| write.harvest_id).collect();
    // A CSV row has no Harvest id; its entry's provenance, when an API run
    // imported it, is found by the entry instead.
    let csv_entries: Vec<Uuid> = writes
        .iter()
        .filter(|write| write.harvest_id.is_none())
        .map(|write| write.entry)
        .collect();
    // Inside an import the tables have no statistics yet, so every index
    // looks like one row: each lookup names its whole key (`te.id = ANY`,
    // and provenance by its primary key) or the planner walks the
    // organization's rows for every page, which is quadratic.
    //
    // Provenance keeps the version the import last wrote, so a later Harvest
    // edit is not mistaken for one made in Horae. It is refreshed only when
    // it changes (`updated_at` is the transaction's start, so an entry this
    // run created needs nothing) and only for an entry not edited in Horae,
    // whose edit must stay detectable (`edited?` in `apply_time_entry`).
    let skipped = sqlx::query!(
        r#"WITH before AS (
           SELECT id, updated_at, created_at, state, invoice_id FROM time_entries WHERE id = ANY($2)),
         written AS (
           UPDATE time_entries AS te
              SET invoice_id = w.invoice_id,
                  state = CASE WHEN w.billed THEN 'invoiced'::entry_state ELSE 'open'::entry_state END,
                  rounded_minutes = CASE WHEN w.billed THEN effective_minutes(
                      te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) END
             FROM unnest($2::uuid[], $3::uuid[], $4::bool[]) AS w(id, invoice_id, billed),
                  organizations AS o
            WHERE te.id = ANY($2) AND te.id = w.id AND te.org_id = $1 AND o.id = $1
              AND te.state <> 'submitted'
              AND (te.invoice_id IS NULL OR EXISTS (
                SELECT 1 FROM invoices AS i WHERE i.id = te.invoice_id AND i.source = 'harvest'))
        RETURNING te.id, te.updated_at),
         by_harvest_id AS (
           UPDATE harvest_import_map AS m SET horae_updated_at = written.updated_at
             FROM written JOIN before ON before.id = written.id
            WHERE m.org_id = $1 AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
              AND m.harvest_import_id = ANY($5) AND m.horae_id = written.id
              AND m.horae_updated_at IS DISTINCT FROM written.updated_at
              AND before.updated_at IS NOT DISTINCT FROM COALESCE(m.horae_updated_at, before.created_at)),
         by_entry AS (
           UPDATE harvest_import_map AS m SET horae_updated_at = written.updated_at
             FROM written JOIN before ON before.id = written.id
            WHERE m.org_id = $1 AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
              AND m.horae_id = ANY($6) AND m.horae_id = written.id
              AND m.horae_updated_at IS DISTINCT FROM written.updated_at
              AND before.updated_at IS NOT DISTINCT FROM COALESCE(m.horae_updated_at, before.created_at))
         SELECT b.id AS "id!", b.state AS "state!: EntryState", i.number AS "number?"
           FROM before AS b LEFT JOIN invoices AS i ON i.id = b.invoice_id
          WHERE NOT EXISTS (SELECT 1 FROM written AS w WHERE w.id = b.id)"#,
        org_id,
        &entries,
        &invoices as &[Option<Uuid>],
        &billed,
        &harvest_ids,
        &csv_entries,
    )
    .fetch_all(&mut *tx)
    .await?;
    for row in skipped {
        let Some(write) = writes.iter().find(|write| write.entry == row.id) else {
            continue;
        };
        write.uncount(report);
        report.note_conflict(LinkConflict {
            source_location: write.source_location.clone(),
            harvest_time_entry_id: write.harvest_id,
            harvest_invoice_id: write.harvest_invoice_id,
            kind: if row.state == EntryState::Submitted {
                LinkConflictKind::SubmittedInHorae
            } else {
                LinkConflictKind::BilledInHorae
            },
            horae_invoice_number: row.number,
        });
    }
    writes.clear();
    Ok(())
}

/// A change to an entry's billing that the report counts.
pub enum LinkNote {
    Linked,
    Unlinked,
    BilledWithoutInvoice,
    Conflict(LinkConflict),
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
    mode: ImportMode,
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

    match apply_within(&mut sp, cache, org, row, source, mode).await {
        Ok((outcomes, mut pending, person, (link, write))) => match sp.commit().await {
            Ok(()) => {
                let notices = std::mem::take(&mut pending.notices);
                cache.merge(pending);
                RowResult {
                    outcomes,
                    person,
                    notices,
                    link,
                    write,
                }
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
    (Option<LinkNote>, Option<LinkWrite>),
);

/// The resolve → create/skip pipeline for one row inside its savepoint. Returns
/// the outcomes plus the cache entries to promote only if the caller commits.
async fn apply_within(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    row: &SourceRow,
    source: SourceKind,
    mode: ImportMode,
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
    let entry = apply_time_entry(
        sp,
        cache,
        org,
        (project_id, task_id, user_id),
        row,
        &mut pending.notices,
    )
    .await
    .map_err(|e| (EntityType::TimeEntry, e))?;
    outcomes.push((EntityType::TimeEntry, entry.outcome));
    pending.entry_slot = entry.slot;

    let (mut link, write) = match entry.id {
        Some(id) => link_entry(sp, cache, row, (id, entry.current), mode)
            .await
            .map_err(|e| (EntityType::TimeEntry, e))?,
        None => (None, None),
    };
    if entry.reopened {
        link = Some(LinkNote::Unlinked);
    }
    Ok((outcomes, pending, person, (link, write)))
}

/// What applying a row did to its time entry.
struct EntryApplied {
    outcome: RowOutcome,
    /// The natural-key slot an id-less row consumed.
    slot: Option<String>,
    /// The Horae entry, unless it was deleted in Horae.
    id: Option<Uuid>,
    /// Reopened from an import's billing so Harvest's edit could apply.
    reopened: bool,
    /// The entry's billing as the row left it, when the row's reads already
    /// know it; the link step reads it otherwise.
    current: Option<Current>,
}

impl EntryApplied {
    fn new(outcome: RowOutcome, slot: Option<String>, id: Option<Uuid>) -> Self {
        Self {
            outcome,
            slot,
            id,
            reopened: false,
            current: None,
        }
    }
}

fn current_of(state: EntryState, invoice: Option<Uuid>, imported: Option<bool>) -> Current {
    match (invoice, state) {
        (Some(invoice), _) if imported == Some(true) => Current::OnImported(invoice),
        (Some(_), _) => Current::OnHorae,
        (None, EntryState::Invoiced) => Current::BilledWithoutInvoice,
        (None, EntryState::Submitted) => Current::Submitted,
        (None, EntryState::Open | EntryState::Approved) => Current::Unbilled,
    }
}

/// Decide whether the entry is linked to an imported invoice, billed with no
/// invoice, reopened or left alone (spec 025, research R-6), and return the
/// write for [`write_links`]. A preview decides and counts but writes
/// nothing: the invoice it would link to may exist only in a rolled-back page.
async fn link_entry(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    row: &SourceRow,
    (entry_id, known): (Uuid, Option<Current>),
    mode: ImportMode,
) -> Result<(Option<LinkNote>, Option<LinkWrite>), RowFailure> {
    let current = match known {
        Some(current) => current,
        None => {
            let now = sqlx::query!(
                r#"SELECT te.state AS "state: EntryState", te.invoice_id,
                          i.source = 'harvest' AS "imported?"
                   FROM time_entries te LEFT JOIN invoices i ON i.id = te.invoice_id
                   WHERE te.id = $1"#,
                entry_id,
            )
            .fetch_one(&mut **sp)
            .await?;
            current_of(now.state, now.invoice_id, now.imported)
        }
    };
    let action = match row.harvest_time_entry_id {
        // A CSV knows only "Invoiced?" (Q5): it marks billed and never
        // unlinks, or moves an entry off an imported invoice.
        None if !row.invoiced => LinkAction::Keep,
        None => match link::decide(Billing::Missing, current) {
            LinkAction::MarkBilled if current != Current::Unbilled => LinkAction::Keep,
            action => action,
        },
        Some(_) => link::decide(billing(cache, row), current),
    };
    let target = match action {
        LinkAction::Keep => return Ok((None, None)),
        LinkAction::Conflict(kind) => {
            let horae_invoice_number = sqlx::query_scalar!(
                "SELECT i.number FROM time_entries te JOIN invoices i ON i.id = te.invoice_id
                 WHERE te.id = $1",
                entry_id,
            )
            .fetch_optional(&mut **sp)
            .await?;
            let conflict = LinkConflict {
                source_location: row.source_location.clone(),
                harvest_time_entry_id: row.harvest_time_entry_id,
                harvest_invoice_id: row.harvest_invoice_id,
                kind,
                horae_invoice_number,
            };
            return Ok((Some(LinkNote::Conflict(conflict)), None));
        }
        LinkAction::Link(invoice) => Some(invoice),
        LinkAction::MarkBilled | LinkAction::Unlink => None,
    };
    let write = (mode == ImportMode::Commit).then_some(LinkWrite {
        entry: entry_id,
        harvest_id: row.harvest_time_entry_id,
        invoice: target,
        billed: action != LinkAction::Unlink,
        source_location: row.source_location.clone(),
        harvest_invoice_id: row.harvest_invoice_id,
    });
    let note = match action {
        LinkAction::Link(_) => LinkNote::Linked,
        LinkAction::Unlink => LinkNote::Unlinked,
        _ => LinkNote::BilledWithoutInvoice,
    };
    Ok((Some(note), write))
}

/// What Harvest says about the row's billing, given the run's invoices.
fn billing(cache: &RunCache, row: &SourceRow) -> Billing {
    if !row.invoiced {
        return Billing::NotBilled;
    }
    match row.harvest_invoice_id.and_then(|id| cache.invoice(id)) {
        Some(InvoiceSlot::Imported { id, voided }) => Billing::Imported {
            invoice: id,
            voided_in_horae: voided,
        },
        Some(InvoiceSlot::Draft) => Billing::Draft,
        Some(InvoiceSlot::Failed) | None => Billing::Missing,
    }
}

/// Back to open, with no invoice and no frozen rounding, as a void reopens.
async fn reopen(sp: &mut Transaction<'_, Postgres>, entry_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE time_entries SET invoice_id = NULL, state = 'open', rounded_minutes = NULL
         WHERE id = $1",
        entry_id,
    )
    .execute(&mut **sp)
    .await?;
    Ok(())
}

/// The import's own billing write is not an edit made in Horae: remember the
/// entry's new version so a later Harvest change can still update it.
async fn remember_version(
    sp: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    entry_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE harvest_import_map AS m SET horae_updated_at = te.updated_at
           FROM time_entries AS te
          WHERE m.org_id = $1 AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
            AND m.horae_id = $2 AND te.id = $2",
        org_id,
        entry_id,
    )
    .execute(&mut **sp)
    .await?;
    Ok(())
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
    (project_id, task_id, user_id): (Uuid, Uuid, Uuid),
    row: &SourceRow,
    notices: &mut Vec<IdNotice>,
) -> Result<EntryApplied, RowFailure> {
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
                        AS "edited?",
                      te.state AS "state?: EntryState",
                      te.invoice_id,
                      (SELECT i.source = 'harvest' FROM invoices AS i WHERE i.id = te.invoice_id)
                        AS "invoice_imported?"
               FROM harvest_import_map AS m
               -- LATERAL starts from the provenance row: without statistics
               -- inside an import, a plain join can start from the entries.
               LEFT JOIN LATERAL (
                 SELECT e.id, e.state, e.updated_at, e.created_at, e.invoice_id
                 FROM time_entries AS e WHERE e.id = m.horae_id AND e.org_id = m.org_id
               ) AS te ON true
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
        let current = mapped
            .state
            .map(|state| current_of(state, mapped.invoice_id, mapped.invoice_imported));
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
            return Ok(EntryApplied {
                current,
                ..EntryApplied::new(outcome, None, mapped.entry_id)
            });
        }
        // Billing and unbilling move an entry's version in Harvest (T002). An
        // entry Harvest bills that is no longer open here, or was edited
        // here, keeps its fields; the link step bills it or reports the
        // conflict (FR-010). Harvest's version is not recorded as applied,
        // since none of its fields were. When Harvest also changed a field,
        // the row says so, as for an entry Harvest does not bill.
        if row.invoiced && (mapped.open != Some(true) || mapped.edited != Some(false)) {
            let differs = sqlx::query_scalar!(
                r#"SELECT project_id <> $2 OR task_id <> $3 OR user_id <> $4 OR spent_date <> $5
                          OR minutes <> $6 OR COALESCE(notes, '') <> COALESCE($7, '') AS "differs!"
                   FROM time_entries WHERE id = $1"#,
                mapped.horae_id,
                project_id,
                task_id,
                user_id,
                row.spent_date as chrono::NaiveDate,
                minutes,
                notes,
            )
            .fetch_one(&mut **sp)
            .await?;
            let outcome = if differs {
                RowOutcome::Errored {
                    source_location: row.source_location.clone(),
                    reason: "Harvest changed this entry while billing it, but it was changed or \
                             approved in Horae; billed with Horae's fields, not updated from Harvest"
                        .into(),
                    person: None,
                    entry_kind: Some(EntryErrorKind::ChangedInHorae),
                }
            } else {
                RowOutcome::Skipped
            };
            return Ok(EntryApplied {
                current,
                ..EntryApplied::new(outcome, None, Some(mapped.horae_id))
            });
        }
        let import_billed = matches!(
            current,
            Some(Current::BilledWithoutInvoice | Current::OnImported(_))
        );
        // Time the import billed that Harvest no longer bills reopens (the
        // page's billing write, FR-012). Edited in Horae meanwhile, it keeps
        // its fields and the row reports it, rather than Harvest's version
        // overwriting the edit.
        if import_billed && mapped.edited != Some(false) {
            let changed = RowOutcome::Errored {
                source_location: row.source_location.clone(),
                reason: "Harvest no longer bills this entry, which was changed in Horae while \
                         billed; its billing follows Harvest, its fields stay as Horae has them"
                    .into(),
                person: None,
                entry_kind: Some(EntryErrorKind::ChangedInHorae),
            };
            return Ok(EntryApplied {
                current,
                ..EntryApplied::new(changed, None, Some(mapped.horae_id))
            });
        }
        let mut reopened = false;
        if import_billed {
            reopen(sp, mapped.horae_id).await?;
            remember_version(sp, org.org_id, mapped.horae_id).await?;
            reopened = true;
        }
        if mapped.open != Some(true) && !reopened {
            return Err(RowFailure::entry(
                EntryErrorKind::ChangedInHorae,
                "entry is no longer open in Horae; not updated from Harvest",
            ));
        }
        if mapped.edited != Some(false) && !reopened {
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
        // Only an open entry takes Harvest's edit.
        return Ok(EntryApplied {
            reopened,
            current: Some(Current::Unbilled),
            ..EntryApplied::new(RowOutcome::Updated, None, Some(mapped.horae_id))
        });
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
        return Ok(EntryApplied::new(RowOutcome::Skipped, entry_slot, Some(id)));
    }

    // Create. State starts `open`; the link step bills it as Harvest did.
    // A row past the day's limit on its own fails here, not on the trigger.
    horae_core::duration::check_daily_total(minutes.into())
        .map_err(|limit| RowFailure::new(limit.to_string()))?;
    crate::db::lock_time_entry_write(sp, user_id).await?;
    let id = Uuid::now_v7();
    let mut wanted = resolve::free_harvest_id(
        sp,
        org.org_id,
        EntityType::TimeEntry,
        row.harvest_time_entry_id,
    )
    .await?;
    // A record that took the number since the check makes this insert do
    // nothing; the next attempt leaves the number to the trigger.
    let api_id = loop {
        let inserted = sqlx::query_scalar!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, harvest_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT (org_id, harvest_id) DO NOTHING RETURNING harvest_id",
        id,
        org.org_id,
        user_id,
        project_id,
        task_id,
        row.spent_date as chrono::NaiveDate,
        minutes,
        notes,
        row.billable,
        wanted,
    )
    .fetch_optional(&mut **sp)
    .await?;
        match (inserted, wanted) {
            (Some(api_id), _) => break api_id,
            (None, Some(_)) => wanted = None,
            (None, None) => return Err(RowFailure::new("no free id for the record")),
        }
    };
    notices.extend(resolve::id_notice(
        EntityType::TimeEntry,
        row.harvest_time_entry_id,
        api_id,
    ));
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
    Ok(EntryApplied {
        current: Some(Current::Unbilled),
        ..EntryApplied::new(RowOutcome::Created, entry_slot, Some(id))
    })
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
    pending.notices.extend(resolved.notice);
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
        notices: Vec::new(),
        link: None,
        write: None,
    }
}
