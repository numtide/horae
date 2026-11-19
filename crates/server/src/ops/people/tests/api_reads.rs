//! `page` and `get`: the people of `list`, by id. Active people for any
//! active person; archived people with `PeopleReadAll`.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::*;
use crate::ops::Caller;
use crate::testing::{SeedIds, add_person, seed};

/// Every active person, from the first page.
fn active(limit: u32) -> PersonQuery {
    PersonQuery {
        active: true,
        updated_since: None,
        after: None,
        limit,
    }
}

async fn read_page(
    pool: &PgPool,
    ids: &SeedIds,
    query: PersonQuery,
) -> Result<Vec<PersonRecord>, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = page(&mut read.tx, &read.actor, &query).await;
    read.finish(result).await
}

async fn read_one(pool: &PgPool, ids: &SeedIds, person_id: Uuid) -> Result<PersonRecord, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = get(&mut read.tx, &read.actor, person_id).await;
    read.finish(result).await
}

/// Every id of `query`'s list, following pages of `query.limit`.
async fn walk(pool: &PgPool, ids: &SeedIds, mut query: PersonQuery) -> Vec<Uuid> {
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

async fn archive(pool: &PgPool, person_id: Uuid) {
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", person_id)
        .execute(pool)
        .await
        .unwrap();
}

/// The seeded person, three active teammates and an archived one.
async fn team(pool: &PgPool, profile: BuiltInProfile) -> (SeedIds, Vec<Uuid>, Uuid) {
    let ids = seed(pool, profile).await;
    let mut active = vec![ids.user_id];
    for _ in 0..3 {
        active.push(add_person(pool, &ids, BuiltInProfile::Member).await.user_id);
    }
    let archived = add_person(pool, &ids, BuiltInProfile::Member).await.user_id;
    archive(pool, archived).await;
    (ids, active, archived)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_walks_the_active_people_in_id_order(pool: PgPool) {
    let (ids, active_people, archived) = team(&pool, BuiltInProfile::Member).await;
    assert_eq!(walk(&pool, &ids, active(2)).await, active_people);

    let person = read_one(&pool, &ids, active_people[1]).await.unwrap();
    assert_eq!((person.name.as_str(), person.active), ("Teammate", true));
    assert_eq!(
        read_one(&pool, &ids, archived).await,
        Err(OpError::not_found(NOT_FOUND)),
        "an archived person is hidden without PeopleReadAll"
    );
    assert_eq!(
        read_page(
            &pool,
            &ids,
            PersonQuery {
                active: false,
                ..active(10)
            }
        )
        .await,
        Err(OpError::forbidden(READ_REQUIRED))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn people_readers_list_and_read_archived_people(pool: PgPool) {
    let (ids, _, archived) = team(&pool, BuiltInProfile::Administrator).await;
    let listed = walk(
        &pool,
        &ids,
        PersonQuery {
            active: false,
            ..active(10)
        },
    )
    .await;
    assert_eq!(listed, [archived]);
    assert!(!read_one(&pool, &ids, archived).await.unwrap().active);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn updated_since_lists_only_changed_people(pool: PgPool) {
    let (ids, active_people, _) = team(&pool, BuiltInProfile::Member).await;
    let since = sqlx::query_scalar!(r#"SELECT now() AS "now!: DateTime<Utc>""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET name = 'Renamed' WHERE id = $1",
        active_people[2]
    )
    .execute(&pool)
    .await
    .unwrap();
    let changed = PersonQuery {
        updated_since: Some(since),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, changed).await, [active_people[2]]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_scope_lists_what_the_pickers_list(pool: PgPool) {
    let (admin, _, _) = team(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    for (who, ids, include_inactive) in
        [("administrator", &admin, true), ("member", &member, false)]
    {
        let mut listed = walk(&pool, ids, active(2)).await;
        if include_inactive {
            listed.extend(
                walk(
                    &pool,
                    ids,
                    PersonQuery {
                        active: false,
                        ..active(2)
                    },
                )
                .await,
            );
        }
        listed.sort();
        let mut read = crate::ops::begin_read(&pool, Caller::session(ids.org_id, ids.user_id))
            .await
            .unwrap();
        let result = list(&mut read.tx, &read.actor, include_inactive).await;
        let mut pickers: Vec<Uuid> = read
            .finish(result)
            .await
            .unwrap()
            .into_iter()
            .map(|person| person.id)
            .collect();
        pickers.sort();
        assert_eq!(listed, pickers, "{who}");
    }
}
