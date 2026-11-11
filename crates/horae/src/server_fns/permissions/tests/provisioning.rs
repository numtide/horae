use super::*;
use crate::models::people::PeopleActivity;
use crate::models::permission_editor::{ProfileAction, ProfileCommand};
use crate::server_fns::permissions::{directory, profiles};
use crate::server_fns::test_seed::{built_in_profile, use_policy_one};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

async fn only_user(pool: &PgPool) -> (Uuid, Uuid) {
    let row = sqlx::query!("SELECT org_id, id FROM users")
        .fetch_one(pool)
        .await
        .unwrap();
    (row.org_id, row.id)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn init_creates_an_administrator_who_administers_under_policy_one(pool: PgPool) {
    crate::init::run(&pool, "Contoso", "ops@contoso.example", "Ops")
        .await
        .unwrap();
    let (org, admin) = only_user(&pool).await;
    assert_eq!(
        built_in_profile(&pool, org, admin).await,
        Some((BuiltInProfile::Administrator, true))
    );

    use_policy_one(&pool, org).await;
    let page = directory::read(&pool, org, admin, PeopleActivity::Active, None)
        .await
        .unwrap();
    assert_eq!(page.people.len(), 1);

    // The permission editor accepts the administrator and keeps them the last one.
    let demote = ProfileCommand {
        request_id: Uuid::now_v7(),
        expected_access_revision: 0,
        user_id: admin,
        expected_person_revision: 0,
        action: ProfileAction::BuiltIn {
            profile: BuiltInProfile::Member,
        },
        grants: BuiltInProfile::Member.selection().iter().collect(),
        remove_projects: vec![],
        remove_people: vec![],
    };
    assert!(matches!(
        profiles::execute(&pool, org, admin, &demote).await,
        Err(profiles::ProfileCommandError::LastAdministrator)
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn seed_gives_the_demo_admin_the_administrator_state(pool: PgPool) {
    crate::seed::run(&pool).await.unwrap();
    let admin = sqlx::query!("SELECT org_id, id FROM users WHERE org_role = 'admin'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let people = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM users u
           WHERE NOT EXISTS (SELECT 1 FROM person_permission_states p WHERE p.user_id = u.id)"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(people, 0, "every demo person has a permission state");
    assert_eq!(
        built_in_profile(&pool, admin.org_id, admin.id).await,
        Some((BuiltInProfile::Administrator, true))
    );
}

#[test]
fn profiles_parse_from_their_stored_names_only() {
    for &profile in BuiltInProfile::ALL {
        assert_eq!(parse_profile(&stored_id(profile).unwrap()), Some(profile));
    }
    for legacy in ["admin", "manager", "Administrator", ""] {
        assert_eq!(parse_profile(legacy), None, "{legacy}");
    }
}
