//! Server operations: one function per business action, shared by every entry
//! point (Dioxus server functions today; the Harvest-compatible API, `/api/v1`
//! and the CLI later).
//!
//! An operation has the shape `fn(tx, &Actor, input) -> Result<Output, OpError>`.
//! It authorizes through the [`Actor`], validates, runs its SQL and builds the
//! plugin events it causes, all inside the caller's transaction, and never
//! commits. Adapters translate their wire format to the input, run the
//! operation inside [`begin`] / [`WriteTx::commit`] (or [`begin_read`] /
//! [`ReadTx::finish`]) and translate the output or [`OpError`] back:
//!
//! ```ignore
//! let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
//! let result = ops::clients::set_active(&mut write.tx, &write.actor, client_id, active).await;
//! Ok(write.commit(result, &*state.plugins).await?)
//! ```
//!
//! Plugin events take one of two paths. Operations return them in [`Done`]
//! and [`WriteTx::commit`] dispatches them in memory after the commit, so a
//! crash between the two loses them. Two writers instead insert their event
//! into the outbox (`jobs::enqueue_outbox`) inside their own transaction:
//! `projects::creation::finalize` (`project_created`) and the Harvest importer
//! (`user_created`). Their delivery survives a crash right after the commit,
//! which is why they stay on the outbox rather than moving to [`Done`].

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db::{OrganizationLock, lock_organization};
use crate::plugin::{AppEvent, PluginRegistry};

mod actor;
pub mod api_tokens;
pub mod approvals;
pub mod clients;
mod error;
pub mod importers;
pub mod invoices;
pub mod organization;
pub mod people;
pub mod permissions;
pub mod profile;
pub mod projects;
pub mod reports;
pub mod time;
pub mod time_reads;

pub use actor::{Actor, Channel};
pub use error::OpError;

pub type Tx = Transaction<'static, Postgres>;

#[derive(Clone, Copy)]
pub enum ReadIsolation {
    ReadCommitted,
    RepeatableRead,
}

/// Begin a read whose only writes are `FOR SHARE` authority locks, with the
/// isolation named in `BEGIN` itself rather than a separate `SET TRANSACTION`.
///
/// No INSERT/UPDATE/DELETE/nextval: this transaction commits asynchronously.
/// READ WRITE is what lets the locks be taken (READ ONLY rejects `FOR SHARE`),
/// so Postgres cannot reject a write here; a data write would still commit,
/// but could be lost on a server crash shortly after its commit returned.
pub async fn begin_lock_only_read_async_commit(
    pool: &PgPool,
    isolation: ReadIsolation,
) -> Result<Tx, sqlx::Error> {
    let begin = match isolation {
        ReadIsolation::ReadCommitted => "BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE",
        ReadIsolation::RepeatableRead => "BEGIN ISOLATION LEVEL REPEATABLE READ, READ WRITE",
    };
    let mut tx = pool.begin_with(begin).await?;
    configure_limits(&mut tx, Commit::LockOnly).await?;
    Ok(tx)
}

/// Set local limits only after the caller selects its transaction isolation.
pub async fn configure_transaction_limits(
    connection: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    configure_limits(connection, Commit::Durable).await
}

#[derive(Clone, Copy, PartialEq)]
enum Commit {
    Durable,
    LockOnly,
}

async fn configure_limits(
    connection: &mut PgConnection,
    commit: Commit,
) -> Result<(), sqlx::Error> {
    // `current_setting` instead of `pg_settings`, which materializes every GUC
    // on each call. Millisecond settings always display with a unit Postgres
    // also accepts as interval input (ms, s, min, h, d), or as a bare 0.
    //
    // A lock-only read commits without waiting for its WAL flush. Its FOR
    // SHARE locks give it an xid, so a synchronous commit costs an fsync even
    // though nothing durable was written. Locks are held and released exactly
    // as before, and commit visibility does not depend on the flush, so
    // concurrent access changes still wait for, or invalidate, the read. A
    // crash can only lose the commit record of a transaction that changed no
    // data, and its locks die with the server either way.
    sqlx::query!(
        "SELECT set_config(name, value, true)
         FROM (
           SELECT limits.name,
             (CASE WHEN current.milliseconds = 0 THEN limits.milliseconds
              ELSE LEAST(current.milliseconds, limits.milliseconds) END)::text AS value
           FROM (VALUES ('statement_timeout', 5000::bigint),
                        ('idle_in_transaction_session_timeout', 10000::bigint))
                AS limits(name, milliseconds)
           CROSS JOIN LATERAL (SELECT (extract(epoch FROM current_setting(limits.name)::interval)
                                       * 1000)::bigint AS milliseconds) current
           UNION ALL
           SELECT 'synchronous_commit', 'off' WHERE $1
         ) settings",
        commit == Commit::LockOnly,
    )
    .fetch_all(connection)
    .await?;
    Ok(())
}

