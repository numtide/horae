//! The catalogue reads of `/api/v1` hold the plan's page target: a page of
//! 1,000 rows in under a second, walking 10,000 projects, clients, tasks and
//! people as an administrator and as a member who works on half of them.

use std::time::{Duration, Instant};

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::testing::{SeedIds, add_person, seed};

const ROWS: usize = 10_000;
const LIMIT: u32 = 1_000;
const TARGET: Duration = Duration::from_secs(1);

fn new_ids() -> Vec<Uuid> {
    (0..ROWS).map(|_| Uuid::now_v7()).collect()
}

/// `ROWS` of each catalogue table; project `n` has client `n` and task `n`,
/// and the member works on every second project.
async fn load(pool: &PgPool, admin: &SeedIds, member: &SeedIds) {
    let (clients, projects, tasks, people) = (new_ids(), new_ids(), new_ids(), new_ids());
    let assignments: Vec<Uuid> = new_ids().into_iter().step_by(2).collect();
    let assigned: Vec<Uuid> = projects.iter().copied().step_by(2).collect();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency)
         SELECT id, $2, 'Client ' || n, 'EUR' FROM unnest($1::uuid[]) WITH ORDINALITY AS c(id, n)",
        &clients,
        admin.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency)
         SELECT id, $3, client_id, 'Project ' || n, 'EUR'
         FROM unnest($1::uuid[], $2::uuid[]) WITH ORDINALITY AS p(id, client_id, n)",
        &projects,
        &clients,
        admin.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name)
         SELECT id, $2, 'Task ' || n FROM unnest($1::uuid[]) WITH ORDINALITY AS t(id, n)",
        &tasks,
        admin.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable)
         SELECT project_id, task_id, true FROM unnest($1::uuid[], $2::uuid[]) AS pt(project_id, task_id)",
        &projects,
        &tasks,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id)
         SELECT id, project_id, $3 FROM unnest($1::uuid[], $2::uuid[]) AS a(id, project_id)",
        &assignments,
        &assigned,
        member.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name)
         SELECT id, $2, id || '@load.test', 'Person ' || n
         FROM unnest($1::uuid[]) WITH ORDINALITY AS u(id, n)",
        &people,
        admin.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("ANALYZE").execute(pool).await.unwrap();
}

/// Walks one list in pages of `LIMIT`, failing on a page slower than
/// `TARGET`, and gives the rows seen and the slowest page.
async fn walk<R>(
    pool: &PgPool,
    ids: &SeedIds,
    read: impl AsyncFn(&mut Tx, &Actor, Option<Uuid>) -> Result<Vec<R>, OpError>,
    id: impl Fn(&R) -> Uuid,
) -> (usize, Duration) {
    let (mut seen, mut slowest, mut after) = (0, Duration::ZERO, None);
    loop {
        let started = Instant::now();
        let mut tx = begin_read(pool, Caller::session(ids.org_id, ids.user_id))
            .await
            .unwrap();
        let result = read(&mut tx.tx, &tx.actor, after).await;
        let mut rows = tx.finish(result).await.unwrap();
        let elapsed = started.elapsed();
        assert!(elapsed < TARGET, "a page took {elapsed:?}");
        slowest = slowest.max(elapsed);
        let more = rows.len() > LIMIT as usize;
        rows.truncate(LIMIT as usize);
        seen += rows.len();
        after = rows.last().map(&id);
        if !more {
            return (seen, slowest);
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn catalogue_pages_of_a_thousand_rows_take_under_a_second(pool: PgPool) {
    use super::clients::ClientQuery;
    use super::people::PersonQuery;
    use super::projects::reads::ProjectQuery;
    use super::projects::task_catalog::TaskQuery;

    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    load(&pool, &admin, &member).await;

    for (who, ids, visible) in [
        ("administrator", &admin, ROWS + 1),
        ("member", &member, ROWS / 2),
    ] {
        let projects = walk(
            &pool,
            ids,
            async |tx, actor, after| {
                let query = ProjectQuery {
                    client_id: None,
                    active: true,
                    updated_since: None,
                    after,
                    limit: LIMIT,
                };
                projects::reads::page(tx, actor, &query).await
            },
            |row| row.id,
        )
        .await;
        let tasks = walk(
            &pool,
            ids,
            async |tx, actor, after| {
                let query = TaskQuery {
                    project_id: None,
                    active: true,
                    updated_since: None,
                    after,
                    limit: LIMIT,
                };
                projects::task_catalog::visible_page(tx, actor, &query).await
            },
            |row| row.id,
        )
        .await;
        let clients = walk(
            &pool,
            ids,
            async |tx, actor, after| {
                let query = ClientQuery {
                    active: true,
                    updated_since: None,
                    after,
                    limit: LIMIT,
                };
                clients::page(tx, actor, &query).await
            },
            |row| row.id,
        )
        .await;
        let people = walk(
            &pool,
            ids,
            async |tx, actor, after| {
                let query = PersonQuery {
                    active: true,
                    updated_since: None,
                    after,
                    limit: LIMIT,
                };
                people::page(tx, actor, &query).await
            },
            |row| row.id,
        )
        .await;
        eprintln!(
            "{who}: slowest page projects {:?}, tasks {:?}, clients {:?}, people {:?}",
            projects.1, tasks.1, clients.1, people.1
        );
        // The member sees half the projects, their tasks and clients; the
        // administrator all of them and the seeded ones.
        assert_eq!(
            (projects.0, tasks.0, clients.0),
            (visible, visible, visible),
            "{who}"
        );
        assert_eq!(people.0, ROWS + 2, "{who}");
    }
}
