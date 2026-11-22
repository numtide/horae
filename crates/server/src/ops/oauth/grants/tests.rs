//! Listing and revoking authorizations, by their person and by an administrator.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::oauth::clients::{self, NewClient};
use crate::ops::oauth::testing::{audit_actions, grant};
use crate::ops::testing::Recorder;
use crate::ops::{Caller, begin, begin_read};
use crate::testing::{SeedIds, add_person, seed};

async fn app(pool: &PgPool, admin: &SeedIds) -> Uuid {
    let mut write = begin(pool, Caller::session(admin.org_id, admin.user_id))
        .await
        .unwrap();
    let input = NewClient {
        name: "Widget".into(),
        kind: ClientKind::Public,
        redirect_uris: vec!["com.example.app:/callback".into()],
        scopes: ScopeSet::of(&[Scope::TimeRead]),
    };
    let result = clients::register(&mut write.tx, &write.actor, input).await;
    write
        .commit(result, &Recorder::default())
        .await
        .unwrap()
        .0
        .id
}

async fn list_own_as(pool: &PgPool, ids: &SeedIds) -> Vec<GrantRow> {
    let mut read = begin_read(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = list_own(&mut read.tx, &read.actor).await;
    read.finish(result).await.unwrap()
}

async fn list_for_client_as(
    pool: &PgPool,
    ids: &SeedIds,
    app_id: Uuid,
) -> Result<Vec<GrantRow>, OpError> {
    let mut read = begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = list_for_client(&mut read.tx, &read.actor, app_id).await;
    read.finish(result).await
}

async fn revoke_as(pool: &PgPool, ids: &SeedIds, id: Uuid) -> Result<(), OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = revoke(&mut write.tx, &write.actor, id).await;
    write.commit(result, &Recorder::default()).await
}

async fn revocation(pool: &PgPool, id: Uuid) -> (Option<Uuid>, Option<String>) {
    let row = sqlx::query!(
        r#"SELECT revoked_by, revoke_reason::text AS reason FROM oauth_grants WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.revoked_by, row.reason)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_lists_only_their_own_authorizations(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let app_id = app(&pool, &admin).await;
    let own = grant(&pool, &member, app_id).await;
    grant(&pool, &admin, app_id).await;
    let listed = list_own_as(&pool, &member).await;
    assert_eq!(listed.len(), 1);
    let row = &listed[0];
    assert_eq!(
        (row.id, row.app_id, row.person_id),
        (own, app_id, member.user_id)
    );
    assert_eq!(row.app_name, "Widget");
    assert_eq!(row.scopes, ScopeSet::of(&[Scope::TimeRead]));
    assert!(!row.app_built_in);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_lists_every_authorization_of_an_app(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let app_id = app(&pool, &admin).await;
    grant(&pool, &member, app_id).await;
    grant(&pool, &admin, app_id).await;
    assert_eq!(
        list_for_client_as(&pool, &admin, app_id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(matches!(
        list_for_client_as(&pool, &member, app_id).await,
        Err(OpError::Forbidden(_))
    ));
    assert!(matches!(
        list_for_client_as(&pool, &admin, Uuid::now_v7()).await,
        Err(OpError::NotFound(_))
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_revokes_their_own_authorization(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let app_id = app(&pool, &admin).await;
    let id = grant(&pool, &member, app_id).await;
    revoke_as(&pool, &member, id).await.unwrap();
    assert_eq!(
        revocation(&pool, id).await,
        (Some(member.user_id), Some("person".into()))
    );
    assert!(list_own_as(&pool, &member).await.is_empty());
    // Revoking again changes and records nothing.
    revoke_as(&pool, &member, id).await.unwrap();
    assert_eq!(audit_actions(&pool, id).await, ["grant_revoked"]);
    let data = sqlx::query_scalar!(
        r#"SELECT data AS "data!" FROM audit_log WHERE entity_id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(data["reason"], "person");
    assert_eq!(data["app_name"], "Widget");
    assert_eq!(data["scopes"], serde_json::json!(["time:read"]));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_revokes_anyones_authorization(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let app_id = app(&pool, &admin).await;
    let id = grant(&pool, &member, app_id).await;
    revoke_as(&pool, &admin, id).await.unwrap();
    assert_eq!(
        revocation(&pool, id).await,
        (Some(admin.user_id), Some("administrator".into()))
    );
    assert!(
        list_for_client_as(&pool, &admin, app_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn cleanup_deletes_codes_and_access_tokens_a_day_after_expiry(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app_id = app(&pool, &admin).await;
    let expired = grant(&pool, &admin, app_id).await;
    let recent = grant(&pool, &admin, app_id).await;
    for (grant_id, expired_at) in [(expired, "25 hours"), (recent, "23 hours")] {
        sqlx::query!(
            "UPDATE oauth_codes SET expires_at = now() - $2::text::interval WHERE grant_id = $1",
            grant_id,
            expired_at,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE oauth_access_tokens SET expires_at = now() - $2::text::interval
             WHERE grant_id = $1",
            grant_id,
            expired_at,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    assert_eq!(crate::ops::oauth::delete_expired(&pool).await.unwrap(), 2);
    let left = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM oauth_codes WHERE grant_id = $1) AS "expired_codes!",
                  (SELECT count(*) FROM oauth_access_tokens WHERE grant_id = $1)
                    AS "expired_tokens!",
                  (SELECT count(*) FROM oauth_codes WHERE grant_id = $2) AS "recent_codes!",
                  (SELECT count(*) FROM oauth_access_tokens WHERE grant_id = $2)
                    AS "recent_tokens!",
                  (SELECT count(*) FROM oauth_refresh_tokens) AS "refresh!""#,
        expired,
        recent,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (left.expired_codes, left.expired_tokens),
        (0, 0),
        "expired a day ago"
    );
    assert_eq!((left.recent_codes, left.recent_tokens), (1, 1));
    assert_eq!(left.refresh, 2);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn another_persons_authorization_is_not_found_for_a_member(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    let app_id = app(&pool, &admin).await;
    let theirs = grant(&pool, &admin, app_id).await;
    assert!(matches!(
        revoke_as(&pool, &member, theirs).await,
        Err(OpError::NotFound(_))
    ));
    assert_eq!(revocation(&pool, theirs).await, (None, None));
    assert!(audit_actions(&pool, theirs).await.is_empty());
}
