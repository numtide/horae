#![cfg(feature = "server")]

//! The server renders the signed-in shell: the first HTML of a page has the
//! person's rail, the rows their grants open and the admin workspace, for that
//! person only, and no cache may keep it.

use std::time::Duration;

use dioxus::prelude::dioxus_fullstack::reqwest;
use sqlx::{ConnectOptions, PgPool};
use tokio::process::Command;
use uuid::Uuid;

/// The rows a member's grants do not open.
const GATED: [&str; 5] = ["People", "Clients", "Tasks", "Invoices", "Approvals"];
/// A week of the Timesheet: `/timesheet` alone redirects in the browser and
/// renders nothing on the server.
const SHEET: &str = "/timesheet/week/2026-11-16";

/// The rail's markup, `<aside …>…</aside>`.
fn rail(html: &str) -> &str {
    let start = html.find("<aside").expect("a rail");
    let end = start + html[start..].find("</aside>").expect("the rail ends");
    &html[start..end]
}

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
    let text = |path: &'static str, cookie: String| {
        let request = page(path, Some(&cookie));
        async move { request.await.unwrap().text().await.unwrap() }
    };
    // The development login signs in the first active administrator by id.
    let sign_in = || async {
        let signed_in = client
            .post(format!("{origin}/auth/dev-login"))
            .send()
            .await
            .unwrap();
        signed_in.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    };

    // Signed out, a page load still goes to the sign-in page.
    let signed_out = page("/timesheet", None).await.unwrap();
    assert_eq!(signed_out.status(), reqwest::StatusCode::SEE_OTHER);
    assert_eq!(signed_out.headers()["location"], "/auth/login");

    let admin = sign_in().await;
    let response = page(SHEET, Some(&admin)).await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    // The page carries the person: no cache may keep or share it.
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let vary: Vec<_> = response.headers().get_all("vary").iter().collect();
    assert!(vary.iter().any(|value| *value == "cookie"), "{vary:?}");
    let sheet = response.text().await.unwrap();
    assert!(rail(&sheet).contains("Render Admin"), "{sheet}");
    assert!(!sheet.contains("Not signed in"), "{sheet}");
    // The Administrator's grants open the gated rows.
    for row in GATED {
        assert!(rail(&sheet).contains(row), "{row}: {sheet}");
    }

    let workspace = text("/admin/importers", admin.clone()).await;
    assert!(workspace.contains("Render Org"), "{workspace}");
    assert!(workspace.contains("adm-nav"), "{workspace}");
    assert!(!workspace.contains("Loading…"), "{workspace}");

    // A second person, a member, signed in at the same time. Created as an
    // administrator with the lowest id so the development login picks them,
    // then reduced to the member floor.
    let member = Uuid::from_u128(1);
    sqlx::query(
        "INSERT INTO users (id, org_id, email, name)
         SELECT $1, org_id, 'member@test.com', 'Second Person' FROM users LIMIT 1",
    )
    .bind(member)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO person_permission_states
             (id, org_id, user_id, catalog_version, grants, is_administrator, source, built_in_profile)
         SELECT gen_random_uuid(), org_id, $1, catalog_version, grants, true, source, built_in_profile
         FROM person_permission_states LIMIT 1",
    )
    .bind(member)
    .execute(&pool)
    .await
    .unwrap();
    let member_cookie = sign_in().await;
    sqlx::query(
        "UPDATE person_permission_states
         SET is_administrator = false, source = 'individual', built_in_profile = NULL,
             grants = ARRAY['time_read_own', 'time_write_own', 'expense_read_own', 'expense_write_own']
         WHERE user_id = $1",
    )
    .bind(member)
    .execute(&pool)
    .await
    .unwrap();

    // Both people's pages rendered at once, interleaved: each sees only their own session.
    let renders = (0..4).flat_map(|_| {
        [
            (true, text(SHEET, admin.clone())),
            (false, text(SHEET, member_cookie.clone())),
        ]
    });
    let (who, pages): (Vec<_>, Vec<_>) = renders.unzip();
    let pages = futures_util::future::join_all(pages).await;
    for (is_admin, html) in who.into_iter().zip(pages) {
        let rail = rail(&html);
        if is_admin {
            assert!(
                rail.contains("Render Admin") && !rail.contains("Second Person"),
                "{rail}"
            );
            assert!(GATED.iter().all(|row| rail.contains(row)), "{rail}");
        } else {
            assert!(
                rail.contains("Second Person") && !rail.contains("Render Admin"),
                "{rail}"
            );
            assert!(GATED.iter().all(|row| !rail.contains(row)), "{rail}");
        }
    }

    // A deactivated account with a live session cookie: the page asks to sign
    // in again, answers 401, and is not cached either.
    sqlx::query("UPDATE users SET active = false WHERE id = $1")
        .bind(member)
        .execute(&pool)
        .await
        .unwrap();
    let ended = page(SHEET, Some(&member_cookie)).await.unwrap();
    assert_eq!(ended.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(ended.headers()["cache-control"], "private, no-store");
    let ended = ended.text().await.unwrap();
    assert!(ended.contains("Session ended"), "{ended}");
    assert!(ended.contains(r#"action="/auth/logout""#), "{ended}");
    assert!(!ended.contains("Second Person"), "{ended}");

    server.kill().await.unwrap();
}
