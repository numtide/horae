use horae_core::importers::harvest::types::{
    EntityType, ImportMode, ImportReport, RowError, RowOutcome, SourceKind,
};
use sqlx::migrate::Migrate;
use uuid::Uuid;

use crate::{config::JobPolicy, jobs};

/// Migration fixtures must use the pre-generation schema, not today's enqueue path.
async fn legacy_csv_job(pool: &sqlx::PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    let payload = jobs::JobPayload::HarvestCsv {
        mode: ImportMode::Commit,
    };
    let encoded = jobs::encode_payload(&payload).unwrap();
    let report = payload.initial_report().unwrap();
    let key = id.to_string();
    let attempts = JobPolicy::default().max_attempts;
    sqlx::query!(
        "INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts, report) VALUES ($1, $2, 'harvest_csv_import', $3, $4, $6, $5)",
        id, org, encoded, key, report, attempts
    ).execute(pool).await.unwrap();
    id
}

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
    let pool = legacy_pool(pool).await;

    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = legacy_csv_job(&pool, org_id).await;
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

async fn legacy_pool(pool: sqlx::PgPool) -> sqlx::PgPool {
    let mut connection = pool.acquire().await.unwrap();
    connection.ensure_migrations_table().await.unwrap();
    for migration in sqlx::migrate!("./migrations")
        .iter()
        .filter(|m| m.version <= 27)
    {
        connection.apply(migration).await.unwrap();
    }
    drop(connection);
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn legacy_upgrade_preserves_job_states_and_existing_archives(pool: sqlx::PgPool) {
    use serde_json::json;

    let pool = legacy_pool(pool).await;
    let org = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut cases = Vec::new();
    for (state, cancellation) in [
        ("queued", false),
        ("running", false),
        ("running", true),
        ("succeeded", false),
        ("failed", false),
        ("cancelled", true),
    ] {
        for version in 0..=2 {
            for with_checkpoint in [false, true] {
                let id = legacy_csv_job(&pool, org).await;
                let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
                let mut expected = Vec::new();
                let mut prefix = Vec::new();
                if version == 2 {
                    report.record(
                        EntityType::TimeEntry,
                        &RowOutcome::Errored {
                            source_location: "previously archived".into(),
                            reason: "keep this fragment".into(),
                        },
                    );
                    expected.extend(report.row_errors.clone());
                    serde_json::to_writer(&mut prefix, &report.row_errors[0]).unwrap();
                    prefix.push(b'\n');
                    super::append_report_chunk(
                        &mut pool.acquire().await.unwrap(),
                        id,
                        org,
                        0,
                        &prefix,
                    )
                    .await
                    .unwrap();
                    report.archive_errors(1).unwrap();
                }
                report.record(
                    EntityType::TimeEntry,
                    &RowOutcome::Errored {
                        source_location: "legacy inline".into(),
                        reason: "legacy overflow ".repeat(2_000),
                    },
                );
                expected.extend(report.row_errors.clone());
                let mut legacy = serde_json::to_value(&report).unwrap();
                if version == 0 {
                    legacy.as_object_mut().unwrap().remove("version");
                }
                assert!(serde_json::to_vec(&legacy).unwrap().len() > super::REPORT_BYTES);
                let checkpoint = with_checkpoint.then(|| {
                    json!({
                        "version": if version == 2 { 2 } else { 1 },
                        "report": legacy,
                        "cursor": {"record": 500}, "cache": {"parent": "keep"},
                    })
                });
                sqlx::query!(
                    "UPDATE horae_jobs SET status = $2, report = $3, checkpoint = $4,
                         attempts = 3, processed_count = 500, phase = 'time_entries',
                         cancellation_requested = $5, last_error = 'retained diagnostic',
                         started_at = now() - interval '1 hour',
                         finished_at = CASE WHEN $2 IN ('failed', 'cancelled', 'succeeded') THEN now() ELSE NULL END,
                         claim_token = CASE WHEN $2 = 'running' THEN $6::uuid ELSE NULL END,
                         lease_until = CASE WHEN $2 = 'running' THEN now() + interval '5 minutes' ELSE NULL END
                     WHERE id = $1",
                    id, state, legacy, checkpoint, cancellation, Uuid::now_v7(),
                ).execute(&pool).await.unwrap();
                let original = legacy_job_metadata(&pool, id).await;
                cases.push((id, state, checkpoint, original, expected, prefix));
            }
        }
    }

    crate::db::run_migrations(&pool).await.unwrap();
    for (id, state, checkpoint, original, expected, prefix) in cases {
        assert_eq!(
            legacy_job_metadata(&pool, id).await,
            original,
            "state: {state}"
        );
        let stored = sqlx::query!(
            "SELECT report, checkpoint, claim_token, account_generation,
                    lease_until <= clock_timestamp() AS expired
             FROM horae_jobs WHERE id = $1",
            id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored.account_generation, 0);
        assert!(stored.claim_token.is_none());
        assert_eq!(stored.expired, (state == "running").then_some(true));
        let metadata = stored.report.unwrap();
        assert!(serde_json::to_vec(&metadata).unwrap().len() <= super::REPORT_BYTES);
        if let Some(mut checkpoint) = checkpoint {
            checkpoint["version"] = json!(2);
            checkpoint["report"] = metadata.clone();
            assert_eq!(stored.checkpoint, Some(checkpoint));
        } else {
            assert!(stored.checkpoint.is_none());
        }
        let report: ImportReport = serde_json::from_value(metadata.clone()).unwrap();
        assert_eq!(report.error_count(), expected.len() as u64);
        let end = i64::try_from(report.archived_error_chunks()).unwrap();
        let fragments = super::chunks(&pool, org, id, 0, end).await.unwrap();
        if !prefix.is_empty() {
            assert_eq!(
                fragments[0], prefix,
                "existing archive must not be rewritten"
            );
        }
        let bytes = fragments.concat();
        let errors = serde_json::Deserializer::from_slice(&bytes)
            .into_iter::<RowError>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(errors, expected);
        assert!(report.row_errors.is_empty());
        assert_eq!(
            jobs::status(&pool, org, id).await.unwrap().unwrap().report,
            Some(metadata)
        );
    }
    // Validated constraints and already-upgraded archives must survive a second startup.
    crate::db::run_migrations(&pool).await.unwrap();
}

async fn legacy_job_metadata(pool: &sqlx::PgPool, id: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        "SELECT to_jsonb(j) - ARRAY['report', 'checkpoint', 'claim_token', 'lease_until', 'updated_at', 'account_generation'] AS \"metadata!\"
         FROM horae_jobs j WHERE id = $1", id,
    ).fetch_one(pool).await.unwrap()
}
