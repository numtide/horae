#![cfg(feature = "server")]

use std::{io::Write, path::Path, time::Duration};

use dioxus::prelude::dioxus_fullstack::reqwest;
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool};
use tokio::process::{Child, Command};

fn binary(database: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_horae"));
    command
        .env("DATABASE_URL", database)
        .env("DEV_LOGIN", "1")
        .env("HORAE_SECURE_COOKIES", "0")
        .kill_on_drop(true);
    command
}

async fn start(database: &str, port: u16, client: &reqwest::Client, assets: &Path) -> Child {
    let mut child = binary(database)
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .env("DIOXUS_PUBLIC_PATH", assets)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut poll = tokio::time::interval(Duration::from_millis(100));
        loop {
            poll.tick().await;
            assert!(
                child.try_wait().unwrap().is_none(),
                "server exited before readiness"
            );
            if client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    child
}

async fn cli(session: &Path, args: &[&str]) -> Value {
    let output = binary("not-a-client-database-url")
        .args(args)
        .arg("--json")
        .arg("--session-file")
        .arg(session)
        .output()
        .await
        .unwrap();
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{args:?}: {result}");
    result
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detached_csv_recovers_after_the_server_process_is_killed(pool: PgPool) {
    let database = pool.connect_options().to_url_lossy().to_string();
    let initialized = binary(&database)
        .args([
            "init",
            "--org-name",
            "CLI restart",
            "--admin-email",
            "restart@test.com",
            "--admin-name",
            "Restart Admin",
        ])
        .output()
        .await
        .unwrap();
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let assets = tempfile::tempdir().unwrap();
    let mut server = start(&database, port, &client, assets.path()).await;
    let origin = format!("http://127.0.0.1:{port}");
    let login = client
        .post(format!("{origin}/auth/dev-login"))
        .send()
        .await
        .unwrap();
    assert!(login.status().is_redirection());
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .strip_prefix("id=")
        .unwrap();
    let mut session = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(
        &mut session,
        &json!({"server_origin":origin,"session_id":cookie}),
    )
    .unwrap();
    session.flush().unwrap();
    let mut source = tempfile::NamedTempFile::new().unwrap();
    writeln!(source, "Date,Client,Project,Task,Hours,Email,Notes\n2026-09-14,Client,Project,Task,1,restart@test.com,cli-restart-acceptance").unwrap();
    source.flush().unwrap();

    // Block domain writes, not enqueue/status: the acknowledged job cannot
    // finish before the deliberate server crash.
    let mut barrier = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let started = cli(
        session.path(),
        &["import", "harvest-csv", source.path().to_str().unwrap()],
    )
    .await;
    let id = started["job_id"].as_str().unwrap();
    let observed = cli(session.path(), &["jobs", "status", id]).await;
    assert!(matches!(
        observed["data"]["status"].as_str(),
        Some("queued" | "running")
    ));
    server.kill().await.unwrap();
    source.close().unwrap();
    barrier.rollback().await.unwrap();
    // Advance only the dead claim's deadline instead of waiting five minutes.
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
        uuid::Uuid::parse_str(id).unwrap()
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut restarted = start(&database, port, &client, assets.path()).await;
    let completed = cli(session.path(), &["jobs", "wait", id, "--timeout", "20"]).await;
    assert_eq!(completed["job_id"], started["job_id"]);
    assert_eq!(completed["outcome"], "succeeded");
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM time_entries WHERE notes = 'cli-restart-acceptance'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
    restarted.kill().await.unwrap();
}