/// Who is asking, before the transaction confirms it: the organization and
/// person an entry point authenticated, and through which channel.
#[derive(Debug, Clone, Copy)]
pub struct Caller {
    pub org_id: Uuid,
    /// The nil UUID for a channel with no person ([`Channel::Operator`],
    /// [`Channel::System`]).
    pub user_id: Uuid,
    pub channel: Channel,
}

impl Caller {
    pub fn session(org_id: Uuid, user_id: Uuid) -> Self {
        Self {
            org_id,
            user_id,
            channel: Channel::Session,
        }
    }

    /// A person through one of their personal access tokens.
    pub fn token(org_id: Uuid, user_id: Uuid, token_id: Uuid) -> Self {
        Self {
            org_id,
            user_id,
            channel: Channel::Token { token_id },
        }
    }

    /// The server's operator at its terminal (the CLI).
    pub fn operator(org_id: Uuid) -> Self {
        Self {
            org_id,
            user_id: Uuid::nil(),
            channel: Channel::Operator,
        }
    }

    /// Horae itself, acting on no one's behalf (an OIDC first sign-in).
    pub fn system(org_id: Uuid) -> Self {
        Self {
            org_id,
            user_id: Uuid::nil(),
            channel: Channel::System,
        }
    }
}

/// Test fixtures hold a whole `User`; the read helpers take its caller.
#[cfg(any(test, feature = "test-support"))]
impl From<&people::User> for Caller {
    fn from(user: &people::User) -> Self {
        Self::session(user.org_id, user.id)
    }
}

/// The person's session caller if their account is still active. Shared by
/// the server functions and the Axum export routes.
pub async fn active_caller(db: &PgPool, user_id: Uuid) -> Result<Option<Caller>, sqlx::Error> {
    let org_id = sqlx::query_scalar!(
        "SELECT org_id FROM users WHERE id = $1 AND active = true",
        user_id,
    )
    .fetch_optional(db)
    .await?;
    Ok(org_id.map(|org_id| Caller::session(org_id, user_id)))
}

/// A write's result and the plugin events it caused, dispatched only once the
/// transaction has committed.
pub struct Done<T> {
    pub value: T,
    pub events: Vec<AppEvent>,
}

impl<T> Done<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            events: Vec::new(),
        }
    }

    pub fn with(mut self, event: AppEvent) -> Self {
        self.events.push(event);
        self
    }
}

/// Where committed events go: the plugin host in the server, a recorder in tests.
pub trait EventSink: Sync {
    fn dispatch(&self, event: AppEvent);
}

impl EventSink for PluginRegistry {
    fn dispatch(&self, event: AppEvent) {
        PluginRegistry::dispatch(self, event);
    }
}

/// Drops committed events: for the CLI, which runs no plugins, and for entry
/// points that report the event themselves.
pub struct NoEvents;

impl EventSink for NoEvents {
    fn dispatch(&self, _: AppEvent) {}
}

/// A write in progress: its transaction and the actor loaded inside it.
/// Run the operation on `tx` and `actor`, then hand its result to
/// [`WriteTx::commit`]. Dropping it uncommitted rolls back.
///
/// This is `with_tx` split in two: an async closure taking `&mut Tx` cannot be
/// proven `Send` for every borrow, so a closure-based runner would not compile
/// inside `tokio::spawn`.
pub struct WriteTx {
    pub tx: Tx,
    pub actor: Actor,
}

