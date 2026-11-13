//! Harvest people are matched or created before their entries (spec 019).

use super::super::api_source::http::{
    ApiHttp,
    test_server::{Response, Server},
};
use super::*;
use crate::jobs;
use horae_core::importers::harvest::types::{PersonOutcome, ReportPerson};
use horae_core::permissions::catalog::BuiltInProfile;

/// Every person case the people phase distinguishes, each with one entry.
fn people_data() -> HarvestData {
    let mut data = valid_data();
    data.users = serde_json::from_value(json!([
        {"id":4,"email":"known@example.com"},
        {"id":5,"email":"new@example.com","first_name":"New","last_name":"Person","is_active":false},
        {"id":6,"email":" OLD@example.com "},
        {"id":7,"first_name":"No","last_name":"Mail"},
        {"id":8,"email":"dup@example.com"},
        {"id":9,"email":"DUP@example.com"}
    ]))
    .unwrap();
    data.time_entries = (4..=9)
        .map(|user| {
            serde_json::from_value(json!({
                "id":100 + user,"spent_date":"2026-01-01","hours":1,"project":{"id":2},
                "task":{"id":3},"user":{"id":user},"updated_at":day(3)
            }))
            .unwrap()
        })
        .collect();
    data
}

/// A deactivated account with rates and a sign-in binding: an import must
/// leave every field of it alone.
async fn add_old_account(pool: &PgPool, org: Uuid) -> serde_json::Value {
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, active, cost_rate_cents,
                            billable_rate_cents, oidc_subject, oidc_issuer)
         VALUES ($1, $2, 'old@example.com', 'Old Hand', false, 4000, 9000,
                 'subject-1', 'https://issuer.example')",
        Uuid::now_v7(),
        org,
    )
    .execute(pool)
    .await
    .unwrap();
    accounts(pool, org).await
}

