use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use super::*;
use crate::config::MailConfig;
use uuid::Uuid;

struct Stub {
    _directory: tempfile::TempDir,
    program: PathBuf,
    config: MailConfig,
}

impl Stub {
    fn new(body: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("sendmail stub");
        let shell = std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .map(|directory| directory.join("sh"))
            .find(|path| path.is_file())
            .unwrap();
        std::fs::write(&program, format!("#!{}\n{body}\n", shell.display())).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config =
            MailConfig::parse(Some(program.to_str().unwrap()), Some("alerts@example.test"))
                .unwrap()
                .unwrap();
        Self {
            _directory: directory,
            program,
            config,
        }
    }

    fn captured(&self, suffix: &str) -> String {
        std::fs::read_to_string(format!("{}{suffix}", self.program.display())).unwrap()
    }
}

fn message() -> BudgetMessage {
    BudgetMessage {
        id: Uuid::now_v7(),
        created_at: "2026-09-21T12:00:00Z".parse().unwrap(),
        project_name: "Project\r\nBcc: stranger@example.test".into(),
        scope: "project".into(),
        period: "2026-09".into(),
        threshold: 80,
    }
}

#[test]
fn mail_configuration_is_disabled_by_default_and_rejects_partial_values() {
    assert!(MailConfig::parse(None, None).unwrap().is_none());
    assert!(MailConfig::parse(Some("/no/such/sendmail"), None).is_err());
    assert!(MailConfig::parse(None, Some("alerts@example.test")).is_err());
    assert!(MailConfig::parse(Some("sendmail -t"), Some("alerts@example.test")).is_err());
}

#[test]
fn mail_configuration_requires_an_executable_file_and_safe_sender() {
    let stub = Stub::new("exit 0");
    assert!(
        MailConfig::parse(
            Some(stub.program.to_str().unwrap()),
            Some("a@example.test\nBcc: b@example.test")
        )
        .is_err()
    );
    std::fs::set_permissions(&stub.program, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        MailConfig::parse(Some(stub.program.to_str().unwrap()), Some("a@example.test")).is_err()
    );
}

#[test]
fn rendered_mail_bounds_body_lines_for_long_unicode_and_control_names() {
    let stub = Stub::new("exit 0");
    let mut message = message();
    message.project_name = "\u{1}é".repeat(200);
    let text = render(&stub.config, "person@example.test", &message).unwrap();
    assert!(text.split("\r\n").all(|line| line.len() <= 998));
}

