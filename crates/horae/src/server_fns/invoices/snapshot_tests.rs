use super::*;
use crate::ops::testing::set_grants;
use crate::server_fns::test_seed::{SeedIds, seed};
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

mod editor;
mod races;
mod settings;

#[derive(Clone, Copy, Debug)]
enum Reader {
    Invoice,
    Fees,
}

async fn read(
    pool: &PgPool,
    identity: (Uuid, Uuid),
    ids: &SeedIds,
    reader: Reader,
) -> Result<i64, ServerFnError> {
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    match reader {
        Reader::Invoice => {
            preview::prepare_with_edits(pool, identity, ids.client_id, period, None, None, None)
                .await
                .map(|preview| preview.subtotal_cents)
        }
        Reader::Fees => fee_tests::fee_balances(pool, identity, ids, period)
            .await
            .map(|balances| balances[0].1.remaining_cents),
    }
}

fn assert_denied(result: Result<i64, ServerFnError>) {
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn financial_snapshots_require_current_same_tenant_invoice_writer(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let mut connection = pool.acquire().await.unwrap();
    for reader in [Reader::Invoice, Reader::Fees] {
        read(&pool, (ids.org_id, ids.user_id), &ids, reader)
            .await
            .unwrap();
        assert_denied(read(&pool, (ids.org_id, Uuid::now_v7()), &ids, reader).await);
        assert_denied(read(&pool, (ids.org_id, foreign.user_id), &ids, reader).await);
        assert_denied(read(&pool, (Uuid::now_v7(), ids.user_id), &ids, reader).await);
        // Reading every invoice is not enough to prepare one.
        set_grants(&mut connection, &ids, &[Permission::InvoiceReadAll])
            .await
            .unwrap();
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
        // Being an administrator is not the grant.
        crate::ops::testing::provision(&pool, &ids, &[], true).await;
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
        // Every grant, but inactive.
        crate::ops::testing::provision(&pool, &ids, Permission::ALL, true).await;
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        set_grants(&mut connection, &ids, &[Permission::InvoiceWriteAll])
            .await
            .unwrap();
        read(&pool, (ids.org_id, ids.user_id), &ids, reader)
            .await
            .unwrap();
    }
}

/// The managed invoice grants authorize nothing until the managed-invoice
/// predicate is decided (T006).
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn managed_invoice_grants_open_no_financial_snapshot(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    set_grants(
        &mut pool.acquire().await.unwrap(),
        &ids,
        &[
            Permission::InvoiceWriteManaged,
            Permission::ProjectWriteManaged,
            Permission::BillableRateWriteManaged,
        ],
    )
    .await
    .unwrap();
    for reader in [Reader::Invoice, Reader::Fees] {
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
    }
}
