use super::*;

const AHEAD: (&str, i64) = ("Pacific/Kiritimati", 14);
const BEHIND: (&str, i64) = ("Etc/GMT+12", -12);

/// The zone the timer should follow, and the other one. UTC+14 and UTC−12 are
/// never on the same day, and the first is chosen to be off UTC's day right
/// now, so a timer dated in the other zone or in UTC fails the test.
fn zones() -> ((&'static str, i64), (&'static str, i64)) {
    use chrono::Timelike;
    if Utc::now().hour() >= 11 {
        (AHEAD, BEHIND)
    } else {
        (BEHIND, AHEAD)
    }
}

fn date_at_offset(hours: i64) -> NaiveDate {
    (Utc::now() + chrono::Duration::hours(hours)).date_naive()
}

async fn set_workspace_zone(pool: &PgPool, f: &Fixture, zone: &str) {
    sqlx::query!(
        "UPDATE organizations SET timezone=$2 WHERE id=$1",
        f.ids.org_id,
        zone
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn set_person_zone(pool: &PgPool, user_id: Uuid, zone: &str) {
    sqlx::query!("UPDATE users SET timezone=$2 WHERE id=$1", user_id, zone)
        .execute(pool)
        .await
        .unwrap();
}

/// Start a timer for the fixture's subject and return the day it was filed on,
/// with the expected day read on both sides in case midnight passes meanwhile.
async fn timer_day(pool: &PgPool, f: &Fixture, offset_hours: i64) -> (NaiveDate, [NaiveDate; 2]) {
    let before = date_at_offset(offset_hours);
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
    let after = date_at_offset(offset_hours);
    let day = sqlx::query_scalar!(
        r#"SELECT spent_date AS "spent_date: NaiveDate" FROM time_entries WHERE user_id=$1 AND is_running"#,
        f.context.subject_id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (day, [before, after])
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_timer_starts_on_the_workspace_day_by_default(pool: PgPool) {
    let f = fixture(&pool).await;
    let (zone, _) = zones();
    set_workspace_zone(&pool, &f, zone.0).await;
    let (day, expected) = timer_day(&pool, &f, zone.1).await;
    assert!(expected.contains(&day), "{day} not in {expected:?}");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn the_persons_own_zone_overrides_the_workspace_day(pool: PgPool) {
    let f = fixture(&pool).await;
    let (zone, other) = zones();
    set_workspace_zone(&pool, &f, other.0).await;
    set_person_zone(&pool, f.context.subject_id, zone.0).await;
    let (day, expected) = timer_day(&pool, &f, zone.1).await;
    assert!(expected.contains(&day), "{day} not in {expected:?}");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_delegated_timer_follows_the_subject_not_the_actor(pool: PgPool) {
    let f = fixture(&pool).await;
    let (zone, other) = zones();
    set_person_zone(&pool, f.ids.user_id, other.0).await;
    set_person_zone(&pool, f.context.subject_id, zone.0).await;
    let (day, expected) = timer_day(&pool, &f, zone.1).await;
    assert!(expected.contains(&day), "{day} not in {expected:?}");
}
