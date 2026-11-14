//! `begin` / `WriteTx::commit`: events leave only after a successful commit.

use std::sync::Mutex;

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::{Connection, PgPool};
use uuid::Uuid;

use super::testing::Recorder;
use super::*;
use crate::plugin::event::ClientPayload;
use crate::server_fns::test_seed::repeatable_read_pool;
use crate::server_fns::test_seed::seed;

fn created(client_id: Uuid, org_id: Uuid) -> AppEvent {
    AppEvent::ClientCreated {
        occurred_at: chrono::Utc::now(),
        org_id,
        client: ClientPayload {
            id: client_id,
            name: "Probe".into(),
            currency: "EUR".into(),
            active: true,
        },
    }
}

/// An operation that writes a client and announces it.
async fn insert_client(tx: &mut Tx, actor: &Actor) -> Result<Done<Uuid>, OpError> {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Probe', 'EUR')",
        id,
        actor.org_id()
    )
    .execute(&mut **tx)
    .await?;
    Ok(Done::new(id).with(created(id, actor.org_id())))
}

async fn client_exists(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM clients WHERE id = $1) AS "exists!""#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// On each event, asks a separate connection whether the event's client is
/// already committed. `dispatch` is synchronous, so the check runs on its own
/// thread and runtime.
struct CommitProbe {
    options: sqlx::postgres::PgConnectOptions,
    seen: Mutex<Vec<bool>>,
}

