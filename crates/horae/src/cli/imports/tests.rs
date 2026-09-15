use super::*;
use clap::Parser;

fn session(origin: &str, id: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut file, &json!({"server_origin":origin,"session_id":id})).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn private_session_accepts_https_and_loopback_only() {
    for origin in [
        "https://horae.example",
        "http://127.0.0.1:8080",
        "http://[::1]:8080",
    ] {
        let file = session(origin, "test_session");
        assert!(transport::Transport::load(file.path()).is_ok(), "{origin}");
    }
}

#[test]
fn session_rejects_plaintext_remote_and_credential_bearing_urls() {
    for origin in [
        "http://horae.example",
        "https://user:secret@horae.example",
        "https://horae.example/path",
        "https://horae.example?token=secret",
        "https://horae.example#secret",
        "file:///tmp/session",
    ] {
        let file = session(origin, "test_session");
        let error = transport::Transport::load(file.path()).err().unwrap();
        assert!(!error.message.contains("secret"));
        assert_eq!(error.code, 2);
    }
}

#[test]
fn session_rejects_cookie_header_injection_without_echoing_the_value() {
    for value in [
        "",
        "secret; other=cookie",
        "secret\r\nHeader: value",
        "secret value",
    ] {
        let file = session("https://horae.example", value);
        let error = transport::Transport::load(file.path()).err().unwrap();
        assert!(!error.message.contains("secret"));
        assert_eq!(error.code, 2);
    }
}

#[cfg(unix)]
#[test]
fn session_rejects_public_permissions_and_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let file = session("https://horae.example", "test_session");
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(transport::Transport::load(file.path()).is_err());
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("session-link");
    symlink(file.path(), &link).unwrap();
    assert!(transport::Transport::load(&link).is_err());
}

#[tokio::test]
async fn missing_session_is_a_structured_configuration_failure() {
    let cli = crate::cli::Cli::try_parse_from(["horae", "jobs", "list", "--json"]).unwrap();
    let result = run_async(
        &cli.command(),
        &RemoteOptions {
            session_file: None,
            json: true,
        },
    )
    .await;
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["error"]["category"], "configuration");
    assert_eq!(result.code, 2);
    assert!(value.get("code").is_none());
}

