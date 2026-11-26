//! The invoice reads of `/api/v1`: who may read, the page's order and filters,
//! and the update time that "updated since" filters on.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::InvoiceStatus;
use sqlx::PgPool;
use uuid::Uuid;

use super::{InvoiceQuery, get, page, require};
use crate::ops::{Caller, OpError, begin_read};
use crate::testing::{SeedIds, add_person, seed};

async fn invoice(pool: &PgPool, ids: &SeedIds, number: &str, status: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency,
                               total_cents)
         VALUES ($1, $2, $3, $4, $5::text::invoice_status, '2026-09-01', '2026-10-01', 'EUR', 12000)",
        id,
        ids.org_id,
        ids.client_id,
        number,
        status,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

fn query() -> InvoiceQuery {
    InvoiceQuery {
        status: None,
        client_id: None,
        updated_since: None,
        after: None,
        limit: 100,
    }
}

async fn numbers(pool: &PgPool, ids: &SeedIds, query: &InvoiceQuery) -> Vec<String> {
    let mut read = begin_read(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = page(&mut read.tx, &read.actor, query).await;
    read.finish(result)
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.item.invoice.number)
        .collect()
}

async fn updated_at(pool: &PgPool, id: Uuid) -> DateTime<Utc> {
    sqlx::query_scalar!(
        r#"SELECT updated_at as "updated_at: DateTime<Utc>" FROM invoices WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_page_is_in_id_order_with_one_row_more_than_its_limit(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    for number in ["A-1", "A-2", "A-3"] {
        invoice(&pool, &admin, number, "sent").await;
    }
    let all = numbers(&pool, &admin, &query()).await;
    assert_eq!(all, ["A-1", "A-2", "A-3"]);

    let first = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            limit: 1,
            ..query()
        },
    )
    .await;
    assert_eq!(first, ["A-1", "A-2"], "the limit and one more");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_page_continues_after_the_last_id(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let first = invoice(&pool, &admin, "B-1", "sent").await;
    invoice(&pool, &admin, "B-2", "sent").await;
    let rest = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            after: Some(first),
            ..query()
        },
    )
    .await;
    assert_eq!(rest, ["B-2"]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn status_and_client_narrow_the_page(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    invoice(&pool, &admin, "C-draft", "draft").await;
    invoice(&pool, &admin, "C-sent", "sent").await;
    invoice(&pool, &admin, "C-paid", "paid").await;
    let sent = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            status: Some(InvoiceStatus::Sent),
            ..query()
        },
    )
    .await;
    assert_eq!(sent, ["C-sent"]);
    let elsewhere = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            client_id: Some(Uuid::now_v7()),
            ..query()
        },
    )
    .await;
    assert!(elsewhere.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_change_advances_updated_at_and_a_no_op_does_not(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let id = invoice(&pool, &admin, "D-1", "sent").await;
    let created = updated_at(&pool, id).await;

    sqlx::query!("UPDATE invoices SET status = status WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(updated_at(&pool, id).await, created, "a no-op save");

    sqlx::query!("UPDATE invoices SET status = 'paid' WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();
    let paid = updated_at(&pool, id).await;
    assert!(paid > created, "{paid} after {created}");

    let since_paid = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            updated_since: Some(paid),
            ..query()
        },
    )
    .await;
    assert_eq!(since_paid, ["D-1"]);
    let later = numbers(
        &pool,
        &admin,
        &InvoiceQuery {
            updated_since: Some(paid + chrono::TimeDelta::microseconds(1)),
            ..query()
        },
    )
    .await;
    assert!(later.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn get_reads_the_detail_with_its_update_time(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let id = invoice(&pool, &admin, "E-1", "sent").await;
    let mut read = begin_read(&pool, Caller::session(admin.org_id, admin.user_id))
        .await
        .unwrap();
    let result = get(&mut read.tx, &read.actor, id).await;
    let record = read.finish(result).await.unwrap();
    assert_eq!(record.detail.invoice.number, "E-1");
    assert_eq!(record.updated_at, updated_at(&pool, id).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_without_the_invoice_grant_is_refused(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let id = invoice(&pool, &admin, "F-1", "sent").await;
    let mut read = begin_read(&pool, Caller::session(member.org_id, member.user_id))
        .await
        .unwrap();
    let listed = page(&mut read.tx, &read.actor, &query()).await.unwrap_err();
    assert!(matches!(listed.kind(), OpError::Forbidden(_)), "{listed:?}");
    let one = get(&mut read.tx, &read.actor, id).await.unwrap_err();
    assert!(matches!(one.kind(), OpError::Forbidden(_)), "{one:?}");
    let required = require(&mut read.tx, &read.actor, id).await.unwrap_err();
    assert!(
        matches!(required.kind(), OpError::Forbidden(_)),
        "{required:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn another_organizations_invoice_is_not_found(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
    let theirs = invoice(&pool, &other, "G-1", "sent").await;
    let mut read = begin_read(&pool, Caller::session(admin.org_id, admin.user_id))
        .await
        .unwrap();
    let required = require(&mut read.tx, &read.actor, theirs)
        .await
        .unwrap_err();
    assert!(
        matches!(required.kind(), OpError::NotFound(_)),
        "{required:?}"
    );
    let one = get(&mut read.tx, &read.actor, theirs).await.unwrap_err();
    assert!(matches!(one.kind(), OpError::NotFound(_)), "{one:?}");
    assert!(numbers(&pool, &admin, &query()).await.is_empty());
}
