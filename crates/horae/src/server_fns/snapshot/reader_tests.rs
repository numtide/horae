use super::*;
use crate::server_fns::{
    invoices, reports,
    test_seed::{SeedIds, seed, time_entry, wait_for_blocked},
};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::{EntryState, InvoiceStatus};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Copy, Debug)]
enum Reader {
    Detailed,
    List,
    Invoice,
}

const READERS: [Reader; 3] = [Reader::Detailed, Reader::List, Reader::Invoice];

struct Fixture {
    ids: SeedIds,
    invoice: Uuid,
}

impl Fixture {
    async fn new(pool: &PgPool) -> Self {
        let ids = seed(pool, BuiltInProfile::ExecutiveManager).await;
        let entry = time_entry(pool, &ids, EntryState::Open).await;
        let invoice = Uuid::now_v7();
        sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents) VALUES ($1,$2,$3,'PRIVATE-1','2026-09-07','2026-10-07','EUR',1200)", invoice, ids.org_id, ids.client_id).execute(pool).await.unwrap();
        sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents) VALUES ($1,$2,$3,'Private line',60,1200,1200)", Uuid::now_v7(), invoice, entry).execute(pool).await.unwrap();
        Self { ids, invoice }
    }

    async fn read(
        &self,
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        reader: Reader,
    ) -> Result<Value, ServerFnError> {
        Ok(match reader {
            Reader::Detailed => json!(
                reports::fetch_detailed(
                    pool,
                    org,
                    actor,
                    ("2026-09-07".parse().unwrap(), "2026-09-07".parse().unwrap()),
                    crate::reports::ReportFilters::default()
                )
                .await?
            ),
            Reader::List => json!(invoices::fetch_list(pool, org, actor, None).await?),
            Reader::Invoice => json!(invoices::fetch_detail(pool, org, actor, self.invoice).await?),
        })
    }

    fn spawn(
        self: &Arc<Self>,
        pool: &PgPool,
        reader: Reader,
    ) -> tokio::task::JoinHandle<Result<Value, ServerFnError>> {
        let fixture = Arc::clone(self);
        let pool = pool.clone();
        tokio::spawn(async move {
            fixture
                .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
                .await
        })
    }
}

fn denied(result: Result<Value, ServerFnError>) {
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: super::super::FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
}

