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

struct Echo;

impl Handler for Echo {
    fn kind(&self) -> &'static str {
        "echo"
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
            Ok(Completed {
                report: serde_json::json!({ "echo": payload }),
                processed_count: 3,
            })
        })
    }
}

fn registry() -> Registry {
    let mut registry = Registry::default();
    registry.register(Echo);
    registry
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
async fn a_kind_without_a_handler_fails_its_attempt(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = insert_job(&pool, org_id, "unknown_kind", serde_json::json!({}), 1).await;
    let job = claim(&pool, "worker").await.unwrap().unwrap();
    execute(&pool, &registry(), "worker", job).await.unwrap();
    let failed = row(&pool, id).await;
    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.last_error.as_deref(),
        Some("no handler for durable job kind unknown_kind")
    );
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
    assert!(lease.renew(&pool).await.is_err());
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