#[tokio::test]
async fn redirects_are_not_followed_or_exposed_as_server_error_text() {
    use axum::{Router, response::Redirect, routing::post};
    let router = Router::new().route(
        "/api/import/harvest/history",
        post(|| async { Redirect::temporary("https://example.invalid/secret") }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let file = session(
        &format!("http://{}", listener.local_addr().unwrap()),
        "test_session",
    );
    let server = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
    let client = transport::Transport::load(file.path()).unwrap();
    let error = client
        .post("/api/import/harvest/history", &json!({}), None)
        .await
        .unwrap_err();
    server.abort();
    assert_eq!(error.http_status, Some(307));
    assert!(!error.message.contains("secret"));
}

struct MockServer {
    session: tempfile::NamedTempFile,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl MockServer {
    async fn start(router: axum::Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let session = session(
            &format!("http://{}", listener.local_addr().unwrap()),
            "test_session",
        );
        let task = tokio::spawn(async { axum::serve(listener, router).await.unwrap() });
        Self { session, task }
    }

    async fn run(&self, args: &[&str]) -> CommandResult {
        let cli =
            crate::cli::Cli::try_parse_from(std::iter::once("horae").chain(args.iter().copied()))
                .unwrap();
        run_async(
            &cli.command(),
            &RemoteOptions {
                session_file: Some(self.session.path().into()),
                json: true,
            },
        )
        .await
    }
}

fn job(status: &str) -> JobStatus {
    JobStatus {
        id: Uuid::now_v7(),
        kind: "harvest_csv_import".into(),
        status: status.into(),
        phase: None,
        processed_count: 0,
        total_count: None,
        report: Some(json!(ImportReport::new(
            horae_core::importers::harvest::types::SourceKind::Csv,
            ImportMode::DryRun
        ))),
        last_error: None,
        created_at: chrono::Utc::now(),
        finished_at: None,
    }
}

#[tokio::test]
async fn cancellation_racing_completion_does_not_claim_cancellation_was_requested() {
    let job = job("succeeded");
    let id = job.id.to_string();
    let server = MockServer::start(axum::Router::new().route(
        "/api/import/harvest/cancel",
        axum::routing::post(move || {
            let job = job.clone();
            async move { axum::Json(job) }
        }),
    ))
    .await;
    let result = server.run(&["jobs", "cancel", &id]).await;
    assert_eq!(result.outcome, "already_complete");
    assert_eq!(result.code, 0);
}

#[tokio::test]
async fn status_and_wait_distinguish_failed_jobs_from_failed_reads() {
    for (state, code) in [("succeeded", 0), ("failed", 3), ("cancelled", 4)] {
        let job = job(state);
        let id = job.id.to_string();
        let server = MockServer::start(axum::Router::new().route(
            "/api/import/harvest/status",
            axum::routing::post(move || {
                let job = job.clone();
                async move { axum::Json(job) }
            }),
        ))
        .await;
        let read = server.run(&["jobs", "status", &id]).await;
        assert_eq!(read.code, 0);
        let waited = server.run(&["jobs", "wait", &id, "--timeout", "5"]).await;
        assert_eq!(waited.code, code);
        assert_eq!(waited.outcome, state);
    }
}

#[tokio::test]
async fn wait_deadline_caps_a_pending_request_without_cancelling_the_job() {
    let server = MockServer::start(axum::Router::new().route(
        "/api/import/harvest/status",
        axum::routing::post(|| async { std::future::pending::<axum::Json<Value>>().await }),
    ))
    .await;
    let id = Uuid::now_v7();
    let start = std::time::Instant::now();
    let result = server
        .run(&["jobs", "wait", &id.to_string(), "--timeout", "1"])
        .await;
    assert_eq!(result.code, 5);
    assert_eq!(result.job_id, Some(id));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn report_marks_confirmed_failed_outcomes_as_incomplete() {
    let job = job("failed");
    let id = job.id.to_string();
    let server = MockServer::start(axum::Router::new().route(
        "/api/import/harvest/status",
        axum::routing::post(move || {
            let job = job.clone();
            async move { axum::Json(job) }
        }),
    ))
    .await;
    let result = server.run(&["jobs", "report", &id]).await;
    assert_eq!(result.data.as_ref().unwrap()["complete"], false);
    assert_eq!(result.code, 0);
}

#[tokio::test]
async fn archived_errors_publish_without_clobbering_and_preserve_the_prior_file_on_failure() {
    use axum::{body::Body, http::header::CONTENT_TYPE, response::IntoResponse, routing::get};
    let archive = "{\"reason\":\"test\"}\n".repeat(100_000);
    let expected = archive.clone();
    let server = MockServer::start(
        axum::Router::new()
            .route(
                "/archive",
                get(move || {
                    let archive = archive.clone();
                    async move { ([(CONTENT_TYPE, "application/x-ndjson")], archive) }
                }),
            )
            .route(
                "/broken",
                get(|| async {
                    let stream = futures_util::stream::iter([
                        Ok::<_, std::io::Error>(vec![b'x'; 1024]),
                        Err(std::io::Error::other("test truncated archive")),
                    ]);
                    (
                        [(CONTENT_TYPE, "application/x-ndjson")],
                        Body::from_stream(stream),
                    )
                        .into_response()
                }),
            ),
    )
    .await;
    let client = transport::Transport::load(server.session.path()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("errors.jsonl");
    assert_eq!(
        client.download("/archive", &output, false).await.unwrap(),
        expected.len() as u64
    );
    assert_eq!(std::fs::read_to_string(&output).unwrap(), expected);
    assert!(client.download("/archive", &output, false).await.is_err());
    assert!(client.download("/broken", &output, true).await.is_err());
    assert_eq!(std::fs::read_to_string(&output).unwrap(), expected);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn malformed_submission_acknowledgement_retains_its_request_identity() {
    let server = MockServer::start(axum::Router::new().route(
        "/api/import/harvest/start",
        axum::routing::post(|| async { "not json" }),
    ))
    .await;
    let id = Uuid::now_v7();
    let result = server
        .run(&["import", "harvest-api", "--request-id", &id.to_string()])
        .await;
    assert_eq!(result.code, 6);
    assert_eq!(result.request_id, Some(id));
}

#[tokio::test]
async fn ordinary_json_is_bounded_and_invalid_replies_are_not_echoed() {
    for body in [
        "secret-invalid-json".to_owned(),
        " ".repeat(4 * 1024 * 1024 + 1),
    ] {
        let server = MockServer::start(axum::Router::new().route(
            "/api/import/harvest/history",
            axum::routing::post(move || {
                let body = body.clone();
                async move { body }
            }),
        ))
        .await;
        let result = server.run(&["jobs", "list"]).await;
        assert_eq!(result.code, 1);
        assert_eq!(result.error.as_ref().unwrap().category, "protocol");
        assert!(!serde_json::to_string(&result).unwrap().contains("secret"));
    }
}

#[tokio::test]
async fn missing_and_mismatched_jobs_fail_without_substitution() {
    for value in [
        Value::Null,
        json!(job("running")),
        json!({"secret":"invalid"}),
    ] {
        let server = MockServer::start(axum::Router::new().route(
            "/api/import/harvest/status",
            axum::routing::post(move || {
                let value = value.clone();
                async move { axum::Json(value) }
            }),
        ))
        .await;
        let id = Uuid::now_v7();
        let result = server.run(&["jobs", "status", &id.to_string()]).await;
        assert_eq!(result.code, 1);
        assert_eq!(result.job_id, Some(id));
        assert!(result.data.is_none());
    }
}

#[tokio::test]
async fn invalid_csv_files_fail_before_contacting_the_server() {
    let server = MockServer::start(axum::Router::new()).await;
    let directory = tempfile::tempdir().unwrap();
    let empty = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    let oversized = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
    oversized
        .as_file()
        .set_len(transport::MAX_UPLOAD + 1)
        .unwrap();
    let missing = directory.path().join("missing.csv");
    for file in [empty.path(), oversized.path(), &missing, directory.path()] {
        let result = server
            .run(&["import", "harvest-csv", file.to_str().unwrap()])
            .await;
        assert_eq!(result.code, 2);
        assert_eq!(result.error.unwrap().category, "configuration");
        assert!(result.job_id.is_none());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn csv_named_pipe_is_rejected_without_waiting_for_a_writer() {
    let server = MockServer::start(axum::Router::new()).await;
    let directory = tempfile::tempdir().unwrap();
    let fifo = directory.path().join("source.csv");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        server.run(&["import", "harvest-csv", fifo.to_str().unwrap()]),
    )
    .await
    .unwrap();
    assert_eq!(result.code, 2);
}
