//! Import API catalog records independently of time entries, in FK order.

use std::collections::HashMap;

use horae_core::importers::harvest::types::{EntityType, RowOutcome};
use sqlx::{Acquire, Postgres, Transaction};

use super::api_source::{ApiClient, ApiProject, ApiTask, HarvestData};
use super::report::ImportReport;
use super::resolve::fields::{ClientFields, ProjectFields, TaskFields};
use super::resolve::{self, OrgDefaults, PendingCache, Resolved, RowFailure, RunCache};

enum Parent<'a> {
    Client(&'a ApiClient),
    Project(&'a ApiProject, Option<&'a ApiClient>),
    Task(&'a ApiTask),
}

impl Parent<'_> {
    fn identity(&self) -> (EntityType, i64) {
        match self {
            Self::Client(c) => (EntityType::Client, c.id),
            Self::Project(p, _) => (EntityType::Project, p.id),
            Self::Task(t) => (EntityType::Task, t.id),
        }
    }
}

fn client_fields(client: &ApiClient) -> ClientFields<'_> {
    ClientFields {
        harvest_client_id: Some(client.id),
        client_name: &client.name,
        client_address: client.address.as_deref(),
        client_active: client.is_active,
        currency: client.currency.as_deref(),
        harvest_updated_at: client.updated_at,
    }
}

/// Each catalog record has its own savepoint. Failed records never populate the
/// cache; their dependents are reported as errors instead of inventing parents.
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    cache: &mut RunCache,
    org: OrgDefaults<'_>,
    data: &HarvestData,
    report: &mut ImportReport,
) -> anyhow::Result<()> {
    let clients: HashMap<_, _> = data.clients.iter().map(|c| (c.id, c)).collect();
    let parents = data
        .clients
        .iter()
        .map(Parent::Client)
        .chain(
            data.projects
                .iter()
                .map(|p| Parent::Project(p, clients.get(&p.client.id).copied())),
        )
        .chain(data.tasks.iter().map(Parent::Task));
    for parent in parents {
        let (entity, id) = parent.identity();
        let mut sp = tx.begin().await?;
        match resolve_parent(&mut sp, cache, org, parent).await {
            Ok(resolved) => {
                sp.commit().await?;
                let mut outcomes = Vec::new();
                let mut pending = PendingCache::default();
                for (kind, resolved) in resolved {
                    super::apply::fold(&mut outcomes, &mut pending, kind, resolved);
                }
                cache.merge(pending);
                for (kind, outcome) in outcomes {
                    report.record(kind, &outcome);
                }
            }
            Err(failure) => {
                sp.rollback().await?;
                cache.mark_failed(entity, id);
                report.record(
                    entity,
                    &RowOutcome::Errored {
                        source_location: format!("{} {id}", entity.as_str()),
                        reason: failure.reason,
                    },
                );
            }
        }
    }
    Ok(())
}

async fn resolve_parent(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    parent: Parent<'_>,
) -> Result<Vec<(EntityType, Resolved)>, RowFailure> {
    match parent {
        Parent::Client(client) => Ok(vec![(
            EntityType::Client,
            resolve::resolve_client(conn, cache, org, &client_fields(client)).await?,
        )]),
        Parent::Project(project, client) => {
            if cache.parent_failed(EntityType::Client, project.client.id) {
                return Err(RowFailure::new(format!(
                    "client {} failed to import",
                    project.client.id
                )));
            }
            // An embedded client name can recover a missing collection record,
            // but an ID alone must resolve existing provenance or fail visibly.
            let client = client.map(client_fields).unwrap_or(ClientFields {
                harvest_client_id: Some(project.client.id),
                client_name: project.client.name.as_deref().unwrap_or_default(),
                client_address: None,
                client_active: true,
                currency: None,
                harvest_updated_at: None,
            });
            let resolved_client = resolve::resolve_client(conn, cache, org, &client).await?;
            let fields = ProjectFields {
                harvest_project_id: Some(project.id),
                client_name: client.client_name,
                project_name: &project.name,
                project_code: project.code.as_deref(),
                project_active: project.is_active,
                project_starts_on: project.starts_on,
                project_ends_on: project.ends_on,
                harvest_updated_at: project.updated_at,
            };
            let resolved_project =
                resolve::resolve_project(conn, cache, org, resolved_client.id, &fields).await?;
            Ok(vec![
                (EntityType::Client, resolved_client),
                (EntityType::Project, resolved_project),
            ])
        }
        Parent::Task(task) => {
            let rate = task
                .default_hourly_rate
                .as_ref()
                .map(super::api_source::decimal);
            let fields = TaskFields {
                harvest_task_id: Some(task.id),
                task_name: &task.name,
                task_billable_default: task.billable_by_default,
                task_active: task.is_active,
                billable_rate: rate.as_deref(),
                harvest_updated_at: task.updated_at,
            };
            Ok(vec![(
                EntityType::Task,
                resolve::resolve_task(conn, cache, org, &fields).await?,
            )])
        }
    }
}
