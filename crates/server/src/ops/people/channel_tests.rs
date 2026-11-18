//! Adding people through the channels with no person: the server's operator
//! (`horae user create`) and the system (an OIDC first sign-in).

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::Recorder;
use crate::ops::{Caller, begin};
use crate::testing::{built_in_profile, seed_tenant};

fn person(email: &str) -> NewPerson {
    NewPerson {
        email: email.into(),
        name: "Ada".into(),
    }
}

async fn add_as(
    pool: &PgPool,
    caller: Caller,
    email: &str,
    profile: BuiltInProfile,
) -> Result<AddedPeople, OpError> {
    let mut write = begin(pool, caller).await?;
    let result = add(
        &mut write.tx,
        &write.actor,
        &[person(email)],
        profile,
        Activation::Active,
        Existing::Refuse,
    )
    .await;
    write.commit(result, &Recorder::default()).await
}

async fn accounts(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users WHERE org_id = $1"#,
        org_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_channel_without_a_person_has_no_authority_of_its_own(pool: PgPool) {
    let ids = seed_tenant(&pool).await;
    for caller in [Caller::operator(ids.org_id), Caller::system(ids.org_id)] {
        let mut write = begin(&pool, caller).await.unwrap();
        assert_eq!(write.actor.channel(), caller.channel);
        assert!(!write.actor.is_active());
        assert!(!write.actor.administrator());
        assert!(!write.actor.has(Permission::PeopleWriteAll));
        let listed = list(&mut write.tx, &write.actor, false).await;
        assert!(matches!(listed, Err(OpError::Forbidden(_))), "{listed:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_operator_adds_every_built_in_profile_including_administrator(pool: PgPool) {
    let ids = seed_tenant(&pool).await;
    for (index, &profile) in BuiltInProfile::ALL.iter().enumerate() {
        let added = add_as(
            &pool,
            Caller::operator(ids.org_id),
            &format!("person{index}@example.com"),
            profile,
        )
        .await
        .unwrap();
        let [person] = &added.added[..] else {
            panic!("one person expected: {added:?}");
        };
        assert_eq!(
            built_in_profile(&pool, ids.org_id, person.id).await,
            Some((profile, profile == BuiltInProfile::Administrator))
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_operator_cannot_reuse_an_email(pool: PgPool) {
    let ids = seed_tenant(&pool).await;
    let operator = Caller::operator(ids.org_id);
    add_as(&pool, operator, "ada@example.com", BuiltInProfile::Member)
        .await
        .unwrap();
    let again = add_as(&pool, operator, "ADA@example.com", BuiltInProfile::Member).await;
    assert_eq!(
        again,
        Err(OpError::conflict(
            "A user with the email ADA@example.com already exists"
        ))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_system_adds_a_member_and_reports_it(pool: PgPool) {
    let ids = seed_tenant(&pool).await;
    let mut write = begin(&pool, Caller::system(ids.org_id)).await.unwrap();
    let result = add(
        &mut write.tx,
        &write.actor,
        &[person("ada@example.com")],
        BuiltInProfile::Member,
        Activation::Active,
        Existing::Skip,
    )
    .await;
    let events = Recorder::default();
    let added = write.commit(result, &events).await.unwrap();
    assert_eq!(
        built_in_profile(&pool, ids.org_id, added.added[0].id).await,
        Some((BuiltInProfile::Member, false))
    );
    assert!(matches!(
        &events.take()[..],
        [AppEvent::UserCreated { user, .. }] if user.id == added.added[0].id
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_adds_no_one_even_for_an_administrator(pool: PgPool) {
    let ids = crate::testing::seed(&pool, BuiltInProfile::Administrator).await;
    let before = accounts(&pool, ids.org_id).await;
    let caller = Caller::token(ids.org_id, ids.user_id, Uuid::now_v7());
    let refused = add_as(&pool, caller, "ada@example.com", BuiltInProfile::Member).await;
    assert_eq!(
        refused,
        Err(OpError::forbidden(
            "People can't be added with an API token"
        ))
    );
    assert_eq!(accounts(&pool, ids.org_id).await, before);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_system_never_adds_an_administrator_or_any_profile_above_member(pool: PgPool) {
    let ids = seed_tenant(&pool).await;
    let before = accounts(&pool, ids.org_id).await;
    for &profile in BuiltInProfile::ALL {
        if profile == BuiltInProfile::Member {
            continue;
        }
        let refused = add_as(
            &pool,
            Caller::system(ids.org_id),
            "ada@example.com",
            profile,
        )
        .await;
        assert_eq!(
            refused,
            Err(OpError::forbidden("Automatic sign-up adds members only")),
            "{profile:?}"
        );
    }
    assert_eq!(accounts(&pool, ids.org_id).await, before);
}
