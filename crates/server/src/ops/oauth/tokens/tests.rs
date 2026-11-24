//! `tokens::authenticate`: each refusal of FR-052 and the `last_used_at`
//! stamp of FR-053. The token endpoint's own rules are tested through its
//! router in `horae-api`.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

use super::*;
use crate::ops::Channel;
use crate::ops::oauth::testing::{public_app, signed_in};
use crate::testing::{SeedIds, seed};

async fn flow(pool: &PgPool) -> (SeedIds, Uuid, TokenSet) {
    let admin = seed(pool, BuiltInProfile::Administrator).await;
    let app = public_app(pool, &admin).await;
    let (grant_id, tokens) = signed_in(pool, &admin, &app).await;
    (admin, grant_id, tokens)
}

async fn last_used(pool: &PgPool, grant_id: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar!(
        r#"SELECT last_used_at AS "last_used_at: chrono::DateTime<chrono::Utc>"
           FROM oauth_grants WHERE id = $1"#,
        grant_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_access_token_acts_as_its_person_through_its_grant(pool: PgPool) {
    let (admin, grant_id, tokens) = flow(&pool).await;
    let caller = authenticate(&pool, tokens.access_token.expose())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (caller.org_id, caller.user_id),
        (admin.org_id, admin.user_id)
    );
    let Channel::OAuth {
        grant_id: found,
        scopes,
        ..
    } = caller.channel
    else {
        panic!("{:?}", caller.channel);
    };
    assert_eq!(found, grant_id);
    assert_eq!(scopes, tokens.scopes);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn other_credentials_are_not_access_tokens(pool: PgPool) {
    let (_, _, tokens) = flow(&pool).await;
    for credential in [
        tokens.refresh_token.expose(),
        "horae_oat_unknown",
        "horae_pat_whatever",
    ] {
        assert!(authenticate(&pool, credential).await.unwrap().is_none());
    }
}

/// Breaks one condition of FR-052 for a grant.
async fn break_condition(pool: &PgPool, case: &str, grant_id: Uuid) {
    let query = match case {
        "token expired" => sqlx::query!(
            "UPDATE oauth_access_tokens SET expires_at = now() WHERE grant_id = $1",
            grant_id
        ),
        "grant revoked" => sqlx::query!(
            "UPDATE oauth_grants SET revoked_at = now(), revoke_reason = 'person' WHERE id = $1",
            grant_id
        ),
        "grant expired" => sqlx::query!(
            "UPDATE oauth_grants SET expires_at = now() WHERE id = $1",
            grant_id
        ),
        "app disabled" => sqlx::query!(
            "UPDATE oauth_clients SET enabled = false
             WHERE id = (SELECT client_id FROM oauth_grants WHERE id = $1)",
            grant_id
        ),
        "person deactivated" => sqlx::query!(
            "UPDATE users SET active = false
             WHERE id = (SELECT user_id FROM oauth_grants WHERE id = $1)",
            grant_id
        ),
        other => panic!("{other}"),
    };
    query.execute(pool).await.unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_stops_working_with_its_grant_app_or_person(pool: PgPool) {
    for case in [
        "token expired",
        "grant revoked",
        "grant expired",
        "app disabled",
        "person deactivated",
    ] {
        let (_, grant_id, tokens) = flow(&pool).await;
        assert!(
            authenticate(&pool, tokens.access_token.expose())
                .await
                .unwrap()
                .is_some()
        );
        break_condition(&pool, case, grant_id).await;
        assert!(
            authenticate(&pool, tokens.access_token.expose())
                .await
                .unwrap()
                .is_none(),
            "{case}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn last_used_is_stamped_at_most_once_a_minute(pool: PgPool) {
    let (_, grant_id, tokens) = flow(&pool).await;
    sqlx::query!(
        "UPDATE oauth_grants SET last_used_at = now() - interval '2 minutes' WHERE id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = last_used(&pool, grant_id).await.unwrap();
    authenticate(&pool, tokens.access_token.expose())
        .await
        .unwrap();
    let stamped = last_used(&pool, grant_id).await.unwrap();
    assert!(stamped > before);
    authenticate(&pool, tokens.access_token.expose())
        .await
        .unwrap();
    assert_eq!(last_used(&pool, grant_id).await.unwrap(), stamped);
}
