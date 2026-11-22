//! Fixtures for the OAuth operations' tests. Approving a request arrives with
//! the authorization server, so these insert what an approval and a token
//! exchange would leave behind.

use sqlx::PgPool;
use uuid::Uuid;

use super::{ACCESS_TOKEN_PREFIX, CODE_PREFIX, REFRESH_TOKEN_PREFIX, Secret};
use crate::testing::SeedIds;

/// An active authorization of `ids`' person for app `app_id`, with all the
/// app's scopes, one code, one access token and one refresh token.
pub async fn grant(pool: &PgPool, ids: &SeedIds, app_id: Uuid) -> Uuid {
    let grant_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO oauth_grants (id, org_id, client_id, user_id, scopes, expires_at)
         SELECT $1, org_id, id, $3, scopes, now() + interval '1 year'
         FROM oauth_clients WHERE id = $2",
        grant_id,
        app_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO oauth_codes (id, org_id, grant_id, code_sha256, redirect_uri, code_challenge,
                                  expires_at)
         VALUES ($1, $2, $3, $4, 'com.example.app:/callback', $5, now() + interval '1 minute')",
        Uuid::now_v7(),
        ids.org_id,
        grant_id,
        Secret::generate(CODE_PREFIX).digest(),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO oauth_access_tokens (id, org_id, grant_id, token_sha256, scopes, expires_at)
         SELECT $1, org_id, id, $2, scopes, now() + interval '1 hour'
         FROM oauth_grants WHERE id = $3",
        Uuid::now_v7(),
        Secret::generate(ACCESS_TOKEN_PREFIX).digest(),
        grant_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO oauth_refresh_tokens (id, org_id, grant_id, token_sha256, scopes,
                                           idle_expires_at)
         SELECT $1, org_id, id, $2, scopes, now() + interval '30 days'
         FROM oauth_grants WHERE id = $3",
        Uuid::now_v7(),
        Secret::generate(REFRESH_TOKEN_PREFIX).digest(),
        grant_id,
    )
    .execute(pool)
    .await
    .unwrap();
    grant_id
}

/// The audit actions recorded about one app or grant, oldest first.
pub async fn audit_actions(pool: &PgPool, entity_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT action FROM audit_log WHERE entity_id = $1 ORDER BY created_at, id",
        entity_id,
    )
    .fetch_all(pool)
    .await
    .unwrap()
}
