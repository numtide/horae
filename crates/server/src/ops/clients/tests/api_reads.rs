//! `page` and `get`: the clients of `list`, by id, over the catalog and the
//! project scopes, with their filters and the catalog-only details.

use super::*;

/// Every active client, from the first page.
fn active(limit: u32) -> ClientQuery {
    ClientQuery {
        active: true,
        updated_since: None,
        after: None,
        limit,
    }
}

async fn read_page(
    pool: &PgPool,
    ids: &SeedIds,
    query: ClientQuery,
) -> Result<Vec<ClientRecord>, OpError> {
    with_read_tx(pool, caller(ids), async |tx, actor| {
        page(tx, actor, &query).await
    })
    .await
}

async fn read_one(pool: &PgPool, ids: &SeedIds, client_id: Uuid) -> Result<ClientRecord, OpError> {
    with_read_tx(pool, caller(ids), async |tx, actor| {
        get(tx, actor, client_id).await
    })
    .await
}

/// Every id of `query`'s list, following pages of `query.limit`.
async fn walk(pool: &PgPool, ids: &SeedIds, mut query: ClientQuery) -> Vec<Uuid> {
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

async fn add_client(pool: &PgPool, ids: &SeedIds, name: &str, active: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, active, address)
         VALUES ($1, $2, $3, 'EUR', $4, 'Road')",
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_catalog_reader_walks_every_client_with_its_details(pool: PgPool) {
    let ids = fixture(&pool, Who::Grants(CATALOG)).await;
    let mut expected = vec![ids.client_id];
    for name in ["B", "C", "D"] {
        expected.push(add_client(&pool, &ids, name, true).await);
    }
    assert_eq!(walk(&pool, &ids, active(2)).await, expected);

    let client = read_one(&pool, &ids, ids.client_id).await.unwrap();
    assert_eq!(client.address.as_deref(), Some("Street"));
    assert_eq!(client.tax_id.as_deref(), Some("VAT"));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_member_sees_the_names_of_their_projects_clients(pool: PgPool) {
    let ids = fixture(&pool, Who::Profile(BuiltInProfile::Member)).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = add_client(&pool, &ids, "Other", true).await;

    assert_eq!(walk(&pool, &ids, active(10)).await, [ids.client_id]);
    let client = read_one(&pool, &ids, ids.client_id).await.unwrap();
    assert_eq!((client.address, client.tax_id), (None, None));
    assert_eq!(
        read_one(&pool, &ids, other).await,
        Err(OpError::not_found(NOT_FOUND))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn without_a_project_a_member_sees_no_client(pool: PgPool) {
    let ids = fixture(&pool, Who::Grants(NOTHING)).await;
    assert_eq!(walk(&pool, &ids, active(10)).await, Vec::<Uuid>::new());
    assert_eq!(
        read_one(&pool, &ids, ids.client_id).await,
        Err(OpError::not_found(NOT_FOUND))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn archived_clients_are_listed_apart_and_read_by_id(pool: PgPool) {
    let ids = fixture(&pool, Who::Grants(CATALOG)).await;
    let archived = add_client(&pool, &ids, "Old", false).await;
    let listed = walk(
        &pool,
        &ids,
        ClientQuery {
            active: false,
            ..active(10)
        },
    )
    .await;
    assert_eq!(listed, [archived]);
    assert!(!walk(&pool, &ids, active(10)).await.contains(&archived));
    assert!(!read_one(&pool, &ids, archived).await.unwrap().active);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn updated_since_lists_only_changed_clients(pool: PgPool) {
    let ids = fixture(&pool, Who::Grants(CATALOG)).await;
    add_client(&pool, &ids, "Untouched", true).await;
    let since = sqlx::query_scalar!(r#"SELECT now() AS "now!: chrono::DateTime<chrono::Utc>""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE clients SET name = 'Renamed' WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let changed = ClientQuery {
        updated_since: Some(since),
        ..active(10)
    };
    assert_eq!(walk(&pool, &ids, changed).await, [ids.client_id]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_scope_lists_what_the_pickers_list(pool: PgPool) {
    let admin = fixture(&pool, Who::Profile(BuiltInProfile::Administrator)).await;
    add_client(&pool, &admin, "Unused", true).await;
    add_client(&pool, &admin, "Old", false).await;
    let member = crate::testing::add_person(&pool, &admin, BuiltInProfile::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        admin.project_id,
        member.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let stranger = crate::testing::add_person(&pool, &admin, BuiltInProfile::Member).await;

    for (who, ids) in [
        ("catalog", &admin),
        ("member", &member),
        ("stranger", &stranger),
    ] {
        let mut listed = walk(&pool, ids, active(2)).await;
        listed.extend(
            walk(
                &pool,
                ids,
                ClientQuery {
                    active: false,
                    ..active(2)
                },
            )
            .await,
        );
        listed.sort();
        let mut pickers: Vec<Uuid> = with_read_tx(&pool, caller(ids), async |tx, actor| {
            list(tx, actor, true).await
        })
        .await
        .unwrap()
        .into_iter()
        .map(|client| client.id)
        .collect();
        pickers.sort();
        assert_eq!(listed, pickers, "{who}");
    }
}
