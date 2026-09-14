use super::super::api_source::http::{
    ApiHttp,
    test_server::{Response, Server},
};
use super::*;
use crate::jobs;

fn config() -> HarvestConfig {
    HarvestConfig {
        client_id: "test".into(),
        client_secret: "test".into(),
        redirect_url: "http://localhost/auth/harvest/callback".into(),
        encryption_key_hex: KEY.into(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FirstEntry {
    Valid,
    InvalidUser,
    MissingTimestamp,
}

fn fixture_page(url: &openidconnect::url::Url, first: FirstEntry) -> Response {
    let collection = url.path().rsplit('/').next().unwrap();
    let second = url
        .query_pairs()
        .any(|(key, value)| key == "cursor" && value == "two");
    let items = match collection {
        "clients" => {
            json!([{"id":if second {6} else {1}, "name":if second {"Second client"} else {"Client"}}])
        }
        "projects" => json!([{"id":2,"name":"Project","client":{"id":1}}]),
        "tasks" => {
            json!([{"id":3,"name":"Task","default_hourly_rate":serde_json::from_str::<serde_json::Value>("1.004999999999999999").unwrap()}])
        }
        "users" => json!([{"id":4,"email":"known@example.com"}]),
        "time_entries" => json!([{
            "id":if second {11} else {10}, "spent_date":"2026-01-01", "hours":1,
            "project":{"id":2}, "task":{"id":3},
            "user":{"id":if !second && first == FirstEntry::InvalidUser {99} else {4}},
            "updated_at":if !second && first == FirstEntry::MissingTimestamp {None} else {Some(if second {day(2)} else {day(3)})}
        }]),
        _ => panic!("unexpected collection"),
    };
    let next = if matches!(collection, "clients" | "time_entries") && !second {
        let mut next = url.clone();
        next.query_pairs_mut().clear().append_pair("cursor", "two");
        Some(next.to_string())
    } else {
        None
    };
    Response::json(json!({collection:items,"links":{"next":next}}))
}

async fn resumed_api(pool: PgPool, interrupted_collection: &'static str, first: FirstEntry) {
    let org = setup(&pool).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Incremental,
        },
        "resume-api",
        Default::default(),
    )
    .await
    .unwrap();
    let mut failed = false;
    let server = Server::start(move |url| {
        let collection = url.path().rsplit('/').next().unwrap();
        let second = url
            .query_pairs()
            .any(|(key, value)| key == "cursor" && value == "two");
        if collection == interrupted_collection && second && !failed {
            failed = true;
            return Response {
                status: 500,
                headers: vec![],
                body: vec![],
            };
        }
        fixture_page(url, first)
    });
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let error = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Incremental,
        ApiHttp::local(server.base.clone()),
        Some(&lease),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("HTTP 500"), "{error}");
    let mut connection = pool.acquire().await.unwrap();
    let saved = lease.load_checkpoint(&mut connection).await.unwrap();
    drop(connection);
    assert!(
        saved.is_some(),
        "completed API pages must have a durable checkpoint"
    );
    let before = jobs::status(&pool, org, id).await.unwrap().unwrap();
    if interrupted_collection == "time_entries" {
        assert_eq!(before.processed_count, 5);
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(if first == FirstEntry::InvalidUser {
                0
            } else {
                1
            })
        );
    }
    assert_eq!(watermark(&pool, org).await, json!({}));
    let prior_requests = server.requests.lock().unwrap().len();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (replacement, _replacement_stop) = jobs::claim_lease_for_test(&pool).await;
    let report = run_api_import_with_http(
        &pool,
        org,
        "EUR",
        &config(),
        ImportMode::Commit,
        SyncScope::Incremental,
        ApiHttp::local(server.base.clone()),
        Some(&replacement),
    )
    .await
    .unwrap();
    assert!(report.reconciles());
    assert_eq!(
        report.error_count(),
        usize::from(first == FirstEntry::InvalidUser)
    );
    assert_eq!(report.summary.clients.created, 2);
    assert_eq!(report.summary.projects.created, 1);
    assert_eq!(report.summary.tasks.created, 1);
    let expected = if first == FirstEntry::InvalidUser {
        1
    } else {
        2
    };
    assert_eq!(report.summary.time_entries.created, expected as u64);
    assert_eq!(report.summary.time_entries.skipped, 0);
    let status = jobs::status(&pool, org, id).await.unwrap().unwrap();
    assert_eq!(status.status, "succeeded");
    assert_eq!(status.processed_count, 6);
    assert_eq!(status.total_count, Some(6));
    assert_eq!(status.report, Some(serde_json::to_value(&report).unwrap()));
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(expected)
    );
    let connection = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    assert_eq!(
        connection.watermark_for(EntityType::TimeEntry),
        if first == FirstEntry::Valid {
            Some(day(3) - chrono::Duration::seconds(1))
        } else {
            None
        }
    );
    let clients = sqlx::query!("SELECT name, currency FROM clients WHERE org_id = $1", org)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(
        clients.iter().all(|client| client.currency == "USD"),
        "retry must preserve the original currency fallback"
    );
    let requests = server.requests.lock().unwrap();
    let resumed = &requests[prior_requests..];
    assert!(resumed[0].starts_with(&format!("GET /v2/{interrupted_collection}?cursor=two ")));
    assert!(
        !resumed
            .iter()
            .any(|request| request.starts_with("GET /v2/clients?per_page="))
    );
    if interrupted_collection == "time_entries" {
        assert_eq!(
            resumed.len(),
            1,
            "confirmed catalog and entry pages must not be downloaded again"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_resumes_committed_pages_and_preserves_the_high_watermark(pool: PgPool) {
    resumed_api(pool, "time_entries", FirstEntry::Valid).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_resumes_a_partially_downloaded_catalog(pool: PgPool) {
    resumed_api(pool, "clients", FirstEntry::Valid).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_resume_keeps_earlier_errors_and_does_not_advance_watermark(pool: PgPool) {
    resumed_api(pool, "time_entries", FirstEntry::InvalidUser).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_resume_remembers_an_earlier_missing_timestamp(pool: PgPool) {
    resumed_api(pool, "time_entries", FirstEntry::MissingTimestamp).await;
}

#[derive(Clone)]
struct Committed {
    remaining: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ready: std::sync::Arc<tokio::sync::Notify>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Committed {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        struct Statement(bool);
        impl tracing::field::Visit for Statement {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if matches!(field.name(), "summary" | "db.statement") && value.trim() == "COMMIT" {
                    self.0 = true;
                }
            }
            fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
        }
        let mut statement = Statement(false);
        event.record(&mut statement);
        if event.metadata().target() == "sqlx::query"
            && statement.0
            && self
                .remaining
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                == 1
        {
            self.ready.notify_one();
        }
    }
}

async fn interrupted_api_batch(pool: PgPool, cancel: bool) {
    use std::sync::{Arc, atomic::AtomicUsize};
    use std::time::Duration;
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::prelude::*;

    let org = setup(&pool).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Full,
        },
        "interrupt-api",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    let (release, wait) = std::sync::mpsc::channel();
    let mut blocked = false;
    let server = Server::start(move |url| {
        if url.path().ends_with("/time_entries")
            && url.query_pairs().any(|(key, _)| key == "cursor")
            && !blocked
        {
            blocked = true;
            wait.recv_timeout(Duration::from_secs(15)).unwrap();
        }
        fixture_page(url, FirstEntry::Valid)
    });
    // Five downloaded catalog pages, one parent batch, then the first entry page.
    let committed = Committed {
        remaining: Arc::new(AtomicUsize::new(7)),
        ready: Arc::new(tokio::sync::Notify::new()),
    };
    let subscriber = tracing_subscriber::registry().with(committed.clone());
    let worker_pool = pool.clone();
    let http = ApiHttp::local(server.base.clone());
    let worker = tokio::spawn(async move {
        let work = async {
            let report = run_api_import_with_http(
                &worker_pool,
                org,
                "USD",
                &config(),
                ImportMode::Commit,
                SyncScope::Full,
                http,
                Some(&lease),
            )
            .await?;
            job_report(&report)
        };
        if cancel {
            jobs::run_claimed(&worker_pool, &lease, stop, work)
                .with_subscriber(subscriber)
                .await
        } else {
            // No heartbeat monitor: the database fence must reject the obsolete batch.
            work.with_subscriber(subscriber).await.map(|_| ())
        }
    });
    tokio::time::timeout(Duration::from_secs(10), committed.ready.notified())
        .await
        .unwrap();
    assert_eq!(
        jobs::status(&pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .processed_count,
        5
    );
    let replacement = if cancel {
        assert!(jobs::cancel(&pool, org, id).await.unwrap());
        assert!(!jobs::retry(&pool, org, id).await.unwrap());
        assert_eq!(
            jobs::status(&pool, org, id).await.unwrap().unwrap().status,
            "running"
        );
        None
    } else {
        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        Some(jobs::claim_lease_for_test(&pool).await)
    };
    release.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .unwrap()
        .unwrap();
    if cancel {
        result.unwrap();
        assert_eq!(
            jobs::status(&pool, org, id).await.unwrap().unwrap().status,
            "cancelled"
        );
        assert!(jobs::retry(&pool, org, id).await.unwrap());
    } else {
        assert!(result.unwrap_err().to_string().contains("lease lost"));
    }
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    let (replacement, _replacement_stop) = match replacement {
        Some(value) => value,
        None => jobs::claim_lease_for_test(&pool).await,
    };
    let report = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&replacement),
    )
    .await
    .unwrap();
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!(report.summary.time_entries.skipped, 0);
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_reclaimed_worker_cannot_commit_its_next_page(pool: PgPool) {
    interrupted_api_batch(pool, false).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_cancellation_preserves_confirmed_pages_for_manual_retry(pool: PgPool) {
    interrupted_api_batch(pool, true).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_finalization_retry_does_not_repeat_completed_downloads(pool: PgPool) {
    let org = setup(&pool).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Full,
        },
        "finalize-api",
        Default::default(),
    )
    .await
    .unwrap();
    sqlx::query!("ALTER TABLE harvest_credentials ADD CONSTRAINT reject_sync_marker CHECK (synced_watermark = '{}'::jsonb)")
        .execute(&pool).await.unwrap();
    let server = Server::start(|url| fixture_page(url, FirstEntry::Valid));
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let error = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&lease),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("reject_sync_marker"));
    assert_eq!(
        jobs::status(&pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .processed_count,
        6
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    let before = server.count.load(std::sync::atomic::Ordering::Acquire);
    sqlx::query!("ALTER TABLE harvest_credentials DROP CONSTRAINT reject_sync_marker")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (replacement, _replacement_stop) = jobs::claim_lease_for_test(&pool).await;
    let report = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&replacement),
    )
    .await
    .unwrap();
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!(report.summary.time_entries.skipped, 0);
    assert_eq!(
        server.count.load(std::sync::atomic::Ordering::Acquire),
        before
    );
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_resumes_after_a_confirmed_parent_batch(pool: PgPool) {
    let org = setup(&pool).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Full,
        },
        "parent-batch-api",
        Default::default(),
    )
    .await
    .unwrap();
    sqlx::query!(
        "ALTER TABLE horae_jobs ADD CONSTRAINT reject_second_batch CHECK (processed_count <= 500)"
    )
    .execute(&pool)
    .await
    .unwrap();
    let server = Server::start(|url| {
        let mut response = fixture_page(url, FirstEntry::Valid);
        if url.path().ends_with("/clients") && !url.query_pairs().any(|(key, _)| key == "cursor") {
            let mut body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            body["clients"]
                .as_array_mut()
                .unwrap()
                .extend((100..600).map(|id| json!({"id":id,"name":format!("Client {id}")})));
            response = Response::json(body);
        }
        response
    });
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let error = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&lease),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("reject_second_batch"));
    assert_eq!(
        jobs::status(&pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .processed_count,
        500
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(500)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    sqlx::query!("ALTER TABLE horae_jobs DROP CONSTRAINT reject_second_batch")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    let requests_before = server.requests.lock().unwrap().len();
    let (replacement, _replacement_stop) = jobs::claim_lease_for_test(&pool).await;
    let report = run_api_import_with_http(
        &pool,
        org,
        "USD",
        &config(),
        ImportMode::Commit,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&replacement),
    )
    .await
    .unwrap();
    assert_eq!(report.error_count(), 0);
    assert_eq!(report.summary.clients.created, 502);
    assert_eq!(report.summary.clients.skipped, 0);
    assert_eq!(report.summary.projects.created, 1);
    assert_eq!(report.summary.tasks.created, 1);
    assert_eq!(report.summary.time_entries.created, 2);
    let task = sqlx::query!("SELECT default_rate_cents, billable_default, active FROM tasks WHERE org_id = $1 AND name = 'Task'", org)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        task.default_rate_cents,
        Some(100),
        "checkpoint JSON must preserve the exact source decimal"
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(502)
    );
    let requests = server.requests.lock().unwrap();
    assert!(
        requests[requests_before..]
            .iter()
            .all(|request| request.starts_with("GET /v2/time_entries?"))
    );
}
