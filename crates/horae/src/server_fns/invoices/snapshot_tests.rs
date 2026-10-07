use super::*;
use crate::server_fns::projects::fetch_project_fee_balances;
use crate::server_fns::test_seed::{SeedIds, seed};
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
        Reader::Fees => {
            fetch_project_fee_balances(pool, identity.0, identity.1, ids.project_id, period)
                .await
                .map(|balances| balances[0].balance.remaining_cents)
        }
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

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_require_current_same_tenant_manager(pool: PgPool) {
    let ids = fee_tests::single_fee(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for reader in [Reader::Invoice, Reader::Fees] {
        read(&pool, (ids.org_id, ids.user_id), &ids, reader)
            .await
            .unwrap();
        assert_denied(read(&pool, (ids.org_id, Uuid::now_v7()), &ids, reader).await);
        assert_denied(read(&pool, (ids.org_id, foreign.user_id), &ids, reader).await);
        assert_denied(read(&pool, (Uuid::now_v7(), ids.user_id), &ids, reader).await);
        sqlx::query!(
            "UPDATE users SET org_role='member' WHERE id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
        sqlx::query!(
            "UPDATE users SET org_role='admin', active=false WHERE id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        read(&pool, (ids.org_id, ids.user_id), &ids, reader)
            .await
            .unwrap();
    }
}
