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
    release_import(connection).await?;
    result
}

fn day(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn api_requires_email_even_when_full_name_matches(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.users[0].email.clear();
    source.users[0].first_name = "Sync".into();
    source.users[0].last_name = "User".into();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(report.summary.time_entries.errored, 1);
    assert_eq!(report.summary.time_entries.created, 0);
    assert!(report.row_errors[0].reason.contains("no user email"));
    assert_eq!(watermark(&pool, org).await, json!({}));
    source.users[0].email = "known@example.com".into();
    let retry = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(retry.summary.time_entries.created, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn api_rejects_ambiguous_email_without_advancing_watermark(pool: PgPool) {
    let org = setup(&pool).await;
    add_user(&pool, org, "KNOWN@EXAMPLE.COM").await;
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(report.summary.time_entries.errored, 1);
    assert_eq!(report.summary.time_entries.created, 0);
    assert!(report.row_errors[0].reason.contains("ambiguous user email"));
    assert_eq!(watermark(&pool, org).await, json!({}));
}

#[sqlx::test(migrations = "./migrations")]
async fn parent_only_api_import_creates_catalog_and_reimport_skips_it(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.time_entries.clear();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            report.summary.clients.created,
            report.summary.projects.created,
            report.summary.tasks.created
        ),
        (1, 1, 1)
    );
    assert_eq!(report.summary.time_entries.processed(), 0);
    assert_eq!(watermark(&pool, org).await, json!({}));
    let repeated = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            repeated.summary.clients.skipped,
            repeated.summary.projects.skipped,
            repeated.summary.tasks.skipped
        ),
        (1, 1, 1)
    );
    assert_eq!(
        (
            repeated.summary.clients.created,
            repeated.summary.projects.created,
            repeated.summary.tasks.created
        ),
        (0, 0, 0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn parent_only_dry_run_previews_without_persisting_catalog(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.time_entries.clear();
    let preview = apply_api_data(&pool, org, "USD", ImportMode::DryRun, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            preview.summary.clients.created,
            preview.summary.projects.created,
            preview.summary.tasks.created
        ),
        (1, 1, 1)
    );
    let committed = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(preview.summary, committed.summary);
}

#[sqlx::test(migrations = "./migrations")]
async fn parent_catalog_preserves_metadata_and_counts_each_entity_once(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.clients = serde_json::from_value(json!([
        {"id":1,"name":"Client","address":"1 Road","currency":"EUR","is_active":false,"updated_at":"2026-01-01T00:00:00Z"},
        {"id":8,"name":"Unused client","currency":"USD"}
    ])).unwrap();
    source.projects = serde_json::from_value(json!([
        {"id":2,"name":"Project","code":"CODE","client":{"id":1},"starts_on":"2025-01-01","ends_on":"2025-12-31","is_active":false}
    ])).unwrap();
    source.tasks = serde_json::from_value(json!([
        {"id":3,"name":"Task","billable_by_default":false,"default_hourly_rate":42.25,"is_active":false},
        {"id":9,"name":"Unused task","default_hourly_rate":0}
    ])).unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(report.error_count(), 0);
    assert_eq!(
        (
            report.summary.clients.processed(),
            report.summary.projects.processed(),
            report.summary.tasks.processed(),
            report.summary.time_entries.created
        ),
        (2, 1, 2, 1)
    );
    let client = sqlx::query!(
        "SELECT address, currency, active FROM clients WHERE org_id = $1 AND name = 'Client'",
        org
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            client.address.as_deref(),
            client.currency.as_str(),
            client.active
        ),
        (Some("1 Road"), "EUR", false)
    );
    let project = sqlx::query!(
        r#"SELECT code, currency, starts_on as "starts_on: chrono::NaiveDate", ends_on as "ends_on: chrono::NaiveDate", active FROM projects WHERE org_id = $1"#,
        org
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            project.code.as_deref(),
            project.currency.as_str(),
            project.active
        ),
        (Some("CODE"), "EUR", false)
    );
    assert_eq!(
        project.starts_on,
        Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
    );
    assert_eq!(
        project.ends_on,
        Some(chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
    );
    let task = sqlx::query!("SELECT default_rate_cents, billable_default, active FROM tasks WHERE org_id = $1 AND name = 'Task'", org).fetch_one(&pool).await.unwrap();
    assert_eq!(
        (task.default_rate_cents, task.billable_default, task.active),
        (Some(4225), false, false)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT default_rate_cents FROM tasks WHERE org_id = $1 AND name = 'Unused task'",
            org
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    let stamp = sqlx::query_scalar!(r#"SELECT harvest_updated_at as "harvest_updated_at: DateTime<Utc>" FROM harvest_import_map WHERE org_id = $1 AND harvest_entity_type = 'client' AND harvest_id = 1"#, org).fetch_one(&pool).await.unwrap();
    assert_eq!(stamp, Some(day(1)));
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_parent_blocks_dependent_time_but_other_catalog_records_survive(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.tasks = serde_json::from_value(json!([
        {"id":3,"name":"Task","default_hourly_rate":-5},
        {"id":8,"name":"Valid unused task","default_hourly_rate":12}
    ]))
    .unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            report.summary.tasks.created,
            report.summary.tasks.errored,
            report.summary.time_entries.errored
        ),
        (1, 1, 1)
    );
    assert!(
        report
            .row_errors
            .iter()
            .any(|e| e.source_location == "task 3")
    );
    assert!(
        report
            .row_errors
            .iter()
            .any(|e| e.reason.contains("task 3 failed to import"))
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    source.tasks[0].default_hourly_rate = Some(serde_json::Number::from(20));
    let retry = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(retry.error_count(), 0);
    assert_eq!(
        (
            retry.summary.clients.skipped,
            retry.summary.projects.skipped,
            retry.summary.tasks.created,
            retry.summary.tasks.skipped,
            retry.summary.time_entries.created
        ),
        (1, 1, 1, 1, 1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_client_fallback_and_failed_project_savepoints_do_not_poison_cache(pool: PgPool) {
    let org = setup(&pool).await;
    let source = HarvestData {
        projects: serde_json::from_value(json!([
            {"id":1,"name":"Missing client","client":{"id":70}},
            {"id":2,"name":"","client":{"id":71,"name":"Recovered client"}},
            {"id":3,"name":"Valid project","client":{"id":71,"name":"Recovered client"}}
        ]))
        .unwrap(),
        ..HarvestData::default()
    };
    let report = apply_api_data(&pool, org, "EUR", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            report.summary.clients.created,
            report.summary.projects.created,
            report.summary.projects.errored
        ),
        (1, 1, 2)
    );
    let clients = sqlx::query!("SELECT name, currency FROM clients WHERE org_id = $1", org)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(
        (clients[0].name.as_str(), clients[0].currency.as_str()),
        ("Recovered client", "EUR")
    );
    assert!(
        super::provenance::lookup(&pool, org, EntityType::Project, 2)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        super::provenance::lookup(&pool, org, EntityType::Project, 3)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_client_propagates_to_project_and_entry_without_placeholder_records(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    source.clients[0].name.clear();
    source.projects[0].client.name = Some("Must not invent".into());
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            report.summary.clients.errored,
            report.summary.projects.errored,
            report.summary.tasks.created,
            report.summary.time_entries.errored
        ),
        (1, 1, 1, 1)
    );
    assert!(
        super::provenance::lookup(&pool, org, EntityType::Client, 1)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        super::provenance::lookup(&pool, org, EntityType::Project, 2)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
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
    release_import(connection).await.unwrap();
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
