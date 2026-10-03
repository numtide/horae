use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug)]
enum EditorRead {
    Load,
    Review,
}

struct Case {
    ids: SeedIds,
    invoice_id: Uuid,
    edit: InvoiceDraftEdit,
}

impl Case {
    async fn new(pool: &PgPool) -> Self {
        let ids = fee_tests::single_fee(pool).await;
        let invoice = generate_invoice_for_period(
            pool,
            ids.org_id,
            ids.client_id,
            "2026-09-01".parse().unwrap(),
            "2026-09-30".parse().unwrap(),
        )
        .await
        .unwrap();
        let invoice_id = invoice.invoice.id;
        let edit = editing::load(pool, ids.org_id, ids.user_id, invoice_id)
            .await
            .unwrap()
            .edit;
        Self {
            ids,
            invoice_id,
            edit,
        }
    }

    async fn read(
        &self,
        pool: &PgPool,
        (org_id, actor_id): (Uuid, Uuid),
        reader: EditorRead,
    ) -> Result<Value, ServerFnError> {
        match reader {
            EditorRead::Load => editing::load(pool, org_id, actor_id, self.invoice_id)
                .await
                .map(|value| serde_json::to_value(value).unwrap()),
            EditorRead::Review => {
                editing::review(pool, org_id, actor_id, self.invoice_id, &self.edit)
                    .await
                    .map(|value| serde_json::to_value(value).unwrap())
            }
        }
    }

    async fn stored(&self, pool: &PgPool) -> Value {
        sqlx::query_scalar!(r#"SELECT jsonb_build_object(
            'invoice', (SELECT to_jsonb(i) FROM invoices i WHERE id=$1),
            'lines', (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l WHERE invoice_id=$1),
            'fees', (SELECT jsonb_agg(to_jsonb(f) ORDER BY id) FROM project_fee_occurrences f WHERE org_id=$2),
            'time', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t WHERE org_id=$2),
            'receipts', (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM invoice_edit_requests r WHERE org_id=$2)
        ) AS "snapshot!""#, self.invoice_id, self.ids.org_id).fetch_one(pool).await.unwrap()
    }
}

