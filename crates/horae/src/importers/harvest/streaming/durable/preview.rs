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
        page_ids: &[i64],
    ) -> anyhow::Result<()> {
        if let Some(parents) = &self.parents {
            parents.restore(conn, org_id).await?;
        }
        // Reserve every adopted real entry against other Harvest IDs. Virtual
        // entries have no remaining row to adopt after rollback, so only IDs
        // repeated on this page need their mapping restored. Replaying all old
        // virtual mappings would perform quadratic writes across fresh pages.
        let (harvest_ids, horae_ids): (Vec<_>, Vec<_>) = self
            .entries
            .iter()
            .map(|(&harvest_id, &horae_id)| (harvest_id, horae_id))
            .unzip();
        sqlx::query!(
            "INSERT INTO harvest_import_map (org_id, harvest_entity_type, harvest_id, horae_id)
             SELECT $1, 'time_entry', ids.harvest_id, ids.horae_id
             FROM unnest($2::bigint[], $3::uuid[]) AS ids(harvest_id, horae_id)
             LEFT JOIN time_entries te ON te.org_id = $1 AND te.id = ids.horae_id
             WHERE te.id IS NOT NULL OR ids.harvest_id = ANY($4)
             ON CONFLICT (org_id, harvest_entity_type, harvest_id) DO NOTHING",
            org_id,
            &harvest_ids,
            &horae_ids,
            page_ids,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn restoration_omits_unneeded_virtual_entries(pool: sqlx::PgPool) {
        let org = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO organizations (id, name, default_currency) VALUES ($1, 'Test Org', 'USD')",
            org,
        )
        .execute(&pool)
        .await
        .unwrap();
        let preview = ApiPreview {
            parents: None,
            entries: (0..10_000).map(|id| (id, Uuid::now_v7())).collect(),
        };
        let mut tx = pool.begin().await.unwrap();
        preview.restore(&mut tx, org, &[]).await.unwrap();
        let restored = sqlx::query_scalar!(
            "SELECT count(*) FROM harvest_import_map WHERE org_id = $1",
            org,
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            restored,
            Some(0),
            "unneeded virtual entries have no adoption target"
        );
        tx.rollback().await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        preview
            .restore(&mut tx, org, &[7, 9_999, 7, 10_000])
            .await
            .unwrap();
        let restored = sqlx::query_scalar!(
            "SELECT harvest_id FROM harvest_import_map WHERE org_id = $1 ORDER BY harvest_id",
            org,
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap();
        assert_eq!(restored, vec![7, 9_999]);
        assert_eq!(
            preview.entries.len(),
            10_000,
            "later pages still need all identities"
        );
        tx.rollback().await.unwrap();
    }
}
