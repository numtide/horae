use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug)]
enum Reader {
    Entries,
    Invoice,
    Pdf,
}

const READERS: [Reader; 3] = [Reader::Entries, Reader::Invoice, Reader::Pdf];

struct Case {
    ids: SeedIds,
    invoice_id: Uuid,
}

impl Case {
    async fn new(pool: &PgPool) -> Self {
        let ids = seed(pool, OrgRole::Manager).await;
        let (invoice_id, _) = add_invoice(pool, &ids, 1).await;
        Self { ids, invoice_id }
    }

    async fn read(
        &self,
        pool: &PgPool,
        (org, actor): (Uuid, Uuid),
        reader: Reader,
    ) -> Result<Value, StatusCode> {
        Ok(match reader {
            Reader::Entries => json!(entries(pool, org, actor, &params()).await?),
            Reader::Invoice => json!(invoice(pool, org, actor, self.invoice_id).await?),
            Reader::Pdf => {
                let doc = pdf(pool, org, actor, self.invoice_id).await?;
                json!({"invoice":doc.invoice, "lines":doc.lines, "client":doc.client_name,
                    "address":doc.client_address, "tax_id":doc.client_tax_id, "branding":doc.branding})
            }
        })
    }

    async fn stored(&self, pool: &PgPool) -> Value {
        sqlx::query_scalar!(r#"SELECT jsonb_build_object(
            'invoice', (SELECT to_jsonb(i) FROM invoices i WHERE id=$1),
            'lines', (SELECT jsonb_agg(to_jsonb(l) ORDER BY id) FROM invoice_line_items l WHERE invoice_id=$1),
            'time', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM time_entries t WHERE org_id=$2)
        ) AS "snapshot!""#, self.invoice_id, self.ids.org_id).fetch_one(pool).await.unwrap()
    }
}

async fn block_payload(pool: &PgPool, reader: Reader) -> (Transaction<'_, Postgres>, i32) {
    let mut tx = pool.begin().await.unwrap();
    match reader {
        Reader::Entries => sqlx::query!("LOCK TABLE project_tasks IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .unwrap(),
        Reader::Invoice | Reader::Pdf => {
            sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *tx)
                .await
                .unwrap()
        }
    };
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    (tx, pid)
}

