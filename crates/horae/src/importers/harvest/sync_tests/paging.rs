use super::super::streaming::{self as pipeline, Page};
use super::*;

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
