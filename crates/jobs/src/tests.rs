use std::time::Duration;

use uuid::Uuid;

use super::outbox::*;
use super::*;

async fn org(pool: &sqlx::PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// A queued job of `kind` whose payload is `value` in the envelope.
async fn insert_job(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    kind: &str,
    value: serde_json::Value,
    max_attempts: i32,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts)
         VALUES ($1, $2, $3, $4, $5, $6)",
        id,
        org_id,
        kind,
        encode_payload(&value).unwrap(),
        id.to_string(),
        max_attempts,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 5 attempts, 10 s, ×2, at most 10 minutes.
const ECHO_POLICY: RetryPolicy = RetryPolicy {
    max_attempts: 5,
    first_delay: Duration::from_secs(10),
    multiplier: 2,
    max_delay: Duration::from_secs(600),
};

/// Echoes its payload; `"fail"` fails permanently and `"flaky"` transiently.
struct Echo;

impl Handler for Echo {
    fn kind(&self) -> &'static str {
        "echo"
    }

    fn policy(&self) -> RetryPolicy {
        ECHO_POLICY
    }

    fn run<'a>(
        &'a self,
        _ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>> {
        Box::pin(async move {
            if payload == "fail" {
                return Err(JobError {
                    error: anyhow::anyhow!("refused"),
                    permanent: true,
                    error_kind: Some("forbidden"),
                });
            }
            if payload == "flaky" {
                return Err(anyhow::anyhow!("try again").into());
            }
            Ok(Completed {
                report: serde_json::json!({ "echo": payload }),
                processed_count: 3,
            })
        })
    }
}

/// Runs until cancelled, with the shortest stale limit.
struct Long;

impl Handler for Long {
    fn kind(&self) -> &'static str {
        "long"
    }

    fn policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_attempts: 1,
            ..RetryPolicy::default()
        }
    }

    fn stale_after(&self) -> Duration {
        Duration::from_secs(15)
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>> {
        Box::pin(async move {
            let stopped = ctx.run(std::future::pending::<anyhow::Result<()>>()).await;
            Err(stopped.unwrap_err().into())
        })
    }
}

/// A kind only an upgraded worker knows.
struct Newer;

impl Handler for Newer {
    fn kind(&self) -> &'static str {
        "newer_kind"
    }

    fn policy(&self) -> RetryPolicy {
        RetryPolicy::default()
    }

    fn run<'a>(
        &'a self,
        _ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>> {
        Box::pin(async {
            Ok(Completed {
                report: serde_json::json!({}),
                processed_count: 0,
            })
        })
    }
}

fn registry() -> Registry {
    let mut registry = Registry::default();
    registry.register(Echo).register(Long);
    registry
}

/// Queues `request` in a transaction of its own.
async fn enqueue(pool: &sqlx::PgPool, request: Enqueue<'_>) -> Result<Enqueued, EnqueueError> {
    let mut tx = pool.begin().await.unwrap();
    let enqueued = enqueue_in(&mut tx, request).await?;
    tx.commit().await.unwrap();
    Ok(enqueued)
}

fn request<'a>(org_id: Uuid, kind: &'a str) -> Enqueue<'a> {
    Enqueue {
        org_id,
        kind,
        payload: serde_json::json!("hello"),
        idempotency_key: None,
        concurrency_key: None,
        run_at: None,
        priority: 0,
        max_attempts: ECHO_POLICY.max_attempts,
    }
}

