//! Integer ids on the Harvest wire (FR-015–FR-018). Numbers live in `api_ids`
//! and are allocated by triggers when a record is inserted (migration 0058);
//! responses read them with the SQL function `api_number`.

use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "api_id_kind", rename_all = "snake_case")]
pub enum Kind {
    Client,
    Project,
    Task,
    TimeEntry,
    User,
}

/// The record a request names: its integer number or, during the transition
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
        return sqlx::query_scalar!(
            "SELECT horae_id FROM api_ids WHERE org_id = $1 AND kind = $2 AND number = $3",
            org_id,
            kind as Kind,
            number,
        )
        .fetch_optional(db)
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
