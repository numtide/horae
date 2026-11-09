//! Import API catalog records independently of time entries, in FK order:
//! clients, projects, tasks, then people, so entries find all of them.

use std::collections::{HashMap, HashSet};

use horae_core::importers::harvest::keys;
use horae_core::importers::harvest::types::{
    EntityType, PersonErrorKind, PersonOutcome, ReportPerson, RowOutcome,
};
use sqlx::{Acquire, Postgres, Transaction};

use super::api_source::{ApiClient, ApiProject, ApiTask, ApiUser, HarvestData};
use super::report::ImportReport;
use super::resolve::fields::{ClientFields, ProjectFields, TaskFields};
use super::resolve::{
    self, OrgDefaults, PendingCache, PersonFields, Resolved, RowFailure, RunCache,
};

enum Parent<'a> {
    Client(&'a ApiClient),
    Project(&'a ApiProject, Option<&'a ApiClient>),
    Task(&'a ApiTask),
    Person(&'a ApiUser),
}

impl Parent<'_> {
    fn identity(&self) -> (EntityType, i64) {
        match self {
            Self::Client(c) => (EntityType::Client, c.id),
            Self::Project(p, _) => (EntityType::Project, p.id),
            Self::Task(t) => (EntityType::Task, t.id),
            Self::Person(u) => (EntityType::Person, u.id),
        }
    }
}

/// What a resolved catalog record contributes once its savepoint commits.
enum Applied {
    Parents(Vec<(EntityType, Resolved)>),
    Person(resolve::UserKey, ReportPerson),
}

/// Normalized emails that more than one Harvest person uses. Matching either
/// person would be a guess, so all of them are reported instead.
fn shared_emails(users: &[ApiUser]) -> HashSet<String> {
    let mut seen = HashSet::new();
    users
        .iter()
        .filter_map(|u| u.email.as_deref().map(keys::normalize))
        .filter(|email| !email.is_empty() && !seen.insert(email.clone()))
        .collect()
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
    apply_batch(tx, cache, org, data, report, 0..usize::MAX).await
}

pub(super) async fn apply_batch(
    tx: &mut Transaction<'_, Postgres>,
    cache: &mut RunCache,
    org: OrgDefaults<'_>,
    data: &HarvestData,
    report: &mut ImportReport,
    range: std::ops::Range<usize>,
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
        .chain(data.tasks.iter().map(Parent::Task))
        .chain(data.users.iter().map(Parent::Person));
    let shared = shared_emails(&data.users);
    for parent in parents.skip(range.start).take(range.len()) {
        let (entity, id) = parent.identity();
        let mut sp = tx.begin().await?;
        match resolve_parent(&mut sp, cache, org, &shared, parent).await {
            Ok(Applied::Parents(resolved)) => {
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
            Ok(Applied::Person(key, person)) => {
                sp.commit().await?;
                cache.merge(PendingCache {
                    user: Some((key, person.horae_id)),
                    ..Default::default()
                });
                report.record(
                    EntityType::Person,
                    &match person.outcome {
                        PersonOutcome::Created => RowOutcome::Created,
                        PersonOutcome::Matched => RowOutcome::Skipped,
                    },
                );
                report.people.push(person);
            }
            Err(failure) => {
                sp.rollback().await?;
                cache.mark_failed(entity, id);
                report.record(
                    entity,
                    &RowOutcome::Errored {
                        source_location: format!("{} {id}", entity.as_str()),
                        reason: failure.reason,
                        person: failure.person,
                        entry_kind: failure.entry_kind,
                    },
                );
            }
        }
    }
    Ok(())
}

async fn resolve_person(
    conn: &mut sqlx::PgConnection,
    org: OrgDefaults<'_>,
    shared: &HashSet<String>,
    user: &ApiUser,
) -> Result<Applied, RowFailure> {
    let name = user.full_name();
    let email = user.email.as_deref().map(keys::trim_ws).unwrap_or_default();
    if email.is_empty() {
        return Err(RowFailure::person(
            "Harvest person has no email; add one in Harvest, or add the person in Horae",
            PersonErrorKind::MissingEmail,
            None,
            Some(&name),
        ));
    }
    if shared.contains(&keys::normalize(email)) {
        return Err(RowFailure::person(
            format!(
                "another Harvest person also uses the email {email:?}; give each a unique email"
            ),
            PersonErrorKind::DuplicateEmail,
            Some(email),
            Some(&name),
        ));
    }
    let fields = PersonFields {
        harvest_id: Some(user.id),
        email,
        name: &name,
    };
    let (key, horae_id, outcome) = resolve::resolve_person(conn, org.org_id, &fields).await?;
    Ok(Applied::Person(
        key,
        ReportPerson {
            harvest_id: Some(user.id),
            horae_id,
            email: Some(email.to_owned()),
            name: if name.is_empty() {
                email.to_owned()
            } else {
                name
            },
            outcome,
            harvest_active: Some(user.is_active),
        },
    ))
}

async fn resolve_parent(
    conn: &mut sqlx::PgConnection,
    cache: &RunCache,
    org: OrgDefaults<'_>,
    shared: &HashSet<String>,
    parent: Parent<'_>,
) -> Result<Applied, RowFailure> {
    Ok(Applied::Parents(match parent {
        Parent::Person(user) => return resolve_person(conn, org, shared, user).await,
        Parent::Client(client) => vec![(
            EntityType::Client,
            resolve::resolve_client(conn, cache, org, &client_fields(client)).await?,
        )],
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
            vec![
                (EntityType::Client, resolved_client),
                (EntityType::Project, resolved_project),
            ]
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
                task_is_default: task.is_default,
                task_active: task.is_active,
                billable_rate: rate.as_deref(),
                // The task catalog response does not identify its currency.
                rate_currency: None,
                harvest_updated_at: task.updated_at,
            };
            vec![(
                EntityType::Task,
                resolve::resolve_task(conn, cache, org, &fields).await?,
            )]
        }
    }))
}
