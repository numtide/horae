//! Provenance access: the `(org, entity_type, harvest_id) → horae_id` mapping
//! that makes API re-syncs exact and edit-robust (data-model.md, FR-026).
//!
//! Both functions take an `impl PgExecutor`, so a caller can enlist them in its
//! own per-record savepoint — the mapping row is written in the same all-or-
//! nothing unit that creates the record it points at.

use horae_core::harvest_import::types::EntityType;
use uuid::Uuid;

use chrono::{DateTime, Utc};

/// Look up the Horae id a Harvest record was previously imported into, if any.
pub async fn lookup<'e, E>(
    exec: E,
    org_id: Uuid,
    entity: EntityType,
    harvest_id: i64,
) -> sqlx::Result<Option<Uuid>>
where
    E: sqlx::PgExecutor<'e>,
{
    let row = sqlx::query_scalar!(
        r#"SELECT horae_id FROM harvest_import_map
           WHERE org_id = $1
             AND harvest_entity_type = $2::harvest_entity_type
             AND harvest_id = $3"#,
        org_id,
        entity.as_str() as _,
        harvest_id,
    )
    .fetch_optional(exec)
    .await?;
    Ok(row)
}

/// Persist (or refresh) the provenance mapping for a Harvest record. Idempotent:
/// re-recording the same `(org, entity, harvest_id)` updates the Horae id and the
/// last-seen `updated_at` rather than failing.
pub async fn upsert<'e, E>(
    exec: E,
    org_id: Uuid,
    entity: EntityType,
    harvest_id: i64,
    horae_id: Uuid,
    harvest_updated_at: Option<DateTime<Utc>>,
) -> sqlx::Result<()>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query!(
        r#"INSERT INTO harvest_import_map
             (org_id, harvest_entity_type, harvest_id, horae_id, harvest_updated_at)
           VALUES ($1, $2::harvest_entity_type, $3, $4, $5)
           ON CONFLICT (org_id, harvest_entity_type, harvest_id)
           DO UPDATE SET horae_id = EXCLUDED.horae_id,
                         harvest_updated_at = EXCLUDED.harvest_updated_at"#,
        org_id,
        entity.as_str() as _,
        harvest_id,
        horae_id,
        harvest_updated_at as Option<chrono::DateTime<chrono::Utc>>,
    )
    .execute(exec)
    .await?;
    Ok(())
}
