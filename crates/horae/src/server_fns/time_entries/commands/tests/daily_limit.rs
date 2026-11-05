use super::*;

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
    let set = |minutes: i32, rounded: Option<i32>| {
        sqlx::query!(
            "UPDATE time_entries SET minutes=$2, rounded_minutes=$3 WHERE id=$1",
            entry,
            minutes,
            rounded
        )
        .execute(&pool)
    };
    // The largest stored entry, rounded up by the largest smallint increment.
    set(14_400, Some(14_400 + i32::from(i16::MAX)))
        .await
        .unwrap();
    let minutes = set(14_401, None).await.unwrap_err();
    assert!(
        minutes
            .to_string()
            .contains("time_entries_minutes_daily_cap"),
        "{minutes}"
    );
    let rounded = set(60, Some(14_400 + i32::from(i16::MAX) + 1))
        .await
        .unwrap_err();
    assert!(
        rounded
            .to_string()
            .contains("time_entries_rounded_minutes_cap"),
        "{rounded}"
    );
    // Existing rows wait for a validating migration once production is checked.
    let validated = sqlx::query_scalar!(
        r#"SELECT bool_or(convalidated) AS "validated!" FROM pg_constraint
           WHERE conname IN ('time_entries_minutes_daily_cap', 'time_entries_rounded_minutes_cap')"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!validated);
}
