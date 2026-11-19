//! Patching an entry: only the named fields change, `null` clears, and the
//! Timesheet's Update rules refuse the same entries with the same reasons.

use super::*;
use crate::ops::time::EntryPatch;

async fn patch(
    f: &Fixture,
    pool: &PgPool,
    entry: Uuid,
    patch: EntryPatch,
) -> Result<TimeEntry, OpError> {
    let mut write = ops::begin(pool, Caller::session(f.ids.org_id, f.ids.user_id)).await?;
    let done =
        crate::ops::time::patch(&mut write.tx, &write.actor, &f.context, entry, patch).await?;
    write.tx.commit().await?;
    Ok(done.value.entry.expect("a patch answers its entry"))
}

async fn stored(pool: &PgPool, id: Uuid) -> (i32, Option<String>, Option<i32>, NaiveDate) {
    let row = sqlx::query!(
        r#"SELECT minutes, notes, start_minute, spent_date AS "spent_date: NaiveDate"
           FROM time_entries WHERE id=$1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.minutes, row.notes, row.start_minute, row.spent_date)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_patch_changes_only_the_named_fields(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    let patched = patch(
        &f,
        &pool,
        entry,
        EntryPatch {
            minutes: Some(90),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!((patched.id, patched.minutes), (entry, 90));
    let day: NaiveDate = "2026-09-07".parse().unwrap();
    assert_eq!(stored(&pool, entry).await, (90, None, None, day));

    let patched = patch(
        &f,
        &pool,
        entry,
        EntryPatch {
            notes: Some(Some("Review".into())),
            start_minute: Some(Some(540)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(patched.notes.as_deref(), Some("Review"));
    assert_eq!(
        stored(&pool, entry).await,
        (90, Some("Review".into()), Some(540), day),
        "the earlier change is kept"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn null_clears_notes_and_the_start(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    sqlx::query!(
        "UPDATE time_entries SET notes='Draft', start_minute=540 WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    patch(
        &f,
        &pool,
        entry,
        EntryPatch {
            notes: Some(None),
            start_minute: Some(None),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let (minutes, notes, start, _) = stored(&pool, entry).await;
    assert_eq!((minutes, notes, start), (60, None, None));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn an_empty_patch_saves_nothing_and_answers_the_entry(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    let mut write = ops::begin(&pool, Caller::session(f.ids.org_id, f.ids.user_id))
        .await
        .unwrap();
    let done = crate::ops::time::patch(
        &mut write.tx,
        &write.actor,
        &f.context,
        entry,
        EntryPatch::default(),
    )
    .await
    .unwrap();
    write.tx.commit().await.unwrap();
    assert!(done.events.is_empty(), "no change, no event");
    assert_eq!(done.value.entry.map(|e| e.id), Some(entry));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_patch_is_refused_like_the_update_on_locked_approved_and_invoiced_entries(pool: PgPool) {
    let f = fixture(&pool).await;
    for state in [
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ] {
        let entry = time_entry(
            &pool,
            &SeedIds {
                user_id: f.context.subject_id,
                ..f.ids
            },
            state,
        )
        .await;
        let patched = patch(
            &f,
            &pool,
            entry,
            EntryPatch {
                minutes: Some(5),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
        let update = f
            .execute(
                &pool,
                TimesheetCommand::Update {
                    entry_id: entry,
                    entry: TimesheetEntryInput {
                        minutes: 5,
                        notes: None,
                        ..f.input()
                    },
                },
            )
            .await
            .unwrap_err();
        assert_eq!(patched, update, "{state:?}");
        assert!(matches!(patched, OpError::Conflict(_)), "{patched:?}");
        assert_eq!(stored(&pool, entry).await.0, 60, "{state:?} unchanged");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_patch_validates_like_the_update(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    for (change, field) in [
        (
            EntryPatch {
                minutes: Some(-1),
                ..Default::default()
            },
            "minutes",
        ),
        (
            EntryPatch {
                start_minute: Some(Some(1440)),
                ..Default::default()
            },
            "start_minute",
        ),
    ] {
        let error = patch(&f, &pool, entry, change).await.unwrap_err();
        assert!(
            matches!(error, OpError::Invalid { field: Some(name), .. } if name == field),
            "{error:?}"
        );
    }
    let unknown = patch(&f, &pool, Uuid::now_v7(), EntryPatch::default()).await;
    assert!(matches!(unknown, Err(OpError::Conflict(_))), "{unknown:?}");
}