/// Seconds from now until the job may be claimed.
async fn wait_secs(pool: &sqlx::PgPool, id: Uuid) -> f64 {
    sqlx::query_scalar!(
        r#"SELECT EXTRACT(EPOCH FROM available_at - now())::float8 AS "secs!" FROM horae_jobs WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

struct Attempt {
    attempt: i32,
    outcome: Option<String>,
    error: Option<String>,
    finished: bool,
}

async fn attempts(pool: &sqlx::PgPool, id: Uuid) -> Vec<Attempt> {
    sqlx::query_as!(
        Attempt,
        r#"SELECT attempt, outcome, error, finished_at IS NOT NULL AS "finished!"
             FROM horae_job_attempts WHERE job_id = $1 ORDER BY attempt"#,
        id
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// Claims and runs the next job as the worker would.
async fn run_next(pool: &sqlx::PgPool, worker: &str) -> Uuid {
    let registry = registry();
    let job = claim_registered(pool, &registry, worker)
        .await
        .unwrap()
        .unwrap();
    let id = job.id;
    execute(pool, &registry, worker, job).await.unwrap();
    id
}

async fn make_available(pool: &sqlx::PgPool, id: Uuid) {
    sqlx::query!(
        "UPDATE horae_jobs SET available_at = now() WHERE id = $1",
        id
    )
    .execute(pool)
    .await
    .unwrap();
}

struct Row {
    status: String,
    report: Option<serde_json::Value>,
    last_error: Option<String>,
    error_kind: Option<String>,
    processed_count: i64,
}

async fn row(pool: &sqlx::PgPool, id: Uuid) -> Row {
    sqlx::query_as!(
        Row,
        "SELECT status, report, last_error, error_kind, processed_count FROM horae_jobs WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn registered_handler_runs_the_claimed_job_and_stores_its_result(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = insert_job(&pool, org_id, "echo", serde_json::json!("hello"), 5).await;
    let job = claim(&pool, "worker").await.unwrap().unwrap();
    execute(&pool, &registry(), "worker", job).await.unwrap();
    let done = row(&pool, id).await;
    assert_eq!(done.status, "succeeded");
    assert_eq!(done.report, Some(serde_json::json!({"echo": "hello"})));
    assert_eq!(done.processed_count, 3);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn permanent_handler_error_fails_the_job_with_its_kind(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = insert_job(&pool, org_id, "echo", serde_json::json!("fail"), 5).await;
    let job = claim(&pool, "worker").await.unwrap().unwrap();
    execute(&pool, &registry(), "worker", job).await.unwrap();
    let failed = row(&pool, id).await;
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.last_error.as_deref(), Some("refused"));
    assert_eq!(failed.error_kind.as_deref(), Some("forbidden"));
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_kind_without_a_handler_stays_queued_until_a_worker_has_one(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let newer = insert_job(&pool, org_id, "newer_kind", serde_json::json!({}), 5).await;
    let behind = enqueue(&pool, request(org_id, "echo")).await.unwrap().id();
    assert_eq!(run_next(&pool, "old-worker").await, behind);
    assert!(
        claim_registered(&pool, &registry(), "old-worker")
            .await
            .unwrap()
            .is_none()
    );
    let waiting = sqlx::query!(
        "SELECT status, attempts FROM horae_jobs WHERE id = $1",
        newer
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((waiting.status.as_str(), waiting.attempts), ("queued", 0));

    let mut upgraded = Registry::default();
    upgraded.register(Newer);
    let job = claim_registered(&pool, &upgraded, "new-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.id, newer);
    execute(&pool, &upgraded, "new-worker", job).await.unwrap();
    assert_eq!(row(&pool, newer).await.status, "succeeded");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn an_exhausted_job_of_a_kind_without_a_handler_is_not_failed(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = insert_job(&pool, org_id, "newer_kind", serde_json::json!({}), 1).await;
    // A newer server may know the kind and fail or finish it itself.
    sqlx::query!("UPDATE horae_jobs SET attempts = 1 WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();
    claim_registered(&pool, &registry(), "worker")
        .await
        .unwrap();
    assert_eq!(row(&pool, id).await.status, "queued");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_concurrency_key_admits_one_queued_or_running_job(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let keyed = || Enqueue {
        concurrency_key: Some("sweep"),
        ..request(org_id, "echo")
    };
    let first = enqueue(&pool, keyed()).await.unwrap();
    let Enqueued::Created(id) = first else {
        panic!("first enqueue created nothing: {first:?}");
    };
    assert_eq!(
        enqueue(&pool, keyed()).await.unwrap(),
        Enqueued::Existing(id)
    );

    // Two workers, one key: one job, so one of them runs it. Either may win
    // the claim, and only the winner may execute it.
    let registry = registry();
    let (left, right) = tokio::join!(
        claim_registered(&pool, &registry, "worker-a"),
        claim_registered(&pool, &registry, "worker-b"),
    );
    let claimed: Vec<_> = [("worker-a", left.unwrap()), ("worker-b", right.unwrap())]
        .into_iter()
        .filter_map(|(worker, job)| Some((worker, job?)))
        .collect();
    assert_eq!(claimed.len(), 1);
    assert_eq!(
        enqueue(&pool, keyed()).await.unwrap(),
        Enqueued::Existing(id)
    );

    let (worker, job) = claimed.into_iter().next().unwrap();
    execute(&pool, &registry, worker, job).await.unwrap();
    let outcomes: Vec<_> = attempts(&pool, id)
        .await
        .into_iter()
        .map(|attempt| (attempt.attempt, attempt.outcome, attempt.error))
        .collect();
    assert_eq!(
        row(&pool, id).await.status,
        "succeeded",
        "{worker} ran the job; attempts: {outcomes:?}"
    );
    let next = enqueue(&pool, keyed()).await.unwrap();
    assert!(
        matches!(next, Enqueued::Created(created) if created != id),
        "{next:?}"
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn cancelling_a_running_job_releases_its_concurrency_key(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let keyed = || Enqueue {
        concurrency_key: Some("pass"),
        ..request(org_id, "echo")
    };
    let cancelling = enqueue(&pool, keyed()).await.unwrap().id();
    let job = claim_registered(&pool, &registry(), "worker")
        .await
        .unwrap()
        .unwrap();
    assert!(cancel(&pool, org_id, cancelling).await.unwrap());
    assert_eq!(row(&pool, cancelling).await.status, "running");

    let next = enqueue(&pool, keyed()).await.unwrap();
    assert!(
        matches!(next, Enqueued::Created(id) if id != cancelling),
        "{next:?}"
    );
    // The cancelling attempt can no longer write, so it ends as cancelled.
    execute(&pool, &registry(), "worker", job).await.unwrap();
    assert_eq!(row(&pool, cancelling).await.status, "cancelled");
    assert_eq!(row(&pool, next.id()).await.status, "queued");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn the_concurrency_key_index_refuses_a_second_holder(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let insert = || {
        sqlx::query!(
            "INSERT INTO horae_jobs (id, org_id, kind, payload, concurrency_key)
             VALUES ($1, $2, 'echo', '{}', 'pass')",
            Uuid::now_v7(),
            org_id,
        )
        .execute(&pool)
    };
    insert().await.unwrap();
    let refused = insert().await.unwrap_err();
    assert!(is_concurrency_key_conflict(&refused), "{refused}");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn cleanup_closes_an_attempt_its_finished_job_outlived(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(&pool, request(org_id, "echo")).await.unwrap().id();
    claim_registered(&pool, &registry(), "new-worker")
        .await
        .unwrap()
        .unwrap();
    // An older binary finishes the job without touching its attempts.
    sqlx::query!(
        "UPDATE horae_jobs SET status = 'succeeded', finished_at = now() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    cleanup(&pool).await.unwrap();
    let recorded = attempts(&pool, id).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].outcome.as_deref(), Some("interrupted"));
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_concurrency_key_belongs_to_one_organization_and_kind(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let other_org = org(&pool).await;
    for (org_id, kind) in [(org_id, "echo"), (org_id, "long"), (other_org, "echo")] {
        let enqueued = enqueue(
            &pool,
            Enqueue {
                concurrency_key: Some("sweep"),
                ..request(org_id, kind)
            },
        )
        .await
        .unwrap();
        assert!(
            matches!(enqueued, Enqueued::Created(_)),
            "{kind}: {enqueued:?}"
        );
    }
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_concurrent_enqueue_waits_and_returns_the_job_holding_the_key(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut first = pool.begin().await.unwrap();
    let created = enqueue_in(
        &mut first,
        Enqueue {
            concurrency_key: Some("pass"),
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap();
    let second = tokio::spawn({
        let pool = pool.clone();
        async move {
            enqueue(
                &pool,
                Enqueue {
                    concurrency_key: Some("pass"),
                    ..request(org_id, "echo")
                },
            )
            .await
            .unwrap()
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !second.is_finished(),
        "the second insert waits for the first"
    );
    first.commit().await.unwrap();
    assert_eq!(second.await.unwrap(), Enqueued::Existing(created.id()));
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn an_idempotency_key_returns_its_job_or_conflicts_on_another_payload(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let keyed = |payload| Enqueue {
        idempotency_key: Some("request-1"),
        payload,
        ..request(org_id, "echo")
    };
    let created = enqueue(&pool, keyed(serde_json::json!("a"))).await.unwrap();
    assert_eq!(
        enqueue(&pool, keyed(serde_json::json!("a"))).await.unwrap(),
        Enqueued::Existing(created.id())
    );
    assert!(matches!(
        enqueue(&pool, keyed(serde_json::json!("b"))).await,
        Err(EnqueueError::Conflict)
    ));
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_delayed_job_waits_for_its_time(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(
        &pool,
        Enqueue {
            run_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap()
    .id();
    assert!(
        claim_registered(&pool, &registry(), "worker")
            .await
            .unwrap()
            .is_none()
    );
    make_available(&pool, id).await;
    assert_eq!(run_next(&pool, "worker").await, id);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_higher_priority_is_claimed_first(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let low = enqueue(
        &pool,
        Enqueue {
            priority: -1,
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap()
    .id();
    let normal = enqueue(&pool, request(org_id, "echo")).await.unwrap().id();
    let high = enqueue(
        &pool,
        Enqueue {
            priority: 10,
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap()
    .id();
    let mut order = Vec::new();
    for _ in 0..3 {
        order.push(run_next(&pool, "worker").await);
    }
    assert_eq!(order, [high, normal, low]);
}

#[test]
fn retry_delays_grow_by_the_multiplier_up_to_the_maximum() {
    let delays: Vec<_> = (1..=8)
        .map(|attempt| ECHO_POLICY.delay(attempt).as_secs())
        .collect();
    assert_eq!(delays, [10, 20, 40, 80, 160, 320, 600, 600]);
    assert_eq!(ECHO_POLICY.delay(i32::MAX), Duration::from_secs(600));
}

#[test]
fn the_default_policy_keeps_the_original_curve() {
    let delays: Vec<_> = (1..=10)
        .map(|attempt| RetryPolicy::default().delay(attempt).as_secs())
        .collect();
    assert_eq!(delays, [2, 4, 8, 16, 32, 64, 128, 256, 300, 300]);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn failed_attempts_back_off_by_policy_and_stop_at_the_limit(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(
        &pool,
        Enqueue {
            payload: serde_json::json!("flaky"),
            max_attempts: 3,
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap()
    .id();
    for expected in [10.0, 20.0] {
        run_next(&pool, "worker").await;
        assert_eq!(row(&pool, id).await.status, "queued");
        let wait = wait_secs(&pool, id).await;
        assert!(
            (wait - expected).abs() < 2.0,
            "waits {wait}s, not {expected}s"
        );
        make_available(&pool, id).await;
    }
    run_next(&pool, "worker").await;
    let failed = row(&pool, id).await;
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.last_error.as_deref(), Some("try again"));
    assert!(
        claim_registered(&pool, &registry(), "worker")
            .await
            .unwrap()
            .is_none()
    );

    let recorded = attempts(&pool, id).await;
    let outcomes: Vec<_> = recorded.iter().map(|a| a.outcome.as_deref()).collect();
    assert_eq!(
        outcomes,
        [Some("retrying"), Some("retrying"), Some("failed")]
    );
    assert!(
        recorded
            .iter()
            .all(|a| a.finished && a.error.as_deref() == Some("try again"))
    );
    assert_eq!(
        recorded.iter().map(|a| a.attempt).collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_permanent_error_ends_the_job_at_its_first_attempt(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(
        &pool,
        Enqueue {
            payload: serde_json::json!("fail"),
            ..request(org_id, "echo")
        },
    )
    .await
    .unwrap()
    .id();
    run_next(&pool, "worker").await;
    assert_eq!(row(&pool, id).await.status, "failed");
    let recorded = attempts(&pool, id).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].outcome.as_deref(), Some("failed"));
    assert_eq!(recorded[0].error.as_deref(), Some("refused"));
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_succeeded_attempt_is_recorded(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(&pool, request(org_id, "echo")).await.unwrap().id();
    run_next(&pool, "worker").await;
    let recorded = attempts(&pool, id).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].outcome.as_deref(), Some("succeeded"));
    assert!(recorded[0].finished);
    assert_eq!(recorded[0].error, None);
}

#[test]
fn attempt_errors_are_cut_on_a_character_boundary() {
    let text = "é".repeat(3000);
    let cut = worker::truncate(&text, 4096);
    assert_eq!(cut.len(), 4096);
    assert_eq!(worker::truncate(&text[..6], 5), "éé");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_job_whose_heartbeat_stopped_is_claimable_after_its_stale_limit(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(&pool, request(org_id, "echo")).await.unwrap().id();
    let registry = registry();
    let first = claim_registered(&pool, &registry, "dead-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.stale_after, Duration::from_secs(60));

    // The last heartbeat 30 s ago: within the stale limit.
    let beat = |secs: i32| {
        sqlx::query!(
            "UPDATE horae_jobs
                SET heartbeat_at = now() - $2::int * interval '1 second',
                    lease_until = now() - $2::int * interval '1 second' + stale_after_secs * interval '1 second'
              WHERE id = $1",
            id,
            secs,
        )
        .execute(&pool)
    };
    beat(30).await.unwrap();
    assert!(
        claim_registered(&pool, &registry, "worker")
            .await
            .unwrap()
            .is_none()
    );
    beat(61).await.unwrap();
    let second = claim_registered(&pool, &registry, "worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.id, id);
    assert_ne!(second.claim_token, first.claim_token);
    execute(&pool, &registry, "worker", second).await.unwrap();

    let recorded = attempts(&pool, id).await;
    let outcomes: Vec<_> = recorded.iter().map(|a| a.outcome.as_deref()).collect();
    assert_eq!(outcomes, [Some("interrupted"), Some("succeeded")]);
    assert!(!recorded[0].finished, "nobody saw the dead attempt end");
}

/// Runs for longer than its stale limit, in real time.
#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_job_whose_heartbeat_continues_is_never_claimed_again(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = enqueue(&pool, request(org_id, "long")).await.unwrap().id();
    let registry = std::sync::Arc::new(registry());
    let job = claim_registered(&pool, &registry, "busy-worker")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.stale_after, Duration::from_secs(15));
    let running = tokio::spawn({
        let (pool, registry) = (pool.clone(), registry.clone());
        async move { execute(&pool, &registry, "busy-worker", job).await }
    });
    let started = tokio::time::Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        assert!(
            claim_registered(&pool, &registry, "idle-worker")
                .await
                .unwrap()
                .is_none()
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let beat = sqlx::query_scalar!(
        r#"SELECT heartbeat_at > started_at + interval '15 seconds' AS "later!"
             FROM horae_jobs WHERE id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(beat, "the heartbeat kept moving past the stale limit");

    assert!(cancel(&pool, org_id, id).await.unwrap());
    running.await.unwrap().unwrap();
    assert_eq!(row(&pool, id).await.status, "cancelled");
    let recorded = attempts(&pool, id).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].outcome.as_deref(), Some("cancelled"));
}

#[test]
fn payload_versions_reject_unversioned_and_unknown_versions() {
    let versioned = encode_payload(&"value").unwrap();
    assert_eq!(versioned["version"], 1);
    assert_eq!(decode_payload::<String>(versioned).unwrap(), "value");
    assert!(decode_payload::<String>(serde_json::json!("value")).is_err());
    assert!(
        decode_payload::<String>(serde_json::json!({"version": 2, "payload": "value"})).is_err()
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn claims_are_atomic_and_expired_leases_recover(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let first = insert_job(&pool, org_id, "echo", serde_json::json!("first"), 5).await;
    let second = insert_job(&pool, org_id, "echo", serde_json::json!("second"), 5).await;

    let (left, right) = tokio::join!(claim(&pool, "worker-a"), claim(&pool, "worker-b"));
    let left = left.unwrap().unwrap();
    let right = right.unwrap().unwrap();
    assert_ne!(left.id, right.id);
    assert_eq!(
        std::collections::HashSet::from([left.id, right.id]),
        std::collections::HashSet::from([first, second])
    );

    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
        left.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let recovered = claim(&pool, "worker-c").await.unwrap().unwrap();
    assert_eq!(recovered.id, left.id);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
#[serial_test::serial]
async fn expired_lease_cannot_renew_or_commit_using_an_old_transaction_timestamp(
    pool: sqlx::PgPool,
) {
    let org_id = org(&pool).await;
    let id = insert_job(&pool, org_id, "echo", serde_json::json!({}), 5).await;
    let (lease, _stop) = claim_for_test(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    // Fixes the transaction's `now()`; text keeps the crate free of a time library.
    sqlx::query!("SELECT now()::text")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        !lease
            .complete(&mut tx, &serde_json::json!({}), 0)
            .await
            .unwrap()
    );
    assert!(lease.heartbeat(&pool).await.is_err());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn shutdown_waits_for_in_flight_work() {
    let (stop, mut receiver) = tokio::sync::watch::channel(false);
    let (finish, finished) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        receiver.changed().await.unwrap();
        finished.await.unwrap();
    });
    let worker = Worker::new(stop, task);
    let shutdown = worker.shutdown(Duration::from_secs(5));
    tokio::pin!(shutdown);
    assert!(futures_util::poll!(&mut shutdown).is_pending());
    finish.send(()).unwrap();
    shutdown.await.unwrap();
}

#[tokio::test]
async fn shutdown_deadline_drops_and_joins_in_flight_work() {
    let (stop, _receiver) = tokio::sync::watch::channel(false);
    let (started, running) = tokio::sync::oneshot::channel();
    let (completion, dropped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let _completion = completion;
        started.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    running.await.unwrap();
    Worker::new(stop, task)
        .shutdown(Duration::ZERO)
        .await
        .unwrap();
    assert!(
        dropped.await.is_err(),
        "shutdown returned before dropping the task"
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
#[serial_test::serial]
async fn outbox_claims_only_the_requested_event_kind(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let mail_id = enqueue_outbox(&mut tx, org_id, "budget_email", serde_json::json!({}))
        .await
        .unwrap();
    let project_id = enqueue_outbox(&mut tx, org_id, "project_created", serde_json::json!({}))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(claim_outbox(&pool, "unknown").await.unwrap().is_none());
    let project = claim_outbox(&pool, "project_created")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.event_kind, "project_created");
    assert_eq!(project.attempts, 1);
    assert!(
        claim_outbox(&pool, "project_created")
            .await
            .unwrap()
            .is_none()
    );
    let mail = claim_outbox(&pool, "budget_email").await.unwrap().unwrap();
    assert_eq!(mail.id, mail_id);
    assert_eq!(
        mail.attempts, 1,
        "Other consumers must not touch this event's lease"
    );
    assert!(mark_outbox_delivered(&pool, &project).await.unwrap());
    assert!(mark_outbox_delivered(&pool, &mail).await.unwrap());
    assert!(
        claim_outbox(&pool, "project_created")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
#[serial_test::serial]
async fn outbox_same_kind_consumers_cannot_claim_the_same_lease(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let id = enqueue_outbox(&mut tx, org_id, "project_created", serde_json::json!({}))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (first, second) = tokio::join!(
        claim_outbox(&pool, "project_created"),
        claim_outbox(&pool, "project_created"),
    );
    let claimed: Vec<_> = [first.unwrap(), second.unwrap()]
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, id);
    assert_eq!(claimed[0].attempts, 1);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn outbox_delivery_is_idempotent(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let id = enqueue_outbox(
        &mut tx,
        org_id,
        "jobs.test",
        serde_json::json!({"ok": true}),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let event = claim_outbox(&pool, "jobs.test").await.unwrap().unwrap();
    assert_eq!(event.id, id);
    assert!(mark_outbox_delivered(&pool, &event).await.unwrap());
    assert!(!mark_outbox_delivered(&pool, &event).await.unwrap());
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
#[serial_test::serial]
async fn outbox_rollback_does_not_publish_an_event(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    enqueue_outbox(&mut tx, org_id, "rollback", serde_json::json!({}))
        .await
        .unwrap();
    assert!(claim_outbox(&pool, "rollback").await.unwrap().is_none());
    tx.rollback().await.unwrap();
    assert!(claim_outbox(&pool, "rollback").await.unwrap().is_none());
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
#[serial_test::serial]
async fn outbox_stale_claims_cannot_acknowledge_or_reschedule_delivery(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    let id = enqueue_outbox(&mut tx, org_id, "delivery", serde_json::json!({}))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let original = claim_outbox(&pool, "delivery").await.unwrap().unwrap();
    assert!(claim_outbox(&pool, "delivery").await.unwrap().is_none());
    sqlx::query!(
        "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(!mark_outbox_delivered(&pool, &original).await.unwrap());
    let replacement = claim_outbox(&pool, "delivery").await.unwrap().unwrap();
    assert_ne!(original.claim_token, replacement.claim_token);
    assert_eq!(replacement.attempts, 2);
    assert!(!mark_outbox_delivered(&pool, &original).await.unwrap());
    assert!(
        !mark_outbox_failed(&pool, &original, "stale failure")
            .await
            .unwrap()
    );
    let mut foreign = replacement.clone();
    foreign.org_id = org(&pool).await;
    assert!(!mark_outbox_delivered(&pool, &foreign).await.unwrap());
    assert!(
        !mark_outbox_failed(&pool, &foreign, "foreign failure")
            .await
            .unwrap()
    );
    assert!(
        mark_outbox_failed(&pool, &replacement, "delivery failed")
            .await
            .unwrap()
    );
    assert!(!mark_outbox_delivered(&pool, &replacement).await.unwrap());
    assert!(
        !mark_outbox_failed(&pool, &replacement, "duplicate failure")
            .await
            .unwrap()
    );
    assert!(
        claim_outbox(&pool, "delivery").await.unwrap().is_none(),
        "failure must back off"
    );
    let failure = sqlx::query!(
        "SELECT last_error, attempts FROM horae_outbox WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failure.last_error.as_deref(), Some("delivery failed"));
    assert_eq!(failure.attempts, 2);
}