/// Begins a write at READ COMMITTED and loads the [`Actor`]. The isolation is
/// explicit so checks that must see rows committed while the operation waited
/// for a lock do not depend on the server default. The statement and idle
/// limits bound how long the actor's `FOR SHARE` locks can hold off access
/// changes.
pub async fn begin(pool: &PgPool, caller: Caller) -> Result<WriteTx, OpError> {
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .await?;
    configure_transaction_limits(&mut tx).await?;
    let (tx, actor) = load_actor(tx, caller).await?;
    Ok(WriteTx { tx, actor })
}

impl WriteTx {
    /// Commits a successful operation and only then dispatches the events it
    /// returned. A failed operation or commit rolls back and dispatches nothing.
    pub async fn commit<T>(
        self,
        result: Result<Done<T>, OpError>,
        events: &dyn EventSink,
    ) -> Result<T, OpError> {
        let done = match result {
            Ok(done) => done,
            Err(error) => return Err(roll_back(self.tx, error).await),
        };
        self.tx.commit().await?;
        for event in done.events {
            events.dispatch(event);
        }
        Ok(done.value)
    }
}

/// A read in progress. Its only writes are the authority locks
/// [`Actor::load`] takes, so it commits without waiting for a WAL flush (see
/// `begin_lock_only_read_async_commit`): an operation run on it must not write.
pub struct ReadTx {
    pub tx: Tx,
    pub actor: Actor,
}

pub async fn begin_read(pool: &PgPool, caller: Caller) -> Result<ReadTx, OpError> {
    let tx = begin_lock_only_read_async_commit(pool, ReadIsolation::ReadCommitted).await?;
    let (tx, actor) = load_actor(tx, caller).await?;
    Ok(ReadTx { tx, actor })
}

/// What a snapshot read bounds besides its authority locks.
#[derive(Clone, Copy)]
pub enum SnapshotLimits {
    /// The statement and idle limits of [`begin_read`], for short reads.
    Short,
    /// Only the wait for the authority locks (at most 5 s); the inherited
    /// limits then apply again, for reads that materialize large payloads
    /// such as invoices.
    LockWait,
}

const SNAPSHOT_ATTEMPTS: usize = 3;

/// [`begin_read`] at REPEATABLE READ: every statement of the read sees the
/// snapshot the authority locks were taken in. An access change committed
/// after the snapshot began makes those locks fail with 40001 (a conflict);
/// a fresh transaction retries, up to three attempts, since retrying only the
/// query would stay stale. Like [`ReadTx`], it must not write.
pub async fn begin_read_snapshot(
    pool: &PgPool,
    caller: Caller,
    limits: SnapshotLimits,
) -> Result<ReadTx, OpError> {
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let (mut tx, inherited_lock_timeout) = match limits {
            SnapshotLimits::Short => (
                begin_lock_only_read_async_commit(pool, ReadIsolation::RepeatableRead).await?,
                None,
            ),
            SnapshotLimits::LockWait => {
                let mut tx = pool
                    .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ, READ WRITE")
                    .await?;
                // The materialized CTE reads the inherited lock_timeout before
                // bounding it, so it can be restored once authorized. The
                // commit need not wait for a WAL flush: see
                // `begin_lock_only_read_async_commit`.
                let previous = sqlx::query_scalar!(
                    r#"WITH previous AS MATERIALIZED (
                         SELECT current_setting('lock_timeout') AS setting,
                           (extract(epoch FROM current_setting('lock_timeout')::interval) * 1000)::bigint
                             AS milliseconds
                       )
                       SELECT setting AS "setting!"
                       FROM previous,
                         LATERAL set_config('lock_timeout',
                           (CASE WHEN milliseconds = 0 THEN 5000 ELSE LEAST(milliseconds, 5000) END)::text,
                           true) bounded,
                         LATERAL set_config('synchronous_commit', 'off', true) asynchronous"#
                )
                .fetch_one(&mut *tx)
                .await?;
                (tx, Some(previous))
            }
        };
        match Actor::load(&mut tx, caller).await {
            Ok(actor) => {
                if let Some(previous) = inherited_lock_timeout {
                    let restored =
                        sqlx::query!("SELECT set_config('lock_timeout', $1, true)", previous)
                            .fetch_one(&mut *tx)
                            .await;
                    if let Err(error) = restored {
                        return Err(roll_back(tx, OpError::from(error)).await);
                    }
                }
                return Ok(ReadTx { tx, actor });
            }
            Err(OpError::Conflict(_)) => tx.rollback().await?,
            Err(error) => return Err(roll_back(tx, error).await),
        }
    }
    Err(OpError::conflict(
        "Access changed while reading. Please try again.",
    ))
}