async fn pid(tx: &mut Transaction<'_, Postgres>) -> i32 {
    sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

async fn block_payload(tx: &mut Transaction<'_, Postgres>, reader: Reader) {
    match reader {
        Reader::Detailed => sqlx::query!("LOCK TABLE project_tasks IN ACCESS EXCLUSIVE MODE")
            .execute(&mut **tx)
            .await
            .unwrap(),
        Reader::List => sqlx::query!("LOCK TABLE invoices IN ACCESS EXCLUSIVE MODE")
            .execute(&mut **tx)
            .await
            .unwrap(),
        Reader::Invoice => sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
            .execute(&mut **tx)
            .await
            .unwrap(),
    };
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_reject_revoked_session_identity(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let mut leaks = Vec::new();
    for reader in READERS {
        for inactive in [false, true] {
            // IDs were captured by the session gate while this actor was Manager.
            let role = if inactive {
                OrgRole::Manager
            } else {
                OrgRole::Member
            };
            sqlx::query!(
                "UPDATE users SET org_role=$2, active=$3 WHERE id=$1",
                fixture.ids.user_id,
                role as OrgRole,
                !inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            let result = fixture
                .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
                .await;
            if result.is_ok() {
                leaks.push((reader, inactive));
            } else {
                denied(result);
            }
        }
    }
    assert!(
        leaks.is_empty(),
        "Readers disclosed payload after revocation (reader, inactive): {leaks:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_preserve_actor_tenant_filters_and_missing_resources(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let foreign = Fixture::new(&pool).await;
    for reader in READERS {
        fixture
            .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
            .await
            .unwrap();
        for (org, actor) in [
            (fixture.ids.org_id, foreign.ids.user_id),
            (fixture.ids.org_id, Uuid::now_v7()),
            (Uuid::now_v7(), fixture.ids.user_id),
        ] {
            denied(fixture.read(&pool, org, actor, reader).await);
        }
    }
    let list = invoices::fetch_list(
        &pool,
        fixture.ids.org_id,
        fixture.ids.user_id,
        Some(InvoiceStatus::Sent),
    )
    .await
    .unwrap();
    assert!(list.is_empty());
    for id in [foreign.invoice, Uuid::now_v7()] {
        let result =
            invoices::fetch_detail(&pool, fixture.ids.org_id, fixture.ids.user_id, id).await;
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: super::super::NOT_FOUND,
                    ..
                })
            ),
            "{result:?}"
        );
    }
    let rows = reports::fetch_detailed(
        &pool,
        fixture.ids.org_id,
        fixture.ids.user_id,
        ("2026-09-07".parse().unwrap(), "2026-09-07".parse().unwrap()),
        crate::reports::ReportFilters {
            project_id: Some(foreign.ids.project_id),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(rows.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_deny_writer_first_revocation(pool: PgPool) {
    for reader in READERS {
        for gated in [false, true] {
            for deactivate in [false, true] {
                let fixture = Arc::new(Fixture::new(&pool).await);
                let mut writer = pool.begin().await.unwrap();
                if gated {
                    sqlx::query!(
                        "SELECT id FROM organizations WHERE id = $1 FOR NO KEY UPDATE",
                        fixture.ids.org_id
                    )
                    .fetch_one(&mut *writer)
                    .await
                    .unwrap();
                }
                if deactivate {
                    sqlx::query!(
                        "UPDATE users SET active=false WHERE id=$1",
                        fixture.ids.user_id
                    )
                    .execute(&mut *writer)
                    .await
                    .unwrap();
                } else {
                    sqlx::query!(
                        "UPDATE users SET org_role='member' WHERE id=$1",
                        fixture.ids.user_id
                    )
                    .execute(&mut *writer)
                    .await
                    .unwrap();
                }
                let blocker = pid(&mut writer).await;
                let pending = fixture.spawn(&pool, reader);
                wait_for_blocked(&pool, blocker).await;
                writer.commit().await.unwrap();
                denied(
                    tokio::time::timeout(Duration::from_secs(5), pending)
                        .await
                        .unwrap()
                        .unwrap(),
                );
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_hold_reader_first_authority_until_materialized(pool: PgPool) {
    for reader in READERS {
        let fixture = Arc::new(Fixture::new(&pool).await);
        let expected = fixture
            .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
            .await
            .unwrap();
        let mut blocker = pool.begin().await.unwrap();
        block_payload(&mut blocker, reader).await;
        let blocker_pid = pid(&mut blocker).await;
        let pending = fixture.spawn(&pool, reader);
        wait_for_blocked(&pool, blocker_pid).await;
        let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker_pid).fetch_one(&pool).await.unwrap().unwrap();
        let revoke = {
            let pool = pool.clone();
            let actor = fixture.ids.user_id;
            tokio::spawn(async move {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", actor)
                    .execute(&pool)
                    .await
                    .unwrap();
            })
        };
        wait_for_blocked(&pool, reader_pid).await;
        blocker.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            expected
        );
        tokio::time::timeout(Duration::from_secs(5), revoke)
            .await
            .unwrap()
            .unwrap();
        denied(
            fixture
                .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
                .await,
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_invoice_detail_keeps_metadata_and_lines_in_one_snapshot(pool: PgPool) {
    let fixture = Arc::new(Fixture::new(&pool).await);
    let expected = fixture
        .read(
            &pool,
            fixture.ids.org_id,
            fixture.ids.user_id,
            Reader::Invoice,
        )
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    block_payload(&mut writer, Reader::Invoice).await;
    let blocker = pid(&mut writer).await;
    let pending = fixture.spawn(&pool, Reader::Invoice);
    wait_for_blocked(&pool, blocker).await;
    sqlx::query!(
        "UPDATE invoices SET number='PRIVATE-2',total_cents=2400 WHERE id=$1",
        fixture.invoice
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    sqlx::query!("UPDATE invoice_line_items SET description='Changed line',rate_cents=2400,amount_cents=2400 WHERE invoice_id=$1", fixture.invoice).execute(&mut *writer).await.unwrap();
    writer.commit().await.unwrap();
    assert_eq!(pending.await.unwrap().unwrap(), expected);
    let fresh = fixture
        .read(
            &pool,
            fixture.ids.org_id,
            fixture.ids.user_id,
            Reader::Invoice,
        )
        .await
        .unwrap();
    assert_eq!(fresh["invoice"]["total_cents"], 2400);
    assert_eq!(fresh["lines"][0]["amount_cents"], 2400);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_cancel_without_retaining_authority_or_pool_connection(pool: PgPool) {
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for reader in READERS {
        let fixture = Arc::new(Fixture::new(&pool).await);
        let mut blocker = pool.begin().await.unwrap();
        block_payload(&mut blocker, reader).await;
        let blocker_pid = pid(&mut blocker).await;
        let pending = fixture.spawn(&reader_pool, reader);
        wait_for_blocked(&pool, blocker_pid).await;
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        blocker.rollback().await.unwrap();
        tokio::time::timeout(
            Duration::from_secs(5),
            fixture.read(
                &reader_pool,
                fixture.ids.org_id,
                fixture.ids.user_id,
                reader,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
            fixture.ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            fixture.ids.user_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        writer.rollback().await.unwrap();
    }
    reader_pool.close().await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn legacy_readers_preserve_inherited_connection_settings(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for isolation in ["read committed", "repeatable read", "serializable"] {
        sqlx::query!("SELECT set_config('default_transaction_isolation', $1, false) AS isolation, set_config('default_transaction_read_only', 'on', false) AS read_only, set_config('lock_timeout', '250ms', false) AS timeout", isolation).fetch_one(&reader_pool).await.unwrap();
        for reader in READERS {
            let expected = fixture
                .read(&pool, fixture.ids.org_id, fixture.ids.user_id, reader)
                .await
                .unwrap();
            assert_eq!(
                fixture
                    .read(
                        &reader_pool,
                        fixture.ids.org_id,
                        fixture.ids.user_id,
                        reader
                    )
                    .await
                    .unwrap(),
                expected
            );
            denied(
                fixture
                    .read(&reader_pool, fixture.ids.org_id, Uuid::now_v7(), reader)
                    .await,
            );
        }
        let settings = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('lock_timeout') AS \"timeout!\"").fetch_one(&reader_pool).await.unwrap();
        assert_eq!(
            (
                settings.isolation.as_str(),
                settings.read_only.as_str(),
                settings.timeout.as_str()
            ),
            (isolation, "on", "250ms")
        );
    }
    reader_pool.close().await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn manager_commits_asynchronously_only_inside_its_snapshot(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    // The server's own commit default first, then `on` forced for the session,
    // so the difference is proven even where the server runs with
    // `synchronous_commit=off` (the CI cluster does). The single connection
    // keeps the session setting, hence this order.
    for forced in [false, true] {
        if forced {
            sqlx::query("SET synchronous_commit = on")
                .execute(&reader_pool)
                .await
                .unwrap();
        }
        let commit = sqlx::query_scalar::<_, String>("SHOW synchronous_commit")
            .fetch_one(&reader_pool)
            .await
            .unwrap();
        if forced {
            assert_eq!(commit, "on");
        }
        for inherited in ["0", "250ms", "7s"] {
            sqlx::query!("SELECT set_config('lock_timeout', $1, false)", inherited)
                .fetch_one(&reader_pool)
                .await
                .unwrap();
            let mut tx = manager(&reader_pool, fixture.ids.org_id, fixture.ids.user_id)
                .await
                .unwrap();
            let inside = sqlx::query!("SELECT current_setting('transaction_isolation') AS \"isolation!\", current_setting('transaction_read_only') AS \"read_only!\", current_setting('lock_timeout') AS \"timeout!\", current_setting('synchronous_commit') AS \"commit!\"").fetch_one(&mut *tx).await.unwrap();
            assert_eq!(
                (
                    inside.isolation.as_str(),
                    inside.read_only.as_str(),
                    inside.timeout.as_str(),
                    inside.commit.as_str()
                ),
                ("repeatable read", "off", inherited, "off")
            );
            tx.commit().await.unwrap();
            let after = sqlx::query!("SELECT current_setting('lock_timeout') AS \"timeout!\", current_setting('synchronous_commit') AS \"commit!\"").fetch_one(&reader_pool).await.unwrap();
            assert_eq!(
                (after.timeout.as_str(), after.commit.as_str()),
                (inherited, commit.as_str())
            );
        }
    }
    reader_pool.close().await;
}

/// With no inherited lock_timeout, the organization `FOR SHARE` gives up after
/// the 5 s bound (55P03) instead of waiting for the holder. `manager` reports
/// that as an internal error, so the bound shows as the wait ending while the
/// blocker still holds the row.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn manager_bounds_its_authority_lock_wait(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        fixture.ids.org_id
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    let started = std::time::Instant::now();
    let result = manager(&pool, fixture.ids.org_id, fixture.ids.user_id)
        .await
        .map(|_| ());
    let waited = started.elapsed();
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: super::super::INTERNAL_ERROR,
                ..
            })
        ),
        "{result:?}"
    );
    assert!(
        (Duration::from_millis(4900)..Duration::from_secs(8)).contains(&waited),
        "{waited:?}"
    );
    hold.rollback().await.unwrap();
}
