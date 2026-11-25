//! Allow and Deny on a checked request. The checks of the request itself are
//! tested through `GET /oauth/authorize` in `horae-api`.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::Scope;
use sqlx::PgPool;

use super::*;
use crate::ops::oauth::testing::{
    ISSUER, REDIRECT_URI, all_scopes, approve as approve_as, audit_actions, pending, public_app,
};
use crate::ops::{Caller, NoEvents, begin, begin_read};
use crate::testing::{SeedIds, seed};

async fn try_approve(
    pool: &PgPool,
    ids: &SeedIds,
    request: &PendingRequest,
) -> Result<String, OpError> {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = approve(
        &mut write.tx,
        &write.actor,
        request,
        ISSUER,
        &OAuthConfig::default(),
    )
    .await;
    write.commit(result, &NoEvents).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn allow_creates_a_grant_and_a_code_bound_to_the_request(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let scopes = ScopeSet::of(&[Scope::TimeRead]);
    let request = pending(&app, "http://127.0.0.1:53682/callback", scopes);
    let url = try_approve(&pool, &admin, &request).await.unwrap();

    let url = url::Url::parse(&url).unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.port(), Some(53682));
    let params: Vec<(String, String)> = url.query_pairs().into_owned().collect();
    let names: Vec<&str> = params.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(names, ["code", "state", "iss"]);
    assert!(params[0].1.starts_with(CODE_PREFIX));
    assert_eq!(params[1].1, "xyz");
    assert_eq!(params[2].1, ISSUER);

    let row = sqlx::query!(
        r#"SELECT g.id, g.user_id, g.scopes, c.redirect_uri, c.code_challenge,
                  c.code_sha256, c.expires_at <= now() + interval '60 seconds' AS "short!"
           FROM oauth_grants g JOIN oauth_codes c ON c.grant_id = g.id
           WHERE g.client_id = $1"#,
        app.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.user_id, admin.user_id);
    assert_eq!(row.scopes, ["time:read"]);
    assert_eq!(row.redirect_uri, "http://127.0.0.1:53682/callback");
    assert_eq!(row.code_challenge, request.code_challenge);
    assert_eq!(row.code_sha256, crate::ops::oauth::digest(&params[0].1));
    assert!(row.short);
    assert_eq!(audit_actions(&pool, row.id).await, ["grant_approved"]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn each_allow_is_its_own_grant(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let request = pending(&app, REDIRECT_URI, all_scopes());
    let (first, _) = approve_as(&pool, &admin, &request).await;
    let (second, _) = approve_as(&pool, &admin, &request).await;
    assert_ne!(first, second);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn allow_finds_nothing_once_the_app_is_disabled_or_narrowed(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let request = pending(&app, REDIRECT_URI, all_scopes());
    sqlx::query!(
        "UPDATE oauth_clients SET scopes = '{time:read}' WHERE id = $1",
        app.id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        try_approve(&pool, &admin, &request).await,
        Err(OpError::NotFound(_))
    ));
    sqlx::query!(
        "UPDATE oauth_clients SET enabled = false, scopes = '{time:read,time:write,catalog:read}'
         WHERE id = $1",
        app.id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        try_approve(&pool, &admin, &request).await,
        Err(OpError::NotFound(_))
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_of_another_organization_finds_nothing(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let other = seed(&pool, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let request = pending(&app, REDIRECT_URI, all_scopes());
    assert!(matches!(
        try_approve(&pool, &other, &request).await,
        Err(OpError::NotFound(_))
    ));
}

#[test]
fn deny_returns_access_denied_with_state_and_issuer() {
    let request = PendingRequest {
        app_id: Uuid::nil(),
        client_id: "horae_app_x".into(),
        redirect_uri: "https://app.example.com/cb?keep=1".into(),
        scope: "time:read".into(),
        state: Some("a b".into()),
        code_challenge: String::new(),
        created_at: Utc::now(),
    };
    assert_eq!(
        deny(&request, ISSUER),
        "https://app.example.com/cb?keep=1&error=access_denied&state=a+b&iss=https%3A%2F%2Fhorae.example.com"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn details_name_the_app_person_and_where_allow_returns(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let request = pending(&app, REDIRECT_URI, all_scopes());
    let mut read = begin_read(&pool, Caller::session(admin.org_id, admin.user_id))
        .await
        .unwrap();
    let result = details(&mut read.tx, &read.actor, &request).await;
    let consent = read.finish(result).await.unwrap();
    assert_eq!(consent.app_name, "Widget");
    assert!(!consent.built_in);
    assert_eq!(consent.scopes, all_scopes());
    assert_eq!(consent.return_to, "com.example.app");
    assert_eq!(consent.organization_name, "Test Org");
    assert_eq!(return_to_display("http://127.0.0.1:5000/cb"), "127.0.0.1");
    assert_eq!(
        return_to_display("https://app.example.com/cb"),
        "app.example.com"
    );
}
