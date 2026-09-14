//! Simulation identity across API pages, without publishing domain mutations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use super::super::super::resolve::{RunCache, preview::ParentSnapshot};

#[derive(Default, Serialize, Deserialize)]
pub(super) struct ApiPreview {
    parents: Option<ParentSnapshot>,
    entries: BTreeMap<i64, Uuid>,
}

impl ApiPreview {
    pub(super) async fn capture(
        &mut self,
        conn: &mut PgConnection,
        org_id: Uuid,
        cache: &RunCache,
        page_ids: &[i64],
    ) -> anyhow::Result<()> {
        self.parents = Some(ParentSnapshot::capture(conn, org_id, cache).await?);
        // Read after row savepoints finish: failed rows must not acquire a
        // simulated mapping that could turn a later retry into a false skip.
        let mappings = sqlx::query!(
            "SELECT harvest_id, horae_id FROM harvest_import_map
             WHERE org_id = $1 AND harvest_entity_type = 'time_entry'
               AND harvest_id = ANY($2)",
            org_id,
            page_ids,
        )
        .fetch_all(&mut *conn)
        .await?;
        self.entries.extend(
            mappings
                .into_iter()
                .map(|row| (row.harvest_id, row.horae_id)),
        );
        Ok(())
    }

    pub(super) async fn restore(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> anyhow::Result<()> {
        if let Some(parents) = &self.parents {
            parents.restore(conn, org_id).await?;
        }
        // Parent identities are already in RunCache. Entry provenance also
        // reserves adopted real entries against other Harvest IDs. A simulated
        // new entry need not be recreated: its mapping is enough to skip its ID,
        // and a different ID cannot adopt an already-mapped simulated entry.
        let (harvest_ids, horae_ids): (Vec<_>, Vec<_>) = self
            .entries
            .iter()
            .map(|(&harvest_id, &horae_id)| (harvest_id, horae_id))
            .unzip();
        sqlx::query!(
            "INSERT INTO harvest_import_map (org_id, harvest_entity_type, harvest_id, horae_id)
             SELECT $1, 'time_entry', ids.harvest_id, ids.horae_id
             FROM unnest($2::bigint[], $3::uuid[]) AS ids(harvest_id, horae_id)
             ON CONFLICT (org_id, harvest_entity_type, harvest_id) DO NOTHING",
            org_id,
            &harvest_ids,
            &horae_ids,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
