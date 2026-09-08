use super::*;
use chrono::TimeZone;
use serde_json::json;

const KEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";

async fn apply_api_data(
    pool: &PgPool,
    org_id: Uuid,
    currency: &str,
    mode: ImportMode,
    data: &HarvestData,
    capture_started_at: DateTime<Utc>,
) -> anyhow::Result<ImportReport> {
    let mut connection = lock_import(pool, org_id).await?;
    let result = super::apply_api_data(
        &mut connection,
        org_id,
        currency,
        mode,
        data,
        capture_started_at,
    )
    .await;
    connection.close().await?;
    result
}

fn day(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn disconnect_and_reconnect_preserves_imported_records_and_exact_identity(pool: PgPool) {
    let org = setup(&pool).await;
    let first = apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(first.summary.time_entries.created, 1);
    credentials::disconnect(&pool, org).await.unwrap();
    assert!(
        credentials::store(
            &pool,
            org,
            KEY,
            "other-account",
            "access",
            "refresh",
            None,
            None
        )
        .await
        .is_err()
    );
    credentials::store(
        &pool,
        org,
        KEY,
        "test-account",
        "access",
        "refresh",
        None,
        None,
    )
    .await
    .unwrap();
    let second = apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 1);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn refreshed_tokens_survive_a_dry_run_rollback_in_the_same_import_session(pool: PgPool) {
    let org = setup(&pool).await;
    let mut connection = lock_import(&pool, org).await.unwrap();
    credentials::update_tokens(
        &mut *connection,
        org,
        KEY,
        "new-access",
        "new-refresh",
        Some(day(4)),
    )
    .await
    .unwrap();
    let report = super::apply_api_data(
        &mut connection,
        org,
        "USD",
        ImportMode::DryRun,
        &valid_data(),
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(report.summary.time_entries.created, 1);
    connection.close().await.unwrap();
    let stored = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    assert_eq!(
        (stored.access_token.as_str(), stored.refresh_token.as_str()),
        ("new-access", "new-refresh")
    );
    assert_eq!(stored.watermark, json!({}));
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

async fn setup(pool: &PgPool) -> Uuid {
    let org = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Sync Test')",
        org
    )
    .execute(pool)
    .await
    .unwrap();
    credentials::store(
        pool,
        org,
        KEY,
        "test-account",
        "access",
        "refresh",
        None,
        None,
    )
    .await
    .unwrap();
    add_user(pool, org, "known@example.com").await;
    org
}

async fn add_user(pool: &PgPool, org: Uuid, email: &str) {
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Sync User')",
        Uuid::now_v7(),
        org,
        email,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn watermark(pool: &PgPool, org: Uuid) -> serde_json::Value {
    credentials::load(pool, org, KEY)
        .await
        .unwrap()
        .unwrap()
        .watermark
}

fn data() -> HarvestData {
    HarvestData {
        clients: serde_json::from_value(json!([{"id":1,"name":"Client","currency":"USD"}]))
            .unwrap(),
        projects: serde_json::from_value(json!([{"id":2,"name":"Project","client":{"id":1}}]))
            .unwrap(),
        tasks: serde_json::from_value(json!([{"id":3,"name":"Task"}])).unwrap(),
        users: serde_json::from_value(json!([
            {"id":4,"email":"known@example.com"},
            {"id":5,"email":"missing@example.com"}
        ]))
        .unwrap(),
        time_entries: serde_json::from_value(json!([
            {"id":10,"spent_date":"2026-01-01","hours":1,"project":{"id":2},
             "task":{"id":3},"user":{"id":4},"updated_at":day(3)},
            {"id":11,"spent_date":"2026-01-01","hours":2,"project":{"id":2},
             "task":{"id":3},"user":{"id":5},"updated_at":day(2)}
        ]))
        .unwrap(),
    }
}

fn valid_data() -> HarvestData {
    let mut data = data();
    data.time_entries.truncate(1);
    data
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_rows_are_available_to_the_next_incremental_retry(pool: PgPool) {
    let org = setup(&pool).await;
    credentials::advance_watermark(&pool, org, &[(EntityType::TimeEntry, day(1))])
        .await
        .unwrap();
    let first = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data(), day(4))
        .await
        .unwrap();
    assert_eq!(first.error_count(), 1);
    assert_eq!(first.summary.time_entries.created, 1);
    let connection = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    assert_eq!(
        connection.watermark_for(EntityType::TimeEntry),
        Some(day(1))
    );

    add_user(&pool, org, "missing@example.com").await;
    let mut retry = data();
    let since = connection.watermark_for(EntityType::TimeEntry).unwrap();
    retry
        .time_entries
        .retain(|entry| entry.updated_at.is_some_and(|at| at >= since));
    let second = apply_api_data(&pool, org, "USD", ImportMode::Commit, &retry, day(4))
        .await
        .unwrap();
    assert_eq!(second.error_count(), 0);
    assert_eq!(second.summary.time_entries.created, 1);
    assert_eq!(second.summary.time_entries.skipped, 1);
    let minutes = sqlx::query_scalar!("SELECT minutes FROM time_entries ORDER BY minutes")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(minutes, vec![60, 120]);
    assert_eq!(
        watermark(&pool, org).await,
        json!({"time_entry":(day(3) - chrono::Duration::seconds(1)).to_rfc3339()})
    );
    let connection = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    let since = connection.watermark_for(EntityType::TimeEntry).unwrap();
    let mut boundary_retry = data();
    boundary_retry
        .time_entries
        .retain(|entry| entry.updated_at.is_some_and(|at| at > since));
    let third = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &boundary_retry,
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(third.summary.time_entries.created, 0);
    assert_eq!(third.summary.time_entries.skipped, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn empty_sync_does_not_invent_a_watermark(pool: PgPool) {
    let org = setup(&pool).await;
    apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &HarvestData::default(),
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(watermark(&pool, org).await, json!({}));
    credentials::advance_watermark(&pool, org, &[(EntityType::TimeEntry, day(1))])
        .await
        .unwrap();
    apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &HarvestData::default(),
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(
        watermark(&pool, org).await,
        json!({"time_entry":day(1).to_rfc3339()})
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn successful_sync_only_marks_the_incrementally_fetched_entity(pool: PgPool) {
    let org = setup(&pool).await;
    credentials::advance_watermark(&pool, org, &[(EntityType::Client, day(1))])
        .await
        .unwrap();
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(
        watermark(&pool, org).await,
        json!({
            "client":day(1).to_rfc3339(), "time_entry":(day(3) - chrono::Duration::seconds(1)).to_rfc3339()
        })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn watermark_never_passes_the_start_of_capture(pool: PgPool) {
    let org = setup(&pool).await;
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(2))
        .await
        .unwrap();
    assert_eq!(
        watermark(&pool, org).await,
        json!({"time_entry":(day(2) - chrono::Duration::seconds(1)).to_rfc3339()})
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_source_timestamp_keeps_the_previous_watermark(pool: PgPool) {
    let org = setup(&pool).await;
    add_user(&pool, org, "missing@example.com").await;
    credentials::advance_watermark(&pool, org, &[(EntityType::TimeEntry, day(1))])
        .await
        .unwrap();
    let mut data = data();
    data.time_entries[1].updated_at = None;
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    assert_eq!(report.error_count(), 0);
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!(
        watermark(&pool, org).await,
        json!({"time_entry":day(1).to_rfc3339()})
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn dry_run_leaves_data_provenance_and_watermark_unchanged(pool: PgPool) {
    let org = setup(&pool).await;
    let report = apply_api_data(&pool, org, "USD", ImportMode::DryRun, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(report.summary.time_entries.created, 1);
    assert_eq!(watermark(&pool, org).await, json!({}));
    let counts = sqlx::query!(
        r#"SELECT
        (SELECT COUNT(*) FROM time_entries) AS "entries!",
        (SELECT COUNT(*) FROM harvest_import_map) AS "mappings!""#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((counts.entries, counts.mappings), (0, 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn watermark_write_failure_rolls_back_imported_data(pool: PgPool) {
    let org = setup(&pool).await;
    sqlx::query!("ALTER TABLE harvest_credentials ADD CONSTRAINT reject_sync_marker CHECK (synced_watermark = '{}'::jsonb)")
        .execute(&pool).await.unwrap();
    let result = apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4)).await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("reject_sync_marker")
    );
    let counts = sqlx::query!(
        r#"SELECT
        (SELECT COUNT(*) FROM time_entries) AS "entries!",
        (SELECT COUNT(*) FROM harvest_import_map) AS "mappings!""#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((counts.entries, counts.mappings), (0, 0));
}
