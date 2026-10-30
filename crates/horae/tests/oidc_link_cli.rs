#![cfg(feature = "server")]

//! `horae user link-oidc` and `horae user list` through the real binary.

use sqlx::{ConnectOptions, PgPool};
use std::process::{Command, Output};
use uuid::Uuid;

const ISSUER: &str = "https://id.example.com";
const SUBJECT: &str = "operator-subject-7f3a";

fn horae(database: &str, issuer_env: Option<&str>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_horae"));
    command
        .env("DATABASE_URL", database)
        .env_remove("HORAE_OIDC_ISSUER");
    if let Some(issuer) = issuer_env {
        command.env("HORAE_OIDC_ISSUER", issuer);
    }
    command.args(args).output().unwrap()
}

fn text(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

async fn setup(pool: &PgPool) -> (String, Uuid, Uuid) {
    let database = pool.connect_options().to_url_lossy().to_string();
    let init = horae(
        &database,
        None,
        &[
            "init",
            "--org-name",
            "Link",
            "--admin-email",
            "admin@test.com",
            "--admin-name",
            "Admin",
        ],
    );
    assert!(init.status.success(), "{init:?}");
    let member = horae(
        &database,
        None,
        &[
            "user",
            "create",
            "--email",
            "member@test.com",
            "--name",
            "Member",
        ],
    );
    assert!(member.status.success(), "{member:?}");
    let id = |email: &'static str| async move {
        sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
            .fetch_one(pool)
            .await
            .unwrap()
    };
    (
        database,
        id("admin@test.com").await,
        id("member@test.com").await,
    )
}

async fn binding(pool: &PgPool, id: Uuid) -> (Option<String>, Option<String>) {
    let row = sqlx::query!(
        "SELECT oidc_issuer, oidc_subject FROM users WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.oidc_issuer, row.oidc_subject)
}

fn link(database: &str, issuer_env: Option<&str>, id: Uuid, issuer: &str, subject: &str) -> Output {
    horae(
        database,
        issuer_env,
        &[
            "user",
            "link-oidc",
            "--user-id",
            &id.to_string(),
            "--issuer",
            issuer,
            "--subject",
            subject,
        ],
    )
}

fn assert_private(output: &Output) {
    let (stdout, stderr) = text(output);
    for value in [ISSUER, SUBJECT, "id.example.com"] {
        assert!(
            !stdout.contains(value) && !stderr.contains(value),
            "{value} leaked: {stdout}{stderr}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn link_reports_each_outcome_with_its_exit_code_and_never_echoes_the_identity(pool: PgPool) {
    let (database, admin, member) = setup(&pool).await;

    let linked = link(&database, None, admin, ISSUER, SUBJECT);
    assert_eq!(linked.status.code(), Some(0), "{linked:?}");
    assert_eq!(
        text(&linked).0.trim(),
        format!("Linked user {admin} to the OIDC identity.")
    );
    assert_eq!(
        binding(&pool, admin).await,
        (Some(ISSUER.into()), Some(SUBJECT.into()))
    );

    let repeat = link(&database, None, admin, ISSUER, SUBJECT);
    assert_eq!(repeat.status.code(), Some(0), "{repeat:?}");
    assert!(text(&repeat).0.contains("nothing changed"), "{repeat:?}");

    let taken = link(&database, None, member, ISSUER, SUBJECT);
    assert_eq!(taken.status.code(), Some(1), "{taken:?}");
    assert_eq!(
        text(&taken).1.trim(),
        "Another user is already linked to this subject."
    );
    assert_eq!(binding(&pool, member).await, (None, None));

    let missing = Uuid::now_v7();
    let unknown = link(&database, None, missing, ISSUER, "other");
    assert_eq!(unknown.status.code(), Some(1), "{unknown:?}");
    assert_eq!(
        text(&unknown).1.trim(),
        format!("No user with ID {missing}.")
    );

    for output in [&linked, &repeat, &taken, &unknown] {
        assert_private(output);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_input_is_refused_before_touching_the_database(pool: PgPool) {
    let (database, admin, _) = setup(&pool).await;
    // An unreachable database proves validation runs before any connection.
    let unreachable = "postgres://nobody@127.0.0.1:1/none";
    for (issuer, subject, reason) in [
        (
            ISSUER,
            " padded",
            "The subject must be 1–255 bytes with no leading or trailing whitespace.",
        ),
        (
            "http://id.example.com",
            SUBJECT,
            "The issuer must be an https URL (or http on a loopback host) with no query or fragment.",
        ),
    ] {
        let refused = link(unreachable, None, admin, issuer, subject);
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert_eq!(text(&refused).1.trim(), reason);
        assert_private(&refused);
    }
    let usage = horae(
        &database,
        None,
        &[
            "user",
            "link-oidc",
            "--user-id",
            "not-a-uuid",
            "--issuer",
            ISSUER,
            "--subject",
            SUBJECT,
        ],
    );
    assert_eq!(usage.status.code(), Some(2), "{usage:?}");
    assert_eq!(binding(&pool, admin).await, (None, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_configured_issuer_alone_guards_against_a_mismatch(pool: PgPool) {
    let (database, admin, _) = setup(&pool).await;
    // Only HORAE_OIDC_ISSUER is set: no client ID, secret or redirect URL.
    let mismatch = link(
        &database,
        Some("https://id.example.com/"),
        admin,
        ISSUER,
        SUBJECT,
    );
    assert_eq!(mismatch.status.code(), Some(1), "{mismatch:?}");
    assert_eq!(
        text(&mismatch).1.trim(),
        "The issuer does not match HORAE_OIDC_ISSUER exactly."
    );
    assert_eq!(binding(&pool, admin).await, (None, None));

    let matching = link(&database, Some(ISSUER), admin, ISSUER, SUBJECT);
    assert_eq!(matching.status.code(), Some(0), "{matching:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn user_list_shows_ids_but_no_identity(pool: PgPool) {
    let (database, admin, member) = setup(&pool).await;
    assert!(
        link(&database, None, admin, ISSUER, SUBJECT)
            .status
            .success()
    );

    let list = horae(&database, None, &["user", "list"]);
    assert!(list.status.success(), "{list:?}");
    let (stdout, _) = text(&list);
    assert!(stdout.starts_with("ID "), "{stdout}");
    for id in [admin, member] {
        assert!(stdout.contains(&id.to_string()), "{stdout}");
    }
    assert_private(&list);
}
