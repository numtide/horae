use super::*;
use crate::server_fns::test_seed::{
    assert_minutes_cannot_rise, legacy_minutes, wait_for_blocked, wait_for_waiters,
};

const DAY: &str = "2026-09-07";
const NEXT_DAY: &str = "2026-09-08";

fn at(f: &Fixture, day: &str, minutes: i32) -> TimesheetEntryInput {
    TimesheetEntryInput {
        spent_date: day.parse().unwrap(),
        minutes,
        ..f.input()
    }
}

async fn create(pool: &PgPool, f: &Fixture, day: &str, minutes: i32) -> Result<(), ServerFnError> {
    f.execute(
        pool,
        TimesheetCommand::Create {
            entry: at(f, day, minutes),
        },
    )
    .await
}

#[track_caller]
fn assert_daily_limit(result: Result<(), ServerFnError>) {
    match result {
        Err(ServerFnError::ServerError {
            code: CONFLICT,
            message,
            ..
        }) => assert_eq!(message, "Daily hours cannot exceed 240."),
        other => panic!("expected the daily limit, got {other:?}"),
    }
}

async fn day_total(pool: &PgPool, f: &Fixture, day: NaiveDate) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0) AS "total!" FROM time_entries WHERE user_id=$1 AND spent_date=$2"#,
        f.context.subject_id,
        day as NaiveDate
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn running_timer(pool: &PgPool, f: &Fixture, ran_for_minutes: i32) -> Uuid {
    f.execute(
        pool,
        TimesheetCommand::StartTimer {
            project_id: f.ids.project_id,
            task_id: f.ids.task_id,
            notes: None,
        },
    )
    .await
    .unwrap();
    sqlx::query_scalar!(
        "UPDATE time_entries SET started_at = now() - make_interval(mins => $2)
         WHERE user_id=$1 AND is_running RETURNING id",
        f.context.subject_id,
        ran_for_minutes
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn a_day_holds_exactly_240_hours_and_no_single_entry_more(pool: PgPool) {
    let f = fixture(&pool).await;
    create(&pool, &f, DAY, 14_400).await.unwrap();
    assert_daily_limit(create(&pool, &f, NEXT_DAY, 14_401).await);
    // The legacy own-time edit refuses it the same way.
    let full = sqlx::query_scalar!(
        "SELECT id FROM time_entries WHERE user_id=$1",
        f.context.subject_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let legacy = update_entry(&pool, f.context.subject_id, full, 14_401, None, true, None).await;
    assert_daily_limit(legacy.map(|_| ()));
    assert_eq!(day_total(&pool, &f, DAY.parse().unwrap()).await, 14_400);
    assert_eq!(day_total(&pool, &f, NEXT_DAY.parse().unwrap()).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn entries_that_sum_past_240_hours_are_refused(pool: PgPool) {
    let f = fixture(&pool).await;
    create(&pool, &f, DAY, 10_000).await.unwrap();
    assert_daily_limit(create(&pool, &f, DAY, 4_401).await);
    create(&pool, &f, DAY, 4_400).await.unwrap();
    // Moving or growing other time onto the full day is refused too.
    create(&pool, &f, NEXT_DAY, 30).await.unwrap();
    let other = sqlx::query_scalar!(
        "SELECT id FROM time_entries WHERE user_id=$1 AND spent_date=$2",
        f.context.subject_id,
        NEXT_DAY.parse::<NaiveDate>().unwrap() as NaiveDate
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_daily_limit(
        f.execute(
            &pool,
            TimesheetCommand::Reschedule {
                entry_id: other,
                spent_date: DAY.parse().unwrap(),
                start_minute: 600,
                minutes: 30,
            },
        )
        .await,
    );
    assert_daily_limit(
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id: other,
                entry: at(&f, DAY, 30),
            },
        )
        .await,
    );
    assert_eq!(day_total(&pool, &f, DAY.parse().unwrap()).await, 14_400);
    assert_eq!(day_total(&pool, &f, NEXT_DAY.parse().unwrap()).await, 30);
}

/// A day stored over the limit before it existed can still be corrected
/// downward, but never raised further.
#[sqlx::test(migrations = "./migrations")]
async fn a_day_over_the_limit_can_be_lowered_but_not_raised(pool: PgPool) {
    let f = fixture(&pool).await;
    let (first, second) = (f.entry(&pool).await, f.entry(&pool).await);
    sqlx::query!(
        "UPDATE time_entries SET minutes=10000 WHERE id=ANY($1)",
        &[first, second]
    )
    .execute(&pool)
    .await
    .unwrap();
    let day: NaiveDate = DAY.parse().unwrap();
    let edit = |entry_id, minutes| {
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id,
                entry: at(&f, DAY, minutes),
            },
        )
    };
    edit(first, 9_000).await.unwrap();
    assert_eq!(day_total(&pool, &f, day).await, 19_000);
    assert_daily_limit(edit(first, 9_001).await);
    assert_daily_limit(create(&pool, &f, DAY, 1).await);
    // The legacy own-time edit lowers it the same way.
    update_entry(&pool, f.context.subject_id, second, 5_000, None, true, None)
        .await
        .unwrap();
    f.execute(
        &pool,
        TimesheetCommand::Delete {
            entry_ids: vec![first],
        },
    )
    .await
    .unwrap();
    assert_eq!(day_total(&pool, &f, day).await, 5_000);
}

/// Stopping is never refused, so a timer cannot be trapped running: it keeps
/// what is left of the day and says what the limit cut.
#[sqlx::test(migrations = "./migrations")]
async fn stopping_a_timer_keeps_only_the_rest_of_the_day(pool: PgPool) {
    let f = fixture(&pool).await;
    let today = Utc::now().date_naive();
    create(&pool, &f, &today.to_string(), 14_350).await.unwrap();

    let timer = running_timer(&pool, &f, 90).await;
    let effects = apply(
        &pool,
        f.ids.org_id,
        f.ids.user_id,
        &f.context,
        TimesheetCommand::StopTimer { entry_id: timer },
    )
    .await
    .unwrap();
    assert_eq!(
        effects.notice.as_deref(),
        Some(
            "Daily hours cannot exceed 240. The timer was stopped at 0:50; 0:40 over the limit was not recorded."
        )
    );
    let stopped = sqlx::query!(
        "SELECT minutes, is_running FROM time_entries WHERE id=$1",
        timer
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((stopped.minutes, stopped.is_running), (50, false));

    // On a full day the legacy stop still succeeds, keeping nothing.
    let timer = running_timer(&pool, &f, 30).await;
    let (stopped, notice) = stop_entry_timer(&pool, f.context.subject_id, timer)
        .await
        .unwrap();
    assert_eq!((stopped.minutes, stopped.is_running), (0, false));
    assert_eq!(
        notice.as_deref(),
        Some(
            "Daily hours cannot exceed 240. The timer was stopped at 0:00; 0:30 over the limit was not recorded."
        )
    );
    assert_eq!(day_total(&pool, &f, today).await, 14_400);
}

#[sqlx::test(migrations = "./migrations")]
async fn the_database_bounds_each_entry(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    let set = |minutes: i32| {
        sqlx::query!(
            "UPDATE time_entries SET minutes=$2 WHERE id=$1",
            entry,
            minutes
        )
        .execute(&pool)
    };
    set(14_400).await.unwrap();
    let refused = set(14_401).await.unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("Daily hours cannot exceed 240."),
        "{refused}"
    );
}

/// The trigger refuses only a new value over the limit: a legacy entry keeps
/// its minutes through every other write, such as the long-timer sweep's
/// claim, and may be lowered.
#[sqlx::test(migrations = "./migrations")]
async fn a_legacy_entry_over_the_limit_can_change_anything_but_raise_its_minutes(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    legacy_minutes(&pool, entry, 20_000).await;
    // `scheduler::sweep` needs the app state; this is the column its claim sets.
    sqlx::query!(
        "UPDATE time_entries SET is_running=true, started_at=now(), notified_long_running_at=now()
         WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_minutes_cannot_rise(&pool, entry).await;
    sqlx::query!("UPDATE time_entries SET minutes=19000 WHERE id=$1", entry)
        .execute(&pool)
        .await
        .unwrap();
}

async fn backend_pid(tx: &mut sqlx::PgConnection) -> i32 {
    sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(tx)
        .await
        .unwrap()
        .unwrap()
}

/// Run `writes` while another transaction holds the lock on `day`, and check
/// that `waiters` of them queue behind it before it is released.
async fn behind_day_lock<T>(
    pool: &PgPool,
    f: &Fixture,
    day: &str,
    waiters: i64,
    writes: impl std::future::Future<Output = T>,
) -> T {
    let mut holder = pool.begin().await.unwrap();
    let pid = backend_pid(&mut holder).await;
    crate::db::daily_minutes_totals(&mut holder, f.context.subject_id, &[day.parse().unwrap()])
        .await
        .unwrap();
    let release = async {
        wait_for_waiters(pool, pid, waiters).await;
        holder.commit().await.unwrap();
    };
    tokio::join!(writes, release).0
}

/// Two writers on the same empty day queue on its lock, so exactly one of two
/// 7,300-minute entries gets in.
#[sqlx::test(migrations = "./migrations")]
async fn concurrent_writers_cannot_overshoot_a_day_together(pool: PgPool) {
    let f = fixture(&pool).await;
    let both = async { tokio::join!(create(&pool, &f, DAY, 7_300), create(&pool, &f, DAY, 7_300)) };
    let (first, second) = behind_day_lock(&pool, &f, DAY, 2, both).await;
    let (created, refused): (Vec<_>, Vec<_>) = [first, second].into_iter().partition(Result::is_ok);
    assert_eq!((created.len(), refused.len()), (1, 1));
    refused.into_iter().for_each(assert_daily_limit);
    assert_eq!(day_total(&pool, &f, DAY.parse().unwrap()).await, 7_300);
}

/// Decreases wait for the day lock too, so they cannot land between another
/// writer's before and after totals: a delete, and the day a reschedule or a
/// reorder moves an entry off.
#[sqlx::test(migrations = "./migrations")]
async fn a_delete_waits_for_the_day_lock(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    let delete = delete_entry(&pool, f.context.subject_id, entry);
    behind_day_lock(&pool, &f, DAY, 1, delete).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn a_reschedule_waits_for_its_source_day_lock(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = f.entry(&pool).await;
    let next_day = NEXT_DAY.parse().unwrap();
    let move_off = reschedule_entry(&pool, f.context.subject_id, entry, next_day, 600, 60);
    behind_day_lock(&pool, &f, DAY, 1, move_off).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn a_reorder_waits_for_its_source_day_lock(pool: PgPool) {
    let f = fixture(&pool).await;
    let entries = [f.entry(&pool).await];
    let next_day = NEXT_DAY.parse().unwrap();
    let move_off = reorder_entries(&pool, f.context.subject_id, next_day, &entries);
    behind_day_lock(&pool, &f, DAY, 1, move_off).await.unwrap();
}

/// The widget stop locks the running entry before its day, like every other
/// write: a writer holding the row and then taking the day lock does not
/// deadlock with it.
#[sqlx::test(migrations = "./migrations")]
async fn a_widget_stop_waits_for_the_entry_lock(pool: PgPool) {
    let f = fixture(&pool).await;
    let timer = running_timer(&pool, &f, 30).await;
    let mut holder = pool.begin().await.unwrap();
    let pid = backend_pid(&mut holder).await;
    let day = sqlx::query_scalar!(
        r#"SELECT spent_date AS "spent_date: NaiveDate" FROM time_entries WHERE id=$1 FOR UPDATE"#,
        timer
    )
    .fetch_one(&mut *holder)
    .await
    .unwrap();
    let holder_writes = async {
        wait_for_blocked(&pool, pid).await;
        crate::db::daily_minutes_totals(&mut holder, f.context.subject_id, &[day]).await?;
        holder.commit().await
    };
    let (stopped, held) = tokio::join!(
        stop_entry_timer(&pool, f.context.subject_id, timer),
        holder_writes
    );
    held.unwrap();
    assert!(!stopped.unwrap().0.is_running);
}
