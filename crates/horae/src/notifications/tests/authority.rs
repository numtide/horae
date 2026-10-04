use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use serde::Deserialize;
use std::io::Write as _;

fn start_delivery(
    pool: &sqlx::PgPool,
    stub: &Stub,
    event: &crate::jobs::OutboxEvent,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let pool = pool.clone();
    let config = stub.config.clone();
    let event = event.clone();
    tokio::spawn(async move { deliver(&pool, &config, &event).await })
}

async fn restricted_pool(pool: &sqlx::PgPool) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
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
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_waits_for_organization_and_rechecks_recipient(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (ids, event) = queued(&pool).await;
    let mut writer = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&mut *writer)
        .await
        .unwrap();
    let running = {
        let pool = pool.clone();
        let config = stub.config.clone();
        let event = event.clone();
        tokio::spawn(async move { deliver(&pool, &config, &event).await })
    };
    wait_for_blocked(&pool, pid).await;
    writer.commit().await.unwrap();
    running.await.unwrap().unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT failed_at IS NOT NULL AND delivered_at IS NULL FROM horae_outbox WHERE id=$1",
            event.id,
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_refreshes_after_recipient_and_project_waits(pool: sqlx::PgPool) {
    let reader = restricted_pool(&pool).await;
    for change in ["recipient", "role", "project", "assignment", "alerts"] {
        let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
        let (ids, event) = queued(&pool).await;
        if change == "assignment" {
            sqlx::query!(
                "UPDATE users SET org_role='member' WHERE id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "INSERT INTO assignments(id,project_id,user_id,role) VALUES($1,$2,$3,'lead')",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let mut writer = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap()
            .unwrap();
        match change {
            "recipient" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *writer)
                    .await
                    .unwrap();
            }
            "role" => {
                sqlx::query!(
                    "UPDATE users SET org_role='member' WHERE id=$1",
                    ids.user_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
                sqlx::query!(
                    "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
                    ids.project_id,
                    ids.user_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "project" => {
                sqlx::query!(
                    "UPDATE projects SET active=false WHERE id=$1",
                    ids.project_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "assignment" => {
                sqlx::query!(
                    "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
                    ids.project_id,
                    ids.user_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "alerts" => {
                sqlx::query!(
                    "UPDATE project_settings SET alert_enabled=false WHERE project_id=$1",
                    ids.project_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let running = start_delivery(&reader, &stub, &event);
        wait_for_blocked(&pool, pid).await;
        writer.commit().await.unwrap();
        running.await.unwrap().unwrap();
        assert!(
            !PathBuf::from(format!("{}.called", stub.program.display())).exists(),
            "{change}"
        );
        assert_eq!(sqlx::query_scalar!(
            "SELECT failed_at IS NOT NULL AND delivered_at IS NULL FROM horae_outbox WHERE id=$1",
            event.id,
        ).fetch_one(&pool).await.unwrap(), Some(true), "{change}");
    }
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_rechecks_claim_and_clock_after_final_wait(pool: sqlx::PgPool) {
    for change in ["margin", "token", "payload", "attempts", "kind", "terminal"] {
        let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
        let (_, event) = queued(&pool).await;
        let mut writer = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
            event.id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        let running = start_delivery(&pool, &stub, &event);
        wait_for_blocked(&pool, pid).await;
        match change {
            // A transaction-start clock would incorrectly retain the margin.
            "margin" => {
                sqlx::query!("UPDATE horae_outbox SET available_at=clock_timestamp()+interval '25 seconds' WHERE id=$1", event.id).execute(&mut *writer).await.unwrap();
            }
            "token" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET claim_token=$2 WHERE id=$1",
                    event.id,
                    Uuid::now_v7()
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "payload" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET payload='{}'::jsonb WHERE id=$1",
                    event.id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "attempts" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET attempts=attempts+1 WHERE id=$1",
                    event.id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "kind" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET event_kind='project_created' WHERE id=$1",
                    event.id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "terminal" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET failed_at=clock_timestamp() WHERE id=$1",
                    event.id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = sqlx::query_scalar!(
            "SELECT to_jsonb(o) AS \"row!\" FROM horae_outbox o WHERE id=$1",
            event.id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        running.await.unwrap().unwrap();
        assert!(
            !PathBuf::from(format!("{}.called", stub.program.display())).exists(),
            "{change}"
        );
        let after = sqlx::query_scalar!(
            "SELECT to_jsonb(o) AS \"row!\" FROM horae_outbox o WHERE id=$1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            before, after,
            "{change}: stale preparation must not change the event"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_uses_current_payload_after_recipient_wait(pool: sqlx::PgPool) {
    let reader = restricted_pool(&pool).await;
    let stub = Stub::new("printf '%s\\n' \"$@\" > \"$0.args\"\ncat > \"$0.message\"");
    let (ids, mut event) = queued(&pool).await;
    event.payload = serde_json::json!({"notification_id": Uuid::now_v7()});
    event.attempts = 999;
    event.event_kind = "forged".into();
    let mut writer = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE users SET email='current@example.test' WHERE id=$1",
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET name='Current project' WHERE id=$1",
        ids.project_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    let running = start_delivery(&reader, &stub, &event);
    wait_for_blocked(&pool, pid).await;
    writer.commit().await.unwrap();
    running.await.unwrap().unwrap();
    assert!(stub.captured(".args").ends_with("current@example.test\n"));
    assert!(stub.captured(".message").contains("Current project"));
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_skips_retargeted_notification_without_terminalizing(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (ids, event) = queued(&pool).await;
    let recipient = Uuid::now_v7();
    sqlx::query!("INSERT INTO users(id,org_id,name,email,org_role) VALUES($1,$2,'Replacement','replacement@example.test','manager')", recipient, ids.org_id)
        .execute(&pool).await.unwrap();
    let notification = BudgetEmail::deserialize(&event.payload)
        .unwrap()
        .notification_id;
    let mut writer = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
        event.id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let running = start_delivery(&pool, &stub, &event);
    wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "UPDATE project_budget_notifications SET recipient_id=$2 WHERE id=$1",
        notification,
        recipient
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    running.await.unwrap().unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
    assert_eq!(sqlx::query_scalar!(
        "SELECT failed_at IS NULL AND delivered_at IS NULL AND claim_token=$2 FROM horae_outbox WHERE id=$1",
        event.id, event.claim_token,
    ).fetch_one(&pool).await.unwrap(), Some(true));
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_skips_project_drift_or_disappearance(pool: sqlx::PgPool) {
    for remove in [false, true] {
        let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
        let (ids, event) = queued(&pool).await;
        let notification = BudgetEmail::deserialize(&event.payload)
            .unwrap()
            .notification_id;
        let replacement = Uuid::now_v7();
        sqlx::query!("INSERT INTO projects(id,org_id,client_id,name,currency) VALUES($1,$2,$3,'Replacement project','EUR')", replacement, ids.org_id, ids.client_id)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO project_settings(id,org_id,project_id,creator_id,rate_mode,alert_enabled) VALUES($1,$2,$3,$4,'project',true)", Uuid::now_v7(), ids.org_id, replacement, ids.user_id)
            .execute(&pool).await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
            event.id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        let running = start_delivery(&pool, &stub, &event);
        wait_for_blocked(&pool, pid).await;
        if remove {
            sqlx::query!(
                "DELETE FROM project_budget_notifications WHERE id=$1",
                notification
            )
            .execute(&mut *writer)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "UPDATE project_budget_notifications SET project_id=$2 WHERE id=$1",
                notification,
                replacement
            )
            .execute(&mut *writer)
            .await
            .unwrap();
        }
        writer.commit().await.unwrap();
        running.await.unwrap().unwrap();
        assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
        assert_eq!(sqlx::query_scalar!("SELECT failed_at IS NULL AND delivered_at IS NULL AND claim_token=$2 FROM horae_outbox WHERE id=$1", event.id, event.claim_token)
            .fetch_one(&pool).await.unwrap(), Some(true));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_observes_winning_activation_and_alert_enable(pool: sqlx::PgPool) {
    let reader = restricted_pool(&pool).await;
    for recipient in [true, false] {
        let stub = Stub::new("cat > \"$0.message\"");
        let (ids, event) = queued(&pool).await;
        if recipient {
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "UPDATE project_settings SET alert_enabled=false WHERE project_id=$1",
                ids.project_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let mut writer = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap()
            .unwrap();
        if recipient {
            sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
                .execute(&mut *writer)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "UPDATE project_settings SET alert_enabled=true WHERE project_id=$1",
                ids.project_id
            )
            .execute(&mut *writer)
            .await
            .unwrap();
        }
        let running = start_delivery(&reader, &stub, &event);
        wait_for_blocked(&pool, pid).await;
        writer.commit().await.unwrap();
        running.await.unwrap().unwrap();
        assert!(stub.captured(".message").contains("Scope: project"));
        assert_eq!(sqlx::query_scalar!("SELECT delivered_at IS NOT NULL AND failed_at IS NULL FROM horae_outbox WHERE id=$1", event.id)
            .fetch_one(&pool).await.unwrap(), Some(true));
    }
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_terminal_rejections_preserve_existing_reasons(pool: sqlx::PgPool) {
    for (case, reason) in [
        ("payload", "Invalid budget email event"),
        ("attempts", "Budget email attempt limit reached"),
        (
            "missing",
            "Budget email recipient or project is no longer eligible",
        ),
    ] {
        let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
        let (_, event) = queued(&pool).await;
        match case {
            "payload" => {
                sqlx::query!(
                    "UPDATE horae_outbox SET payload='{}'::jsonb WHERE id=$1",
                    event.id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "attempts" => {
                sqlx::query!("UPDATE horae_outbox SET attempts=6 WHERE id=$1", event.id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            _ => {
                sqlx::query!("UPDATE horae_outbox SET payload=jsonb_build_object('notification_id',$2::uuid) WHERE id=$1", event.id, Uuid::now_v7()).execute(&pool).await.unwrap();
            }
        }
        deliver(&pool, &stub.config, &event).await.unwrap();
        assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
        let row = sqlx::query!(
            "SELECT last_error, failed_at, delivered_at, claim_token FROM horae_outbox WHERE id=$1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.last_error.as_deref(), Some(reason));
        assert!(row.failed_at.is_some());
        assert!(row.delivered_at.is_none());
        assert!(row.claim_token.is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_skips_notification_appearing_after_discovery(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (ids, event) = queued(&pool).await;
    let notification = BudgetEmail::deserialize(&event.payload)
        .unwrap()
        .notification_id;
    sqlx::query!(
        "DELETE FROM project_budget_notifications WHERE id=$1",
        notification
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut writer = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
        event.id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let running = start_delivery(&pool, &stub, &event);
    wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "INSERT INTO project_budget_notifications(id,org_id,project_id,recipient_id,period_key,threshold) VALUES($1,$2,$3,$4,'lifetime',0)",
        notification, ids.org_id, ids.project_id, ids.user_id,
    ).execute(&mut *writer).await.unwrap();
    writer.commit().await.unwrap();
    running.await.unwrap().unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
    assert_eq!(sqlx::query_scalar!(
        "SELECT failed_at IS NULL AND delivered_at IS NULL AND claim_token=$2 FROM horae_outbox WHERE id=$1",
        event.id, event.claim_token,
    ).fetch_one(&pool).await.unwrap(), Some(true));
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_rejects_missing_settings_without_waiting_on_child_locks(
    pool: sqlx::PgPool,
) {
    let stub = Stub::new("cat > \"$0.message\"");
    let (ids, event) = queued(&pool).await;
    let mut child = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM project_settings WHERE project_id=$1 FOR UPDATE",
        ids.project_id
    )
    .fetch_one(&mut *child)
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), deliver(&pool, &stub.config, &event))
        .await
        .unwrap()
        .unwrap();
    assert!(stub.captured(".message").contains("Scope: project"));
    child.rollback().await.unwrap();

    let (ids, event) = queued(&pool).await;
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    sqlx::query!(
        "DELETE FROM project_settings WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    deliver(&pool, &stub.config, &event).await.unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT failed_at IS NOT NULL FROM horae_outbox WHERE id=$1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_preparation_overrides_read_only_without_leaking_transaction_settings(
    pool: sqlx::PgPool,
) {
    let (_, event) = queued(&pool).await;
    let reader = restricted_pool(&pool).await;
    sqlx::query!("SET default_transaction_read_only=on")
        .execute(&reader)
        .await
        .unwrap();
    let prepared = preparation::prepare(&reader, &event)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(prepared.attempts, 1);
    let settings = sqlx::query!(
        "SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\"",
    ).fetch_one(&reader).await.unwrap();
    assert_eq!(settings.isolation, "repeatable read");
    assert_eq!(settings.read_only, "on");
    reader.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_preparation_cancellation_and_deadline_release_single_connection(
    pool: sqlx::PgPool,
) {
    for cancel in [true, false] {
        let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
        let (ids, event) = queued(&pool).await;
        let reader = restricted_pool(&pool).await;
        let mut writer = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *writer)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!(
            "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
            event.id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        if cancel {
            let running = start_delivery(&reader, &stub, &event);
            wait_for_blocked(&pool, pid).await;
            running.abort();
            assert!(running.await.unwrap_err().is_cancelled());
        } else {
            sqlx::query!("SET lock_timeout='20ms'")
                .execute(&reader)
                .await
                .unwrap();
            let error = deliver(&reader, &stub.config, &event).await.unwrap_err();
            assert!(
                matches!(error.downcast_ref::<sqlx::Error>(), Some(sqlx::Error::Database(error)) if error.code().as_deref()==Some("55P03"))
            );
        }
        writer.rollback().await.unwrap();
        let mut connection = tokio::time::timeout(Duration::from_secs(5), reader.acquire())
            .await
            .unwrap()
            .unwrap();
        sqlx::query!("SELECT 1 AS value")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        drop(connection);
        // A different connection proves the cancelled reader released locks;
        // reentrant locking on the recovered reader would not prove cleanup.
        let mut observer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *observer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            ids.user_id
        )
        .fetch_one(&mut *observer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM projects WHERE id=$1 FOR UPDATE NOWAIT",
            ids.project_id
        )
        .fetch_one(&mut *observer)
        .await
        .unwrap();
        observer.rollback().await.unwrap();
        assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
        assert_eq!(sqlx::query_scalar!("SELECT failed_at IS NULL AND delivered_at IS NULL AND claim_token=$2 FROM horae_outbox WHERE id=$1", event.id, event.claim_token)
            .fetch_one(&pool).await.unwrap(), Some(true));
        reader.close().await;
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_delivery_releases_all_gates_before_blocked_transport(pool: sqlx::PgPool) {
    let stub =
        Stub::new("cat > \"$0.message\"\ntouch \"$0.started\"\nread -r resume < \"$0.release\"");
    let release = PathBuf::from(format!("{}.release", stub.program.display()));
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&release)
            .status()
            .unwrap()
            .success()
    );
    // Keep both ends open so a failed sender cannot leave a blocking FIFO open.
    let mut release = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(release)
        .unwrap();
    let (ids, event) = queued(&pool).await;
    let reader = restricted_pool(&pool).await;
    let mut outbox = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *outbox)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE",
        event.id
    )
    .fetch_one(&mut *outbox)
    .await
    .unwrap();
    let running = start_delivery(&reader, &stub, &event);
    wait_for_blocked(&pool, pid).await;
    for (id, resource) in [
        (ids.org_id, "organization"),
        (ids.user_id, "recipient"),
        (ids.project_id, "project"),
    ] {
        let mut writer = pool.begin().await.unwrap();
        let result = match resource {
            "organization" => sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
                id
            )
            .fetch_one(&mut *writer)
            .await
            .map(|_| ()),
            "recipient" => sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT", id)
                .fetch_one(&mut *writer)
                .await
                .map(|_| ()),
            _ => sqlx::query!("SELECT id FROM projects WHERE id=$1 FOR UPDATE NOWAIT", id)
                .fetch_one(&mut *writer)
                .await
                .map(|_| ()),
        };
        assert!(
            matches!(result, Err(sqlx::Error::Database(error)) if error.code().as_deref()==Some("55P03")),
            "{resource}"
        );
        writer.rollback().await.unwrap();
    }
    outbox.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !PathBuf::from(format!("{}.started", stub.program.display())).exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut writer = tokio::time::timeout(Duration::from_secs(5), reader.begin())
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
        ids.user_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM projects WHERE id=$1 FOR UPDATE NOWAIT",
        ids.project_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM horae_outbox WHERE id=$1 FOR UPDATE NOWAIT",
        event.id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    writer.rollback().await.unwrap();
    release.write_all(b"resume\n").unwrap();
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT delivered_at IS NOT NULL FROM horae_outbox WHERE id=$1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(true)
    );
    reader.close().await;
}
