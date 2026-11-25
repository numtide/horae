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
        scopes: vec![Scope::TimeRead.as_str().to_owned()],
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

/// Moves one of a grant's timestamps `ago` (an interval such as `25 hours`)
/// into the past.
async fn age(pool: &PgPool, grant_id: Uuid, column: &str, ago: &str) {
    let query = match column {
        "code" => sqlx::query!(
            "UPDATE oauth_codes SET expires_at = now() - $2::text::interval WHERE grant_id = $1",
            grant_id,
            ago,
        ),
        "access" => sqlx::query!(
            "UPDATE oauth_access_tokens SET expires_at = now() - $2::text::interval
             WHERE grant_id = $1",
            grant_id,
            ago,
        ),
        "refresh_used" => sqlx::query!(
            "UPDATE oauth_refresh_tokens SET used_at = now() - $2::text::interval
             WHERE grant_id = $1",
            grant_id,
            ago,
        ),
        "refresh_idle" => sqlx::query!(
            "UPDATE oauth_refresh_tokens SET idle_expires_at = now() - $2::text::interval
             WHERE grant_id = $1",
            grant_id,
            ago,
        ),
        "revoked" => sqlx::query!(
            "UPDATE oauth_grants SET revoked_at = now() - $2::text::interval,
                                     revoke_reason = 'person'
             WHERE id = $1",
            grant_id,
            ago,
        ),
        "expires" => sqlx::query!(
            "UPDATE oauth_grants SET expires_at = now() - $2::text::interval WHERE id = $1",
            grant_id,
            ago,
        ),
        other => panic!("{other}"),
    };
    query.execute(pool).await.unwrap();
}

async fn rows(pool: &PgPool, grant_id: Uuid) -> [i64; 4] {
    let row = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM oauth_grants WHERE id = $1) AS "grants!",
                  (SELECT count(*) FROM oauth_codes WHERE grant_id = $1) AS "codes!",
                  (SELECT count(*) FROM oauth_access_tokens WHERE grant_id = $1) AS "access!",
                  (SELECT count(*) FROM oauth_refresh_tokens WHERE grant_id = $1) AS "refresh!""#,
        grant_id,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    [row.grants, row.codes, row.access, row.refresh]
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn cleanup_deletes_what_can_no_longer_be_used(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app_id = app(&pool, &admin).await;

    // A used refresh token stays while its grant lives, so its reuse is caught.
    let old_credentials = grant(&pool, &admin, app_id).await;
    age(&pool, old_credentials, "code", "25 hours").await;
    age(&pool, old_credentials, "access", "25 hours").await;
    age(&pool, old_credentials, "refresh_used", "400 days").await;
    age(&pool, old_credentials, "refresh_idle", "370 days").await;

    let recent_credentials = grant(&pool, &admin, app_id).await;
    age(&pool, recent_credentials, "code", "23 hours").await;
    age(&pool, recent_credentials, "access", "23 hours").await;
    age(&pool, recent_credentials, "refresh_idle", "23 hours").await;

    let idle_refresh = grant(&pool, &admin, app_id).await;
    age(&pool, idle_refresh, "refresh_idle", "25 hours").await;

    let revoked_long_ago = grant(&pool, &admin, app_id).await;
    age(&pool, revoked_long_ago, "revoked", "25 hours").await;
    let revoked_recently = grant(&pool, &admin, app_id).await;
    age(&pool, revoked_recently, "revoked", "23 hours").await;
    let expired_long_ago = grant(&pool, &admin, app_id).await;
    age(&pool, expired_long_ago, "expires", "25 hours").await;

    crate::ops::oauth::delete_expired(&pool).await.unwrap();

    assert_eq!(rows(&pool, old_credentials).await, [1, 0, 0, 1]);
    assert_eq!(rows(&pool, recent_credentials).await, [1, 1, 1, 1]);
    assert_eq!(rows(&pool, idle_refresh).await, [1, 1, 1, 0]);
    assert_eq!(rows(&pool, revoked_long_ago).await, [0, 0, 0, 0]);
    assert_eq!(rows(&pool, revoked_recently).await, [1, 1, 1, 1]);
    assert_eq!(rows(&pool, expired_long_ago).await, [0, 0, 0, 0]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn no_token_lists_or_revokes_authorizations(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app_id = app(&pool, &admin).await;
    let id = grant(&pool, &admin, app_id).await;
    let all = ScopeSet::of(&Scope::ALL);
    for caller in [
        Caller::token(admin.org_id, admin.user_id, Uuid::now_v7()),
        Caller::oauth(admin.org_id, admin.user_id, id, Uuid::now_v7(), all),
    ] {
        let mut write = begin(&pool, caller).await.unwrap();
        write.actor.unlock_scope(Scope::TimeWrite).unwrap();
        let refused = |result: Result<_, OpError>| {
            assert!(matches!(result, Err(OpError::Forbidden(_))), "{caller:?}");
        };
        refused(list_own(&mut write.tx, &write.actor).await.map(drop));
        refused(
            list_for_client(&mut write.tx, &write.actor, app_id)
                .await
                .map(drop),
        );
        refused(revoke(&mut write.tx, &write.actor, id).await.map(drop));
    }
    assert_eq!(revocation(&pool, id).await, (None, None));
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
