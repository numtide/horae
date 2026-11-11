//! People created or changed here carry the permission state of their role.

use super::*;
use crate::server_fns::test_seed::{built_in_profile, seed, use_policy_one};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

async fn create(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    role: OrgRole,
) -> Result<User, ServerFnError> {
    insert_user(
        pool,
        org,
        actor,
        &format!("{}@test.com", Uuid::now_v7()),
        "Person",
        role,
    )
    .await
}

/// The seeded admin with the Administrator state the provisioning paths give.
async fn administrator(pool: &PgPool) -> crate::server_fns::test_seed::SeedIds {
    let ids = seed(pool, OrgRole::Admin).await;
    let mut connection = pool.acquire().await.unwrap();
    provisioning::provision(
        &mut connection,
        ids.org_id,
        ids.user_id,
        BuiltInProfile::Administrator,
    )
    .await
    .unwrap();
    ids
}

fn status(error: &ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => *code,
        error => panic!("unexpected error: {error}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn created_people_get_the_profile_of_their_role(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for (role, expected) in [
        (OrgRole::Admin, (BuiltInProfile::Administrator, true)),
        (OrgRole::Manager, (BuiltInProfile::ExecutiveManager, false)),
        (OrgRole::Member, (BuiltInProfile::Member, false)),
    ] {
        let user = create(&pool, ids.org_id, ids.user_id, role).await.unwrap();
        assert_eq!(user.org_role, role);
        assert_eq!(
            built_in_profile(&pool, ids.org_id, user.id).await,
            Some(expected),
            "{role:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn added_harvest_people_are_members(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let added = insert_users(
        &pool,
        ids.org_id,
        ids.user_id,
        &[NewPerson {
            email: "dana@x.com".into(),
            name: "Dana".into(),
        }],
        OrgRole::Member,
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

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn policy_one_creates_members_only(pool: PgPool) {
    let ids = administrator(&pool).await;
    use_policy_one(&pool, ids.org_id).await;
    for role in [OrgRole::Admin, OrgRole::Manager] {
        let refused = create(&pool, ids.org_id, ids.user_id, role)
            .await
            .unwrap_err();
        assert_eq!(status(&refused), CONFLICT, "{role:?}");
    }
    let member = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    assert_eq!(
        built_in_profile(&pool, ids.org_id, member.id).await,
        Some((BuiltInProfile::Member, false))
    );
    let people: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM users WHERE org_id = $1"#,
        ids.org_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(people, 2);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn role_changes_carry_the_state_under_policy_zero(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let user = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    for (role, expected, revision) in [
        (
            OrgRole::Manager,
            (BuiltInProfile::ExecutiveManager, false),
            1,
        ),
        (OrgRole::Admin, (BuiltInProfile::Administrator, true), 2),
        // An unchanged role leaves the state and its revision alone.
        (OrgRole::Admin, (BuiltInProfile::Administrator, true), 2),
        (OrgRole::Member, (BuiltInProfile::Member, false), 3),
    ] {
        change_user_role(&pool, ids.org_id, ids.user_id, user.id, role)
            .await
            .unwrap();
        assert_eq!(
            built_in_profile(&pool, ids.org_id, user.id).await,
            Some(expected)
        );
        let stored = sqlx::query_scalar!(
            "SELECT revision FROM person_permission_states WHERE user_id = $1",
            user.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, revision, "{role:?}");
    }

    // A person from before provisioning gets a state on their next role change.
    change_user_role(&pool, ids.org_id, ids.user_id, ids.user_id, OrgRole::Admin)
        .await
        .unwrap();
    assert_eq!(
        built_in_profile(&pool, ids.org_id, ids.user_id).await,
        Some((BuiltInProfile::Administrator, true))
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn policy_one_refuses_role_changes(pool: PgPool) {
    let ids = administrator(&pool).await;
    let user = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    use_policy_one(&pool, ids.org_id).await;
    let refused = change_user_role(&pool, ids.org_id, ids.user_id, user.id, OrgRole::Admin)
        .await
        .unwrap_err();
    assert_eq!(status(&refused), CONFLICT);
    let role = sqlx::query_scalar!(
        r#"SELECT org_role AS "role: OrgRole" FROM users WHERE id = $1"#,
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(role, OrgRole::Member);
    assert_eq!(
        built_in_profile(&pool, ids.org_id, user.id).await,
        Some((BuiltInProfile::Member, false))
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn policy_one_keeps_an_active_administrator(pool: PgPool) {
    let ids = administrator(&pool).await;
    // A legacy admin without the Administrator state does not count.
    let legacy = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'admin' WHERE id = $1",
        legacy.id
    )
    .execute(&pool)
    .await
    .unwrap();
    use_policy_one(&pool, ids.org_id).await;

    let refused = change_user_active(&pool, ids.org_id, ids.user_id, ids.user_id, false)
        .await
        .unwrap_err();
    assert_eq!(status(&refused), CONFLICT);

    // Deactivating someone who is not an administrator needs no other one.
    change_user_active(&pool, ids.org_id, ids.user_id, legacy.id, false)
        .await
        .unwrap();

    // With a second Administrator, either one may be deactivated. The legacy
    // role only lets them through the server function's own admin check.
    let second = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'admin' WHERE id = $1",
        second.id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = true, grants = $2,
         built_in_profile = 'administrator' WHERE user_id = $1",
        second.id,
        &BuiltInProfile::Administrator
            .selection()
            .iter()
            .map(|grant| serde_json::to_value(grant)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned())
            .collect::<Vec<_>>(),
    )
    .execute(&pool)
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
