//! Fixtures for the OAuth tests: [`grant`] inserts what an approval and a
//! token exchange leave behind; [`public_app`], [`approve`] and [`exchange`]
//! go through the operations themselves.

use chrono::Utc;
use horae_core::permissions::oauth::{ClientKind, ScopeSet};
use sqlx::PgPool;
use uuid::Uuid;

use super::authorize::{self, PendingRequest};
use super::clients::{self, NewClient};
use super::tokens::{self, ClientAuth, TokenError, TokenSet};
use super::{ACCESS_TOKEN_PREFIX, CODE_PREFIX, REFRESH_TOKEN_PREFIX, Secret};
use crate::config::OAuthConfig;
use crate::ops::{Caller, NoEvents, begin};
use crate::testing::SeedIds;

/// RFC 7636 appendix B's verifier and its S256 challenge.
pub const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
pub const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
pub const REDIRECT_URI: &str = "com.example.app:/callback";
pub const ISSUER: &str = "https://horae.example.com";

/// A registered app: its row id, client id and, when confidential, secret.
pub struct App {
    pub id: Uuid,
    pub client_id: String,
    pub secret: Option<String>,
}

impl App {
    pub fn auth(&self) -> ClientAuth {
        ClientAuth {
            client_id: self.client_id.clone(),
            secret: self.secret.clone(),
        }
    }
}

/// Registers an app named `name` as `admin`, redirecting to
/// [`REDIRECT_URI`] (public) or `https://app.example.com/callback`
/// (confidential), and also to `http://127.0.0.1/callback` when public.
pub async fn register(
    pool: &PgPool,
    admin: &SeedIds,
    name: &str,
    kind: ClientKind,
    scopes: ScopeSet,
) -> App {
    let redirect_uris = match kind {
        ClientKind::Public => vec![REDIRECT_URI.into(), "http://127.0.0.1/callback".into()],
        ClientKind::Confidential => vec!["https://app.example.com/callback".into()],
    };
    let mut write = begin(pool, Caller::session(admin.org_id, admin.user_id))
        .await
        .unwrap();
    let input = NewClient {
        name: name.into(),
        kind,
        redirect_uris,
        scopes: scopes.names().into_iter().map(str::to_owned).collect(),
    };
    let result = clients::register(&mut write.tx, &write.actor, input).await;
    let (client, secret) = write.commit(result, &NoEvents).await.unwrap();
    App {
        id: client.id,
        client_id: client.client_id,
        secret: secret.map(|secret| secret.expose().to_owned()),
    }
}

/// A public app with every scope.
pub async fn public_app(pool: &PgPool, admin: &SeedIds) -> App {
    register(pool, admin, "Widget", ClientKind::Public, all_scopes()).await
}

pub fn all_scopes() -> ScopeSet {
    ScopeSet::of(&horae_core::permissions::oauth::Scope::ALL)
}

/// A request of `app` for `scopes`, as `/oauth/authorize` would leave it.
pub fn pending(app: &App, redirect_uri: &str, scopes: ScopeSet) -> PendingRequest {
    PendingRequest {
        app_id: app.id,
        client_id: app.client_id.clone(),
        redirect_uri: redirect_uri.into(),
        scope: scopes.display(),
        state: Some("xyz".into()),
        code_challenge: CHALLENGE.into(),
        created_at: Utc::now(),
    }
}

/// `ids`' person allows `pending`: the new grant's id and its code.
pub async fn approve(pool: &PgPool, ids: &SeedIds, pending: &PendingRequest) -> (Uuid, String) {
    let mut write = begin(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = authorize::approve(
        &mut write.tx,
        &write.actor,
        pending,
        ISSUER,
        &OAuthConfig::default(),
    )
    .await;
    let url = write.commit(result, &NoEvents).await.unwrap();
    let code = url::Url::parse(&url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "code")
        .unwrap()
        .1
        .into_owned();
    let grant_id = sqlx::query_scalar!(
        "SELECT id FROM oauth_grants WHERE user_id = $1 ORDER BY approved_at DESC, id DESC
         LIMIT 1",
        ids.user_id,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (grant_id, code)
}

/// Exchanges `code` as `app` with [`VERIFIER`], committing as the endpoint
/// does.
pub async fn exchange(
    pool: &PgPool,
    app: &App,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenSet, TokenError> {
    let mut tx = tokens::begin(pool).await.unwrap();
    let result = tokens::exchange_code(
        &mut tx,
        &OAuthConfig::default(),
        &app.auth(),
        code,
        redirect_uri,
        VERIFIER,
    )
    .await;
    tokens::finish(tx, result).await
}

/// A full flow for `ids`' person with `app`: the grant and its first tokens.
pub async fn signed_in(pool: &PgPool, ids: &SeedIds, app: &App) -> (Uuid, TokenSet) {
    let request = pending(app, REDIRECT_URI, all_scopes());
    let (grant_id, code) = approve(pool, ids, &request).await;
    (
        grant_id,
        exchange(pool, app, &code, REDIRECT_URI).await.unwrap(),
    )
}

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
