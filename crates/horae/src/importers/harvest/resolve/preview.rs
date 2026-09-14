//! Parent state needed to continue a rollback-only import simulation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use super::RunCache;

/// Restore cached parents and project/task links after a simulation rolls back.
/// CSV occurrence counters and API entry provenance are retained separately.
#[derive(Serialize, Deserialize)]
pub(in crate::importers::harvest) struct ParentSnapshot {
    clients: Value,
    projects: Value,
    tasks: Value,
    project_tasks: Value,
}

impl ParentSnapshot {
    pub(in crate::importers::harvest) async fn capture(
        conn: &mut PgConnection,
        org_id: Uuid,
        cache: &RunCache,
    ) -> sqlx::Result<Self> {
        let clients: Vec<_> = cache.clients.values().copied().collect();
        let projects: Vec<_> = cache.projects.values().copied().collect();
        let tasks: Vec<_> = cache.tasks.values().copied().collect();
        let (linked_projects, linked_tasks): (Vec<_>, Vec<_>) =
            cache.project_tasks.iter().copied().unzip();
        // OFFSET 0 keeps the selected pairs driving indexed link lookups even
        // when statistics cannot see the parents created by this simulation.
        Ok(Self {
            clients: sqlx::query_scalar!(
                r#"SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb) AS "rows!"
                   FROM (SELECT id, org_id, name, currency, address, active FROM clients
                         WHERE org_id = $1 AND id = ANY($2)) r"#,
                org_id, &clients,
            ).fetch_one(&mut *conn).await?,
            projects: sqlx::query_scalar!(
                r#"SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb) AS "rows!"
                   FROM (SELECT id, org_id, client_id, code, name, currency, starts_on, ends_on, active
                         FROM projects WHERE org_id = $1 AND id = ANY($2)) r"#,
                org_id, &projects,
            ).fetch_one(&mut *conn).await?,
            tasks: sqlx::query_scalar!(
                r#"SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb) AS "rows!"
                   FROM (SELECT id, org_id, name, billable_default, default_rate_cents, active
                         FROM tasks WHERE org_id = $1 AND id = ANY($2)) r"#,
                org_id, &tasks,
            ).fetch_one(&mut *conn).await?,
            project_tasks: sqlx::query_scalar!(
                r#"SELECT COALESCE(jsonb_agg(to_jsonb(pt)), '[]'::jsonb) AS "rows!"
                   FROM unnest($2::uuid[], $3::uuid[]) AS selected(project_id, task_id)
                   JOIN LATERAL (
                     SELECT pt.* FROM project_tasks pt
                     WHERE pt.project_id = selected.project_id AND pt.task_id = selected.task_id
                       AND (SELECT org_id FROM projects WHERE id = pt.project_id) = $1
                       AND (SELECT org_id FROM tasks WHERE id = pt.task_id) = $1
                     OFFSET 0
                   ) pt ON true"#,
                org_id, &linked_projects, &linked_tasks,
            ).fetch_one(&mut *conn).await?,
        })
    }

    /// Call only inside a transaction that will be rolled back. Restore stored
    /// identities/attributes directly, without rerunning completed source rows
    /// or changing their accumulated outcomes. Existing domain rows are untouched.
    pub(in crate::importers::harvest) async fn restore(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> anyhow::Result<()> {
        let owner = org_id.to_string();
        for rows in [&self.clients, &self.projects, &self.tasks] {
            anyhow::ensure!(
                rows.as_array().is_some_and(|rows| rows.iter().all(|row| {
                    row.get("org_id").and_then(Value::as_str) == Some(owner.as_str())
                })),
                "preview checkpoint organization mismatch"
            );
        }
        sqlx::query!(
            "INSERT INTO clients (id, org_id, name, currency, address, active)
             SELECT id, org_id, name, currency, address, active
             FROM jsonb_populate_recordset(NULL::clients, $1) WHERE org_id = $2
             ON CONFLICT (id) DO NOTHING",
            &self.clients,
            org_id,
        )
        .execute(&mut *conn)
        .await?;
        // Rollback-only parents can have near-empty table statistics despite
        // thousands of simulated rows. Scalar lookups keep ownership checks
        // from becoming full-catalog nested loops under those estimates.
        sqlx::query!(
            "INSERT INTO projects (id, org_id, client_id, code, name, currency, starts_on, ends_on, active)
             SELECT r.id, r.org_id, r.client_id, r.code, r.name, r.currency, r.starts_on, r.ends_on, r.active
             FROM jsonb_populate_recordset(NULL::projects, $1) r
             WHERE r.org_id = $2
               AND (SELECT org_id FROM clients WHERE id = r.client_id) = $2
             ON CONFLICT (id) DO NOTHING",
            &self.projects, org_id,
        ).execute(&mut *conn).await?;
        sqlx::query!(
            "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, active)
             SELECT id, org_id, name, billable_default, default_rate_cents, active
             FROM jsonb_populate_recordset(NULL::tasks, $1) WHERE org_id = $2
             ON CONFLICT (id) DO NOTHING",
            &self.tasks,
            org_id,
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
             SELECT r.project_id, r.task_id, r.billable, r.rate_cents
             FROM jsonb_populate_recordset(NULL::project_tasks, $1) r
             WHERE (SELECT org_id FROM projects WHERE id = r.project_id) = $2
               AND (SELECT org_id FROM tasks WHERE id = r.task_id) = $2
             ON CONFLICT (project_id, task_id) DO NOTHING",
            &self.project_tasks,
            org_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn rejects_foreign_parent_snapshots_before_restoring_any_rows(pool: sqlx::PgPool) {
        let owner = Uuid::now_v7();
        for field in ["clients", "projects", "tasks"] {
            let mut value = serde_json::json!({
                "clients": [], "projects": [], "tasks": [], "project_tasks": []
            });
            value[field] = serde_json::json!([{"org_id": Uuid::now_v7()}]);
            let snapshot: ParentSnapshot = serde_json::from_value(value).unwrap();
            let mut tx = pool.begin().await.unwrap();
            let error = snapshot.restore(&mut tx, owner).await.unwrap_err();
            assert!(
                error.to_string().contains("organization mismatch"),
                "{field}: {error}"
            );
            tx.rollback().await.unwrap();
        }
    }
}
