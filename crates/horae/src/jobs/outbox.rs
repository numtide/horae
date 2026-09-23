//! Confirmed delivery of transactional project-created events to plugins.

use std::time::Duration;

use super::{OutboxEvent, POLL, Worker, claim_outbox, mark_outbox_delivered, mark_outbox_failed};
use crate::{plugin::AppEvent, state::AppState};

const DELIVERY_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn spawn(state: &AppState) -> Worker {
    let (stop, mut receiver) = tokio::sync::watch::channel(false);
    let state = state.clone();
    let task = tokio::spawn(async move {
        while !*receiver.borrow() && receiver.has_changed().is_ok() {
            // Finish a claim even during shutdown; dropping its SQL future can
            // leave a successfully assigned lease with no owning consumer.
            match claim_outbox(&state.db, "project_created").await {
                Ok(Some(event)) => {
                    if let Err(error) = deliver(&state, &event).await {
                        tracing::warn!(%error, event_id = %event.id, "project event delivery storage failed");
                    }
                    continue;
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "project event poll failed"),
            }
            tokio::select! {
                _ = receiver.changed() => break,
                _ = tokio::time::sleep(POLL) => {}
            }
        }
    });
    Worker { stop, task }
}

async fn deliver(state: &AppState, event: &OutboxEvent) -> anyhow::Result<()> {
    let live = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM horae_outbox
         WHERE id = $1 AND org_id = $2 AND claim_token = $3
           AND delivered_at IS NULL AND available_at > now())",
        event.id,
        event.org_id,
        event.claim_token,
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(false);
    if !live {
        return Ok(());
    }
    let decoded = serde_json::from_value::<AppEvent>(event.payload.clone());
    let failure = match decoded {
        // The deadline is shorter than the outbox lease. A cancelled or crashed
        // consumer leaves the same event available after lease expiry.
        Ok(decoded @ AppEvent::ProjectCreated { org_id, .. })
            if org_id == event.org_id && event.event_kind == "project_created" =>
        {
            match tokio::time::timeout(DELIVERY_TIMEOUT, state.plugins.dispatch_confirmed(&decoded))
                .await
            {
                Ok(Ok(())) => None,
                Ok(Err(_)) => Some("Project-created plugin delivery failed"),
                Err(_) => Some("Project-created plugin delivery timed out"),
            }
        }
        _ => Some("Invalid project-created event"),
    };
    // Never persist plugin error strings or malformed payloads: they may contain
    // confidential input. The event ID and attempt count identify the failure.
    if let Some(message) = failure {
        if mark_outbox_failed(&state.db, event, message).await? {
            tracing::warn!(event_id = %event.id, attempt = event.attempts, "{message}");
        }
    } else {
        mark_outbox_delivered(&state.db, event).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        jobs,
        plugin::{AppEvent, PluginRegistry, event::ProjectPayload},
        state::AppState,
    };
    use uuid::Uuid;

    async fn enqueue(pool: &sqlx::PgPool) -> jobs::OutboxEvent {
        let org_id = super::super::tests::org(pool).await;
        let event = AppEvent::ProjectCreated {
            occurred_at: chrono::Utc::now(),
            org_id,
            project: ProjectPayload {
                id: Uuid::now_v7(),
                client_id: Uuid::now_v7(),
                name: "Created project".into(),
                project_type: "time_and_materials".into(),
                budget_kind: "none".into(),
                active: true,
            },
        };
        let mut tx = pool.begin().await.unwrap();
        jobs::enqueue_outbox(
            &mut tx,
            org_id,
            "project_created",
            serde_json::to_value(event).unwrap(),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        jobs::claim_outbox(pool, "project_created")
            .await
            .unwrap()
            .unwrap()
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn acknowledges_confirmed_delivery_without_reclaiming_it(pool: sqlx::PgPool) {
        let event = enqueue(&pool).await;
        let state = AppState::new(pool.clone(), Arc::new(PluginRegistry::empty()));
        deliver(&state, &event).await.unwrap();
        let delivered = sqlx::query_scalar!(
            "SELECT delivered_at IS NOT NULL FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(delivered, Some(true));
        assert!(
            jobs::claim_outbox(&pool, "project_created")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn worker_delivers_available_events_and_stops_cleanly(pool: sqlx::PgPool) {
        let event = enqueue(&pool).await;
        sqlx::query!(
            "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
            event.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let state = AppState::new(pool.clone(), Arc::new(PluginRegistry::empty()));
        let worker = spawn(&state);
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if sqlx::query_scalar!(
                    "SELECT delivered_at IS NOT NULL FROM horae_outbox WHERE id = $1",
                    event.id
                )
                .fetch_one(&pool)
                .await
                .unwrap()
                    == Some(true)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        worker.shutdown(Duration::from_secs(5)).await.unwrap();
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn invalid_payload_is_not_delivered_and_does_not_leak_into_errors(pool: sqlx::PgPool) {
        let mut event = enqueue(&pool).await;
        event.payload["org_id"] = serde_json::json!(Uuid::now_v7());
        event.payload["project"]["name"] = serde_json::json!("confidential name");
        let state = AppState::new(pool.clone(), Arc::new(PluginRegistry::empty()));
        deliver(&state, &event).await.unwrap();
        let row = sqlx::query!(
            "SELECT delivered_at, last_error, claim_token FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(row.delivered_at.is_none());
        assert_eq!(
            row.last_error.as_deref(),
            Some("Invalid project-created event")
        );
        assert!(row.claim_token.is_none());
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn plugin_failure_retries_with_the_same_event_identity(pool: sqlx::PgPool) {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("delivery");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            "[plugin]\nname = 'delivery-test'\nversion = '1.0.0'\nhooks = ['project_created']\n",
        )
        .unwrap();
        let wasm_path = plugin_dir.join("test.wasm");
        std::fs::write(
            &wasm_path,
            r#"(module (func (export "project_created") (result i32) unreachable))"#,
        )
        .unwrap();
        let registry = PluginRegistry::load(dir.path(), None).await.unwrap();
        assert_eq!(registry.plugin_count(), 1);
        let event = enqueue(&pool).await;
        let state = AppState::new(pool.clone(), Arc::new(registry));
        deliver(&state, &event).await.unwrap();
        let failure = sqlx::query!(
            "SELECT delivered_at, last_error, claim_token FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(failure.delivered_at.is_none());
        assert_eq!(
            failure.last_error.as_deref(),
            Some("Project-created plugin delivery failed")
        );
        assert!(failure.claim_token.is_none());
        assert!(
            jobs::claim_outbox(&pool, "project_created")
                .await
                .unwrap()
                .is_none()
        );
        sqlx::query!(
            "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
            event.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let retry = jobs::claim_outbox(&pool, "project_created")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retry.id, event.id);
        assert_eq!(retry.payload, event.payload);
        assert_eq!(retry.attempts, 2);
        std::fs::write(
            &wasm_path,
            r#"(module (func (export "project_created") (result i32) i32.const 0))"#,
        )
        .unwrap();
        let registry = PluginRegistry::load(dir.path(), None).await.unwrap();
        assert_eq!(registry.plugin_count(), 1);
        let state = AppState::new(pool.clone(), Arc::new(registry));
        deliver(&state, &retry).await.unwrap();
        let delivered = sqlx::query_scalar!(
            "SELECT delivered_at IS NOT NULL FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(delivered, Some(true));
    }
}
