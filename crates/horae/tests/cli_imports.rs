#![cfg(feature = "server")]

use serde_json::{Value, json};
use std::io::Write;
use std::process::Command;

fn session_file(origin: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(
        &mut file,
        &json!({"server_origin":origin,"session_id":"test_session"}),
    )
    .unwrap();
    file.flush().unwrap();
    file
}

fn job(id: uuid::Uuid, status: &str) -> Value {
    use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind};
    json!({
        "id":id,"kind":"harvest_csv_import","status":status,"phase":null,
        "processed_count":0,"total_count":null,
        "report":ImportReport::new(SourceKind::Csv, ImportMode::DryRun),
        "last_error":null,"created_at":chrono::Utc::now(),"finished_at":null,
    })
}

async fn run(session: &std::path::Path, args: &[&str]) -> std::process::Output {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_horae"))
        .args(args)
        .arg("--json")
        .arg("--session-file")
        .arg(session)
        .env("DATABASE_URL", "not-a-database-url")
        .env("HORAE_JOB_MAX_ATTEMPTS", "not-a-number")
        .kill_on_drop(true)
        .output()
        .await
        .unwrap()
}

#[test]
fn remote_configuration_errors_are_json_and_do_not_load_server_configuration() {
    let output = Command::new(env!("CARGO_BIN_EXE_horae"))
        .args([
            "jobs",
            "list",
            "--json",
            "--session-file",
            "/missing-horae-cli-session.json",
        ])
        .env("HORAE_JOB_MAX_ATTEMPTS", "not-a-number")
        .env("DATABASE_URL", "not-a-database-url")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("expected JSON: {error}; stderr={:?}", output.stderr));
    assert_eq!(result["version"], 1);
    assert_eq!(result["error"]["category"], "configuration");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("HORAE_JOB_MAX_ATTEMPTS"));
}

