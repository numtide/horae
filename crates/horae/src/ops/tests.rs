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
