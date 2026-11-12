//! Adding the people a Harvest run could not match (spec 019 FR-018).

use super::*;
use crate::ops::testing::Recorder;
use crate::server_fns::test_seed::{built_in_profile, seed};
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
) -> Result<AddedPeople, ServerFnError> {
    add_people(
        pool,
        &Recorder::default(),
        Caller::session(org, actor),
        people,
        BuiltInProfile::Member,
        activation,
        Existing::Skip,
    )
    .await
}

/// Members and whether they are active, in Rust's byte order.
async fn emails(pool: &PgPool, org: uuid::Uuid) -> Vec<(String, bool)> {
    let mut rows: Vec<_> = sqlx::query!(
        "SELECT u.email, u.active FROM users u
           JOIN person_permission_states p ON p.user_id = u.id
          WHERE u.org_id = $1 AND p.built_in_profile = 'member'",
        org
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.email, row.active))
    .collect();
    rows.sort();
    rows
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn people_are_added_deactivated_or_active_as_chosen(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let before = emails(&pool, ids.org_id).await;
    let added = add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("b@x.com"), person("a@x.com")],
        Activation::Deactivated,
    )
    .await
    .unwrap()
    .added;
    assert_eq!(added.len(), 2);
    for user in &added {
        assert!(!user.active);
        assert_eq!(
            built_in_profile(&pool, ids.org_id, user.id).await,
            Some((BuiltInProfile::Member, false))
        );
    }
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn an_email_already_in_use_is_reported_present_and_the_rest_are_added(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("Dana@X.com")],
        Activation::Deactivated,
    )
    .await
    .unwrap();
    let mut expected = emails(&pool, ids.org_id).await;
    expected.push(("new@x.com".to_owned(), false));
    expected.sort();
    // A reload, or another tab, adds the same people again.
    let result = add(
        &pool,
        ids.org_id,
        ids.user_id,
        &[person("new@x.com"), person("dana@x.com")],
        Activation::Deactivated,
    )
    .await
    .unwrap();
    let added: Vec<_> = result
        .added
        .iter()
        .map(|user| user.email.as_str())
        .collect();
    assert_eq!(added, ["new@x.com"]);
    assert_eq!(result.present, ["dana@x.com"]);
    assert_eq!(emails(&pool, ids.org_id).await, expected);

    // Adding one person from People still refuses an email in use.
    let created = insert_user(
        &pool,
        &Recorder::default(),
        Caller::session(ids.org_id, ids.user_id),
        "DANA@x.com",
        "Dana",
        BuiltInProfile::Member,
    )
    .await;
    assert!(
        matches!(
            created,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{created:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn an_email_listed_twice_in_any_case_refuses_the_batch(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let before = emails(&pool, ids.org_id).await;
    for batch in [
        vec![person("twice@x.com"), person("TWICE@x.com")],
        vec![person("Élodie@x.com"), person(" élodie@x.com")],
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
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST,
                    ..
                })
            ),
            "{batch:?}: {result:?}"
        );
        assert_eq!(emails(&pool, ids.org_id).await, before, "{batch:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn adding_people_needs_permission_to_manage_people(pool: PgPool) {
    // Project Managers manage projects, not people.
    let ids = seed(&pool, BuiltInProfile::ProjectManager).await;
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
