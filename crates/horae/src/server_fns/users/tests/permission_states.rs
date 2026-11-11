//! People created here carry the permission state of the chosen profile.

use super::*;
use crate::server_fns::test_seed::{built_in_profile, seed};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

async fn create(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    profile: BuiltInProfile,
) -> Result<User, ServerFnError> {
    insert_user(
        pool,
        org,
        actor,
        &format!("{}@test.com", Uuid::now_v7()),
        "Person",
        profile,
    )
    .await
}

fn status(error: &ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => *code,
        error => panic!("unexpected error: {error}"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn created_people_get_the_chosen_profile(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    for &profile in BuiltInProfile::ALL {
        let user = create(&pool, ids.org_id, ids.user_id, profile)
            .await
            .unwrap();
        assert_eq!(
            built_in_profile(&pool, ids.org_id, user.id).await,
            Some((profile, profile == BuiltInProfile::Administrator)),
            "{profile:?}"
        );
        assert_eq!(user.org_role, provisioning::placeholder_role(profile));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn added_harvest_people_are_members(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let added = insert_users(
        &pool,
        ids.org_id,
        ids.user_id,
        &[NewPerson {
            email: "dana@x.com".into(),
            name: "Dana".into(),
        }],
        BuiltInProfile::Member,
        Activation::Deactivated,
        Existing::Skip,
    )
    .await
    .unwrap();
    assert_eq!(
        built_in_profile(&pool, ids.org_id, added.added[0].id).await,
        Some((BuiltInProfile::Member, false))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn deactivation_keeps_an_active_administrator(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    // An admin by role whose state is not Administrator does not count.
    let by_role = create(&pool, ids.org_id, ids.user_id, BuiltInProfile::Member)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'admin' WHERE id = $1",
        by_role.id
    )
    .execute(&pool)
    .await
    .unwrap();

    let refused = change_user_active(&pool, ids.org_id, ids.user_id, ids.user_id, false)
        .await
        .unwrap_err();
    assert_eq!(status(&refused), CONFLICT);
    let active = sqlx::query_scalar!("SELECT active FROM users WHERE id = $1", ids.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(active);

    // Deactivating someone who is not an Administrator needs no other one.
    change_user_active(&pool, ids.org_id, ids.user_id, by_role.id, false)
        .await
        .unwrap();

    // With a second Administrator, either one may be deactivated.
    let second = create(
        &pool,
        ids.org_id,
        ids.user_id,
        BuiltInProfile::Administrator,
    )
    .await
    .unwrap();
    let (user, _) = change_user_active(&pool, ids.org_id, ids.user_id, ids.user_id, false)
        .await
        .unwrap();
    assert!(!user.active);

    // The remaining one is now the last.
    let refused = change_user_active(&pool, ids.org_id, second.id, second.id, false)
        .await
        .unwrap_err();
    assert_eq!(status(&refused), CONFLICT);
}
