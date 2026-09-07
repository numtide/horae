use super::*;
use crate::server_fns::test_seed::{seed, time_entry};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn invoicing_leaves_running_time_open(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let running = time_entry(&pool, &ids, EntryState::Open).await;
    let stopped = time_entry(&pool, &ids, EntryState::Open).await;
    let approved = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now(), minutes = 0 WHERE id = $1",
        running,
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();

    let billed: Vec<_> = result.lines.iter().map(|line| line.time_entry_id).collect();
    assert_eq!(billed, vec![stopped, approved]);
    let timer = sqlx::query!(
        r#"SELECT state as "state: EntryState", invoice_id, is_running FROM time_entries WHERE id = $1"#,
        running,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (timer.state, timer.invoice_id, timer.is_running),
        (EntryState::Open, None, true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn only_running_time_does_not_create_an_invoice(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let running = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now() WHERE id = $1",
        running,
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day).await;

    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM invoices")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}