#[derive(Clone, Copy)]
pub enum WriteIsolation {
    ReadCommitted,
    Serializable,
}

/// [`begin`] for writes that hold the organization `gate` (see
/// [`OrganizationLock`]): it is taken before the actor's own `FOR SHARE`
/// locks, because a gate is never upgraded. `Serializable` is for the project
/// editor's save.
pub async fn begin_gated(
    pool: &PgPool,
    caller: Caller,
    gate: OrganizationLock,
    isolation: WriteIsolation,
) -> Result<WriteTx, OpError> {
    let begin = match isolation {
        WriteIsolation::ReadCommitted => "BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE",
        WriteIsolation::Serializable => "BEGIN ISOLATION LEVEL SERIALIZABLE, READ WRITE",
    };
    let mut tx = pool.begin_with(begin).await?;
    configure_transaction_limits(&mut tx).await?;
    lock_organization(&mut tx, caller.org_id, gate).await?;
    let (tx, actor) = load_actor(tx, caller).await?;
    Ok(WriteTx { tx, actor })
}

/// Loads the [`Actor`] inside an open transaction, with [`Actor::load`]'s
/// locks, and refuses anyone but an active administrator. For actions that run
/// in a transaction of their own rather than through [`begin`], such as the
/// Harvest connection changes and the import jobs.
pub(crate) async fn require_administrator(
    connection: &mut sqlx::PgConnection,
    caller: Caller,
) -> Result<Actor, OpError> {
    let actor = Actor::load(connection, caller).await?;
    actor.require_administrator()?;
    Ok(actor)
}

impl ReadTx {
    /// Releases the authority locks, held until the result was materialized.
    pub async fn finish<T>(self, result: Result<T, OpError>) -> Result<T, OpError> {
        let value = match result {
            Ok(value) => value,
            Err(error) => return Err(roll_back(self.tx, error).await),
        };
        self.tx.commit().await?;
        Ok(value)
    }

    /// Rolls back a read the adapter refuses outside its operation, and
    /// returns the adapter's error.
    pub async fn refuse<E>(self, error: E) -> E {
        roll_back(self.tx, error).await
    }
}

/// [`Actor::load`], rolling back when it fails: by then it may hold the
/// organization's and person's `FOR SHARE` locks, and any gate taken before it.
async fn load_actor(mut tx: Tx, caller: Caller) -> Result<(Tx, Actor), OpError> {
    match Actor::load(&mut tx, caller).await {
        Ok(actor) => Ok((tx, actor)),
        Err(error) => Err(roll_back(tx, error).await),
    }
}

/// Commits `tx` if `result` succeeded and rolls it back otherwise, before the
/// result is answered. For writers that run their own transaction instead of
/// [`begin`] and [`WriteTx::commit`]; `commit_failed` reports a failed commit.
pub async fn commit_or_roll_back<T, E>(
    tx: Transaction<'_, Postgres>,
    result: Result<T, E>,
    commit_failed: impl FnOnce(sqlx::Error) -> E,
) -> Result<T, E> {
    match result {
        Ok(value) => {
            tx.commit().await.map_err(commit_failed)?;
            Ok(value)
        }
        Err(error) => Err(roll_back(tx, error).await),
    }
}

/// Rolls back a failed operation before its error is returned. Dropping the
/// transaction would leave the `ROLLBACK` to the task that returns the
/// connection to the pool, so the actor's `FOR SHARE` locks could outlive the
/// response. The operation's error is the one to report; a failed rollback
/// falls back to that pool cleanup.
async fn roll_back<E>(tx: Transaction<'_, Postgres>, error: E) -> E {
    let _ = tx.rollback().await;
    error
}

#[cfg(any(test, feature = "test-support"))]
pub mod testing;
#[cfg(test)]
mod tests;
