use super::*;
use serial_test::serial;

async fn snapshot(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_array(
            (SELECT jsonb_agg(to_jsonb(o) ORDER BY id) FROM organizations o),
            (SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            (SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM clients c),
            (SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM projects p),
            (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tasks t),
            (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM assignments a),
            (SELECT jsonb_agg(to_jsonb(pt) ORDER BY project_id, task_id) FROM project_tasks pt),
            (SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM time_entries e)
        ) AS "snapshot!""#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn repeated_seed_preserves_every_row_and_user_edits(pool: PgPool) {
    run(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents = 123 WHERE project_id = $1",
        PROJ_ACME_ID
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE time_entries SET minutes = 1, notes = 'Edited demo'")
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    run(&pool).await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn seed_refuses_an_initialized_organization_without_writes(pool: PgPool) {
    crate::init::run(&pool, "Real Org", "admin@real.example", "Owner")
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert!(run(&pool).await.is_err());
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn late_seed_failure_rolls_back_all_demo_data(pool: PgPool) {
    sqlx::query!("ALTER TABLE time_entries ADD CONSTRAINT seed_test_failure CHECK (minutes < 300)")
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    assert!(run(&pool).await.is_err());
    assert_eq!(snapshot(&pool).await, before);
    sqlx::query!("ALTER TABLE time_entries DROP CONSTRAINT seed_test_failure")
        .execute(&pool)
        .await
        .unwrap();
    run(&pool).await.unwrap();
    let entries = sqlx::query!(
        r#"SELECT id, spent_date as "spent_date: NaiveDate", minutes FROM time_entries"#
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(entries.len(), 10);
    assert_eq!(entries.iter().map(|e| e.minutes).sum::<i32>(), 1455);
    let monday = iso_week_monday(Utc::now().date_naive());
    assert!(entries.iter().all(
        |e| e.id.get_version_num() == 7 && (monday..=monday + days(4)).contains(&e.spent_date)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn concurrent_seed_runs_create_only_one_demo(pool: PgPool) {
    let (first, second) = tokio::join!(run(&pool), run(&pool));
    first.unwrap();
    second.unwrap();
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM time_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, Some(10));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn seed_leaves_an_older_or_partial_demo_unchanged(pool: PgPool) {
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Renamed old demo')",
        ORG_ID
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    run(&pool).await.unwrap();
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn seed_refuses_a_second_organization_even_when_the_demo_exists(pool: PgPool) {
    run(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Other organization')",
        Uuid::now_v7()
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool).await;
    assert!(run(&pool).await.is_err());
    assert_eq!(snapshot(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn concurrent_init_and_seed_cannot_mix_organizations(pool: PgPool) {
    let (seed, init) = tokio::join!(
        run(&pool),
        crate::init::run(&pool, "Real Org", "owner@real.example", "Owner")
    );
    assert_ne!(seed.is_ok(), init.is_ok(), "seed: {seed:?}; init: {init:?}");
    let organizations = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(organizations, Some(1));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn concurrent_initializers_cannot_create_two_organizations(pool: PgPool) {
    let (first, second) = tokio::join!(
        crate::init::run(&pool, "First", "first@real.example", "First"),
        crate::init::run(&pool, "Second", "second@real.example", "Second")
    );
    assert_ne!(
        first.is_ok(),
        second.is_ok(),
        "first: {first:?}; second: {second:?}"
    );
    let organizations = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(organizations, Some(1));
}
