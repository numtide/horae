//! Loading the actor and answering from the stored permission state.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::Tx;
use crate::ops::testing::provision;
use crate::testing::{clear_state, seed};

async fn load(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<Actor, OpError> {
    let mut tx: Tx = pool.begin().await.unwrap();
    Actor::load(&mut tx, Caller::session(org_id, user_id)).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn answers_from_the_stored_grants_and_identity(pool: PgPool) {
    // Grants and administrator identity are independent: a person seeded as
    // Administrator who now holds a member's grants, and a Member made
    // administrator with no grants.
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    provision(&pool, &admin, &[Permission::ClientReadAll], false).await;
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
    let state = actor.permissions().unwrap();
    assert_eq!(Some(&state.grants), actor.grants());
    assert!(!state.is_administrator);

    let member = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &member, &[], true).await;
    let actor = load(&pool, member.org_id, member.user_id).await.unwrap();
    assert!(actor.administrator());
    assert!(!actor.has(Permission::ClientReadAll));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn foreign_missing_and_inactive_people_have_no_authority(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    for seeded in [&ids, &foreign] {
        provision(&pool, seeded, Permission::ALL, true).await;
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
        assert!(actor.permissions().is_none());
        assert!(!actor.administrator());
        assert!(Permission::ALL.iter().all(|p| !actor.has(*p)));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn missing_or_invalid_state_fails_closed(pool: PgPool) {
    // A person created before permission state existed has none.
    let unprovisioned = seed(&pool, BuiltInProfile::Administrator).await;
    clear_state(&pool, unprovisioned.user_id).await;
    let error = load(&pool, unprovisioned.org_id, unprovisioned.user_id)
        .await
        .unwrap_err();
    assert!(matches!(error, OpError::Forbidden(_)), "{error:?}");

    for state in ["missing", "malformed"] {
        let ids = seed(&pool, BuiltInProfile::Administrator).await;
        provision(&pool, &ids, &[Permission::ClientReadAll], true).await;
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
    provision(&pool, &ids, &[Permission::ClientReadAll], true).await;
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_the_stored_administrator_identity_passes_the_administrator_guard(pool: PgPool) {
    use crate::ops::testing::assign_profile;

    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let caller = Caller::session(ids.org_id, ids.user_id);
    let mut connection = pool.acquire().await.unwrap();
    for profile in BuiltInProfile::ALL.iter().copied() {
        assign_profile(&pool, ids.org_id, ids.user_id, profile).await;
        for actor in [
            load(&pool, ids.org_id, ids.user_id).await.unwrap(),
            Actor::load_unlocked(&mut connection, caller).await.unwrap(),
        ] {
            let result = actor.require_administrator();
            if profile == BuiltInProfile::Administrator {
                result.unwrap();
            } else {
                assert_eq!(
                    result,
                    Err(OpError::forbidden(ADMINISTRATOR_REQUIRED)),
                    "{profile:?}"
                );
            }
        }
    }
    // Every grant without the identity is still refused.
    let everything = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &everything, Permission::ALL, false).await;
    let actor = load(&pool, everything.org_id, everything.user_id)
        .await
        .unwrap();
    assert!(actor.require_administrator().is_err());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unlocked_load_does_not_wait_for_an_access_change(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let mut change = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *change)
    .await
    .unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *change)
        .await
        .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let actor = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        Actor::load_unlocked(&mut connection, Caller::session(ids.org_id, ids.user_id)),
    )
    .await
    .expect("an unlocked load must not queue behind an access change")
    .unwrap();
    assert!(actor.administrator());
    change.rollback().await.unwrap();
}

#[test]
fn a_page_bound_to_someone_else_is_refused_with_the_domain_text() {
    let caller = Caller::session(Uuid::now_v7(), Uuid::now_v7());
    let own = PermissionRequester {
        org_id: caller.org_id,
        user_id: caller.user_id,
    };
    let other = PermissionRequester {
        user_id: Uuid::now_v7(),
        ..own
    };
    assert_eq!(super::expect(own, None, "changed"), Ok(own));
    assert_eq!(super::expect(own, Some(own), "changed"), Ok(own));
    assert_eq!(
        super::expect(own, Some(other), "changed"),
        Err(OpError::forbidden("changed"))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_loads_the_same_person_and_permissions_as_a_session(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &ids, &[Permission::ClientReadAll], true).await;
    let token_id = Uuid::now_v7();
    let mut tx: Tx = pool.begin().await.unwrap();
    let token = Actor::load(&mut tx, Caller::token(ids.org_id, ids.user_id, token_id))
        .await
        .unwrap();
    let session = Actor::load(&mut tx, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    assert_eq!(token.channel(), Channel::Token { token_id });
    assert!(token.is_active());
    assert_eq!(
        (token.id(), token.org_id(), token.is_active()),
        (session.id(), session.org_id(), session.is_active())
    );
    assert_eq!(token.permissions(), session.permissions());
    assert!(token.has(Permission::ClientReadAll));
    assert!(token.administrator());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_of_an_inactive_person_has_no_authority(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx: Tx = pool.begin().await.unwrap();
    let actor = Actor::load(
        &mut tx,
        Caller::token(ids.org_id, ids.user_id, Uuid::now_v7()),
    )
    .await
    .unwrap();
    assert!(!actor.is_active());
    assert!(actor.permissions().is_none());
    assert!(!actor.administrator());
}
