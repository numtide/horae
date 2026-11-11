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

/// Give `user` a built-in profile through the permission editor, as `actor`.
async fn assign_profile(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    user: Uuid,
    profile: BuiltInProfile,
) {
    use crate::models::permission_editor::{ProfileAction, ProfileCommand};
    let access = sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1",
        org
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let person = sqlx::query_scalar!(
        "SELECT revision FROM person_permission_states WHERE user_id = $1",
        user
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let command = ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: access,
        user_id: user,
        expected_person_revision: person,
        action: ProfileAction::BuiltIn { profile },
        grants: profile.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    };
    permissions::profiles::execute(pool, org, actor, &command)
        .await
        .unwrap();
}

fn status(error: &ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => *code,
        error => panic!("unexpected error: {error}"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_lowers_roles_but_never_raises_them(pool: PgPool) {
    let ids = administrator(&pool).await;
    let member = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    let demoted = create(&pool, ids.org_id, ids.user_id, OrgRole::Admin)
        .await
        .unwrap();
    use_policy_one(&pool, ids.org_id).await;

    for role in [OrgRole::Admin, OrgRole::Manager] {
        let refused = change_user_role(&pool, ids.org_id, ids.user_id, member.id, role)
            .await
            .unwrap_err();
        assert_eq!(status(&refused), CONFLICT, "{role:?}");
    }
    let role = sqlx::query_scalar!(
        r#"SELECT org_role AS "role: OrgRole" FROM users WHERE id = $1"#,
        member.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(role, OrgRole::Member);

    // Demoted in the editor, a legacy admin keeps the role user administration
    // still checks until the role is lowered too.
    assign_profile(
        &pool,
        ids.org_id,
        ids.user_id,
        demoted.id,
        BuiltInProfile::Member,
    )
    .await;
    assert!(
        begin_user_access_change(&pool, ids.org_id, demoted.id)
            .await
            .is_ok()
    );
    let (user, _) = change_user_role(&pool, ids.org_id, ids.user_id, demoted.id, OrgRole::Member)
        .await
        .unwrap();
    assert!(!user.is_admin(), "require_admin refuses the lowered role");
    let Err(refused) = begin_user_access_change(&pool, ids.org_id, demoted.id).await else {
        panic!("the lowered role no longer administers users");
    };
    assert_eq!(status(&refused), FORBIDDEN);
    // The editor's state stays as the editor left it.
    let revision = sqlx::query_scalar!(
        "SELECT revision FROM person_permission_states WHERE user_id = $1",
        demoted.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(revision, 1);
    assert_eq!(
        built_in_profile(&pool, ids.org_id, demoted.id).await,
        Some((BuiltInProfile::Member, false))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_keeps_an_administrator_by_role_while_the_role_still_decides(pool: PgPool) {
    let ids = administrator(&pool).await;
    let member = create(&pool, ids.org_id, ids.user_id, OrgRole::Member)
        .await
        .unwrap();
    use_policy_one(&pool, ids.org_id).await;
    // The editor makes a member an Administrator; their role stays member.
    assign_profile(
        &pool,
        ids.org_id,
        ids.user_id,
        member.id,
        BuiltInProfile::Administrator,
    )
    .await;

    // Nobody with the role user administration requires would remain.
    let refused = change_user_active(&pool, ids.org_id, ids.user_id, ids.user_id, false)
        .await
        .unwrap_err();
    assert_eq!(status(&refused), CONFLICT);
    let active = sqlx::query_scalar!("SELECT active FROM users WHERE id = $1", ids.user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(active);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_keeps_an_active_administrator(pool: PgPool) {
    let ids = administrator(&pool).await;
    // A legacy admin with no permission state at all does not count.
    let legacy = create(&pool, ids.org_id, ids.user_id, OrgRole::Admin)
        .await
        .unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
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

    // With a second administrator, by role and by state, either one may be
    // deactivated.
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
    assign_profile(
        &pool,
        ids.org_id,
        ids.user_id,
        second.id,
        BuiltInProfile::Administrator,
    )
    .await;
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
