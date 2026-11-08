//! Running timers and later Harvest edits, through the durable job path.

use super::super::api_source::http::{
    ApiHttp,
    test_server::{Response, Server},
};
use super::*;
use crate::jobs;

/// Run one durable API job whose only time entry is `entry` (id 10).
async fn sync(
    pool: &PgPool,
    org: Uuid,
    mode: ImportMode,
    entry: serde_json::Value,
) -> ImportReport {
    jobs::enqueue(
        pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode,
            sync: SyncScope::Full,
        },
        &Uuid::now_v7().to_string(),
        crate::config::JobPolicy { max_attempts: 1 },
    )
    .await
    .unwrap();
    let server = Server::start(move |url| {
        let collection = url.path().rsplit('/').next().unwrap();
        let items = match collection {
            "clients" => json!([{"id":1,"name":"Client","currency":"USD"}]),
            "projects" => json!([{"id":2,"name":"Project","client":{"id":1}}]),
            "tasks" => json!([{"id":3,"name":"Task"}]),
            "users" => json!([{"id":4,"email":"known@example.com"}]),
            "time_entries" => {
                let mut entry = entry.clone();
                let fields = json!({"id":10,"spent_date":"2026-01-01","project":{"id":2},
                                    "task":{"id":3},"user":{"id":4}});
                for (key, value) in fields.as_object().unwrap() {
                    entry[key] = value.clone();
                }
                json!([entry])
            }
            _ => panic!("unexpected collection {collection}"),
        };
        Response::json(json!({collection:items,"links":{"next":null}}))
    });
    let mut report = None;
    let (lease, stop) = jobs::claim_lease_for_test(pool).await;
    jobs::run_claimed(pool, &lease, stop, async {
        let imported = run_api_import_with_http(
            pool,
            org,
            "USD",
            &HarvestConfig {
                client_id: "test".into(),
                client_secret: "test".into(),
                redirect_url: "http://localhost/auth/harvest/callback".into(),
                encryption_key_hex: KEY.into(),
            },
            mode,
            SyncScope::Full,
            ApiHttp::local(server.base.clone()),
            Some(&lease),
        )
        .await?;
        let result = job_report(&imported);
        report = Some(imported);
        result
    })
    .await
    .unwrap();
    report.expect("the job ran")
}

async fn stored(pool: &PgPool, org: Uuid) -> Vec<(i32, Option<String>)> {
    sqlx::query!(
        "SELECT minutes, notes FROM time_entries WHERE org_id = $1",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.minutes, row.notes))
    .collect()
}

fn reasons(report: &ImportReport) -> Vec<&str> {
    report
        .row_errors
        .iter()
        .map(|error| error.reason.as_str())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn running_timer_is_reported_until_it_stops(pool: PgPool) {
    let org = setup(&pool).await;
    let running = sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":1,"is_running":true,"updated_at":day(3)}),
    )
    .await;
    assert_eq!(reasons(&running), ["timer still running in Harvest"]);
    assert_eq!(running.summary.time_entries.created, 0);
    assert!(stored(&pool, org).await.is_empty());

    let stopped = sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":8,"is_running":false,"updated_at":day(4)}),
    )
    .await;
    assert!(stopped.row_errors.is_empty(), "{:?}", stopped.row_errors);
    assert_eq!(stopped.summary.time_entries.created, 1);
    assert_eq!(stored(&pool, org).await, [(480, None)]);
}

#[sqlx::test(migrations = "./migrations")]
async fn later_harvest_edits_update_the_open_entry_in_preview_and_commit(pool: PgPool) {
    let org = setup(&pool).await;
    let first = sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":1,"updated_at":day(3)}),
    )
    .await;
    assert_eq!(first.summary.time_entries.created, 1);

    let edited = json!({"hours":8,"notes":"finished","updated_at":day(5)});
    let preview = sync(&pool, org, ImportMode::DryRun, edited.clone()).await;
    assert_eq!(
        (
            preview.summary.time_entries.updated,
            preview.summary.time_entries.skipped
        ),
        (1, 0)
    );
    assert_eq!(stored(&pool, org).await, [(60, None)]);

    let commit = sync(&pool, org, ImportMode::Commit, edited.clone()).await;
    assert!(commit.row_errors.is_empty(), "{:?}", commit.row_errors);
    assert_eq!(commit.summary.time_entries.updated, 1);
    assert_eq!(stored(&pool, org).await, [(480, Some("finished".into()))]);

    let unchanged = sync(&pool, org, ImportMode::Commit, edited).await;
    assert_eq!(
        (
            unchanged.summary.time_entries.updated,
            unchanged.summary.time_entries.skipped
        ),
        (0, 1)
    );

    // The importer's own update must not read as an edit made in Horae.
    let again = sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":9,"notes":"finished","updated_at":day(6)}),
    )
    .await;
    assert!(again.row_errors.is_empty(), "{:?}", again.row_errors);
    assert_eq!(stored(&pool, org).await, [(540, Some("finished".into()))]);
}

