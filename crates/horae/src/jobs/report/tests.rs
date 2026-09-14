use horae_core::importers::harvest::types::{
    EntityType, ImportMode, ImportReport, RowError, RowOutcome, SourceKind,
};
use sqlx::migrate::Migrate;
use uuid::Uuid;

use crate::{config::JobPolicy, jobs};

#[sqlx::test]
#[serial_test::serial]
async fn report_fragments_survive_until_the_owning_job_expires(pool: sqlx::PgPool) {
    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = jobs::enqueue(
        &pool,
        org_id,
        &jobs::JobPayload::HarvestCsv {
            mode: ImportMode::DryRun,
        },
        "report-retention",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::DryRun);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 1".into(),
            reason: "invalid date ".repeat(2_000),
        },
    );
    let mut tx = pool.begin().await.unwrap();
    lease.archive_report(&mut tx, &mut report).await.unwrap();
    let metadata = serde_json::to_value(&report).unwrap();
    lease
        .save_checkpoint(
            &mut tx,
            &serde_json::json!({ "version": 2, "report": metadata }),
            &metadata,
            "time_entries",
            1,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let end = i64::try_from(report.archived_error_chunks()).unwrap();
    let expected = super::chunks(&pool, org_id, id, 0, end).await.unwrap();
    assert!(!expected.is_empty());
    jobs::cleanup(&pool).await.unwrap();
    assert_eq!(
        super::chunks(&pool, org_id, id, 0, end).await.unwrap(),
        expected
    );

    let mut tx = pool.begin().await.unwrap();
    assert!(lease.complete(&mut tx, &metadata, 1).await.unwrap());
    tx.commit().await.unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET finished_at = now() - interval '2 days' WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    jobs::cleanup(&pool).await.unwrap();
    assert_eq!(
        super::chunks(&pool, org_id, id, 0, end).await.unwrap(),
        expected
    );

    sqlx::query!(
        "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    jobs::cleanup(&pool).await.unwrap();
    assert!(jobs::status(&pool, org_id, id).await.unwrap().is_none());
    assert!(super::chunks(&pool, org_id, id, 0, end).await.is_err());
}

/// Start before report bounds exist: inserting this fixture into the current
/// schema would bypass the deployment path that must preserve old checkpoints.
#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn upgrading_legacy_reports_preserves_errors_and_fences_old_attempts(pool: sqlx::PgPool) {
    let mut connection = pool.acquire().await.unwrap();
    connection.ensure_migrations_table().await.unwrap();
    for migration in sqlx::migrate!("./migrations")
        .iter()
        .filter(|m| m.version <= 27)
    {
        connection.apply(migration).await.unwrap();
    }
    drop(connection);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();

    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = jobs::enqueue(
        &pool,
        org_id,
        &jobs::JobPayload::HarvestCsv {
            mode: ImportMode::Commit,
        },
        "legacy-report-upgrade",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let (old_lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
    for index in 0..1_000 {
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: format!("CSV row {index}"),
                reason: if index == 500 {
                    "Razón 日本語 \"quoted\"\n".repeat(20_000)
                } else {
                    format!("Invalid date at record {index}")
                },
            },
        );
    }
    let mut legacy_report = serde_json::to_value(&report).unwrap();
    legacy_report.as_object_mut().unwrap().remove("version");
    let checkpoint = serde_json::json!({
        "version": 1,
        "report": legacy_report,
        "cursor": { "record": 1000, "byte": 987654 },
        "cache": { "confirmed_parent": "preserve-me" }
    });
    let mut tx = pool.begin().await.unwrap();
    old_lease
        .save_checkpoint(&mut tx, &checkpoint, &legacy_report, "time_entries", 1_000)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    sqlx::query!(
        "ALTER TABLE horae_job_report_error_chunks ADD CONSTRAINT reject_legacy_archive CHECK (sequence < 1)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let failure = crate::db::run_migrations(&pool).await.unwrap_err();
    assert!(failure.to_string().contains("reject_legacy_archive"));
    assert_eq!(
        old_lease
            .load_checkpoint(&mut pool.acquire().await.unwrap())
            .await
            .unwrap(),
        Some(checkpoint.clone()),
        "a failed upgrade must preserve the original checkpoint and claim"
    );
    assert!(super::chunks(&pool, org_id, id, 0, 1).await.is_err());
    sqlx::query!("ALTER TABLE horae_job_report_error_chunks DROP CONSTRAINT reject_legacy_archive")
        .execute(&pool)
        .await
        .unwrap();
    crate::db::run_migrations(&pool).await.unwrap();
    let status = jobs::status(&pool, org_id, id).await.unwrap().unwrap();
    let bounded = status.report.unwrap();
    assert!(
        serde_json::to_vec(&bounded).unwrap().len() <= super::REPORT_BYTES,
        "legacy report metadata must remain bounded after upgrading"
    );
    assert_eq!(status.processed_count, 1_000);
    assert_eq!(status.phase.as_deref(), Some("time_entries"));
    let history = jobs::list(&pool, org_id, 10, None).await.unwrap();
    assert_eq!(history[0].report.as_ref(), Some(&bounded));
    let archived: ImportReport = serde_json::from_value(bounded.clone()).unwrap();
    assert!(archived.reconciles());
    assert_eq!(archived.error_count(), report.error_count());
    let end = i64::try_from(archived.archived_error_chunks()).unwrap();
    assert!(end > 0);
    let mut next = 0;
    let mut bytes = Vec::new();
    while next < end {
        let page = super::chunks(&pool, org_id, id, next, end).await.unwrap();
        assert!(page.len() <= 16);
        next += page.len() as i64;
        for chunk in page {
            assert!(chunk.len() <= super::CHUNK_BYTES);
            bytes.extend(chunk);
        }
    }
    let mut errors: Vec<RowError> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    errors.extend(archived.row_errors);
    assert_eq!(errors, report.row_errors);

    let mut tx = pool.begin().await.unwrap();
    assert!(
        !old_lease
            .complete(&mut tx, &legacy_report, 1_000)
            .await
            .unwrap()
    );
    tx.rollback().await.unwrap();
    let (new_lease, _new_stop) = jobs::claim_lease_for_test(&pool).await;
    let saved = new_lease
        .load_checkpoint(&mut pool.acquire().await.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved["version"], 2);
    assert_eq!(saved["cursor"], checkpoint["cursor"]);
    assert_eq!(saved["cache"], checkpoint["cache"]);
    assert_eq!(saved["report"], bounded);
    crate::db::run_migrations(&pool).await.unwrap();
    assert_eq!(
        jobs::status(&pool, org_id, id)
            .await
            .unwrap()
            .unwrap()
            .report,
        Some(bounded)
    );
    let mut tx = pool.begin().await.unwrap();
    let rejected = new_lease
        .save_checkpoint(&mut tx, &checkpoint, &legacy_report, "time_entries", 1_000)
        .await
        .unwrap_err();
    assert!(rejected.to_string().contains("report_budget"));
    tx.rollback().await.unwrap();
    assert_eq!(
        new_lease
            .load_checkpoint(&mut pool.acquire().await.unwrap())
            .await
            .unwrap(),
        Some(saved),
        "even a live claim cannot restore an oversized report after upgrading"
    );
}
