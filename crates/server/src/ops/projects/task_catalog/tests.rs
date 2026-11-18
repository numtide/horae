//! `visible_page` and `visible_get`: the catalog for task readers, the
//! trackable tasks for anyone else.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::catalog::Permission::TaskReadAll;
use sqlx::PgPool;

use super::*;
use crate::ops::Caller;
use crate::ops::testing::provision;
use crate::testing::{SeedIds, seed};

/// Every active task, from the first page.
fn active(limit: u32) -> TaskQuery {
    TaskQuery {
        project_id: None,
        active: true,
        updated_since: None,
        after: None,
        limit,
    }
}

async fn read_page(
    pool: &PgPool,
    ids: &SeedIds,
    query: TaskQuery,
) -> Result<Vec<TaskRecord>, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = visible_page(&mut read.tx, &read.actor, &query).await;
    read.finish(result).await
}

async fn read_one(pool: &PgPool, ids: &SeedIds, task_id: Uuid) -> Result<TaskRecord, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = visible_get(&mut read.tx, &read.actor, task_id).await;
    read.finish(result).await
}

/// Every id of `query`'s list, following pages of `query.limit`.
async fn walk(pool: &PgPool, ids: &SeedIds, mut query: TaskQuery) -> Vec<Uuid> {
    let mut seen = Vec::new();
    loop {
        let mut rows = read_page(pool, ids, query).await.unwrap();
        let more = rows.len() > query.limit as usize;
        rows.truncate(query.limit as usize);
        seen.extend(rows.iter().map(|row| row.id));
        if !more {
            return seen;
        }
        query.after = seen.last().copied();
    }
}

async fn add_task(pool: &PgPool, ids: &SeedIds, name: &str, active: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, active) VALUES ($1, $2, $3, $4)",
        id,
        ids.org_id,
        name,
        active,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn link(pool: &PgPool, project_id: Uuid, task_id: Uuid) {
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        project_id,
        task_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn add_project(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Other', 'EUR')",
        id,
        ids.org_id,
        ids.client_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn assign(pool: &PgPool, ids: &SeedIds, project_id: Uuid) {
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        project_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_task_reader_walks_the_catalog(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let mut expected = vec![ids.task_id];
    for name in ["B", "C", "D"] {
        expected.push(add_task(&pool, &ids, name, true).await);
    }
    let archived = add_task(&pool, &ids, "Old", false).await;
    assert_eq!(walk(&pool, &ids, active(2)).await, expected);
    let listed = walk(
        &pool,
        &ids,
        TaskQuery {
            active: false,
            ..active(10)
        },
    )
    .await;
    assert_eq!(listed, [archived]);
    assert!(!read_one(&pool, &ids, archived).await.unwrap().active);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_task_reader_narrows_to_a_project_they_may_read(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    provision(&pool, &ids, &[TaskReadAll], false).await;
    let other = add_project(&pool, &ids).await;
    let linked = add_task(&pool, &ids, "Linked", true).await;
    link(&pool, ids.project_id, linked).await;
    link(&pool, other, linked).await;
    assign(&pool, &ids, ids.project_id).await;
    add_task(&pool, &ids, "Loose", true).await;

    let mine = TaskQuery {
        project_id: Some(ids.project_id),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, mine).await, [linked]);
    let unreadable = TaskQuery {
        project_id: Some(other),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, unreadable).await, Vec::<Uuid>::new());
    assert_eq!(
        walk(&pool, &ids, active(10)).await.len(),
        3,
        "the whole catalog"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn anyone_else_sees_the_tasks_they_may_track(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    link(&pool, ids.project_id, ids.task_id).await;
    assign(&pool, &ids, ids.project_id).await;
    let other = add_project(&pool, &ids).await;
    let elsewhere = add_task(&pool, &ids, "Elsewhere", true).await;
    link(&pool, other, elsewhere).await;
    assign(&pool, &ids, other).await;
    let untracked = add_task(&pool, &ids, "Untracked", true).await;

    assert_eq!(walk(&pool, &ids, active(1)).await, [ids.task_id, elsewhere]);
    let of_project = TaskQuery {
        project_id: Some(other),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, of_project).await, [elsewhere]);
    let archived = TaskQuery {
        active: false,
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, archived).await, Vec::<Uuid>::new());
    assert_eq!(
        read_one(&pool, &ids, elsewhere).await.unwrap().name,
        "Elsewhere"
    );
    assert_eq!(
        read_one(&pool, &ids, untracked).await,
        Err(OpError::not_found(NOT_FOUND))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn updated_since_lists_only_changed_tasks(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_task(&pool, &ids, "Untouched", true).await;
    let since = sqlx::query_scalar!(r#"SELECT now() AS "now!: DateTime<Utc>""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE tasks SET name = 'Renamed' WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let changed = TaskQuery {
        updated_since: Some(since),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, changed).await, [ids.task_id]);
}
