use super::*;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

/// (connection default, expected local statement_timeout, expected local
/// idle_in_transaction_session_timeout). Covers each unit Postgres prints for
/// millisecond settings, so the bound never depends on how a value is spelled.
const CASES: [(&str, &str, &str); 7] = [
    ("0", "5s", "10s"),
    ("250ms", "250ms", "250ms"),
    ("3s", "3s", "3s"),
    ("2min", "5s", "10s"),
    ("1h", "5s", "10s"),
    ("7d", "5s", "10s"),
    ("7500ms", "5s", "7500ms"),
];

async fn shown(connection: &mut PgConnection, name: &str) -> String {
    sqlx::query_scalar::<_, String>(&format!("SHOW {name}"))
        .fetch_one(connection)
        .await
        .unwrap()
}

/// One connection, so every check after COMMIT sees the same pooled session.
async fn single_connection(
    pool_opts: &PgPoolOptions,
    connect_opts: &PgConnectOptions,
    default: &str,
) -> PgPool {
    pool_opts
        .clone()
        .max_connections(1)
        .connect_with(connect_opts.clone().options([
            ("statement_timeout", default),
            ("idle_in_transaction_session_timeout", default),
        ]))
        .await
        .unwrap()
}

/// The server's own default, then `on` forced for the session, so the
/// asynchronous/durable difference is proven even where the server default is
/// already `off` (the CI cluster runs with `synchronous_commit=off`).
const COMMIT_DEFAULTS: [Option<&str>; 2] = [None, Some("on")];

/// Apply `forced` at session level and return the connection's commit default,
/// read before any transaction begins.
async fn commit_default(pool: &PgPool, forced: Option<&str>) -> String {
    let mut conn = pool.acquire().await.unwrap();
    if let Some(value) = forced {
        sqlx::query(&format!("SET synchronous_commit = {value}"))
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    let default = shown(&mut conn, "synchronous_commit").await;
    if let Some(value) = forced {
        assert_eq!(default, value);
    }
    default
}

async fn assert_limits(connection: &mut PgConnection, default: &str, statement: &str, idle: &str) {
    assert_eq!(
        shown(connection, "statement_timeout").await,
        statement,
        "{default}"
    );
    assert_eq!(
        shown(connection, "idle_in_transaction_session_timeout").await,
        idle,
        "{default}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn lock_only_reads_bound_limits_and_commit_asynchronously(
    pool_opts: PgPoolOptions,
    connect_opts: PgConnectOptions,
) {
    for forced in COMMIT_DEFAULTS {
        for (default, statement, idle) in CASES {
            let pool = single_connection(&pool_opts, &connect_opts, default).await;
            let commit = commit_default(&pool, forced).await;
            for (isolation, shown_isolation) in [
                (ReadIsolation::ReadCommitted, "read committed"),
                (ReadIsolation::RepeatableRead, "repeatable read"),
            ] {
                let mut tx = begin_lock_only_read(&pool, isolation).await.unwrap();
                assert_eq!(
                    shown(&mut tx, "transaction_isolation").await,
                    shown_isolation
                );
                assert_eq!(shown(&mut tx, "transaction_read_only").await, "off");
                assert_limits(&mut tx, default, statement, idle).await;
                assert_eq!(shown(&mut tx, "synchronous_commit").await, "off");
                tx.commit().await.unwrap();

                // Every setting is transaction-local: after COMMIT the same
                // pooled connection is back to its defaults.
                let mut conn = pool.acquire().await.unwrap();
                assert_limits(&mut conn, default, default, default).await;
                assert_eq!(shown(&mut conn, "synchronous_commit").await, commit);
            }
            pool.close().await;
        }
    }
}

/// Transactions that may write data never set the commit mode locally: they
/// keep the connection's default, durable unless the server says otherwise.
#[sqlx::test(migrations = "./migrations")]
async fn administration_transactions_bound_limits_and_commit_durably(
    pool_opts: PgPoolOptions,
    connect_opts: PgConnectOptions,
) {
    for forced in COMMIT_DEFAULTS {
        for (default, statement, idle) in CASES {
            let pool = single_connection(&pool_opts, &connect_opts, default).await;
            let commit = commit_default(&pool, forced).await;
            let mut tx = pool.begin().await.unwrap();
            configure_administration(&mut tx).await.unwrap();
            assert_eq!(
                shown(&mut tx, "transaction_isolation").await,
                "read committed"
            );
            assert_limits(&mut tx, default, statement, idle).await;
            assert_eq!(shown(&mut tx, "synchronous_commit").await, commit);
            tx.commit().await.unwrap();

            let mut tx = pool.begin().await.unwrap();
            configure_transaction_limits(&mut tx).await.unwrap();
            assert_limits(&mut tx, default, statement, idle).await;
            assert_eq!(shown(&mut tx, "synchronous_commit").await, commit);
            tx.commit().await.unwrap();

            let mut conn = pool.acquire().await.unwrap();
            assert_limits(&mut conn, default, default, default).await;
            assert_eq!(shown(&mut conn, "synchronous_commit").await, commit);
            // `close` waits for every checked-out connection.
            drop(conn);
            pool.close().await;
        }
    }
}
