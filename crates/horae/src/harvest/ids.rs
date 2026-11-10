//! Integer ids on the Harvest wire (FR-015–FR-018). Each numbered table has a
//! `harvest_id` column: the id in the Harvest-compatible API, filled on insert
//! by a trigger when the insert gives none (migration 0059). Horae's identity
//! is still `id` (UUID v7).

use horae_core::importers::harvest::types::EntityType;
use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Client,
    Project,
    Task,
    TimeEntry,
    User,
}

impl From<EntityType> for Kind {
    fn from(entity: EntityType) -> Self {
        match entity {
            EntityType::Client => Self::Client,
            EntityType::Project => Self::Project,
            EntityType::Task => Self::Task,
            EntityType::TimeEntry => Self::TimeEntry,
            EntityType::Person => Self::User,
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

/// The record a request names: its integer id or, during the transition
/// period (D-1), its UUID. A trailing `.json` is ignored (FR-026). `Ok(None)`
/// when this organization has no such record.
pub async fn resolve(
    db: &PgPool,
    org_id: Uuid,
    kind: Kind,
    raw: &str,
) -> Result<Option<Uuid>, ApiError> {
    let raw = raw.strip_suffix(".json").unwrap_or(raw);
    if let Ok(number) = raw.parse::<i64>() {
        return record(db, org_id, kind, number)
            .await
            .map_err(ApiError::internal);
    }
    raw.parse::<Uuid>()
        .map(Some)
        .map_err(|_| ApiError::unprocessable(format!("{raw:?} is not a valid id")))
}

/// A path id: an unknown record is 404.
pub async fn path(db: &PgPool, org_id: Uuid, kind: Kind, raw: &str) -> Result<Uuid, ApiError> {
    resolve(db, org_id, kind, raw)
        .await?
        .ok_or_else(ApiError::not_found)
}

/// A filter id: an unknown record matches nothing, so the list comes back
/// empty rather than unfiltered.
pub async fn filter(
    db: &PgPool,
    org_id: Uuid,
    kind: Kind,
    raw: Option<&str>,
) -> Result<Option<Uuid>, ApiError> {
    match raw {
        None => Ok(None),
        Some(raw) => Ok(Some(
            resolve(db, org_id, kind, raw).await?.unwrap_or(Uuid::nil()),
        )),
    }
}

/// A record's API id, for tests.
#[cfg(test)]
pub async fn number(db: &PgPool, kind: Kind, id: Uuid) -> i64 {
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