#[test]
fn invalid_remote_arguments_have_a_machine_readable_exit() {
    let output = Command::new(env!("CARGO_BIN_EXE_horae"))
        .args(["jobs", "list", "--limit", "101", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["category"], "arguments");
}

#[tokio::test]
async fn executable_commands_use_the_documented_authenticated_wire_contract() {
    use axum::{Router, extract::Request, response::IntoResponse};
    let id = uuid::Uuid::now_v7();
    let observed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let calls = observed.clone();
    let router = Router::new().fallback(move |request: Request| {
        let calls = calls.clone();
        async move {
            assert_eq!(request.headers()["cookie"], "id=test_session");
            let path = request.uri().path().to_owned();
            let key = request.headers().get("X-Horae-Idempotency-Key").cloned();
            let csv_header = request.headers().get("X-Horae-Import").cloned();
            let body = axum::body::to_bytes(request.into_body(), 1024 * 1024)
                .await
                .unwrap();
            calls.lock().unwrap().push(path.clone());
            if path == "/api/import/harvest/start" {
                assert_eq!(
                    uuid::Uuid::parse_str(key.unwrap().to_str().unwrap())
                        .unwrap()
                        .get_version_num(),
                    7
                );
                let body: Value = serde_json::from_slice(&body).unwrap();
                assert!(matches!(
                    body["sync"].as_str(),
                    Some("Full" | "Incremental")
                ));
                return axum::Json(job(id, "succeeded")).into_response();
            }
            if path.starts_with("/api/import/harvest/csv-job/") {
                assert_eq!(csv_header.unwrap(), "csv");
                assert!(key.is_some());
                assert!(body.starts_with(b"Date,Client,Project,Task,Hours,Email"));
                return axum::Json(job(id, "succeeded")).into_response();
            }
            if path.ends_with("/errors") {
                return (
                    [("content-type", "application/x-ndjson")],
                    "{\"reason\":\"fixture\"}\n",
                )
                    .into_response();
            }
            let body: Value = serde_json::from_slice(&body).unwrap();
            if path == "/api/import/harvest/history" {
                assert_eq!(body["limit"], 1);
                return axum::Json(json!([job(id, "succeeded")])).into_response();
            }
            assert_eq!(body["job_id"], json!(id));
            match path.as_str() {
                "/api/import/harvest/status" | "/api/import/harvest/retry" => {
                    axum::Json(job(id, "succeeded")).into_response()
                }
                "/api/import/harvest/cancel" => axum::Json(job(id, "cancelled")).into_response(),
                _ => axum::http::StatusCode::NOT_FOUND.into_response(),
            }
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let session = session_file(&format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
    let directory = tempfile::tempdir().unwrap();
    let csv = directory.path().join("source.csv");
    std::fs::write(&csv, "Date,Client,Project,Task,Hours,Email\n").unwrap();
    let archive = directory.path().join("errors.jsonl");
    let id_string = id.to_string();
    for args in [
        vec!["import", "harvest-api"],
        vec![
            "import",
            "harvest-api",
            "--dry-run",
            "--full",
            "--wait",
            "--timeout",
            "5",
        ],
        vec!["import", "harvest-csv", csv.to_str().unwrap()],
        vec![
            "import",
            "harvest-csv",
            csv.to_str().unwrap(),
            "--dry-run",
            "--wait",
            "--timeout",
            "5",
        ],
        vec!["jobs", "status", &id_string],
        vec!["jobs", "list", "--limit", "1"],
        vec!["jobs", "report", &id_string],
        vec![
            "jobs",
            "errors",
            &id_string,
            "--output",
            archive.to_str().unwrap(),
        ],
        vec!["jobs", "cancel", &id_string],
        vec!["jobs", "retry", &id_string, "--wait", "--timeout", "5"],
        vec!["jobs", "wait", &id_string, "--timeout", "5"],
    ] {
        let output = run(session.path(), &args).await;
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{args:?}: {result}");
        assert_eq!(result["version"], 1);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("test_session"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("test_session"));
    }
    assert_eq!(
        std::fs::read_to_string(&archive).unwrap(),
        "{\"reason\":\"fixture\"}\n"
    );
    assert!(observed.lock().unwrap().len() >= 11);
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn interrupting_the_executable_waiter_keeps_job_identity_and_never_sends_cancel() {
    use axum::{Router, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let id = uuid::Uuid::now_v7();
    let cancelled = Arc::new(AtomicUsize::new(0));
    let cancel_calls = cancelled.clone();
    let (sender, mut received) = tokio::sync::mpsc::unbounded_channel();
    let router = Router::new()
        .route(
            "/api/import/harvest/status",
            post(move || {
                let sender = sender.clone();
                async move {
                    sender.send(()).unwrap();
                    axum::Json(job(id, "running"))
                }
            }),
        )
        .route(
            "/api/import/harvest/cancel",
            post(move || {
                let cancel_calls = cancel_calls.clone();
                async move {
                    cancel_calls.fetch_add(1, Ordering::SeqCst);
                    axum::Json(job(id, "cancelled"))
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let session = session_file(&format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_horae"))
        .args(["jobs", "wait", &id.to_string(), "--json", "--timeout", "30"])
        .arg("--session-file")
        .arg(session.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        received.recv().await.unwrap();
        received.recv().await.unwrap();
    })
    .await
    .unwrap();
    let killed = Command::new("sh")
        .args([
            "-c",
            "kill -INT \"$1\"",
            "horae-cli-test",
            &child.id().unwrap().to_string(),
        ])
        .status()
        .unwrap();
    assert!(killed.success());
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.status.code(), Some(130));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["job_id"], json!(id));
    assert_eq!(result["error"]["category"], "interrupted");
    let observed = run(session.path(), &["jobs", "status", &id.to_string()]).await;
    assert_eq!(observed.status.code(), Some(0));
    let observed: Value = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(observed["data"]["status"], "running");
    assert_eq!(cancelled.load(Ordering::SeqCst), 0);
    server.abort();
}

#[tokio::test]
async fn executable_wait_and_failure_exit_codes_match_the_json_contract() {
    use axum::{Router, extract::Request, response::IntoResponse};
    for (state, expected_code, expected_outcome) in [
        ("failed", 3, "failed"),
        ("cancelled", 4, "cancelled"),
        ("partial", 0, "partial_success"),
        ("running", 5, "error"),
        ("forbidden", 1, "error"),
    ] {
        let id = uuid::Uuid::now_v7();
        let router = Router::new().fallback(move |_request: Request| async move {
            if state == "forbidden" {
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    "secret-server-diagnostic",
                )
                    .into_response();
            }
            let mut value = job(
                id,
                if state == "partial" {
                    "succeeded"
                } else {
                    state
                },
            );
            if state == "partial" {
                use horae_core::importers::harvest::types::{EntityType, ImportReport, RowOutcome};
                let mut report: ImportReport =
                    serde_json::from_value(value["report"].clone()).unwrap();
                report.record(
                    EntityType::TimeEntry,
                    &RowOutcome::Errored {
                        source_location: "fixture".into(),
                        reason: "unknown user".into(),
                    },
                );
                value["report"] = json!(report);
            }
            axum::Json(value).into_response()
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let session = session_file(&format!("http://{}", listener.local_addr().unwrap()));
        let server = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
        let id_string = id.to_string();
        for args in [
            vec!["jobs", "wait", &id_string, "--timeout", "1"],
            vec!["jobs", "retry", &id_string, "--wait", "--timeout", "1"],
            vec!["import", "harvest-api", "--wait", "--timeout", "1"],
        ] {
            let output = run(session.path(), &args).await;
            let result: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                output.status.code(),
                Some(expected_code),
                "{args:?}: {result}"
            );
            assert_eq!(result["outcome"], expected_outcome);
            assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
            assert!(!String::from_utf8_lossy(&output.stderr).contains("secret"));
        }
        server.abort();
    }
}
