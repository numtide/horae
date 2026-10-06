use futures_util::StreamExt;
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, ImportReport, RowError, RowOutcome, SourceKind,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::JobPolicy, jobs};

struct ArchivedJob {
    org: Uuid,
    actor: Uuid,
    id: Uuid,
    lease: jobs::JobLease,
    _stop: tokio::sync::watch::Sender<bool>,
    report: ImportReport,
    expected: Vec<RowError>,
}

impl ArchivedJob {
    fn body(&self, pool: &PgPool) -> axum::body::Body {
        let mut tail = Vec::new();
        for error in &self.report.row_errors {
            serde_json::to_writer(&mut tail, error).unwrap();
            tail.push(b'\n');
        }
        super::download_body(
            pool.clone(),
            self.org,
            self.actor,
            self.id,
            self.report.archived_error_chunks().try_into().unwrap(),
            tail,
        )
    }
}

async fn archived_job(pool: &PgPool) -> ArchivedJob {
    let org = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org
    )
    .execute(pool)
    .await
    .unwrap();
    let actor = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $2, $3, 'Report reader', 'admin')",
        actor, org, format!("{actor}@example.test"),
    ).execute(pool).await.unwrap();
    let id = jobs::enqueue(
        pool,
        org,
        &jobs::JobPayload::HarvestCsv {
            mode: ImportMode::Commit,
        },
        "stream-snapshot",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(pool).await;
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 1".into(),
            reason: "x".repeat(super::CHUNK_BYTES * 33),
        },
    );
    let mut expected = report.row_errors.clone();
    let mut tx = pool.begin().await.unwrap();
    lease.archive_report(&mut tx, &mut report).await.unwrap();
    assert!(report.archived_error_chunks() > 32);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 2".into(),
            reason: "inline at the captured checkpoint".into(),
        },
    );
    expected.extend(report.row_errors.clone());
    let metadata = serde_json::to_value(&report).unwrap();
    lease
        .save_checkpoint(
            &mut tx,
            &serde_json::json!({ "version": 2, "report": metadata }),
            &metadata,
            "time_entries",
            2,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ArchivedJob {
        org,
        actor,
        id,
        lease,
        _stop: stop,
        report,
        expected,
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn download_preparation_denies_revoked_authority(pool: PgPool) {
    let job = archived_job(&pool).await;
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        job.actor
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = super::download_response(&pool, job.org, job.actor, job.id).await;
    assert!(
        matches!(result, Err(axum::http::StatusCode::FORBIDDEN)),
        "{result:?}"
    );
}

#[sqlx::test]
#[serial_test::serial]
async fn download_denies_revocation_before_first_page_and_inline_tail(pool: PgPool) {
    let job = archived_job(&pool).await;
    for (end, tail) in [
        (0, Vec::new()),
        (0, b"private tail\n".to_vec()),
        (
            job.report.archived_error_chunks().try_into().unwrap(),
            Vec::new(),
        ),
    ] {
        sqlx::query!(
            "UPDATE users SET org_role = 'admin' WHERE id = $1",
            job.actor
        )
        .execute(&pool)
        .await
        .unwrap();
        let body = super::download_body(pool.clone(), job.org, job.actor, job.id, end, tail);
        sqlx::query!(
            "UPDATE users SET org_role = 'member' WHERE id = $1",
            job.actor
        )
        .execute(&pool)
        .await
        .unwrap();
        let result = body
            .into_data_stream()
            .next()
            .await
            .expect("denial must not become successful EOF");
        assert!(result.is_err(), "revoked reader received report bytes");
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn download_rechecks_after_buffered_pages_and_before_the_captured_tail(pool: PgPool) {
    let job = archived_job(&pool).await;
    let end = job.report.archived_error_chunks() as usize;
    for consumed in [1_usize, 16, end] {
        sqlx::query!("UPDATE users SET active = true WHERE id = $1", job.actor)
            .execute(&pool)
            .await
            .unwrap();
        let mut stream = job.body(&pool).into_data_stream();
        for _ in 0..consumed {
            stream.next().await.unwrap().unwrap();
        }
        sqlx::query!("UPDATE users SET active = false WHERE id = $1", job.actor)
            .execute(&pool)
            .await
            .unwrap();
        // Only the bounded page already authorized may finish draining.
        for _ in consumed..(consumed.div_ceil(16) * 16).min(end) {
            stream.next().await.unwrap().unwrap();
        }
        let error = stream
            .next()
            .await
            .expect("revocation is not successful EOF")
            .unwrap_err();
        assert!(error.to_string().contains("403"), "{error}");
        assert!(stream.next().await.is_none());
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn download_preparation_is_tenant_bound_and_releases_failed_transactions(pool: PgPool) {
    use axum::http::StatusCode;
    use horae_core::types::OrgRole;
    let job = archived_job(&pool).await;
    let foreign = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    for (org, actor, id, expected) in [
        (job.org, foreign.user_id, job.id, StatusCode::FORBIDDEN),
        (job.org, Uuid::now_v7(), job.id, StatusCode::FORBIDDEN),
        (Uuid::now_v7(), job.actor, job.id, StatusCode::FORBIDDEN),
        (
            foreign.org_id,
            foreign.user_id,
            job.id,
            StatusCode::NOT_FOUND,
        ),
        (job.org, job.actor, Uuid::now_v7(), StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            super::download_response(&pool, org, actor, id)
                .await
                .unwrap_err(),
            expected
        );
    }
    for (role, active) in [
        (OrgRole::Member, true),
        (OrgRole::Manager, true),
        (OrgRole::Admin, false),
    ] {
        sqlx::query!(
            "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
            job.actor,
            role as OrgRole,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            super::download_response(&pool, job.org, job.actor, job.id)
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", job.actor)
        .execute(&pool)
        .await
        .unwrap();
    let single = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        // Status prefers the checkpoint; remove it to exercise the malformed final report.
        sqlx::query!(
            "UPDATE horae_jobs SET checkpoint = NULL, report = '{}' WHERE id = $1",
            job.id
        )
        .execute(&single)
        .await
        .unwrap();
        assert_eq!(
            super::download_response(&single, job.org, job.actor, job.id)
                .await
                .unwrap_err(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        sqlx::query!(
            "UPDATE horae_jobs SET report = $2 WHERE id = $1",
            job.id,
            serde_json::to_value(&job.report).unwrap()
        )
        .execute(&single)
        .await
        .unwrap();
        let response = super::download_response(&single, job.org, job.actor, job.id)
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), super::CHUNK_BYTES * 40)
            .await
            .unwrap();
        let errors: Vec<RowError> = serde_json::Deserializer::from_slice(&body)
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(errors, job.expected);
    })
    .await
    .expect("failed and successful preparation must release the single connection");
    single.close().await;
}

#[derive(Clone, Copy, Debug)]
enum Boundary {
    Preparation,
    Page,
    Tail,
    Empty,
}

async fn read_boundary(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    job: Uuid,
    boundary: Boundary,
) -> Result<(), String> {
    if matches!(boundary, Boundary::Preparation) {
        return super::download_response(pool, org, actor, job)
            .await
            .map(|_| ())
            .map_err(|status| status.to_string());
    }
    let end = i64::from(matches!(boundary, Boundary::Page));
    let tail = if matches!(boundary, Boundary::Tail) {
        b"private tail\n".to_vec()
    } else {
        Vec::new()
    };
    let mut stream =
        super::download_body(pool.clone(), org, actor, job, end, tail).into_data_stream();
    stream
        .next()
        .await
        .transpose()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[sqlx::test]
#[serial_test::serial]
async fn download_boundaries_recheck_after_organization_and_actor_waits(pool: PgPool) {
    use crate::server_fns::test_seed::wait_for_blocked;
    use horae_core::types::OrgRole;
    let reading = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!(
                    "SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL REPEATABLE READ"
                )
                .execute(connection)
                .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let job = archived_job(&pool).await;
    for boundary in [
        Boundary::Preparation,
        Boundary::Page,
        Boundary::Tail,
        Boundary::Empty,
    ] {
        for actor_only in [false, true] {
            for deactivate in [false, true] {
                sqlx::query!(
                    "UPDATE users SET org_role = 'admin', active = true WHERE id = $1",
                    job.actor
                )
                .execute(&pool)
                .await
                .unwrap();
                let mut revoke = pool.begin().await.unwrap();
                let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                    .fetch_one(&mut *revoke)
                    .await
                    .unwrap()
                    .unwrap();
                if !actor_only {
                    sqlx::query!(
                        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                        job.org
                    )
                    .fetch_one(&mut *revoke)
                    .await
                    .unwrap();
                }
                sqlx::query!(
                    "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                    job.actor,
                    if deactivate {
                        OrgRole::Admin
                    } else {
                        OrgRole::Member
                    } as OrgRole,
                    !deactivate
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
                let db = reading.clone();
                let mut pending = tokio::task::JoinSet::new();
                pending.spawn(async move {
                    read_boundary(&db, job.org, job.actor, job.id, boundary).await
                });
                tokio::select! {
                    result = pending.join_next() => panic!("{boundary:?} bypassed authorization wait: {result:?}"),
                    () = wait_for_blocked(&pool, blocker) => {},
                }
                revoke.commit().await.unwrap();
                let error =
                    tokio::time::timeout(std::time::Duration::from_secs(5), pending.join_next())
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap()
                        .unwrap_err();
                assert!(error.contains("403"), "{boundary:?}: {error}");
            }
        }
    }
    reading.close().await;
}

#[sqlx::test]
#[serial_test::serial]
async fn download_readers_retain_authority_through_their_bounded_reads(pool: PgPool) {
    use crate::server_fns::test_seed::wait_for_blocked;
    let job = archived_job(&pool).await;
    for boundary in [Boundary::Preparation, Boundary::Page] {
        sqlx::query!(
            "UPDATE users SET org_role = 'admin' WHERE id = $1",
            job.actor
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *barrier)
            .await
            .unwrap()
            .unwrap();
        match boundary {
            Boundary::Preparation => sqlx::query!("LOCK TABLE horae_jobs IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *barrier)
                .await
                .unwrap(),
            _ => sqlx::query!("LOCK TABLE horae_job_report_error_chunks IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *barrier)
                .await
                .unwrap(),
        };
        let db = pool.clone();
        let mut read = tokio::task::JoinSet::new();
        read.spawn(async move { read_boundary(&db, job.org, job.actor, job.id, boundary).await });
        wait_for_blocked(&pool, blocker).await;
        let reader = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker,
        ).fetch_one(&pool).await.unwrap();
        let db = pool.clone();
        let mut revoke = tokio::task::JoinSet::new();
        revoke.spawn(async move {
            sqlx::query!(
                "UPDATE users SET org_role = 'member' WHERE id = $1",
                job.actor
            )
            .execute(&db)
            .await
        });
        tokio::select! {
            result = revoke.join_next() => panic!("{boundary:?} released authority early: {result:?}"),
            () = wait_for_blocked(&pool, reader) => {},
        }
        barrier.commit().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), read.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), revoke.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        let error = read_boundary(&pool, job.org, job.actor, job.id, boundary)
            .await
            .unwrap_err();
        assert!(error.contains("403"), "{boundary:?}: {error}");
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn download_keeps_its_snapshot_when_later_progress_archives_the_inline_tail(pool: PgPool) {
    let mut job = archived_job(&pool).await;
    let captured_end = job.report.archived_error_chunks();
    let mut stream = job.body(&pool).into_data_stream();
    let mut bytes = stream.next().await.unwrap().unwrap().to_vec();

    job.report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 3".into(),
            reason: "later error ".repeat(1_000),
        },
    );
    let mut tx = pool.begin().await.unwrap();
    job.lease
        .archive_report(&mut tx, &mut job.report)
        .await
        .unwrap();
    assert!(job.report.row_errors.is_empty());
    assert!(job.report.archived_error_chunks() > captured_end);
    assert!(
        job.lease
            .complete(&mut tx, &serde_json::to_value(&job.report).unwrap(), 3)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();

    while let Some(part) = stream.next().await {
        bytes.extend(part.unwrap());
    }
    let errors: Vec<RowError> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(errors, job.expected);
}

#[sqlx::test]
#[serial_test::serial]
async fn download_reports_missing_fragments_after_its_buffered_page(pool: PgPool) {
    let job = archived_job(&pool).await;
    let mut stream = job.body(&pool).into_data_stream();
    assert_eq!(
        stream.next().await.unwrap().unwrap().len(),
        super::CHUNK_BYTES
    );

    let mut tx = pool.begin().await.unwrap();
    assert!(
        job.lease
            .complete(&mut tx, &serde_json::to_value(&job.report).unwrap(), 2)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
        job.id
    )
    .execute(&pool)
    .await
    .unwrap();
    jobs::cleanup(&pool).await.unwrap();

    for _ in 1..16 {
        assert_eq!(
            stream.next().await.unwrap().unwrap().len(),
            super::CHUNK_BYTES
        );
    }
    let error = stream
        .next()
        .await
        .expect("missing data must not become a successful EOF")
        .unwrap_err();
    assert!(error.to_string().contains("report archive is incomplete"));
}

#[sqlx::test]
#[serial_test::serial]
async fn download_reads_only_when_consumed_and_releases_its_connection(pool: PgPool) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    let acquisitions = Arc::new(AtomicUsize::new(0));
    let observed = acquisitions.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .before_acquire(move |_, _| {
            let observed = observed.clone();
            Box::pin(async move {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(true)
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let job = archived_job(&pool).await;
    acquisitions.store(0, Ordering::SeqCst);
    let mut stream = job.body(&pool).into_data_stream();
    assert_eq!(acquisitions.load(Ordering::SeqCst), 0);
    for _ in 0..16 {
        assert_eq!(
            stream.next().await.unwrap().unwrap().len(),
            super::CHUNK_BYTES
        );
        assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    }
    assert_eq!(
        stream.next().await.unwrap().unwrap().len(),
        super::CHUNK_BYTES
    );
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
    drop(stream);
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .unwrap();
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
}
