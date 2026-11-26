//! Events written in the transaction of the change that produced them, for
//! consumers that claim and acknowledge them on their own schedule.

use uuid::Uuid;

/// Insert an event in the same transaction as the state change that produced
/// it. Consumers can claim undelivered rows independently of the job worker.
pub async fn enqueue_outbox(
    tx: &mut sqlx::PgConnection,
    org_id: Uuid,
    event_kind: &str,
    payload: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"INSERT INTO horae_outbox (id, org_id, event_kind, payload)
           VALUES ($1, $2, $3, $4)"#,
        id,
        org_id,
        event_kind,
        payload,
    )
    .execute(tx)
    .await?;
    Ok(id)
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub org_id: Uuid,
    pub event_kind: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
    pub claim_token: Uuid,
}

/// Claim one event of the consumer's kind. Moving `available_at` acts as a short
/// lease, so a crashed consumer can safely retry it later without taking events
/// intended for another consumer.
pub async fn claim_outbox(
    pool: &sqlx::PgPool,
    event_kind: &str,
) -> anyhow::Result<Option<OutboxEvent>> {
    let claim_token = Uuid::now_v7();
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id FROM horae_outbox
              WHERE delivered_at IS NULL AND failed_at IS NULL AND available_at <= now() AND event_kind = $2
              ORDER BY available_at, created_at
              FOR UPDATE SKIP LOCKED LIMIT 1
           )
           UPDATE horae_outbox o
              SET attempts = o.attempts + 1,
                  claim_token = $1,
                  available_at = now() + interval '5 minutes'
             FROM candidate
            WHERE o.id = candidate.id
        RETURNING o.id, o.org_id, o.event_kind, o.payload, o.attempts,
                  o.claim_token as "claim_token!""#,
        claim_token,
        event_kind,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| OutboxEvent {
        id: r.id,
        org_id: r.org_id,
        event_kind: r.event_kind,
        payload: r.payload,
        attempts: r.attempts,
        claim_token: r.claim_token,
    }))
}

pub async fn mark_outbox_delivered(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_outbox SET delivered_at = now(), last_error = NULL, claim_token = NULL
            WHERE id = $1 AND org_id = $2 AND claim_token = $3
              AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()"#,
        event.id,
        event.org_id,
        event.claim_token,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn mark_outbox_failed(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
    error: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_outbox
              SET available_at = now() + LEAST(power(2::double precision, LEAST(attempts, 9)), 300)::int * interval '1 second',
                  last_error = $4, claim_token = NULL
            WHERE id = $1 AND org_id = $2 AND claim_token = $3
              AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()"#,
        event.id,
        event.org_id,
        event.claim_token,
        error,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Retain a terminal failure without acknowledging delivery or permitting another claim.
pub async fn stop_outbox_delivery(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
    error: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "UPDATE horae_outbox SET failed_at = now(), last_error = $4, claim_token = NULL
         WHERE id = $1 AND org_id = $2 AND claim_token = $3
           AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()",
        event.id,
        event.org_id,
        event.claim_token,
        error,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}
