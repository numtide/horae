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

#[sqlx::test(migrations = "./migrations")]
async fn void_waits_for_a_concurrent_payment_and_then_refuses(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Approved).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Sent)
        .await
        .unwrap();

    // Hold an uncommitted payment so the competing request must wait. A plain
    // SELECT still sees 'sent', which is precisely the stale-read window.
    let mut payment = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE invoices SET status = 'paid' WHERE id = $1",
        invoice.id
    )
    .execute(&mut *payment)
    .await
    .unwrap();
    let pid = sqlx::query_scalar!(r#"SELECT pg_backend_pid() as "pid!""#)
        .fetch_one(&mut *payment)
        .await
        .unwrap();

    let mut tasks = tokio::task::JoinSet::new();
    let competing_pool = pool.clone();
    tasks.spawn(async move {
        transition_invoice(&competing_pool, ids.org_id, invoice.id, InvoiceStatus::Void).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                   WHERE $1 = ANY(pg_blocking_pids(pid))) as "waiting!""#,
                pid,
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
        }
    })
    .await
    .expect("the competing transition must reach the locked invoice");

    payment.commit().await.unwrap();
    let result = tasks.join_next().await.unwrap().unwrap();
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let row = sqlx::query!(
        r#"SELECT i.status as "status: InvoiceStatus", te.invoice_id,
                  te.state as "state: EntryState"
           FROM invoices i JOIN time_entries te ON te.id = $2 WHERE i.id = $1"#,
        invoice.id,
        entry,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (row.status, row.invoice_id, row.state),
        (InvoiceStatus::Paid, Some(invoice.id), EntryState::Invoiced)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn void_reopens_time_without_stale_rounding(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!(
        "UPDATE time_entries SET rounded_minutes = 75 WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;

    let result = transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();

    assert_eq!(result.status, InvoiceStatus::Void);
    let row = sqlx::query!(
        r#"SELECT state as "state: EntryState", invoice_id, rounded_minutes, minutes
           FROM time_entries WHERE id = $1"#,
        entry,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (row.state, row.invoice_id, row.rounded_minutes, row.minutes),
        (EntryState::Open, None, None, 60)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invoice_transitions_reject_invalid_states_and_foreign_organizations(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;

    assert!(matches!(
        transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Paid).await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert!(matches!(
        transition_invoice(&pool, uuid::Uuid::now_v7(), invoice.id, InvoiceStatus::Sent).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Sent)
        .await
        .unwrap();
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Paid)
        .await
        .unwrap();
    assert!(matches!(
        transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Void).await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
}
