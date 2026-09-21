//! Optional budget email delivery through a configured sendmail executable.

use std::{process::Stdio, time::Duration};

use chrono::{DateTime, Utc};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::config::{MailConfig, valid_mailbox};

const SEND_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_ATTEMPTS: i32 = 5;

pub(crate) fn spawn(state: &crate::state::AppState) -> crate::jobs::Worker {
    let (stop, mut receiver) = tokio::sync::watch::channel(false);
    let state = state.clone();
    let task = tokio::spawn(async move {
        let Some(config) = state.mail.as_ref() else {
            return;
        };
        while !*receiver.borrow() && receiver.has_changed().is_ok() {
            // Complete a claim and its bounded delivery before honoring a drain.
            match crate::jobs::claim_outbox(&state.db, "budget_email").await {
                Ok(Some(event)) => {
                    if deliver(&state.db, config, &event).await.is_err() {
                        tracing::warn!(event_id = %event.id, "budget email storage failed");
                    }
                    continue;
                }
                Ok(None) => {}
                Err(_) => tracing::warn!("budget email poll failed"),
            }
            tokio::select! {
                _ = receiver.changed() => break,
                _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            }
        }
    });
    crate::jobs::Worker::new(stop, task)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetEmail {
    notification_id: Uuid,
}

async fn deliver(
    pool: &sqlx::PgPool,
    config: &MailConfig,
    event: &crate::jobs::OutboxEvent,
) -> anyhow::Result<()> {
    let Some(stored) = sqlx::query!(
        "SELECT payload, attempts FROM horae_outbox WHERE id = $1 AND org_id = $2
         AND claim_token = $3 AND event_kind = 'budget_email'
         AND delivered_at IS NULL AND failed_at IS NULL
         AND available_at > now() + interval '25 seconds'",
        event.id,
        event.org_id,
        event.claim_token,
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };
    if stored.attempts > MAX_ATTEMPTS {
        crate::jobs::stop_outbox_delivery(pool, event, "Budget email attempt limit reached")
            .await?;
        return Ok(());
    }
    let Ok(payload) = serde_json::from_value::<BudgetEmail>(stored.payload) else {
        crate::jobs::stop_outbox_delivery(pool, event, "Invalid budget email event").await?;
        return Ok(());
    };
    let Some(row) = sqlx::query!(
        r#"SELECT n.id, n.created_at AS "created_at: DateTime<Utc>", n.period_key, n.threshold,
                  p.name, u.email,
                  CASE WHEN n.task_id IS NOT NULL THEN 'task' WHEN n.user_id IS NOT NULL THEN 'person' ELSE 'project' END AS "scope!"
           FROM project_budget_notifications n
           JOIN projects p ON p.id = n.project_id AND p.org_id = n.org_id
           JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = n.org_id
           JOIN users u ON u.id = n.recipient_id AND u.org_id = n.org_id
           WHERE n.id = $1 AND n.org_id = $2 AND p.active AND ps.alert_enabled AND u.active
             AND (u.org_role IN ('admin', 'manager') OR EXISTS (
                 SELECT 1 FROM assignments a WHERE a.project_id = p.id AND a.user_id = u.id
                 AND (a.role IN ('lead', 'admin') OR u.id = ps.creator_id)))"#,
        payload.notification_id, event.org_id,
    ).fetch_optional(pool).await? else {
        crate::jobs::stop_outbox_delivery(pool, event, "Budget email recipient or project is no longer eligible").await?;
        return Ok(());
    };
    let message = BudgetMessage {
        id: row.id,
        created_at: row.created_at,
        project_name: row.name,
        scope: row.scope,
        period: row.period_key,
        threshold: row.threshold,
    };
    match send(config, &row.email, &message, SEND_TIMEOUT).await {
        Ok(()) => {
            crate::jobs::mark_outbox_delivered(pool, event).await?;
        }
        Err(error) => {
            let terminal = stored.attempts >= MAX_ATTEMPTS
                || matches!(error, MailError::InvalidAddress | MailError::InvalidMessage);
            let recorded = if terminal {
                crate::jobs::stop_outbox_delivery(pool, event, &error.to_string()).await?
            } else {
                crate::jobs::mark_outbox_failed(pool, event, &error.to_string()).await?
            };
            if recorded {
                tracing::warn!(event_id = %event.id, attempt = stored.attempts, terminal, %error, "budget email attempt failed");
            }
        }
    }
    Ok(())
}

struct BudgetMessage {
    id: Uuid,
    created_at: DateTime<Utc>,
    project_name: String,
    scope: String,
    period: String,
    threshold: i16,
}

#[derive(Debug, PartialEq, thiserror::Error)]
enum MailError {
    #[error("Invalid budget email address")]
    InvalidAddress,
    #[error("Invalid budget email message")]
    InvalidMessage,
    #[error("Budget email transport failed")]
    Transport,
    #[error("Budget email was not accepted by the mail transport")]
    Rejected,
    #[error("Budget email transport timed out")]
    Timeout,
}

fn render(
    config: &MailConfig,
    recipient: &str,
    message: &BudgetMessage,
) -> Result<String, MailError> {
    if !valid_mailbox(recipient) || !valid_mailbox(&config.sender) {
        return Err(MailError::InvalidAddress);
    }
    if message.project_name.len() > 4096
        || message.period.len() > 20
        || !matches!(message.scope.as_str(), "project" | "task" | "person")
        || !(0..=100).contains(&message.threshold)
    {
        return Err(MailError::InvalidMessage);
    }
    // JSON string quoting keeps control characters in imported names or periods
    // from impersonating message structure, without losing Unicode names.
    let project =
        serde_json::to_string(&message.project_name).map_err(|_| MailError::InvalidMessage)?;
    let period = serde_json::to_string(&message.period).map_err(|_| MailError::InvalidMessage)?;
    let body = format!(
        "Project: {project}\nScope: {}\nPeriod: {period}\n\nThe configured budget reached {}%.\nOpen Horae to review current progress.",
        message.scope, message.threshold,
    );
    let mut wrapped = String::new();
    // Even a multibyte Unicode name stays below the message line byte limit.
    for line in body.lines() {
        for (index, character) in line.chars().enumerate() {
            if index > 0 && index % 78 == 0 {
                wrapped.push_str("\r\n");
            }
            wrapped.push(character);
        }
        wrapped.push_str("\r\n");
    }
    Ok(format!(
        "From: {}\r\nTo: {recipient}\r\nDate: {}\r\nMessage-ID: <budget-{}@horae.invalid>\r\nSubject: Horae project budget alert\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{wrapped}",
        config.sender,
        message.created_at.to_rfc2822(),
        message.id,
    ))
}

async fn send(
    config: &MailConfig,
    recipient: &str,
    message: &BudgetMessage,
    timeout: Duration,
) -> Result<(), MailError> {
    let body = render(config, recipient, message)?;
    let mut child = tokio::process::Command::new(&config.executable)
        .args(["-i", "-f", &config.sender, "--", recipient])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| MailError::Transport)?;
    let mut stdin = child.stdin.take().ok_or(MailError::Transport)?;
    let result = tokio::time::timeout(timeout, async {
        stdin.write_all(body.as_bytes()).await?;
        drop(stdin);
        child.wait().await
    })
    .await;
    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(_)) => Err(MailError::Rejected),
        other => {
            // Reap explicitly on normal error/timeout; kill_on_drop also covers
            // cancellation during shutdown without leaving a detached sender.
            let _ = child.kill().await;
            Err(if other.is_err() {
                MailError::Timeout
            } else {
                MailError::Transport
            })
        }
    }
}

#[cfg(test)]
mod tests;