#[sqlx::test(migrations = "./migrations")]
async fn later_harvest_edits_never_overwrite_submitted_or_horae_edited_entries(pool: PgPool) {
    let org = setup(&pool).await;
    for (edited_in_horae, reason) in [
        (
            false,
            "entry is no longer open in Horae; not updated from Harvest",
        ),
        (
            true,
            "entry was edited in Horae after import; not updated from Harvest",
        ),
    ] {
        sqlx::query!("DELETE FROM harvest_import_map WHERE org_id = $1", org)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM time_entries WHERE org_id = $1", org)
            .execute(&pool)
            .await
            .unwrap();
        sync(
            &pool,
            org,
            ImportMode::Commit,
            json!({"hours":1,"updated_at":day(3)}),
        )
        .await;
        if edited_in_horae {
            sqlx::query!(
                "UPDATE time_entries SET notes = 'mine' WHERE org_id = $1",
                org
            )
            .execute(&pool)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "UPDATE time_entries SET state = 'submitted' WHERE org_id = $1",
                org
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let before = stored(&pool, org).await;
        for mode in [ImportMode::DryRun, ImportMode::Commit] {
            let report = sync(&pool, org, mode, json!({"hours":8,"updated_at":day(5)})).await;
            assert_eq!(reasons(&report), [reason]);
            assert_eq!(stored(&pool, org).await, before);
        }
    }
}

async fn entry(pool: &PgPool, org: Uuid) -> (i32, String, Option<String>) {
    let row = sqlx::query!(
        r#"SELECT minutes, state::text AS "state!", notes FROM time_entries WHERE org_id = $1"#,
        org
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.minutes, row.state, row.notes)
}

#[sqlx::test(migrations = "./migrations")]
async fn a_submit_racing_an_update_keeps_the_submitted_entry(pool: PgPool) {
    let org = setup(&pool).await;
    sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":1,"updated_at":day(3)}),
    )
    .await;
    let user = sqlx::query_scalar!("SELECT user_id FROM time_entries WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    // Submit holds the exclusive timesheet lock while it freezes the week.
    let mut submission = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *submission)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
        user,
    )
    .execute(&mut *submission)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let run = tokio::spawn(async move {
        sync(
            &run_pool,
            org,
            ImportMode::Commit,
            json!({"hours":8,"updated_at":day(5)}),
        )
        .await
    });
    crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
    sqlx::query!(
        "UPDATE time_entries SET state = 'submitted', rounded_minutes = minutes WHERE org_id = $1",
        org
    )
    .execute(&mut *submission)
    .await
    .unwrap();
    submission.commit().await.unwrap();
    let report = run.await.unwrap();
    assert_eq!(
        reasons(&report),
        ["entry changed in Horae during the import; not updated from Harvest"]
    );
    assert_eq!(entry(&pool, org).await, (60, "submitted".into(), None));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_horae_edit_from_an_older_transaction_still_counts_as_edited(pool: PgPool) {
    let org = setup(&pool).await;
    sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":1,"updated_at":day(3)}),
    )
    .await;
    // `updated_at` is the transaction's start time, so this edit is stamped
    // earlier than the importer's update that commits before it.
    let mut web = pool.begin().await.unwrap();
    sqlx::query!("SELECT now()")
        .fetch_one(&mut *web)
        .await
        .unwrap();
    sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":2,"updated_at":day(4)}),
    )
    .await;
    sqlx::query!(
        "UPDATE time_entries SET notes = 'mine' WHERE org_id = $1",
        org
    )
    .execute(&mut *web)
    .await
    .unwrap();
    web.commit().await.unwrap();
    let report = sync(
        &pool,
        org,
        ImportMode::Commit,
        json!({"hours":3,"updated_at":day(5)}),
    )
    .await;
    assert_eq!(
        reasons(&report),
        ["entry was edited in Horae after import; not updated from Harvest"]
    );
    assert_eq!(
        entry(&pool, org).await,
        (120, "open".into(), Some("mine".into()))
    );
}