fn assert_status(result: Result<Value, ServerFnError>, status: u16) {
    assert!(
        matches!(result, Err(ServerFnError::ServerError { code, .. }) if code == status),
        "{result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_snapshots_preserve_rows_and_override_inherited_read_only(pool: PgPool) {
    let case = Case::new(&pool).await;
    let before = case.stored(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only=on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation='serializable'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let foreign = seed(&pool, OrgRole::Admin).await;
    for reader in [EditorRead::Load, EditorRead::Review] {
        let expected = case
            .read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
            .unwrap();
        assert_eq!(
            case.read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader)
                .await
                .unwrap(),
            expected
        );
        for actor in [Uuid::now_v7(), foreign.user_id] {
            assert_status(
                case.read(&reader_pool, (case.ids.org_id, actor), reader)
                    .await,
                FORBIDDEN,
            );
        }
        assert_status(
            case.read(&reader_pool, (foreign.org_id, foreign.user_id), reader)
                .await,
            NOT_FOUND,
        );
        assert_eq!(
            case.read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader)
                .await
                .unwrap(),
            expected
        );
    }
    assert_eq!(case.stored(&pool).await, before);
    let settings = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('lock_timeout') AS \"timeout!\"")
        .fetch_one(&reader_pool).await.unwrap();
    assert_eq!(
        (settings.isolation.as_str(), settings.read_only.as_str()),
        ("serializable", "on")
    );
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_snapshots_deny_winning_legacy_revocation(pool: PgPool) {
    for reader in [EditorRead::Load, EditorRead::Review] {
        for gated in [false, true] {
            for deactivate in [false, true] {
                let case = Arc::new(Case::new(&pool).await);
                let before = case.stored(&pool).await;
                let mut writer = pool.begin().await.unwrap();
                let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
                    .fetch_one(&mut *writer)
                    .await
                    .unwrap();
                if gated {
                    crate::db::lock_organization(
                        &mut writer,
                        case.ids.org_id,
                        crate::db::OrganizationLock::AccessChange,
                    )
                    .await
                    .unwrap();
                }
                if deactivate {
                    sqlx::query!(
                        "UPDATE users SET active=false WHERE id=$1",
                        case.ids.user_id
                    )
                    .execute(&mut *writer)
                    .await
                    .unwrap();
                } else {
                    sqlx::query!(
                        "UPDATE users SET org_role='member' WHERE id=$1",
                        case.ids.user_id
                    )
                    .execute(&mut *writer)
                    .await
                    .unwrap();
                }
                let pending = {
                    let pool = pool.clone();
                    let case = Arc::clone(&case);
                    tokio::spawn(async move {
                        case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                            .await
                    })
                };
                wait_for_blocked(&pool, pid).await;
                writer.commit().await.unwrap();
                assert_status(
                    tokio::time::timeout(Duration::from_secs(5), pending)
                        .await
                        .unwrap()
                        .unwrap(),
                    FORBIDDEN,
                );
                assert_eq!(
                    sqlx::query_scalar!(
                        "SELECT access_revision FROM organizations WHERE id=$1",
                        case.ids.org_id
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                    0
                );
                assert_eq!(case.stored(&pool).await, before);
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_snapshots_refresh_after_revision_change_without_rebasing_edits(pool: PgPool) {
    for reader in [EditorRead::Load, EditorRead::Review] {
        let case = Arc::new(Case::new(&pool).await);
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            case.ids.org_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let case = Arc::clone(&case);
            tokio::spawn(async move {
                case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                    .await
            })
        };
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "UPDATE invoices SET po_number='New PO' WHERE id=$1",
            case.invoice_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap();
        match reader {
            EditorRead::Load => {
                let loaded = result.unwrap();
                assert_eq!(loaded["edit"]["defaults"]["po_number"], "New PO");
                assert!(loaded["edit"]["revision"].as_i64().unwrap() > case.edit.revision);
            }
            EditorRead::Review => assert_status(result, CONFLICT),
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_snapshots_retain_authority_and_release_cancelled_reads(pool: PgPool) {
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for reader in [EditorRead::Load, EditorRead::Review] {
        for cancel in [false, true] {
            let case = Arc::new(Case::new(&pool).await);
            let before = case.stored(&pool).await;
            let expected = case
                .read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader)
                .await
                .unwrap();
            let mut blocker = pool.begin().await.unwrap();
            sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await
                .unwrap();
            let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
            let pending = {
                let pool = reader_pool.clone();
                let case = Arc::clone(&case);
                tokio::spawn(async move {
                    case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                        .await
                })
            };
            wait_for_blocked(&pool, pid).await;
            let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", pid).fetch_one(&pool).await.unwrap().unwrap();
            let revoke = {
                let pool = pool.clone();
                let case = Arc::clone(&case);
                tokio::spawn(async move {
                    let mut tx = pool.begin().await.unwrap();
                    crate::db::lock_organization(
                        &mut tx,
                        case.ids.org_id,
                        crate::db::OrganizationLock::AccessChange,
                    )
                    .await
                    .unwrap();
                    sqlx::query!(
                        "UPDATE users SET active=false WHERE id=$1",
                        case.ids.user_id
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                    tx.commit().await.unwrap();
                })
            };
            wait_for_blocked(&pool, reader_pid).await;
            if cancel {
                pending.abort();
                assert!(pending.await.unwrap_err().is_cancelled());
                blocker.rollback().await.unwrap();
            } else {
                blocker.rollback().await.unwrap();
                assert_eq!(
                    tokio::time::timeout(Duration::from_secs(5), pending)
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap(),
                    expected
                );
            }
            tokio::time::timeout(Duration::from_secs(5), revoke)
                .await
                .unwrap()
                .unwrap();
            assert_status(
                tokio::time::timeout(
                    Duration::from_secs(5),
                    case.read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader),
                )
                .await
                .unwrap(),
                FORBIDDEN,
            );
            assert_eq!(case.stored(&pool).await, before);
            let mut writer = pool.begin().await.unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
                case.ids.org_id
            )
            .fetch_one(&mut *writer)
            .await
            .unwrap();
            sqlx::query!(
                "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
                case.ids.user_id
            )
            .fetch_one(&mut *writer)
            .await
            .unwrap();
            writer.rollback().await.unwrap();
        }
    }
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn editor_snapshots_keep_invoice_metadata_revision_and_lines_together(pool: PgPool) {
    for reader in [EditorRead::Load, EditorRead::Review] {
        let case = Arc::new(Case::new(&pool).await);
        let expected = case
            .read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
            .unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *writer)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let case = Arc::clone(&case);
            tokio::spawn(async move {
                case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                    .await
            })
        };
        wait_for_blocked(&pool, pid).await;
        sqlx::query!("UPDATE invoice_line_items SET amount_cents=15000, description='Revised fee' WHERE invoice_id=$1", case.invoice_id).execute(&mut *writer).await.unwrap();
        sqlx::query!(
            "UPDATE invoices SET total_cents=15000, po_number='New PO' WHERE id=$1",
            case.invoice_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            expected
        );
        let loaded = editing::load(&pool, case.ids.org_id, case.ids.user_id, case.invoice_id)
            .await
            .unwrap();
        assert_eq!(loaded.edit.defaults.po_number, "New PO");
        assert_eq!(loaded.edit.fees[0].amount_cents, 15000);
        assert_eq!(loaded.edit.fees[0].description, "Revised fee");
        assert_eq!(loaded.review.amounts.total_cents, 15000);
        assert_eq!(loaded.review.fees[0].excess_cents, 2500);
        assert!(loaded.edit.revision > case.edit.revision);
        assert_status(
            case.read(
                &pool,
                (case.ids.org_id, case.ids.user_id),
                EditorRead::Review,
            )
            .await,
            CONFLICT,
        );
    }
}
