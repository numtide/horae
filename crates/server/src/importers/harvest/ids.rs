//! Integer ids on the Harvest wire (FR-015–FR-018). Each numbered table has a
//! `harvest_id` column: the id in the Harvest-compatible API, filled on insert
//! by a trigger when the insert gives none. Horae's identity
//! is still `id` (UUID v7).

use horae_core::importers::harvest::types::EntityType;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Client,
    Project,
    Task,
    TimeEntry,
    User,
}

impl Kind {
    /// The numbered table of an imported entity; invoices have no API number.
    pub fn of(entity: EntityType) -> Option<Self> {
        match entity {
            EntityType::Client => Some(Self::Client),
            EntityType::Project => Some(Self::Project),
            EntityType::Task => Some(Self::Task),
            EntityType::TimeEntry => Some(Self::TimeEntry),
            EntityType::Person => Some(Self::User),
            EntityType::Invoice => None,
        }
    }
}

/// The record of this kind in the organization that has this API id.
pub async fn record<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    kind: Kind,
    number: i64,
) -> sqlx::Result<Option<Uuid>> {
    match kind {
        Kind::Client => {
            sqlx::query_scalar!(
                "SELECT id FROM clients WHERE org_id = $1 AND harvest_id = $2",
                org_id,
                number
            )
            .fetch_optional(exec)
            .await
        }
        Kind::Project => {
            sqlx::query_scalar!(
                "SELECT id FROM projects WHERE org_id = $1 AND harvest_id = $2",
                org_id,
                number
            )
            .fetch_optional(exec)
            .await
        }
        Kind::Task => {
            sqlx::query_scalar!(
                "SELECT id FROM tasks WHERE org_id = $1 AND harvest_id = $2",
                org_id,
                number
            )
            .fetch_optional(exec)
            .await
        }
        Kind::TimeEntry => {
            sqlx::query_scalar!(
                "SELECT id FROM time_entries WHERE org_id = $1 AND harvest_id = $2",
                org_id,
                number
            )
            .fetch_optional(exec)
            .await
        }
        Kind::User => {
            sqlx::query_scalar!(
                "SELECT id FROM users WHERE org_id = $1 AND harvest_id = $2",
                org_id,
                number
            )
            .fetch_optional(exec)
            .await
        }
    }
}

/// A record's API id, for tests.
#[cfg(any(test, feature = "test-support"))]
pub async fn number(db: &sqlx::PgPool, kind: Kind, id: Uuid) -> i64 {
    let number = match kind {
        Kind::Client => {
            sqlx::query_scalar!("SELECT harvest_id FROM clients WHERE id = $1", id)
                .fetch_one(db)
                .await
        }
        Kind::Project => {
            sqlx::query_scalar!("SELECT harvest_id FROM projects WHERE id = $1", id)
                .fetch_one(db)
                .await
        }
        Kind::Task => {
            sqlx::query_scalar!("SELECT harvest_id FROM tasks WHERE id = $1", id)
                .fetch_one(db)
                .await
        }
        Kind::TimeEntry => {
            sqlx::query_scalar!("SELECT harvest_id FROM time_entries WHERE id = $1", id)
                .fetch_one(db)
                .await
        }
        Kind::User => {
            sqlx::query_scalar!("SELECT harvest_id FROM users WHERE id = $1", id)
                .fetch_one(db)
                .await
        }
    };
    number.unwrap()
}
