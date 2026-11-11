//! Loading the actor and answering under both permission policies.

use horae_core::permissions::catalog::Permission;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::policy_one;
use crate::server_fns::test_seed::seed;

async fn load(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<Actor, OpError> {
    let mut tx: Tx = pool.begin().await.unwrap();
    Actor::load(&mut tx, Caller::session(org_id, user_id)).await
}

#[test]
fn legacy_roles_map_to_their_grants() {
    for permission in Permission::ALL {
        assert!(legacy_has(OrgRole::Admin, *permission), "{permission:?}");
    }
    let member: Vec<_> = Permission::ALL
        .iter()
        .copied()
        .filter(|permission| legacy_has(OrgRole::Member, *permission))
        .collect();
    assert_eq!(
        member,
        [
            Permission::TimeReadOwn,
            Permission::TimeWriteOwn,
            Permission::ExpenseReadOwn,
            Permission::ExpenseWriteOwn,
            Permission::ClientReadAll,
        ]
    );
    // The clients domain relies on these Manager rows.
    for permission in [
        Permission::ClientReadAll,
        Permission::ClientWriteAll,
        Permission::BillableRateReadAll,
        Permission::BillableRateWriteAll,
        Permission::InvoiceReadAll,
    ] {
        assert!(legacy_has(OrgRole::Manager, permission), "{permission:?}");
    }
    for permission in [
        Permission::CostRateReadAll,
        Permission::PeopleWriteAll,
        Permission::TimeWriteAll,
        Permission::CompanyWrite,
    ] {
        assert!(!legacy_has(OrgRole::Manager, permission), "{permission:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn policy_zero_answers_from_the_role(pool: PgPool) {
    for (role, administrator, writes) in [
        (OrgRole::Admin, true, true),
        (OrgRole::Manager, false, true),
        (OrgRole::Member, false, false),
    ] {
        let ids = seed(&pool, role).await;
        let actor = load(&pool, ids.org_id, ids.user_id).await.unwrap();
        assert!(actor.is_active() && actor.is_legacy());
        assert_eq!(actor.legacy_role(), Some(role));
        assert_eq!(actor.channel(), Channel::Session);
        assert_eq!(
            (actor.identity().id, actor.identity().org_id),
            (ids.user_id, ids.org_id)
        );
        assert_eq!(actor.administrator(), administrator, "{role:?}");
        assert_eq!(actor.has(Permission::ClientWriteAll), writes, "{role:?}");
        assert!(actor.has(Permission::ClientReadAll), "{role:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn policy_one_answers_from_the_grants_not_the_role(pool: PgPool) {
    // A legacy Admin with a member's grants and a legacy Member who is an
    // administrator: the stored state decides, never the role.
    let admin = seed(&pool, OrgRole::Admin).await;
    policy_one(&pool, &admin, &[Permission::ClientReadAll], false).await;
    let actor = load(&pool, admin.org_id, admin.user_id).await.unwrap();
    assert!(actor.is_active() && !actor.is_legacy());
    assert_eq!(actor.legacy_role(), None);
    assert!(actor.has(Permission::ClientReadAll));
    assert!(!actor.has(Permission::ClientWriteAll));
    assert!(!actor.administrator());

    let member = seed(&pool, OrgRole::Member).await;
    policy_one(&pool, &member, &[], true).await;
    let actor = load(&pool, member.org_id, member.user_id).await.unwrap();
    assert!(actor.administrator());
    assert!(!actor.has(Permission::ClientReadAll));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn foreign_missing_and_inactive_people_have_no_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE users SET active = false WHERE id = $1",
        foreign.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for (org_id, user_id) in [
        (ids.org_id, foreign.user_id),
        (Uuid::now_v7(), ids.user_id),
        (foreign.org_id, foreign.user_id),
        (ids.org_id, Uuid::now_v7()),
    ] {
        let actor = load(&pool, org_id, user_id).await.unwrap();
        assert!(!actor.is_active());
        assert!(!actor.administrator());
        assert!(Permission::ALL.iter().all(|p| !actor.has(*p)));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn unknown_policy_and_missing_or_invalid_state_fail_closed(pool: PgPool) {
    for state in ["unknown policy", "missing", "malformed"] {
        let ids = seed(&pool, OrgRole::Admin).await;
        policy_one(&pool, &ids, &[Permission::ClientReadAll], true).await;
        match state {
            "unknown policy" => sqlx::query!(
                "UPDATE organizations SET permission_policy_version = 2 WHERE id = $1",
                ids.org_id
            ),
            "missing" => sqlx::query!(
                "DELETE FROM person_permission_states WHERE user_id = $1",
                ids.user_id
            ),
            _ => sqlx::query!(
                "UPDATE person_permission_states SET grants = ARRAY['unknown'] WHERE user_id = $1",
                ids.user_id
            ),
        }
        .execute(&pool)
        .await
        .unwrap();
        let error = load(&pool, ids.org_id, ids.user_id).await.unwrap_err();
        assert!(matches!(error, OpError::Forbidden(_)), "{state}: {error:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn storage_failures_loading_state_are_not_a_refusal(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    policy_one(&pool, &ids, &[Permission::ClientReadAll], true).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE person_permission_states IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *holder)
        .await
        .unwrap();
    let mut tx: Tx = pool.begin().await.unwrap();
    sqlx::query!("SET LOCAL lock_timeout = '50ms'")
        .execute(&mut *tx)
        .await
        .unwrap();
    let error = Actor::load(&mut tx, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap_err();
    assert!(matches!(error, OpError::Internal(_)), "{error:?}");
    holder.rollback().await.unwrap();
}
