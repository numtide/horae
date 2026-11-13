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

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::plugin::{AppEvent, PluginRegistry};

mod actor;
pub(crate) mod clients;
mod error;
pub(crate) mod people;
pub(crate) mod projects;
pub(crate) mod reports;

pub(crate) use actor::{Actor, Channel};
pub(crate) use error::OpError;

pub(crate) type Tx = Transaction<'static, Postgres>;

/// Who is asking, before the transaction confirms it: the organization and
/// person an entry point authenticated, and through which channel.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Caller {
    pub org_id: Uuid,
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
}

/// A write's result and the plugin events it caused, dispatched only once the
/// transaction has committed.
pub(crate) struct Done<T> {
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
pub(crate) trait EventSink: Sync {
    fn dispatch(&self, event: AppEvent);
}

impl EventSink for PluginRegistry {
    fn dispatch(&self, event: AppEvent) {
        PluginRegistry::dispatch(self, event);
    }
}

/// A write in progress: its transaction and the actor loaded inside it.
/// Run the operation on `tx` and `actor`, then hand its result to
/// [`WriteTx::commit`]. Dropping it uncommitted rolls back.
///
/// This is `with_tx` split in two: an async closure taking `&mut Tx` cannot be
/// proven `Send` for every borrow, so a closure-based runner would not compile
/// inside `tokio::spawn`.
pub(crate) struct WriteTx {
    pub tx: Tx,
    pub actor: Actor,
}

/// Begins a write at READ COMMITTED and loads the [`Actor`]. The isolation is
/// explicit so checks that must see rows committed while the operation waited
/// for a lock do not depend on the server default. The statement and idle
/// limits bound how long the actor's `FOR SHARE` locks can hold off access
/// changes.
pub(crate) async fn begin(pool: &PgPool, caller: Caller) -> Result<WriteTx, OpError> {
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .await?;
    crate::server_fns::configure_transaction_limits(&mut tx).await?;
    let actor = Actor::load(&mut tx, caller).await?;
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
        let done = result?;
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
pub(crate) struct ReadTx {
    pub tx: Tx,
    pub actor: Actor,
}

pub(crate) async fn begin_read(pool: &PgPool, caller: Caller) -> Result<ReadTx, OpError> {
    use crate::server_fns::{ReadIsolation, begin_lock_only_read_async_commit};
    let mut tx = begin_lock_only_read_async_commit(pool, ReadIsolation::ReadCommitted).await?;
    let actor = Actor::load(&mut tx, caller).await?;
    Ok(ReadTx { tx, actor })
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
        let value = result?;
        self.tx.commit().await?;
        Ok(value)
    }
}

#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;