impl EventSink for CommitProbe {
    fn dispatch(&self, event: AppEvent) {
        let AppEvent::ClientCreated { client, .. } = event else {
            panic!("unexpected event {event:?}");
        };
        let options = self.options.clone();
        let committed = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let mut connection = sqlx::PgConnection::connect_with(&options).await.unwrap();
                    sqlx::query_scalar!(
                        r#"SELECT EXISTS(SELECT 1 FROM clients WHERE id = $1) AS "exists!""#,
                        client.id
                    )
                    .fetch_one(&mut connection)
                    .await
                    .unwrap()
                })
        })
        .join()
        .unwrap();
        self.seen.lock().unwrap().push(committed);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn events_are_dispatched_after_the_commit(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let probe = CommitProbe {
        options: (*pool.connect_options()).clone(),
        seen: Mutex::default(),
    };
    let mut write = begin(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = insert_client(&mut write.tx, &write.actor).await;
    let id = write.commit(result, &probe).await.unwrap();
    assert_eq!(*probe.seen.lock().unwrap(), [true]);
    assert!(client_exists(&pool, id).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_failed_operation_rolls_back_and_dispatches_nothing(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let events = Recorder::default();
    let mut write = begin(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let inserted = insert_client(&mut write.tx, &write.actor).await.unwrap();
    let refused: Result<Done<()>, _> = Err(OpError::conflict("refused after writing"));
    let result = write.commit(refused, &events).await;
    assert_eq!(result, Err(OpError::conflict("refused after writing")));
    assert!(events.take().is_empty());
    assert!(!client_exists(&pool, inserted.value).await);
}

/// A deferred constraint fails at COMMIT, after the operation succeeded.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_failed_commit_dispatches_nothing(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let events = Recorder::default();
    let mut write = begin(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = insert_client(&mut write.tx, &write.actor).await;
    for statement in [
        "CREATE TEMP TABLE probe_parent (id int PRIMARY KEY) ON COMMIT DROP",
        "CREATE TEMP TABLE probe_child (parent int REFERENCES probe_parent
           DEFERRABLE INITIALLY DEFERRED) ON COMMIT DROP",
        "INSERT INTO probe_child VALUES (1)",
    ] {
        sqlx::query(statement)
            .execute(&mut *write.tx)
            .await
            .unwrap();
    }
    let id = result.as_ref().unwrap().value;
    let result = write.commit(result, &events).await;
    assert!(matches!(result, Err(OpError::Internal(_))), "{result:?}");
    assert!(events.take().is_empty());
    assert!(!client_exists(&pool, id).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn writes_run_at_read_committed_whatever_the_session_default(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let rr = repeatable_read_pool(&pool).await;
    let mut write = begin(&rr, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let isolation =
        sqlx::query_scalar!(r#"SELECT current_setting('transaction_isolation') AS "i!""#)
            .fetch_one(&mut *write.tx)
            .await
            .unwrap();
    assert_eq!(isolation, "read committed");
}

/// The backend pid of `tx`, for waiting until another session queues behind it.
async fn backend_pid(tx: &mut Tx) -> i32 {
    sqlx::query_scalar!(r#"SELECT pg_backend_pid() AS "pid!""#)
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

/// A setting as the transaction sees it.
async fn setting(tx: &mut Tx, name: &str) -> String {
    sqlx::query_scalar!(r#"SELECT current_setting($1) AS "value!""#, name)
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_snapshot_read_retries_when_access_changes_after_its_snapshot(pool: PgPool) {
    use horae_core::permissions::catalog::Permission;

    use super::testing::{policy_one, set_grants};
    use crate::server_fns::test_seed::wait_for_blocked;

    for limits in [SnapshotLimits::Short, SnapshotLimits::LockWait] {
        let ids = seed_profile(&pool, BuiltInProfile::Member).await;
        policy_one(&pool, &ids, &[], false).await;
        let mut change: Tx = pool.begin().await.unwrap();
        let blocker = backend_pid(&mut change).await;
        set_grants(&mut change, &ids, &[Permission::ClientReadAll])
            .await
            .unwrap();
        // The read takes its snapshot, then queues on the organization lock.
        let read = tokio::spawn({
            let pool = pool.clone();
            async move {
                begin_read_snapshot(&pool, Caller::session(ids.org_id, ids.user_id), limits).await
            }
        });
        wait_for_blocked(&pool, blocker).await;
        change.commit().await.unwrap();
        // Its first attempt fails with 40001; the retry sees the change.
        let mut read = read.await.unwrap().unwrap();
        assert!(read.actor.has(Permission::ClientReadAll));
        assert_eq!(
            setting(&mut read.tx, "transaction_isolation").await,
            "repeatable read"
        );
        assert_eq!(setting(&mut read.tx, "synchronous_commit").await, "off");
        let (statement_timeout, lock_timeout) = match limits {
            SnapshotLimits::Short => ("5s", "0"),
            // The bounded lock wait is lifted again once authorized.
            SnapshotLimits::LockWait => ("0", "0"),
        };
        assert_eq!(
            setting(&mut read.tx, "statement_timeout").await,
            statement_timeout
        );
        assert_eq!(setting(&mut read.tx, "lock_timeout").await, lock_timeout);
        read.finish(Ok(())).await.unwrap();
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_gated_write_chooses_its_isolation_and_holds_the_gate(pool: PgPool) {
    use crate::server_fns::test_seed::wait_for_blocked;

    let ids = seed_profile(&pool, BuiltInProfile::Administrator).await;
    let caller = Caller::session(ids.org_id, ids.user_id);
    let rr = repeatable_read_pool(&pool).await;
    for (isolation, expected) in [
        (WriteIsolation::ReadCommitted, "read committed"),
        (WriteIsolation::Serializable, "serializable"),
    ] {
        let mut write = begin_gated(&rr, caller, OrganizationLock::AccessChange, isolation)
            .await
            .unwrap();
        assert!(write.actor.administrator());
        assert_eq!(
            setting(&mut write.tx, "transaction_isolation").await,
            expected
        );
        // An access-change gate excludes readers' `FOR SHARE` until it ends.
        let blocker = backend_pid(&mut write.tx).await;
        let read = tokio::spawn({
            let pool = pool.clone();
            async move {
                begin_read(&pool, caller)
                    .await
                    .map(|read| read.actor.is_active())
            }
        });
        wait_for_blocked(&pool, blocker).await;
        write.tx.rollback().await.unwrap();
        assert!(read.await.unwrap().unwrap());
    }
}

/// The actor loads after the gate wait, so it sees an access change that held
/// the organization while the write queued.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_gated_write_loads_the_actor_after_waiting_for_the_gate(pool: PgPool) {
    use super::testing::revoke;
    use crate::server_fns::test_seed::wait_for_blocked;

    let ids = seed_profile(&pool, BuiltInProfile::Administrator).await;
    let mut change: Tx = pool.begin().await.unwrap();
    let blocker = backend_pid(&mut change).await;
    revoke(&mut change, ids.org_id, ids.user_id, false).await;
    let write = tokio::spawn({
        let pool = pool.clone();
        async move {
            begin_gated(
                &pool,
                Caller::session(ids.org_id, ids.user_id),
                OrganizationLock::Shared,
                WriteIsolation::ReadCommitted,
            )
            .await
            .map(|write| write.actor.administrator())
        }
    });
    wait_for_blocked(&pool, blocker).await;
    change.commit().await.unwrap();
    assert!(!write.await.unwrap().unwrap());
}
