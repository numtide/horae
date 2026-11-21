use super::*;
use crate::models::permission_editor::{PermissionRequester, ProfileAction, ProfileCommand};
use crate::ops::OpError;
use crate::ops::permissions::profiles;
use crate::ops::testing::Recorder;
use crate::testing::built_in_profile;
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
async fn init_creates_an_administrator_who_administers(pool: PgPool) {
    crate::init::run(&pool, "Contoso", "ops@contoso.example", "Ops")
        .await
        .unwrap();
    let (org, admin) = only_user(&pool).await;
    assert_eq!(
        built_in_profile(&pool, org, admin).await,
        Some((BuiltInProfile::Administrator, true))
    );

    let mut read = crate::ops::begin_read(&pool, crate::ops::Caller::session(org, admin))
        .await
        .unwrap();
    let page = crate::ops::people::directory::page(
        &mut read.tx,
        &read.actor,
        chrono::NaiveDate::default(),
    )
    .await;
    let page = read.finish(page).await.unwrap();
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
    let mut write = crate::ops::begin_gated(
        &pool,
        crate::ops::Caller::session(org, admin),
        crate::db::OrganizationLock::AccessChange,
        crate::ops::WriteIsolation::ReadCommitted,
    )
    .await
    .unwrap();
    let requester = PermissionRequester {
        org_id: org,
        user_id: admin,
    };
    let result = profiles::save(&mut write.tx, &write.actor, &demote, requester).await;
    assert!(matches!(
        write.commit(result, &Recorder::default()).await,
        Err(e) if e == OpError::conflict(profiles::LAST_ADMINISTRATOR)
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn seed_gives_the_demo_admin_the_administrator_state(pool: PgPool) {
    crate::seed::run(&pool).await.unwrap();
    let admin = sqlx::query!("SELECT org_id, id FROM users WHERE email = 'admin@example.com'")
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
