//! `reads::page` and `reads::get`: the Projects page's projects, by id, over
//! the own, managed and whole scopes, with their filters and money rule.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::catalog::Permission::{self, *};
use sqlx::PgPool;

use super::*;
use crate::ops::Caller;
use crate::ops::testing::provision;
use crate::testing::{SeedIds, seed};

use super::super::reads::{self, ProjectQuery, ProjectRecord};

const MANAGER: &[Permission] = &[ProjectReadManaged, BillableRateReadManaged];

/// Every active project, from the first page.
fn active(limit: u32) -> ProjectQuery {
    ProjectQuery {
        client_id: None,
        active: true,
        updated_since: None,
        after: None,
        limit,
    }
}

async fn page(
    pool: &PgPool,
    ids: &SeedIds,
    query: ProjectQuery,
) -> Result<Vec<ProjectRecord>, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = reads::page(&mut read.tx, &read.actor, &query).await;
    read.finish(result).await
}

async fn get(pool: &PgPool, ids: &SeedIds, project_id: Uuid) -> Result<ProjectRecord, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = reads::get(&mut read.tx, &read.actor, project_id).await;
    read.finish(result).await
}

/// Every id of `query`'s list, following pages of `query.limit`.
async fn walk(pool: &PgPool, ids: &SeedIds, mut query: ProjectQuery) -> Vec<Uuid> {
    let mut seen = Vec::new();
    loop {
        let mut rows = page(pool, ids, query).await.unwrap();
        let more = rows.len() > query.limit as usize;
        rows.truncate(query.limit as usize);
        seen.extend(rows.iter().map(|row| row.id));
        if !more {
            return seen;
        }
        query.after = seen.last().copied();
    }
}

async fn add_project(pool: &PgPool, ids: &SeedIds, name: &str, active: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, active, rate_cents)
         VALUES ($1, $2, $3, $4, 'EUR', $5, 9000)",
        id,
        ids.org_id,
        ids.client_id,
        name,
        active,
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

async fn manage(pool: &PgPool, ids: &SeedIds, project_id: Uuid) {
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, manager_id, project_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        project_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

/// The ids the Projects page shows the seeded person, archived included.
async fn overview_ids(pool: &PgPool, ids: &SeedIds) -> Vec<Uuid> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = reads::overview(&mut read.tx, &read.actor, None).await;
    let overview = read.finish(result).await.unwrap();
    let mut ids: Vec<Uuid> = overview.projects.iter().map(|row| row.project.id).collect();
    ids.sort();
    ids
}

async fn page_ids(pool: &PgPool, ids: &SeedIds) -> Vec<Uuid> {
    let mut all = walk(pool, ids, active(2)).await;
    all.extend(
        walk(
            pool,
            ids,
            ProjectQuery {
                active: false,
                ..active(2)
            },
        )
        .await,
    );
    all.sort();
    all
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_whole_scope_walks_every_project_once_in_id_order(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let mut expected = vec![ids.project_id];
    for name in ["B", "C", "D", "E"] {
        expected.push(add_project(&pool, &ids, name, true).await);
    }
    add_project(&pool, &ids, "Archived", false).await;
    assert_eq!(walk(&pool, &ids, active(2)).await, expected);

    let first = page(&pool, &ids, active(2)).await.unwrap();
    assert_eq!(first.len(), 3, "a page reads one row past its limit");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn archived_projects_are_listed_apart_and_read_by_id(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let archived = add_project(&pool, &ids, "Old", false).await;
    let listed = walk(
        &pool,
        &ids,
        ProjectQuery {
            active: false,
            ..active(10)
        },
    )
    .await;
    assert_eq!(listed, [archived]);
    assert!(!walk(&pool, &ids, active(10)).await.contains(&archived));

    let project = get(&pool, &ids, archived).await.unwrap();
    assert!(!project.active);
    assert_eq!(
        project.rate_cents,
        Some(9000),
        "an administrator reads rates"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn client_and_updated_since_narrow_the_list(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other_client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Other', 'EUR')",
        other_client,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let elsewhere = add_project(&pool, &ids, "Elsewhere", true).await;
    sqlx::query!(
        "UPDATE projects SET client_id = $2 WHERE id = $1",
        elsewhere,
        other_client
    )
    .execute(&pool)
    .await
    .unwrap();
    let of_client = ProjectQuery {
        client_id: Some(other_client),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, of_client).await, [elsewhere]);

    let since = sqlx::query_scalar!(r#"SELECT now() AS "now!: chrono::DateTime<chrono::Utc>""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET name = 'Renamed' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let changed = ProjectQuery {
        updated_since: Some(since),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, changed).await, [ids.project_id]);
    let renamed = get(&pool, &ids, ids.project_id).await.unwrap();
    assert!(renamed.updated_at >= since && renamed.created_at < since);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_sees_their_projects_without_money(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    assign(&pool, &ids, ids.project_id).await;
    let hidden = add_project(&pool, &ids, "Not mine", true).await;
    let managers_only = add_project(&pool, &ids, "Managers only", true).await;
    assign(&pool, &ids, managers_only).await;
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode)
         VALUES ($1, $2, $3, $4, 'project')",
        Uuid::now_v7(),
        ids.org_id,
        managers_only,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents = 5000 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(walk(&pool, &ids, active(10)).await, [ids.project_id]);
    let own = get(&pool, &ids, ids.project_id).await.unwrap();
    assert_eq!((own.rate_cents, own.budget_amount_cents), (None, None));
    for project in [hidden, managers_only, Uuid::now_v7()] {
        assert_eq!(
            get(&pool, &ids, project).await,
            Err(OpError::not_found("Project not found"))
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_manager_sees_managed_projects_and_their_money(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    provision(&pool, &ids, MANAGER, false).await;
    let managed = add_project(&pool, &ids, "Managed", true).await;
    manage(&pool, &ids, managed).await;
    add_project(&pool, &ids, "Other", true).await;

    assert_eq!(walk(&pool, &ids, active(10)).await, [managed]);
    assert_eq!(
        get(&pool, &ids, managed).await.unwrap().rate_cents,
        Some(9000)
    );
    assert_eq!(
        get(&pool, &ids, ids.project_id).await,
        Err(OpError::not_found("Project not found"))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_scope_lists_what_the_projects_page_shows(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let managed = add_project(&pool, &admin, "Managed", true).await;
    let archived = add_project(&pool, &admin, "Archived", false).await;
    add_project(&pool, &admin, "Nobody's", true).await;
    let member = crate::testing::add_person(&pool, &admin, BuiltInProfile::Member).await;
    assign(&pool, &member, admin.project_id).await;
    assign(&pool, &member, archived).await;
    let manager = crate::testing::add_person(&pool, &admin, BuiltInProfile::ProjectManager).await;
    manage(&pool, &manager, managed).await;

    for (who, ids) in [
        ("administrator", &admin),
        ("member", &member),
        ("manager", &manager),
    ] {
        let listed = page_ids(&pool, ids).await;
        assert!(!listed.is_empty(), "{who}");
        assert_eq!(listed, overview_ids(&pool, ids).await, "{who}");
    }
}