#[tokio::test]
async fn sendmail_receives_fixed_arguments_and_stable_message_identity() {
    let stub = Stub::new("printf '%s\\n' \"$@\" > \"$0.args\"\ncat > \"$0.message\"");
    let message = message();
    send(
        &stub.config,
        "person+budget@example.test",
        &message,
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    assert_eq!(
        stub.captured(".args"),
        "-i\n-f\nalerts@example.test\n--\nperson+budget@example.test\n"
    );
    let first = stub.captured(".message");
    assert!(first.contains(&format!(
        "Message-ID: <budget-{}@horae.invalid>\r\n",
        message.id
    )));
    let (headers, body) = first.split_once("\r\n\r\n").unwrap();
    assert!(!headers.contains("Bcc:"));
    assert!(body.contains("80%"));
    send(
        &stub.config,
        "person+budget@example.test",
        &message,
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    assert_eq!(stub.captured(".message"), first);
}

#[tokio::test]
async fn recipient_injection_is_rejected_before_starting_a_process() {
    let stub = Stub::new("touch \"$0.called\"");
    for recipient in [
        "-X/tmp/leak",
        "a@example.test\r\nBcc: b@example.test",
        "a@example.test,b@example.test",
        "|program",
        "a@b\0",
    ] {
        assert_eq!(
            send(&stub.config, recipient, &message(), Duration::from_secs(5)).await,
            Err(MailError::InvalidAddress)
        );
    }
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
}

#[tokio::test]
async fn sendmail_failure_does_not_expose_program_output() {
    let stub = Stub::new("cat > /dev/null\nprintf 'private detail' >&2\nexit 75");
    assert_eq!(
        send(
            &stub.config,
            "person@example.test",
            &message(),
            Duration::from_secs(5)
        )
        .await,
        Err(MailError::Rejected)
    );
}

#[tokio::test]
async fn sendmail_timeout_kills_and_reaps_the_child() {
    let stub = Stub::new("printf '%s' \"$$\" > \"$0.pid\"\nwhile :; do :; done");
    assert_eq!(
        send(
            &stub.config,
            "person@example.test",
            &message(),
            Duration::from_secs(1)
        )
        .await,
        Err(MailError::Timeout)
    );
    let pid = stub.captured(".pid");
    assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
}

#[tokio::test]
async fn cancelling_delivery_kills_the_active_sender() {
    let stub = Stub::new("printf '%s' \"$$\" > \"$0.pid\"\nwhile :; do :; done");
    let config = stub.config.clone();
    let task = tokio::spawn(async move {
        send(
            &config,
            "person@example.test",
            &message(),
            Duration::from_secs(30),
        )
        .await
    });
    let pid = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(format!("{}.pid", stub.program.display()))
                && let Ok(pid) = value.parse::<u32>()
            {
                break pid;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(5), async {
        while PathBuf::from(format!("/proc/{pid}")).exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn verbose_sender_output_is_discarded_without_blocking_delivery() {
    let stub = Stub::new(
        "cat > /dev/null\ni=0\nwhile test \"$i\" -lt 10000; do printf 'private stdout detail\\n'; printf 'private stderr detail\\n' >&2; i=$((i + 1)); done",
    );
    send(
        &stub.config,
        "person@example.test",
        &message(),
        Duration::from_secs(5),
    )
    .await
    .unwrap();
}

async fn queued(
    pool: &sqlx::PgPool,
) -> (
    crate::server_fns::test_seed::SeedIds,
    crate::jobs::OutboxEvent,
) {
    let ids = crate::server_fns::test_seed::seed(pool, horae_core::types::OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 100 WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,alert_enabled,alert_threshold) VALUES ($1,$2,$3,$4,'project',true,0)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(pool).await.unwrap();
    crate::server_fns::budgets::enqueue_alerts(
        pool,
        ids.org_id,
        ids.project_id,
        "2026-09-21".parse().unwrap(),
    )
    .await
    .unwrap();
    let event = crate::jobs::claim_outbox(pool, "budget_email")
        .await
        .unwrap()
        .unwrap();
    (ids, event)
}

#[sqlx::test(migrations = "./migrations")]
async fn acknowledged_email_is_never_claimed_again(pool: sqlx::PgPool) {
    let stub = Stub::new("cat > \"$0.message\"");
    let (_, event) = queued(&pool).await;
    deliver(&pool, &stub.config, &event).await.unwrap();
    let row = sqlx::query!(
        "SELECT delivered_at,failed_at FROM horae_outbox WHERE id = $1",
        event.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.delivered_at.is_some());
    assert!(row.failed_at.is_none());
    assert!(
        crate::jobs::claim_outbox(&pool, "budget_email")
            .await
            .unwrap()
            .is_none()
    );
    assert!(stub.captured(".message").contains("Scope: project"));
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_email_retries_same_identity_and_eventually_stops(pool: sqlx::PgPool) {
    let stub = Stub::new("cat > \"$0.message\"\nprintf 'secret detail' >&2\nexit 75");
    let (_, mut event) = queued(&pool).await;
    let original = event.id;
    let mut first_message = None;
    for attempt in 1..=5 {
        assert_eq!(event.attempts, attempt);
        deliver(&pool, &stub.config, &event).await.unwrap();
        let message = stub.captured(".message");
        if let Some(first) = &first_message {
            assert_eq!(first, &message);
        } else {
            first_message = Some(message);
        }
        assert!(
            crate::jobs::claim_outbox(&pool, "budget_email")
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
        if attempt < 5 {
            event = crate::jobs::claim_outbox(&pool, "budget_email")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(event.id, original);
        }
    }
    let row = sqlx::query!(
        "SELECT delivered_at,failed_at,last_error FROM horae_outbox WHERE id = $1",
        original
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.delivered_at.is_none());
    assert!(row.failed_at.is_some());
    assert_eq!(
        row.last_error.as_deref(),
        Some("Budget email was not accepted by the mail transport")
    );
    assert!(
        crate::jobs::claim_outbox(&pool, "budget_email")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn revoked_recipient_does_not_receive_a_pending_email(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (ids, event) = queued(&pool).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    deliver(&pool, &stub.config, &event).await.unwrap();
    let row = sqlx::query!(
        "SELECT delivered_at,failed_at FROM horae_outbox WHERE id = $1",
        event.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.delivered_at.is_none());
    assert!(row.failed_at.is_some());
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
}

#[sqlx::test(migrations = "./migrations")]
async fn stale_email_lease_cannot_start_delivery(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (_, event) = queued(&pool).await;
    sqlx::query!(
        "UPDATE horae_outbox SET claim_token = $2 WHERE id = $1",
        event.id,
        Uuid::now_v7()
    )
    .execute(&pool)
    .await
    .unwrap();
    deliver(&pool, &stub.config, &event).await.unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_worker_delivers_only_its_kind_and_drains_cleanly(pool: sqlx::PgPool) {
    let stub = Stub::new("cat > \"$0.message\"");
    let (ids, event) = queued(&pool).await;
    sqlx::query!(
        "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
        event.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let other = crate::jobs::enqueue_outbox(
        &mut tx,
        ids.org_id,
        "project_created",
        serde_json::json!({}),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let state = crate::state::AppState::new(
        pool.clone(),
        std::sync::Arc::new(crate::plugin::PluginRegistry::empty()),
    )
    .with_mail(Some(stub.config.clone()));
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
    assert_eq!(
        sqlx::query_scalar!("SELECT attempts FROM horae_outbox WHERE id = $1", other)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn disabled_mail_worker_does_not_claim_pending_notifications(pool: sqlx::PgPool) {
    let (_, event) = queued(&pool).await;
    sqlx::query!(
        "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
        event.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = crate::state::AppState::new(
        pool.clone(),
        std::sync::Arc::new(crate::plugin::PluginRegistry::empty()),
    );
    let worker = spawn(&state);
    let stop = worker.stop_sender();
    tokio::time::timeout(Duration::from_secs(5), stop.closed())
        .await
        .unwrap();
    worker.shutdown(Duration::from_secs(5)).await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT attempts FROM horae_outbox WHERE id = $1", event.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn foreign_notification_payload_is_terminal_and_never_sent(pool: sqlx::PgPool) {
    let stub = Stub::new("touch \"$0.called\"\ncat > /dev/null");
    let (_, event) = queued(&pool).await;
    let (_, foreign) = queued(&pool).await;
    sqlx::query!(
        "UPDATE horae_outbox SET payload = $2 WHERE id = $1",
        event.id,
        foreign.payload
    )
    .execute(&pool)
    .await
    .unwrap();
    deliver(&pool, &stub.config, &event).await.unwrap();
    assert!(!PathBuf::from(format!("{}.called", stub.program.display())).exists());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT failed_at IS NOT NULL FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn retry_can_recover_without_changing_message_identity(pool: sqlx::PgPool) {
    let stub = Stub::new(
        "cat > \"$0.message\"\nif test ! -f \"$0.retry\"; then touch \"$0.retry\"; exit 75; fi",
    );
    let (_, event) = queued(&pool).await;
    deliver(&pool, &stub.config, &event).await.unwrap();
    let first = stub.captured(".message");
    sqlx::query!(
        "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
        event.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let retry = crate::jobs::claim_outbox(&pool, "budget_email")
        .await
        .unwrap()
        .unwrap();
    deliver(&pool, &stub.config, &retry).await.unwrap();
    assert_eq!(stub.captured(".message"), first);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT delivered_at IS NOT NULL FROM horae_outbox WHERE id = $1",
            event.id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(true)
    );
}
