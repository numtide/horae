//! CLI parser and transport against the real registered/session-checked routes.

use std::io::Write;

use super::*;

async fn call(api: &Api, cookie: &str, args: &[&str]) -> (u8, Value) {
    let mut session = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(
        &mut session,
        &json!({
            "server_origin": api.base,
            "session_id": cookie.strip_prefix("id=").unwrap(),
        }),
    )
    .unwrap();
    session.flush().unwrap();
    if let Some(binary) = std::env::var_os("HORAE_CLI_TEST_BINARY") {
        let output = tokio::process::Command::new(binary)
            .args(args)
            .arg("--json")
            .arg("--session-file")
            .arg(session.path())
            .env("DATABASE_URL", "invalid-client-database-url")
            .env("HORAE_JOB_MAX_ATTEMPTS", "invalid-client-job-policy")
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        let secret = cookie.strip_prefix("id=").unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
        return (
            output.status.code().unwrap().try_into().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        );
    }
    crate::cli::imports::test_run(args, session.path()).await
}

pub(super) async fn exercise(
    api: &Api,
    pool: &PgPool,
    owner: &crate::server_fns::test_seed::SeedIds,
    admin: &str,
    rejected: &[&str],
    outsider: &str,
    archive: (Uuid, &[u8]),
) {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("source.csv");
    let csv = format!(
        "Date,Client,Project,Task,Hours,Email,Notes\n2026-09-14,Acme,Widget,Dev,1,{}@test.com,cli-acceptance\n",
        owner.user_id
    );
    std::fs::write(&csv_path, &csv).unwrap();
    let output = dir.path().join("errors.jsonl");
    let output_str = output.to_str().unwrap();
    let csv_str = csv_path.to_str().unwrap();
    let archived_id = archive.0.to_string();

    for cookie in rejected {
        for args in [
            vec!["import", "harvest-api", "--dry-run"],
            vec!["import", "harvest-csv", csv_str, "--dry-run"],
            vec!["jobs", "list"],
            vec!["jobs", "status", &archived_id],
            vec!["jobs", "report", &archived_id],
            vec!["jobs", "errors", &archived_id, "--output", output_str],
            vec!["jobs", "cancel", &archived_id],
            vec!["jobs", "retry", &archived_id],
            vec!["jobs", "wait", &archived_id, "--timeout", "1"],
        ] {
            let (code, result) = call(api, cookie, &args).await;
            assert_eq!(code, 1, "{args:?}: {result}");
            assert!(matches!(
                result["error"]["http_status"].as_u64(),
                Some(401 | 403)
            ));
            assert!(
                !result
                    .to_string()
                    .contains(cookie.strip_prefix("id=").unwrap())
            );
        }
    }
    for command in ["status", "report", "cancel", "retry", "wait"] {
        let (code, result) = call(api, outsider, &["jobs", command, &archived_id]).await;
        assert_eq!(code, 1, "{command}: {result}");
        assert!(result.get("data").is_none());
    }
    let (code, history) = call(api, outsider, &["jobs", "list"]).await;
    assert_eq!(code, 0);
    assert_eq!(history["data"], json!([]));
    let (code, _) = call(
        api,
        outsider,
        &["jobs", "errors", &archived_id, "--output", output_str],
    )
    .await;
    assert_eq!(code, 1);
    assert!(!output.exists());

    let (code, downloaded) = call(
        api,
        admin,
        &["jobs", "errors", &archived_id, "--output", output_str],
    )
    .await;
    assert_eq!(code, 0, "{downloaded}");
    assert_eq!(std::fs::read(&output).unwrap(), archive.1);
    let (code, report) = call(api, admin, &["jobs", "report", &archived_id]).await;
    assert_eq!(code, 0);
    assert_eq!(report["data"]["complete"], true);
    let (code, waited) = call(api, admin, &["jobs", "wait", &archived_id]).await;
    assert_eq!(code, 0);
    assert_eq!(waited["outcome"], "partial_success");

    for mode in ["preview", "commit"] {
        let key = Uuid::now_v7().to_string();
        let mut args = vec!["import", "harvest-api", "--full", "--request-id", &key];
        if mode == "preview" {
            args.push("--dry-run");
        }
        let (code, started) = call(api, admin, &args).await;
        assert_eq!(code, 0, "{started}");
        let id = started["job_id"].as_str().unwrap();
        let (code, replay) = call(api, admin, &args).await;
        assert_eq!(code, 0);
        assert_eq!(replay["job_id"], started["job_id"]);
        let (code, conflict) = call(
            api,
            admin,
            &[
                "import",
                "harvest-api",
                "--incremental",
                "--request-id",
                &key,
            ],
        )
        .await;
        assert_eq!(code, 1);
        assert_eq!(conflict["error"]["http_status"], 409);
        let (code, cancelled) = call(api, admin, &["jobs", "cancel", id]).await;
        assert_eq!(code, 0);
        assert_eq!(cancelled["data"]["status"], "cancelled");
        assert_eq!(call(api, admin, &["jobs", "retry", id]).await.0, 0);
        let (code, timeout) = call(api, admin, &["jobs", "wait", id, "--timeout", "1"]).await;
        assert_eq!(code, 5);
        assert_eq!(timeout["job_id"], id);
        assert_eq!(call(api, admin, &["jobs", "cancel", id]).await.0, 0);
    }

    let state = crate::state::global_state().await;
    for (mode, expected_entries) in [
        (ImportMode::DryRun, 0i64),
        (ImportMode::Commit, 1),
        (ImportMode::Commit, 1),
    ] {
        let mut args = vec!["import", "harvest-api", "--full"];
        if mode == ImportMode::DryRun {
            args.push("--dry-run");
        }
        let (code, started) = call(api, admin, &args).await;
        assert_eq!(code, 0, "{started}");
        let id = started["job_id"].as_str().unwrap();
        // The submitting client has exited before the server claims the job.
        crate::importers::harvest::complete_cli_api_fixture(
            pool,
            owner.org_id,
            Uuid::parse_str(id).unwrap(),
            format!("{}@test.com", owner.user_id),
            state.harvest.as_ref().unwrap(),
            mode,
        )
        .await;
        let (code, completed) = call(api, admin, &["jobs", "wait", id, "--timeout", "5"]).await;
        assert_eq!(code, 0, "{completed}");
        assert_eq!(completed["outcome"], "succeeded");
        let entries = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM time_entries WHERE org_id = $1 AND notes = 'cli-api-acceptance'",
            owner.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap();
        assert_eq!(entries, expected_entries);
        let web = api
            .json("get_harvest_import_job", json!({"job_id":id}), admin)
            .await;
        assert_eq!(completed["data"], web);
    }
    for (preview, expected_entries) in [(true, 0i64), (false, 1), (false, 1)] {
        let key = Uuid::now_v7().to_string();
        let mut args = vec!["import", "harvest-csv", csv_str, "--request-id", &key];
        if preview {
            args.push("--dry-run");
        }
        let (code, started) = call(api, admin, &args).await;
        assert_eq!(code, 0, "{started}");
        let id = started["job_id"].as_str().unwrap();
        std::fs::remove_file(&csv_path).unwrap();
        // Start a fresh worker only after acknowledgement, client exit and
        // removal of the local file; the database owns the accepted input.
        let worker = crate::jobs::spawn(state);
        let (code, completed) = call(api, admin, &["jobs", "wait", id, "--timeout", "30"]).await;
        assert_eq!(code, 0, "{completed}");
        assert_eq!(completed["data"]["status"], "succeeded");
        worker.shutdown(Duration::from_secs(5)).await.unwrap();
        let entries = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM time_entries WHERE org_id = $1 AND notes = 'cli-acceptance'",
            owner.org_id
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap();
        assert_eq!(entries, expected_entries);
        std::fs::write(&csv_path, &csv).unwrap();
        let (code, replay) = call(api, admin, &args).await;
        assert_eq!(code, 0);
        assert_eq!(replay["job_id"], completed["job_id"]);
    }

    let (code, first) = call(api, admin, &["jobs", "list", "--limit", "1"]).await;
    assert_eq!(code, 0);
    let before = first["data"][0]["id"].as_str().unwrap();
    let (code, next) = call(
        api,
        admin,
        &["jobs", "list", "--limit", "1", "--before", before],
    )
    .await;
    assert_eq!(code, 0);
    assert_ne!(first["data"][0]["id"], next["data"][0]["id"]);
}