fn spawn_read(
    pool: &PgPool,
    case: &Arc<Case>,
    reader: Reader,
) -> tokio::task::JoinHandle<Result<Value, StatusCode>> {
    let pool = pool.clone();
    let case = Arc::clone(case);
    tokio::spawn(async move {
        case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
    })
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_deny_revoked_managers(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let (id, _) = add_invoice(&pool, &ids, 1).await;
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        entries(&pool, ids.org_id, ids.user_id, &params()).await,
        Err(StatusCode::FORBIDDEN)
    ));
    assert!(matches!(
        invoice(&pool, ids.org_id, ids.user_id, id).await,
        Err(StatusCode::FORBIDDEN)
    ));
    assert!(matches!(
        pdf(&pool, ids.org_id, ids.user_id, id).await,
        Err(StatusCode::FORBIDDEN)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_require_same_tenant_active_actor(pool: PgPool) {
    let case = Case::new(&pool).await;
    let before = case.stored(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for reader in READERS {
        for identity in [
            (case.ids.org_id, Uuid::now_v7()),
            (case.ids.org_id, foreign.user_id),
            (Uuid::now_v7(), case.ids.user_id),
        ] {
            assert_eq!(
                case.read(&pool, identity, reader).await,
                Err(StatusCode::FORBIDDEN)
            );
        }
        for role in [OrgRole::Manager, OrgRole::Admin] {
            sqlx::query!(
                "UPDATE users SET org_role=$2, active=true WHERE id=$1",
                case.ids.user_id,
                role as OrgRole
            )
            .execute(&pool)
            .await
            .unwrap();
            case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                .await
                .unwrap();
            sqlx::query!(
                "UPDATE users SET active=false WHERE id=$1",
                case.ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            assert_eq!(
                case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                    .await,
                Err(StatusCode::FORBIDDEN)
            );
        }
        let other = case
            .read(&pool, (foreign.org_id, foreign.user_id), reader)
            .await;
        match reader {
            Reader::Entries => assert_eq!(other.unwrap(), json!([])),
            _ => assert_eq!(other, Err(StatusCode::NOT_FOUND)),
        }
    }
    assert_eq!(case.stored(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_deny_winning_legacy_revocations(pool: PgPool) {
    for reader in READERS {
        for gated in [false, true] {
            for deactivate in [false, true] {
                let case = Arc::new(Case::new(&pool).await);
                let before = case.stored(&pool).await;
                let mut writer = pool.begin().await.unwrap();
                if gated {
                    crate::db::lock_organization(
                        &mut writer,
                        case.ids.org_id,
                        crate::db::OrganizationLock::AccessChange,
                    )
                    .await
                    .unwrap();
                }
                let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
                    .fetch_one(&mut *writer)
                    .await
                    .unwrap();
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
                let pending = spawn_read(&pool, &case, reader);
                wait_for_blocked(&pool, pid).await;
                writer.commit().await.unwrap();
                assert_eq!(
                    tokio::time::timeout(Duration::from_secs(5), pending)
                        .await
                        .unwrap()
                        .unwrap(),
                    Err(StatusCode::FORBIDDEN)
                );
                assert_eq!(case.stored(&pool).await, before);
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_refresh_after_access_revision_change(pool: PgPool) {
    for reader in READERS {
        let case = Arc::new(Case::new(&pool).await);
        let old = case
            .read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
            .unwrap();
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
        let pending = spawn_read(&pool, &case, reader);
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "UPDATE time_entries SET notes='Refreshed' WHERE org_id=$1",
            case.ids.org_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE invoices SET notes='Refreshed' WHERE id=$1",
            case.invoice_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        let current = case
            .read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
            .unwrap();
        assert_ne!(current, old);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            current
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_retain_authority_and_release_cancelled_reads(pool: PgPool) {
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for reader in READERS {
        for cancel in [false, true] {
            let case = Arc::new(Case::new(&pool).await);
            let before = case.stored(&pool).await;
            let expected = case
                .read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader)
                .await
                .unwrap();
            let (blocker, pid) = block_payload(&pool, reader).await;
            let pending = spawn_read(&reader_pool, &case, reader);
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
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_secs(5),
                    case.read(&reader_pool, (case.ids.org_id, case.ids.user_id), reader)
                )
                .await
                .unwrap(),
                Err(StatusCode::FORBIDDEN)
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
async fn materialized_exports_preserve_checked_snapshots(pool: PgPool) {
    for reader in READERS {
        let case = Arc::new(Case::new(&pool).await);
        let before = case
            .read(&pool, (case.ids.org_id, case.ids.user_id), reader)
            .await
            .unwrap();
        let (mut writer, pid) = block_payload(&pool, reader).await;
        let pending = spawn_read(&pool, &case, reader);
        wait_for_blocked(&pool, pid).await;
        match reader {
            Reader::Entries => {
                sqlx::query!(
                    "UPDATE time_entries SET notes=repeat('x',40000) WHERE org_id=$1",
                    case.ids.org_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            Reader::Invoice => {
                sqlx::query!(
                    "UPDATE invoices SET notes=repeat('x',40000) WHERE id=$1",
                    case.invoice_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            Reader::Pdf => {
                sqlx::query!(
                    "UPDATE clients SET address=repeat('x',40000) WHERE id=$1",
                    case.ids.client_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
        }
        writer.commit().await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), pending)
            .await
            .unwrap()
            .unwrap();
        // Time exports bind size and payload to one statement, not an earlier
        // transaction snapshot. Either coherent snapshot must enforce the limit.
        match (reader, result) {
            (Reader::Entries, Err(StatusCode::PAYLOAD_TOO_LARGE)) => {}
            (_, result) => assert_eq!(result.unwrap(), before),
        }
        assert_eq!(
            case.read(&pool, (case.ids.org_id, case.ids.user_id), reader)
                .await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_keep_deadlines_and_restore_pool_defaults(pool: PgPool) {
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
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET lock_timeout='250ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for reader in READERS {
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
    }
    let mut tx = begin_manager(&reader_pool, case.ids.org_id, case.ids.user_id)
        .await
        .unwrap();
    let settings = sqlx::query!(r#"SELECT current_setting('transaction_isolation') AS "isolation!", current_setting('transaction_read_only') AS "read_only!", current_setting('statement_timeout') AS "statement!", current_setting('idle_in_transaction_session_timeout') AS "idle!", current_setting('lock_timeout') AS "lock!""#).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(
        (
            settings.isolation.as_str(),
            settings.read_only.as_str(),
            settings.statement.as_str(),
            settings.idle.as_str(),
            settings.lock.as_str()
        ),
        ("repeatable read", "off", "5s", "10s", "250ms")
    );
    let error = sqlx::query!("DO $$ BEGIN PERFORM pg_sleep(10); END $$")
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(database_error(error), StatusCode::GATEWAY_TIMEOUT);
    tx.rollback().await.unwrap();
    let settings = sqlx::query!(r#"SELECT current_setting('transaction_isolation') AS "isolation!", current_setting('transaction_read_only') AS "read_only!", current_setting('statement_timeout') AS "statement!", current_setting('idle_in_transaction_session_timeout') AS "idle!", current_setting('lock_timeout') AS "lock!""#).fetch_one(&reader_pool).await.unwrap();
    assert_eq!(
        (
            settings.isolation.as_str(),
            settings.read_only.as_str(),
            settings.statement.as_str(),
            settings.idle.as_str(),
            settings.lock.as_str()
        ),
        ("serializable", "on", "0", "0", "250ms")
    );
    assert_eq!(case.stored(&pool).await, before);
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_recheck_after_render_without_retaining_authority(pool: PgPool) {
    use crate::reports::{bounded::ExportPermit, render_manager_export};
    for gated in [false, true] {
        for deactivate in [false, true] {
            let case = Case::new(&pool).await;
            let (started, ready) = tokio::sync::oneshot::channel();
            let (release, wait) = std::sync::mpsc::channel();
            let pending = {
                let pool = pool.clone();
                let org = case.ids.org_id;
                let actor = case.ids.user_id;
                tokio::spawn(async move {
                    let _rows = entries(&pool, org, actor, &params()).await.unwrap();
                    render_manager_export(
                        ExportPermit::acquire().unwrap(),
                        &pool,
                        org,
                        actor,
                        move || {
                            started.send(()).unwrap();
                            wait.recv_timeout(Duration::from_secs(5)).unwrap();
                            Ok(b"private export".to_vec())
                        },
                    )
                    .await
                })
            };
            ready.await.unwrap();
            let mut writer = pool.begin().await.unwrap();
            if gated {
                sqlx::query!(
                    "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
                    case.ids.org_id
                )
                .fetch_one(&mut *writer)
                .await
                .unwrap();
            }
            sqlx::query!(
                "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
                case.ids.user_id
            )
            .fetch_one(&mut *writer)
            .await
            .unwrap();
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
            writer.commit().await.unwrap();
            release.send(()).unwrap();
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(5), pending)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StatusCode::FORBIDDEN)
            ));
            let first = ExportPermit::acquire().unwrap();
            let second = ExportPermit::acquire().unwrap();
            drop((first, second));
        }
    }
    let ids = seed(&pool, OrgRole::Admin).await;
    let body = render_manager_export(
        ExportPermit::acquire().unwrap(),
        &pool,
        ids.org_id,
        ids.user_id,
        || Ok(b"authorized export".to_vec()),
    )
    .await
    .unwrap();
    assert_eq!(
        axum::body::to_bytes(body, 100).await.unwrap().as_ref(),
        b"authorized export"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn materialized_exports_release_rendered_body_when_final_check_is_interrupted(pool: PgPool) {
    use crate::reports::{bounded::ExportPermit, render_manager_export};
    let ids = seed(&pool, OrgRole::Manager).await;
    for cancel in [false, true] {
        let lock_timeout = if cancel { "0" } else { "250ms" };
        let reader_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .after_connect(move |connection, _| {
                Box::pin(async move {
                    sqlx::query!("SELECT set_config('lock_timeout', $1, false)", lock_timeout)
                        .fetch_one(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let pending = {
            let pool = reader_pool.clone();
            let org = ids.org_id;
            let actor = ids.user_id;
            tokio::spawn(async move {
                render_manager_export(
                    ExportPermit::acquire().unwrap(),
                    &pool,
                    org,
                    actor,
                    move || {
                        started.send(()).unwrap();
                        wait.recv_timeout(Duration::from_secs(5)).unwrap();
                        Ok(b"private export".to_vec())
                    },
                )
                .await
            })
        };
        ready.await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        release.send(()).unwrap();
        wait_for_blocked(&pool, pid).await;
        if cancel {
            pending.abort();
            assert!(pending.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                tokio::time::timeout(Duration::from_secs(5), pending)
                    .await
                    .unwrap()
                    .unwrap(),
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            ));
        }
        writer.rollback().await.unwrap();
        let first = ExportPermit::acquire().unwrap();
        let second = ExportPermit::acquire().unwrap();
        drop((first, second));
        tokio::time::timeout(Duration::from_secs(5), async {
            begin_manager(&reader_pool, ids.org_id, ids.user_id)
                .await
                .unwrap()
                .commit()
                .await
                .unwrap();
        })
        .await
        .unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            ids.user_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        writer.rollback().await.unwrap();
        reader_pool.close().await;
    }
}
