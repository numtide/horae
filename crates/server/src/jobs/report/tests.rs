use horae_core::importers::harvest::types::{
    EntityType, ID_NOTICES_KEPT, IdNotice, ImportMode, ImportReport, PersonOutcome, ReportPerson,
    RowOutcome, SourceKind,
};
use uuid::Uuid;

use crate::{config::JobPolicy, jobs};

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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
            person: None,
            entry_kind: None,
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
            None,
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

/// A clean import of an account whose Harvest ids collide often has nothing to
/// archive; its checkpoint must still save, with the notices bounded.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_report_with_notices_and_people_but_no_errors_checkpoints(pool: sqlx::PgPool) {
    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    jobs::enqueue(
        &pool,
        org_id,
        &jobs::JobPayload::HarvestCsv {
            mode: ImportMode::Commit,
        },
        "notices",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit);
    for n in 0..12 {
        report.record(EntityType::Person, &RowOutcome::Created);
        report.people.push(ReportPerson {
            harvest_id: Some(5_000_000 + n),
            horae_id: Uuid::now_v7(),
            email: Some(format!("person{n}@example.com")),
            name: format!("Person Number {n}"),
            outcome: PersonOutcome::Created,
            harvest_active: Some(true),
        });
    }
    for n in 0..150 {
        report.record(EntityType::TimeEntry, &RowOutcome::Created);
        report.note_id(IdNotice {
            entity: EntityType::TimeEntry,
            harvest_id: 1_000_000_000 + n,
            api_id: Some(2_000_000 + n),
        });
    }

    let mut tx = pool.begin().await.unwrap();
    lease.archive_report(&mut tx, &mut report).await.unwrap();
    let metadata = serde_json::to_value(&report).unwrap();
    lease
        .save_checkpoint(
            &mut tx,
            &serde_json::json!({ "version": 2, "report": metadata }),
            &metadata,
            "time_entries",
            162,
            None,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(report.id_notices.len(), ID_NOTICES_KEPT);
    assert_eq!(report.omitted_id_notices, 150 - ID_NOTICES_KEPT as u64);
    assert_eq!(report.archived_error_chunks(), 0);
}
