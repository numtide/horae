#![cfg(feature = "server")]

//! The server renders the signed-in shell: the first HTML of a page has the
//! person's rail, the rows their grants open and the admin workspace.

use std::time::Duration;

use dioxus::prelude::dioxus_fullstack::reqwest;
use sqlx::{ConnectOptions, PgPool};
use tokio::process::Command;

#[sqlx::test(migrations = "../server/migrations")]
#[serial_test::serial]
async fn the_first_html_has_the_signed_in_shell(pool: PgPool) {
    let database = pool.connect_options().to_url_lossy().to_string();
    let initialized = Command::new(env!("CARGO_BIN_EXE_horae"))
        .args([
            "init",
            "--org-name",
            "Render Org",
            "--admin-email",
            "render@test.com",
            "--admin-name",
            "Render Admin",
        ])
        .env("DATABASE_URL", &database)
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
    let assets = tempfile::tempdir().unwrap();
    let mut server = Command::new(env!("CARGO_BIN_EXE_horae"))
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .env("DATABASE_URL", &database)
        .env("DEV_LOGIN", "1")
        .env("HORAE_SECURE_COOKIES", "0")
        .env("DIOXUS_PUBLIC_PATH", assets.path())
        .stdout(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let origin = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut poll = tokio::time::interval(Duration::from_millis(100));
        loop {
            poll.tick().await;
            assert!(server.try_wait().unwrap().is_none(), "the server exited");
            if client
                .get(format!("{origin}/health"))
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

    let page = |path: &str, cookie: Option<&str>| {
        let mut request = client
            .get(format!("{origin}{path}"))
            .header("accept", "text/html");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        request.send()
    };

    // Signed out, a page load still goes to the sign-in page.
    let signed_out = page("/timesheet", None).await.unwrap();
    assert_eq!(signed_out.status(), reqwest::StatusCode::SEE_OTHER);
    assert_eq!(signed_out.headers()["location"], "/auth/login");

    let signed_in = client
        .post(format!("{origin}/auth/dev-login"))
        .send()
        .await
        .unwrap();
    let cookie = signed_in.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();

    // Twice, so a render that left the pool or the session behind would show.
    for _ in 0..2 {
        // A dated week: `/timesheet` alone redirects in the browser and renders nothing here.
        let sheet = page("/timesheet/week/2026-11-16", Some(&cookie))
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(sheet.contains("Render Admin"), "{sheet}");
        assert!(!sheet.contains("Not signed in"), "{sheet}");
        // The Administrator's grants open the gated rows.
        for row in ["People", "Clients", "Tasks", "Invoices", "Approvals"] {
            assert!(sheet.contains(row), "{row}: {sheet}");
        }
    }

    let admin = page("/admin/importers", Some(&cookie))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(admin.contains("Render Org"), "{admin}");
    assert!(admin.contains("adm-nav"), "{admin}");
    assert!(!admin.contains("Loading…"), "{admin}");

    server.kill().await.unwrap();
}
