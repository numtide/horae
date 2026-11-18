//! Ids as a request names them. The records behind them, and the integer
//! `harvest_id` itself, live with the importer in
//! [`crate::importers::harvest::ids`].

use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;
pub use crate::importers::harvest::ids::Kind;
use crate::importers::harvest::ids::record;

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
