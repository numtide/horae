use super::super::streaming::{self as pipeline, Page};
use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_stale_commit_cannot_advance_data_or_watermark(pool: PgPool) {
    use crate::jobs;
    use std::time::Duration;

    let org = setup(&pool).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Full,
        },
        "stale-api",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let connection = lock_import(&pool, org).await.unwrap();
    let mut catalog = valid_data();
    let entry = catalog.time_entries.pop().unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    // Bypass heartbeat observation so only the transaction's fence can stop it.
    let run = tokio::spawn(async move {
        pipeline::run(
            connection,
            org,
            "USD",
            ImportMode::Commit,
            day(4),
            move |send| {
                send.blocking_send(Page::Catalog(catalog))?;
                for n in 0..3 {
                    let mut row = entry.clone();
                    row.id += n;
                    send.blocking_send(Page::Entries(vec![row]))?;
                }
                let _ = started.send(());
                wait.recv_timeout(Duration::from_secs(10))?;
                Ok(())
            },
            Some(&lease),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), ready)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (_replacement, _replacement_stop) = jobs::claim_lease_for_test(&pool).await;
    release.send(()).unwrap();
    let error = tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("lease lost"), "{error}");
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    let pending = jobs::status(&pool, org, id).await.unwrap().unwrap();
    assert_eq!(pending.status, "running");
    assert!(pending.report.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_api_cancel_waits_for_the_producer_before_acknowledging(pool: PgPool) {
    use crate::jobs;
    use std::time::Duration;

    let org = setup(&pool).await;
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    let old_watermark = watermark(&pool, org).await;
    let id = jobs::enqueue(
        &pool,
        org,
        &jobs::JobPayload::HarvestApi {
            mode: ImportMode::Commit,
            sync: SyncScope::Full,
        },
        "cancel-api",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    let connection = lock_import(&pool, org).await.unwrap();
    let mut catalog = valid_data();
    let entry = catalog.time_entries.pop().unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (closed, consumer_closed) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let runtime = tokio::runtime::Handle::current();
    let run_pool = pool.clone();
    let run = tokio::spawn(async move {
        jobs::run_claimed(&run_pool, &lease, stop, async {
            let report = pipeline::run(
                connection,
                org,
                "USD",
                ImportMode::Commit,
                day(4),
                move |send| {
                    send.blocking_send(Page::Catalog(catalog))?;
                    for n in 1..=3 {
                        let mut row = entry.clone();
                        row.id += n;
                        send.blocking_send(Page::Entries(vec![row]))?;
                    }
                    let _ = started.send(());
                    runtime.block_on(send.closed());
                    let _ = closed.send(());
                    wait.recv_timeout(Duration::from_secs(10))?;
                    Ok(())
                },
                Some(&lease),
            )
            .await?;
            job_report(&report)
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), ready)
        .await
        .unwrap()
        .unwrap();
    assert!(jobs::cancel(&pool, org, id).await.unwrap());
    tokio::time::timeout(Duration::from_secs(10), consumer_closed)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "running"
    );
    assert!(!jobs::retry(&pool, org, id).await.unwrap());
    assert!(matches!(
        lock_import(&pool, org).await,
        Err(ApiImportError::Busy)
    ));
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "cancelled"
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(watermark(&pool, org).await, old_watermark);
    assert!(jobs::retry(&pool, org, id).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn later_download_failure_rolls_back_parents_rows_and_watermark(pool: PgPool) {
    let org = setup(&pool).await;
    let connection = lock_import(&pool, org).await.unwrap();
    let mut catalog = valid_data();
    let entries = std::mem::take(&mut catalog.time_entries);
    let error = pipeline::run(
        connection,
        org,
        "USD",
        ImportMode::Commit,
        day(4),
        move |send| {
            send.blocking_send(Page::Catalog(catalog))?;
            send.blocking_send(Page::Entries(entries))?;
            anyhow::bail!("fixture HTTP 500 on the next page")
        },
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "fixture HTTP 500 on the next page");
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM harvest_import_map WHERE org_id = $1",
            org
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    let retry = apply_api_data(&pool, org, "USD", ImportMode::Commit, &valid_data(), day(4))
        .await
        .unwrap();
    assert_eq!(retry.summary.time_entries.created, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelled_page_consumer_retains_lock_until_worker_exits_and_rolls_back(pool: PgPool) {
    let org = setup(&pool).await;
    let one = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let connection = lock_import(&one, org).await.unwrap();
    let mut catalog = valid_data();
    let entry = catalog.time_entries.pop().unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let pending = tokio::spawn(pipeline::run(
        connection,
        org,
        "USD",
        ImportMode::Commit,
        day(4),
        move |send| {
            send.blocking_send(Page::Catalog(catalog))?;
            // A capacity-one channel cannot accept the third entry until SQL has
            // finished applying the first and received the second.
            for n in 0..3 {
                let mut row = entry.clone();
                row.id += n;
                send.blocking_send(Page::Entries(vec![row]))?;
            }
            let _ = started.send(());
            wait.recv_timeout(std::time::Duration::from_secs(10))?;
            Ok(())
        },
        None,
    ));
    tokio::time::timeout(std::time::Duration::from_secs(5), ready)
        .await
        .unwrap()
        .unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    let competing = lock_import(&pool, org).await;
    release.send(()).unwrap();
    assert!(matches!(competing, Err(ApiImportError::Busy)));
    let retry = tokio::time::timeout(std::time::Duration::from_secs(5), lock_import(&one, org))
        .await
        .unwrap()
        .unwrap();
    retry.close().await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    one.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_and_missing_timestamp_rows_cross_page_boundaries_without_advancing(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    let entry = source.time_entries[0].clone();
    source.time_entries = (0..205)
        .map(|n| {
            let mut row = entry.clone();
            row.id += n;
            row.notes = Some(format!("page record {n}"));
            if n == 100 {
                row.user.id = 5
            }
            row
        })
        .collect();
    let preview = apply_api_data(&pool, org, "USD", ImportMode::DryRun, &source, day(4))
        .await
        .unwrap();
    let first = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(preview.summary, first.summary);
    assert_eq!(
        (
            first.summary.time_entries.created,
            first.summary.time_entries.errored
        ),
        (204, 1)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    source.time_entries[100].user.id = 4;
    source.time_entries[204].updated_at = None;
    let retry = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (
            retry.summary.time_entries.created,
            retry.summary.time_entries.skipped
        ),
        (1, 204)
    );
    assert_eq!(watermark(&pool, org).await, json!({}));
    source.time_entries[204].updated_at = Some(day(3));
    let final_run = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(final_run.summary.time_entries.skipped, 205);
    assert_eq!(
        credentials::load(&pool, org, KEY)
            .await
            .unwrap()
            .unwrap()
            .watermark_for(EntityType::TimeEntry),
        Some(day(3) - chrono::Duration::seconds(1))
    );
}
