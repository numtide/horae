use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};

use super::{BudgetEmail, BudgetMessage, DateTime, MAX_ATTEMPTS, Utc};
use crate::jobs::OutboxEvent;

const INELIGIBLE: &str = "Budget email recipient or project is no longer eligible";

pub(super) struct Prepared {
    pub recipient: String,
    pub message: BudgetMessage,
    pub attempts: i32,
}

pub(super) async fn prepare(
    pool: &PgPool,
    event: &OutboxEvent,
) -> anyhow::Result<Option<Prepared>> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("SET LOCAL idle_in_transaction_session_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let prepared = prepare_locked(&mut tx, event).await?;
    tx.commit().await?;
    Ok(prepared)
}

async fn prepare_locked(
    tx: &mut Transaction<'_, Postgres>,
    event: &OutboxEvent,
) -> anyhow::Result<Option<Prepared>> {
    if sqlx::query_scalar!(
        "SELECT id FROM organizations WHERE id=$1 FOR SHARE",
        event.org_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .is_none()
    {
        return Ok(None);
    }
    let Some(stored) = sqlx::query!(
        "SELECT payload, attempts FROM horae_outbox WHERE id=$1 AND org_id=$2
         AND claim_token=$3 AND event_kind='budget_email'
         AND delivered_at IS NULL AND failed_at IS NULL
         AND available_at > clock_timestamp() + interval '25 seconds'",
        event.id,
        event.org_id,
        event.claim_token,
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let payload = BudgetEmail::deserialize(&stored.payload).ok();
    let target = if let Some(payload) = &payload {
        sqlx::query!(
            "SELECT recipient_id, project_id FROM project_budget_notifications WHERE id=$1 AND org_id=$2",
            payload.notification_id, event.org_id,
        ).fetch_optional(&mut **tx).await?
    } else {
        None
    };
    if let Some(target) = &target {
        sqlx::query_scalar!(
            "SELECT id FROM users WHERE id=$1 AND org_id=$2 FOR SHARE",
            target.recipient_id,
            event.org_id
        )
        .fetch_optional(&mut **tx)
        .await?;
        sqlx::query_scalar!(
            "SELECT id FROM projects WHERE id=$1 AND org_id=$2 FOR SHARE",
            target.project_id,
            event.org_id
        )
        .fetch_optional(&mut **tx)
        .await?;
    }
    // Claim bookkeeping comes after domain locks, and cannot change between
    // validation and a terminal rejection. Never upgrade a shared outbox lock.
    sqlx::query_scalar!(
        "SELECT id FROM horae_outbox WHERE id=$1 AND org_id=$2 FOR UPDATE",
        event.id,
        event.org_id
    )
    .fetch_optional(&mut **tx)
    .await?;
    let current = sqlx::query_scalar!(
        "SELECT id FROM horae_outbox WHERE id=$1 AND org_id=$2
         AND claim_token=$3 AND event_kind='budget_email' AND payload=$4 AND attempts=$5
         AND delivered_at IS NULL AND failed_at IS NULL
         AND available_at > clock_timestamp() + interval '25 seconds'",
        event.id,
        event.org_id,
        event.claim_token,
        stored.payload,
        stored.attempts,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if current.is_none() {
        return Ok(None);
    }
    if stored.attempts > MAX_ATTEMPTS {
        reject(tx, event, "Budget email attempt limit reached").await?;
        return Ok(None);
    }
    let Some(payload) = payload else {
        reject(tx, event, "Invalid budget email event").await?;
        return Ok(None);
    };
    let Some(target) = target else {
        let appeared = sqlx::query_scalar!(
            "SELECT id FROM project_budget_notifications WHERE id=$1 AND org_id=$2",
            payload.notification_id,
            event.org_id,
        )
        .fetch_optional(&mut **tx)
        .await?
        .is_some();
        if !appeared {
            reject(tx, event, INELIGIBLE).await?;
        }
        return Ok(None);
    };
    // Settings/assignment writers update the parent through revision triggers.
    // Read after the parent wait; child locks here would invert that order.
    let Some(row) = sqlx::query!(
        r#"SELECT n.id, n.created_at AS "created_at: DateTime<Utc>", n.period_key, n.threshold,
                  p.name, u.email,
                  CASE WHEN n.task_id IS NOT NULL THEN 'task' WHEN n.user_id IS NOT NULL THEN 'person' ELSE 'project' END AS "scope!",
                  (p.active AND u.active AND COALESCE(ps.alert_enabled, false)
                   AND (u.org_role IN ('admin', 'manager') OR EXISTS (
                       SELECT 1 FROM assignments a WHERE a.project_id=p.id AND a.user_id=u.id
                       AND (a.role IN ('lead', 'admin') OR u.id=ps.creator_id)))) AS "eligible!"
           FROM project_budget_notifications n
           JOIN projects p ON p.id=n.project_id AND p.org_id=n.org_id
           JOIN users u ON u.id=n.recipient_id AND u.org_id=n.org_id
           LEFT JOIN project_settings ps ON ps.project_id=p.id AND ps.org_id=n.org_id
           WHERE n.id=$1 AND n.org_id=$2 AND n.recipient_id=$3 AND n.project_id=$4"#,
        payload.notification_id, event.org_id, target.recipient_id, target.project_id,
    ).fetch_optional(&mut **tx).await? else {
        return Ok(None);
    };
    if !row.eligible {
        reject(tx, event, INELIGIBLE).await?;
        return Ok(None);
    }
    Ok(Some(Prepared {
        recipient: row.email,
        message: BudgetMessage {
            id: row.id,
            created_at: row.created_at,
            project_name: row.name,
            scope: row.scope,
            period: row.period_key,
            threshold: row.threshold,
        },
        attempts: stored.attempts,
    }))
}

async fn reject(
    tx: &mut Transaction<'_, Postgres>,
    event: &OutboxEvent,
    reason: &'static str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE horae_outbox SET failed_at=clock_timestamp(), last_error=$4, claim_token=NULL
         WHERE id=$1 AND org_id=$2 AND claim_token=$3 AND event_kind='budget_email'
         AND delivered_at IS NULL AND failed_at IS NULL AND available_at > clock_timestamp()",
        event.id,
        event.org_id,
        event.claim_token,
        reason,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