async fn accounts(pool: &PgPool, org: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(jsonb_agg(to_jsonb(u) ORDER BY u.email), '[]') AS "rows!"
           FROM users u WHERE org_id = $1"#,
        org,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn person_links(pool: &PgPool, org: Uuid) -> Vec<i64> {
    sqlx::query_scalar!(
        "SELECT harvest_import_id FROM harvest_import_map
         WHERE org_id = $1 AND harvest_entity_type = 'person' ORDER BY harvest_import_id",
        org,
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

/// "User created" events queued for delivery, committed with their accounts.
pub(in crate::importers::harvest) async fn queued_user_events(pool: &PgPool, org: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM horae_outbox
           WHERE org_id = $1 AND event_kind = 'user_created'
             AND payload->>'event' = 'user_created' AND payload->'user'->>'method' IS NULL"#,
        org,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn kinds(report: &ImportReport) -> Vec<(String, PersonErrorKind)> {
    report
        .row_errors
        .iter()
        .map(|e| (e.source_location.clone(), e.person.as_ref().unwrap().kind))
        .collect()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn people_are_matched_or_created_and_their_entries_import(pool: PgPool) {
    let org = setup(&pool).await;
    let before = add_old_account(&pool, org).await;
    let report = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &people_data(),
        day(4),
    )
    .await
    .unwrap();

    assert_eq!(
        (
            report.summary.people.created,
            report.summary.people.skipped,
            report.summary.people.errored
        ),
        (1, 2, 3)
    );
    assert_eq!(report.summary.time_entries.created, 3);
    assert_eq!(
        kinds(&report),
        vec![
            ("person 7".into(), PersonErrorKind::MissingEmail),
            ("person 8".into(), PersonErrorKind::DuplicateEmail),
            ("person 9".into(), PersonErrorKind::DuplicateEmail),
            ("time_entry 107".into(), PersonErrorKind::PersonNotImported),
            ("time_entry 108".into(), PersonErrorKind::PersonNotImported),
            ("time_entry 109".into(), PersonErrorKind::PersonNotImported),
        ]
    );
    assert_eq!(person_links(&pool, org).await, vec![4, 5, 6]);

    let created = sqlx::query!(
        r#"SELECT id, name, active, cost_rate_cents,
                  billable_rate_cents, oidc_subject, oidc_issuer
           FROM users WHERE org_id = $1 AND email = 'new@example.com'"#,
        org,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (created.name.as_str(), created.active),
        ("New Person", false)
    );
    assert_eq!(
        (created.cost_rate_cents, created.billable_rate_cents),
        (None, None)
    );
    assert_eq!((created.oidc_subject, created.oidc_issuer), (None, None));
    assert_eq!(
        crate::server_fns::test_seed::built_in_profile(&pool, org, created.id).await,
        Some((BuiltInProfile::Member, false))
    );

    // Everyone who existed before the import is exactly as they were.
    let after = accounts(&pool, org).await;
    let existing = |rows: &serde_json::Value| {
        rows.as_array()
            .unwrap()
            .iter()
            .filter(|row| row["email"] != "new@example.com")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(existing(&after), existing(&before));

    let new_person = report
        .people
        .iter()
        .find(|p| p.harvest_id == Some(5))
        .unwrap();
    assert_eq!(
        new_person,
        &ReportPerson {
            harvest_id: Some(5),
            horae_id: created.id,
            email: Some("new@example.com".into()),
            name: "New Person".into(),
            outcome: PersonOutcome::Created,
            harvest_active: Some(false),
        }
    );
    assert_eq!(report.people.len(), 3);
    assert_eq!(queued_user_events(&pool, org).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_second_commit_creates_nobody_and_follows_the_harvest_identity(pool: PgPool) {
    let org = setup(&pool).await;
    add_old_account(&pool, org).await;
    let data = people_data();
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    // An administrator changes the created person's email; the link holds.
    sqlx::query!(
        "UPDATE users SET email = 'renamed@example.com' WHERE org_id = $1 AND email = 'new@example.com'",
        org,
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = accounts(&pool, org).await;
    let again = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    assert_eq!(
        (again.summary.people.created, again.summary.people.skipped),
        (0, 3)
    );
    assert_eq!(
        (
            again.summary.time_entries.created,
            again.summary.time_entries.skipped
        ),
        (0, 3)
    );
    assert!(
        again
            .people
            .iter()
            .all(|p| p.outcome == PersonOutcome::Matched)
    );
    assert_eq!(accounts(&pool, org).await, before);
    assert_eq!(
        queued_user_events(&pool, org).await,
        1,
        "only the first run creates"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_preview_reports_people_and_writes_none(pool: PgPool) {
    let org = setup(&pool).await;
    let before = add_old_account(&pool, org).await;
    let preview = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::DryRun,
        &people_data(),
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(accounts(&pool, org).await, before);
    assert!(person_links(&pool, org).await.is_empty());
    assert_eq!(queued_user_events(&pool, org).await, 0);
    let commit = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &people_data(),
        day(4),
    )
    .await
    .unwrap();
    assert_eq!(preview.summary, commit.summary);
    assert_eq!(preview.row_errors, commit.row_errors);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_email_shared_by_two_accounts_is_never_guessed(pool: PgPool) {
    let org = setup(&pool).await;
    add_user(&pool, org, "same@example.com").await;
    add_user(&pool, org, "SAME@example.com").await;
    let mut data = valid_data();
    data.users = serde_json::from_value(json!([{"id":4,"email":"Same@example.com"}])).unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    assert_eq!(
        kinds(&report),
        vec![
            ("person 4".into(), PersonErrorKind::AmbiguousPerson),
            ("time_entry 10".into(), PersonErrorKind::PersonNotImported),
        ]
    );
    assert!(person_links(&pool, org).await.is_empty());
    assert_eq!(report.summary.time_entries.created, 0);
}

/// The created person's entries arrive on two pages, so a durable preview must
/// bring the rolled-back account back for the second page.
fn paged_people(url: &openidconnect::url::Url) -> Response {
    let collection = url.path().rsplit('/').next().unwrap();
    let second = url.query_pairs().any(|(k, v)| k == "cursor" && v == "two");
    let items = match collection {
        "clients" => json!([{"id":1,"name":"Client"}]),
        "projects" => json!([{"id":2,"name":"Project","client":{"id":1}}]),
        "tasks" => json!([{"id":3,"name":"Task"}]),
        "users" => {
            json!([{"id":5,"email":"new@example.com","first_name":"New","last_name":"Person"}])
        }
        "time_entries" => json!([{
            "id":if second {11} else {10}, "spent_date":"2026-01-01", "hours":1,
            "project":{"id":2}, "task":{"id":3}, "user":{"id":5}, "updated_at":day(3)
        }]),
        _ => panic!("unexpected collection"),
    };
    let next = (collection == "time_entries" && !second).then(|| {
        let mut next = url.clone();
        next.query_pairs_mut().clear().append_pair("cursor", "two");
        next.to_string()
    });
    Response::json(json!({collection:items,"links":{"next":next}}))
}

async fn durable_people_run(pool: &PgPool, org: Uuid, mode: ImportMode) -> ImportReport {
    durable_people_job(pool, org, mode, paged_people)
        .await
        .unwrap()
}

async fn durable_people_job(
    pool: &PgPool,
    org: Uuid,
    mode: ImportMode,
    pages: fn(&openidconnect::url::Url) -> Response,
) -> Result<ImportReport, ApiImportError> {
    jobs::enqueue(
        pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode,
            sync: SyncScope::Full,
        },
        &format!("people-{mode}"),
        Default::default(),
    )
    .await
    .unwrap();
    let server = Server::start(pages);
    let (lease, _stop) = jobs::claim_lease_for_test(pool).await;
    let cfg = HarvestConfig {
        client_id: "test".into(),
        client_secret: "test".into(),
        redirect_url: "http://localhost/auth/harvest/callback".into(),
        encryption_key_hex: KEY.into(),
    };
    run_api_import_with_http(
        pool,
        org,
        "USD",
        &cfg,
        mode,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
        Some(&lease),
    )
    .await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_durable_preview_keeps_simulated_people_across_pages(pool: PgPool) {
    let org = setup(&pool).await;
    let before = accounts(&pool, org).await;
    let preview = durable_people_run(&pool, org, ImportMode::DryRun).await;
    assert_eq!(preview.error_count(), 0, "{:?}", preview.row_errors);
    assert_eq!(preview.summary.people.created, 1);
    assert_eq!(preview.summary.time_entries.created, 2);
    assert_eq!(accounts(&pool, org).await, before);
    assert!(person_links(&pool, org).await.is_empty());

    let commit = durable_people_run(&pool, org, ImportMode::Commit).await;
    assert_eq!(preview.summary, commit.summary);
    assert_eq!(person_links(&pool, org).await, vec![5]);
    assert_eq!(queued_user_events(&pool, org).await, 1);
}

/// Harvest fails while serving entries, after the people batch has committed.
fn failing_entries(url: &openidconnect::url::Url) -> Response {
    if url.path().ends_with("/time_entries") {
        return Response {
            status: 500,
            headers: vec![],
            body: vec![],
        };
    }
    paged_people(url)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_created_person_is_announced_even_when_the_job_fails_later(pool: PgPool) {
    let org = setup(&pool).await;
    let error = durable_people_job(&pool, org, ImportMode::Commit, failing_entries)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("HTTP 500"), "{error}");
    assert_eq!(person_links(&pool, org).await, vec![5]);
    assert_eq!(queued_user_events(&pool, org).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_email_match_never_takes_over_another_harvest_persons_account(pool: PgPool) {
    let org = setup(&pool).await;
    let mut data = valid_data();
    data.users = serde_json::from_value(json!([{"id":1,"email":"a@example.com"}])).unwrap();
    data.time_entries[0].user.id = 1;
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    // In Harvest, person 1 moved to a new email and person 3 took the old one.
    data.users = serde_json::from_value(json!([
        {"id":1,"email":"a2@example.com"},
        {"id":3,"email":"a@example.com"}
    ]))
    .unwrap();
    let mut entry = data.time_entries[0].clone();
    entry.id = 30;
    entry.user.id = 3;
    data.time_entries.push(entry);
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    assert_eq!(
        kinds(&report),
        vec![
            ("person 3".into(), PersonErrorKind::AmbiguousPerson),
            ("time_entry 30".into(), PersonErrorKind::PersonNotImported),
        ]
    );
    assert!(report.row_errors[0].reason.contains("Harvest person 1"));
    assert_eq!(person_links(&pool, org).await, vec![1]);
    assert_eq!(report.summary.time_entries.skipped, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_email_used_outside_the_workspace_is_reported_not_created(pool: PgPool) {
    let org = setup(&pool).await;
    let elsewhere = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Elsewhere')",
        elsewhere
    )
    .execute(&pool)
    .await
    .unwrap();
    add_user(&pool, elsewhere, "Taken@Example.com").await;
    let mut data = valid_data();
    data.users = serde_json::from_value(json!([{"id":4,"email":"taken@example.com"}])).unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4))
        .await
        .unwrap();
    assert_eq!(
        kinds(&report),
        vec![
            ("person 4".into(), PersonErrorKind::EmailInUse),
            ("time_entry 10".into(), PersonErrorKind::PersonNotImported),
        ]
    );
    assert!(person_links(&pool, org).await.is_empty());
    assert_eq!(queued_user_events(&pool, org).await, 0);
}

/// Sign-in creates `Late@Example.com` while the import waits on that email's
/// lock. Even on a REPEATABLE READ connection the import's recheck must see
/// it, or it would add `late@example.com` beside it.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_account_created_while_the_import_waits_is_never_duplicated(pool: PgPool) {
    let org = setup(&pool).await;
    let elsewhere = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Elsewhere')",
        elsewhere
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut signup = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *signup)
        .await
        .unwrap();
    crate::db::lock_new_user_email(&mut signup, "Late@Example.com")
        .await
        .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'Late@Example.com', 'Late')",
        Uuid::now_v7(),
        elsewhere,
    )
    .execute(&mut *signup)
    .await
    .unwrap();
    let mut data = valid_data();
    data.users = serde_json::from_value(json!([{"id":4,"email":"late@example.com"}])).unwrap();
    let rr = crate::server_fns::test_seed::repeatable_read_pool(&pool).await;
    let import = tokio::spawn(async move {
        apply_api_data(&rr, org, "USD", ImportMode::Commit, &data, day(4)).await
    });
    crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
    signup.commit().await.unwrap();
    let report = tokio::time::timeout(std::time::Duration::from_secs(10), import)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        kinds(&report),
        vec![
            ("person 4".into(), PersonErrorKind::EmailInUse),
            ("time_entry 10".into(), PersonErrorKind::PersonNotImported),
        ]
    );
    let accounts = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM users WHERE lower(email) = 'late@example.com'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(accounts, 1);
}
