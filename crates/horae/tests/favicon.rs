#![cfg(feature = "server")]

use std::time::Duration;

use dioxus::prelude::dioxus_fullstack::reqwest;
use sqlx::{ConnectOptions, PgPool};
use tokio::process::Command;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn favicon_is_public_and_every_page_links_it(pool: PgPool) {
    let database = pool.connect_options().to_url_lossy().to_string();
    let initialized = Command::new(env!("CARGO_BIN_EXE_horae"))
        .args([
            "init",
            "--org-name",
            "Favicon",
            "--admin-email",
            "favicon@test.com",
            "--admin-name",
            "Favicon Admin",
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
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut poll = tokio::time::interval(Duration::from_millis(100));
        loop {
            poll.tick().await;
            assert!(
                server.try_wait().unwrap().is_none(),
                "server exited before readiness"
            );
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

    // A direct navigation sends `Accept: text/html`, which the login guard
    // would otherwise redirect for a signed-out visitor.
    let icon = client
        .get(format!("{origin}/favicon.ico"))
        .header("accept", "text/html,*/*")
        .send()
        .await
        .unwrap();
    assert_eq!(icon.status(), reqwest::StatusCode::OK);
    assert_eq!(icon.headers()["content-type"], "image/x-icon");
    assert!(!icon.bytes().await.unwrap().is_empty());

    let login = client
        .get(format!("{origin}/auth/login"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(login.contains(r#"rel="icon""#), "{login}");

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
    let shell = client
        .get(format!("{origin}/"))
        .header("accept", "text/html")
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(shell.contains(r#"rel="icon""#), "{shell}");
    assert!(shell.contains(r#"rel="apple-touch-icon""#), "{shell}");

    server.kill().await.unwrap();
}
