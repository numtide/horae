//! Adding the people a Harvest run could not match (spec 019 FR-018).

use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;

fn person(email: &str) -> NewPerson {
    NewPerson {
        email: email.into(),
        name: "Dana Dev".into(),
    }
}

async fn add(
    pool: &PgPool,
    org: uuid::Uuid,
    actor: uuid::Uuid,
    people: &[NewPerson],
    activation: Activation,
) -> Result<Vec<User>, ServerFnError> {
    insert_users(pool, org, actor, people, OrgRole::Member, activation).await
}

async fn emails(pool: &PgPool, org: uuid::Uuid) -> Vec<(String, bool)> {
    sqlx::query!(
        "SELECT email, active FROM users WHERE org_id = $1 AND org_role = 'member' ORDER BY email",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.email, row.active))
    .collect()
}

#[test]
fn a_new_person_needs_an_email_and_falls_back_to_it_for_a_name() {
    assert_eq!(
        NewPerson {
            email: " dana@x.com ".into(),
            name: "  ".into(),
        }
        .normalized(),
        Some(NewPerson {
            email: "dana@x.com".into(),
            name: "dana@x.com".into(),
        })
    );
    for email in ["", "   ", "no-at-sign"] {
        assert_eq!(person(email).normalized(), None, "{email:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn people_are_added_deactivated_or_active_as_chosen(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let before = emails(&pool, ids.org_id).await;
    let added = add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("b@x.com"), person("a@x.com")],
        Activation::Deactivated,
    )
    .await
    .unwrap();
    assert_eq!(added.len(), 2);
    assert!(
        added
            .iter()
            .all(|user| !user.active && user.org_role == OrgRole::Member)
    );
    add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("c@x.com")],
        Activation::Active,
    )
    .await
    .unwrap();
    let mut expected = before;
    expected.extend([
        ("a@x.com".to_owned(), false),
        ("b@x.com".to_owned(), false),
        ("c@x.com".to_owned(), true),
    ]);
    expected.sort();
    assert_eq!(emails(&pool, ids.org_id).await, expected);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn an_email_in_use_in_any_case_refuses_the_whole_batch(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("Dana@X.com")],
        Activation::Deactivated,
    )
    .await
    .unwrap();
    let before = emails(&pool, ids.org_id).await;
    for batch in [
        vec![person("new@x.com"), person("dana@x.com")],
        vec![person("twice@x.com"), person("TWICE@x.com")],
    ] {
        let result = add(
            &pool,
            ids.org_id,
            ids.user_id,
            &batch,
            Activation::Deactivated,
        )
        .await;
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError { code, .. }) if code == CONFLICT || code == BAD_REQUEST
            ),
            "{batch:?}: {result:?}"
        );
        assert_eq!(emails(&pool, ids.org_id).await, before, "{batch:?}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn only_an_active_administrator_adds_people(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("a@x.com")],
        Activation::Deactivated,
    )
    .await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ),
        "{result:?}"
    );
}
