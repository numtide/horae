//! Loading the actor and answering from the stored permission state.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::policy_one;
use crate::server_fns::test_seed::{seed, seed_before_permission_state};

async fn load(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<Actor, OpError> {
    let mut tx: Tx = pool.begin().await.unwrap();
    Actor::load(&mut tx, Caller::session(org_id, user_id)).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn answers_from_the_stored_state_not_the_role(pool: PgPool) {
    // A legacy Admin role with a member's grants and a legacy Member who is an
    // administrator: the stored state decides, never the role.
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    policy_one(&pool, &admin, &[Permission::ClientReadAll], false).await;
    let actor = load(&pool, admin.org_id, admin.user_id).await.unwrap();
    assert!(actor.is_active());
    assert_eq!(actor.channel(), Channel::Session);
    assert_eq!(
        (actor.identity().id, actor.identity().org_id),
        (admin.user_id, admin.org_id)
    );
    assert!(actor.has(Permission::ClientReadAll));
    assert!(!actor.has(Permission::ClientWriteAll));
    assert!(!actor.administrator());

    let member = seed(&pool, BuiltInProfile::Member).await;
    policy_one(&pool, &member, &[], true).await;
    let actor = load(&pool, member.org_id, member.user_id).await.unwrap();
    assert!(actor.administrator());
    assert!(!actor.has(Permission::ClientReadAll));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn foreign_missing_and_inactive_people_have_no_authority(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    for seeded in [&ids, &foreign] {
        policy_one(&pool, seeded, Permission::ALL, true).await;
    }
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
async fn missing_or_invalid_state_fails_closed(pool: PgPool) {
    // A person created before permission states existed has none.
    let unprovisioned = seed_before_permission_state(&pool, OrgRole::Admin).await;
    let error = load(&pool, unprovisioned.org_id, unprovisioned.user_id)
        .await
        .unwrap_err();
    assert!(matches!(error, OpError::Forbidden(_)), "{error:?}");

    for state in ["missing", "malformed"] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        policy_one(&pool, &ids, &[Permission::ClientReadAll], true).await;
        match state {
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
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
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
