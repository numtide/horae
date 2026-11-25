//! The token and revocation endpoints' operations (RFC 6749 §4.1.3 and §6,
//! RFC 7009), and turning an access token back into its person.
//!
//! These have no [`Actor`](crate::ops::Actor): their authority is the
//! presented code with its verifier, the refresh token, or the client's
//! secret (constitution 7.1.0, spec D-3).

use horae_core::permissions::oauth::{ClientKind, ScopeSet, pkce};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use super::audit::{self, Action};
use super::{ACCESS_TOKEN_PREFIX, REFRESH_TOKEN_PREFIX, Secret, digest};
use crate::config::OAuthConfig;
use crate::ops::{Caller, OpError, Tx};

/// How an app identified itself at the token or revocation endpoint: HTTP
/// Basic or the form body (research R-12).
#[derive(Clone)]
pub struct ClientAuth {
    pub client_id: String,
    pub secret: Option<String>,
}

impl std::fmt::Debug for ClientAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientAuth")
            .field("client_id", &self.client_id)
            .field("secret", &self.secret.as_ref().map(|_| "…"))
            .finish()
    }
}

/// A refusal in RFC 6749 §5.2's terms. Descriptions never hold a credential.
#[derive(Debug)]
pub enum TokenError {
    InvalidRequest(&'static str),
    InvalidClient,
    InvalidGrant,
    InvalidScope,
    /// A used code or refresh token was presented again. Its grant was
    /// revoked in this transaction, which the caller must commit before
    /// answering `invalid_grant` (FR-031, FR-043).
    Reused,
    Storage(OpError),
}

impl From<sqlx::Error> for TokenError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl From<OpError> for TokenError {
    fn from(error: OpError) -> Self {
        Self::Storage(error)
    }
}

/// What a successful exchange or refresh hands the app, once.
#[derive(Debug)]
pub struct TokenSet {
    pub access_token: Secret,
    pub refresh_token: Secret,
    pub expires_in: u64,
    pub scopes: ScopeSet,
}

/// Begins the transaction a token endpoint operation runs in.
pub async fn begin(pool: &PgPool) -> Result<Tx, OpError> {
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .await?;
    crate::ops::configure_transaction_limits(&mut tx).await?;
    Ok(tx)
}

/// Commits a success, and a refusal that revoked a grant; rolls back any
/// other refusal.
pub async fn finish<T>(tx: Tx, result: Result<T, TokenError>) -> Result<T, TokenError> {
    match &result {
        Ok(_) | Err(TokenError::Reused) => tx.commit().await?,
        Err(_) => tx.rollback().await?,
    }
    result
}

/// Exchanges a code for the first tokens of its grant (research R-9).
pub async fn exchange_code(
    tx: &mut Tx,
    config: &OAuthConfig,
    client: &ClientAuth,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenSet, TokenError> {
    let apps = authenticate_client(tx, client).await?;
    if !pkce::well_formed_verifier(verifier) {
        return Err(TokenError::InvalidRequest(
            "code_verifier must be 43 to 128 unreserved characters",
        ));
    }
    // The lock makes a concurrent second exchange wait, then see `used_at`.
    let Some(found) = sqlx::query!(
        r#"SELECT c.id, c.redirect_uri, c.code_challenge, c.used_at IS NOT NULL AS "used!",
                  c.expires_at > now() AS "live!", g.id AS grant_id, g.org_id,
                  g.client_id AS app_id, g.scopes,
                  g.revoked_at IS NULL AND g.expires_at > now() AND u.active AS "active!"
           FROM oauth_codes c
           JOIN oauth_grants g ON g.id = c.grant_id
           JOIN users u ON u.org_id = g.org_id AND u.id = g.user_id
           WHERE c.code_sha256 = $1
           FOR UPDATE OF c"#,
        digest(code),
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Err(TokenError::InvalidGrant);
    };
    if !apps.contains(&found.app_id) {
        return Err(TokenError::InvalidGrant);
    }
    if found.used {
        revoke_reused(tx, found.org_id, found.grant_id, Action::CodeReused).await?;
        return Err(TokenError::Reused);
    }
    // `pkce::verify` compares in constant time.
    let matches = pkce::verify(verifier, &found.code_challenge);
    if !found.live || !found.active || found.redirect_uri != redirect_uri || !matches {
        return Err(TokenError::InvalidGrant);
    }
    sqlx::query!(
        "UPDATE oauth_codes SET used_at = now() WHERE id = $1",
        found.id
    )
    .execute(&mut **tx)
    .await?;
    let scopes = stored_scopes(&found.scopes)?;
    issue(tx, config, found.org_id, found.grant_id, scopes).await
}

/// Rotates a refresh token (research R-8). `scope`, when sent, narrows the
/// new tokens to a subset of the presented token's scopes (FR-045).
pub async fn refresh(
    tx: &mut Tx,
    config: &OAuthConfig,
    client: &ClientAuth,
    refresh_token: &str,
    scope: Option<&str>,
) -> Result<TokenSet, TokenError> {
    let apps = authenticate_client(tx, client).await?;
    let Some(found) = sqlx::query!(
        r#"SELECT r.id, r.scopes, r.used_at IS NOT NULL AS "used!",
                  r.idle_expires_at > now() AS "live!", g.id AS grant_id, g.org_id,
                  g.client_id AS app_id,
                  g.revoked_at IS NULL AND g.expires_at > now() AND u.active AS "active!"
           FROM oauth_refresh_tokens r
           JOIN oauth_grants g ON g.id = r.grant_id
           JOIN users u ON u.org_id = g.org_id AND u.id = g.user_id
           WHERE r.token_sha256 = $1
           FOR UPDATE OF r"#,
        digest(refresh_token),
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Err(TokenError::InvalidGrant);
    };
    if !apps.contains(&found.app_id) {
        return Err(TokenError::InvalidGrant);
    }
    if found.used {
        revoke_reused(tx, found.org_id, found.grant_id, Action::RefreshReused).await?;
        return Err(TokenError::Reused);
    }
    if !found.live || !found.active {
        return Err(TokenError::InvalidGrant);
    }
    let held = stored_scopes(&found.scopes)?;
    let scopes = match scope.filter(|scope| !scope.is_empty()) {
        None => held,
        Some(scope) => ScopeSet::parse_list(scope)
            .ok()
            .map(ScopeSet::normalize)
            .filter(|scopes| !scopes.is_empty() && scopes.is_subset(held))
            .ok_or(TokenError::InvalidScope)?,
    };
    sqlx::query!(
        "UPDATE oauth_refresh_tokens SET used_at = now() WHERE id = $1",
        found.id
    )
    .execute(&mut **tx)
    .await?;
    issue(tx, config, found.org_id, found.grant_id, scopes).await
}

/// Ends the grant of an access or refresh token issued to `client` (RFC
/// 7009). Another app's token, an unknown one and an ended grant change
/// nothing, and answer the same.
pub async fn revoke_token(tx: &mut Tx, client: &ClientAuth, token: &str) -> Result<(), TokenError> {
    let apps = authenticate_client(tx, client).await?;
    let token = digest(token);
    let Some(grant) = sqlx::query!(
        r#"SELECT g.id, g.org_id, g.user_id, g.scopes, c.name AS app_name, c.client_id
           FROM oauth_grants g
           JOIN oauth_clients c ON c.id = g.client_id
           WHERE g.client_id = ANY($2) AND g.revoked_at IS NULL
             AND g.id IN (SELECT grant_id FROM oauth_access_tokens WHERE token_sha256 = $1
                          UNION ALL
                          SELECT grant_id FROM oauth_refresh_tokens WHERE token_sha256 = $1)
           FOR UPDATE OF g"#,
        token,
        &apps,
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(());
    };
    sqlx::query!(
        "UPDATE oauth_grants SET revoked_at = now(), revoke_reason = 'client' WHERE id = $1",
        grant.id,
    )
    .execute(&mut **tx)
    .await?;
    let data = serde_json::json!({
        "app_name": grant.app_name,
        "client_id": grant.client_id,
        "person_id": grant.user_id,
        "scopes": grant.scopes,
        "reason": "client",
    });
    audit::record_for(tx, grant.org_id, None, Action::GrantRevoked, grant.id, data).await?;
    Ok(())
}

/// The caller an OAuth access token stands for, if it is unexpired, its
/// grant active, its app enabled and its person active (FR-052). Stamps the
/// grant's `last_used_at` at most once a minute (FR-053).
pub async fn authenticate(pool: &PgPool, token: &str) -> Result<Option<Caller>, sqlx::Error> {
    if !token.starts_with(ACCESS_TOKEN_PREFIX) {
        return Ok(None);
    }
    let Some(found) = sqlx::query!(
        "SELECT t.id, t.org_id, t.scopes, g.id AS grant_id, g.user_id
         FROM oauth_access_tokens t
         JOIN oauth_grants g ON g.id = t.grant_id
         JOIN oauth_clients c ON c.id = g.client_id
         JOIN users u ON u.org_id = g.org_id AND u.id = g.user_id
         WHERE t.token_sha256 = $1
           AND t.expires_at > now()
           AND g.revoked_at IS NULL AND g.expires_at > now()
           AND c.enabled AND u.active",
        digest(token),
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let Ok(scopes) = ScopeSet::parse_each(found.scopes.iter().map(String::as_str)) else {
        tracing::error!("Invalid stored OAuth token scopes");
        return Ok(None);
    };
    stamp_last_used(pool, found.grant_id).await?;
    Ok(Some(Caller::oauth(
        found.org_id,
        found.user_id,
        found.grant_id,
        found.id,
        scopes,
    )))
}

async fn stamp_last_used(
    executor: impl sqlx::PgExecutor<'_>,
    grant_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_grants SET last_used_at = now()
         WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')",
        grant_id,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// The enabled apps `client` authenticates as: a public app sends no
/// secret, a confidential one its secret, compared by digest in constant
/// time. More than one row only for the built-in id, which every
/// organization has.
async fn authenticate_client(tx: &mut Tx, client: &ClientAuth) -> Result<Vec<Uuid>, TokenError> {
    let rows = sqlx::query!(
        r#"SELECT id, kind::text AS "kind!", secret_sha256
           FROM oauth_clients WHERE client_id = $1 AND enabled"#,
        client.client_id,
    )
    .fetch_all(&mut **tx)
    .await?;
    let presented = client.secret.as_deref().map(digest);
    let apps: Vec<Uuid> = rows
        .into_iter()
        .filter(|row| match (ClientKind::parse(&row.kind), &presented) {
            (Some(ClientKind::Public), None) => true,
            (Some(ClientKind::Confidential), Some(presented)) => row
                .secret_sha256
                .as_ref()
                .is_some_and(|stored| bool::from(stored.ct_eq(presented))),
            _ => false,
        })
        .map(|row| row.id)
        .collect();
    if apps.is_empty() {
        return Err(TokenError::InvalidClient);
    }
    Ok(apps)
}

/// Issues an access token and the next refresh token of a grant. The
/// refresh token idle-expires, and never after its grant (FR-044).
async fn issue(
    tx: &mut Tx,
    config: &OAuthConfig,
    org_id: Uuid,
    grant_id: Uuid,
    scopes: ScopeSet,
) -> Result<TokenSet, TokenError> {
    let access = Secret::generate(ACCESS_TOKEN_PREFIX);
    let refresh = Secret::generate(REFRESH_TOKEN_PREFIX);
    let names = scopes.names();
    sqlx::query!(
        "INSERT INTO oauth_access_tokens (id, org_id, grant_id, token_sha256, scopes, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))",
        Uuid::now_v7(),
        org_id,
        grant_id,
        access.digest(),
        &names as &[&str],
        config.access_ttl.as_secs_f64(),
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO oauth_refresh_tokens (id, org_id, grant_id, token_sha256, scopes,
                                           idle_expires_at)
         SELECT $1, $2, $3, $4, $5, LEAST(now() + make_interval(secs => $6), g.expires_at)
         FROM oauth_grants g WHERE g.id = $3",
        Uuid::now_v7(),
        org_id,
        grant_id,
        refresh.digest(),
        &names as &[&str],
        config.refresh_idle.as_secs_f64(),
    )
    .execute(&mut **tx)
    .await?;
    stamp_last_used(&mut **tx, grant_id).await?;
    Ok(TokenSet {
        access_token: access,
        refresh_token: refresh,
        expires_in: config.access_ttl.as_secs(),
        scopes,
    })
}

/// Ends a grant whose code or refresh token was presented twice: someone
/// other than the app may hold its tokens.
async fn revoke_reused(
    tx: &mut Tx,
    org_id: Uuid,
    grant_id: Uuid,
    action: Action,
) -> Result<(), TokenError> {
    let reason = match action {
        Action::CodeReused => "code_reuse",
        _ => "refresh_reuse",
    };
    let revoked = sqlx::query!(
        r#"UPDATE oauth_grants g
           SET revoked_at = now(), revoke_reason = $2::text::oauth_revoke_reason
           FROM oauth_clients c
           WHERE g.id = $1 AND g.revoked_at IS NULL AND c.id = g.client_id
           RETURNING g.user_id, g.scopes, c.name AS app_name, c.client_id"#,
        grant_id,
        reason,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(grant) = revoked {
        let data = serde_json::json!({
            "app_name": grant.app_name,
            "client_id": grant.client_id,
            "person_id": grant.user_id,
            "scopes": grant.scopes,
            "reason": reason,
        });
        audit::record_for(tx, org_id, None, action, grant_id, data).await?;
    }
    Ok(())
}

fn stored_scopes(scopes: &[String]) -> Result<ScopeSet, TokenError> {
    ScopeSet::parse_each(scopes.iter().map(String::as_str)).map_err(|_| {
        tracing::error!("Invalid stored OAuth scopes");
        TokenError::Storage(OpError::Internal(
            "Could not complete the request. Please retry.".into(),
        ))
    })
}

#[cfg(test)]
mod tests;
