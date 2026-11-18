//! Personal access tokens (spec 022, research R-6): a person creates, lists and
//! revokes their own; an administrator lists and revokes everyone's.
//!
//! A token is `horae_pat_` followed by 32 random bytes in base64url. Only the
//! SHA-256 digest of the whole string is stored, with its first characters for
//! display, so the secret exists only in the creation answer and in the
//! requests that carry it. [`authenticate`] turns a presented token into a
//! [`Caller`] that [`Actor::load`](super::Actor::load) treats as a session.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::aead::rand_core::RngCore;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{Actor, Caller, Channel, Done, OpError, Tx};
use crate::models::api_tokens::ApiTokenRow;

/// What every token starts with, so people and secret scanners recognise it.
pub const PREFIX: &str = "horae_pat_";
/// How many characters after [`PREFIX`] are kept to tell tokens apart.
const DISPLAY_LEN: usize = 8;
const SECRET_BYTES: usize = 32;
const MAX_NAME: usize = 100;

const ACTIVE_REQUIRED: &str = "An active account is required";
const NOT_FOUND: &str = "API token not found";
const NAME_LENGTH: &str = "Name the token in 1 to 100 characters";
const EXPIRY_PAST: &str = "Choose an expiry in the future";

/// A token's secret. `Debug` prints only its display prefix.
pub struct Secret(String);

impl Secret {
    fn generate() -> Self {
        let mut bytes = [0u8; SECRET_BYTES];
        OsRng.fill_bytes(&mut bytes);
        Self(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// The whole token, to hand to its owner once.
    pub fn expose(&self) -> &str {
        &self.0
    }

    fn display_prefix(&self) -> &str {
        &self.0[PREFIX.len()..PREFIX.len() + DISPLAY_LEN]
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{PREFIX}{}…", self.display_prefix())
    }
}

fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn require_active(actor: &Actor) -> Result<(), OpError> {
    if actor.is_active() {
        Ok(())
    } else {
        Err(OpError::forbidden(ACTIVE_REQUIRED))
    }
}

/// The actor's own tokens, newest first.
pub async fn list_own(tx: &mut Tx, actor: &Actor) -> Result<Vec<ApiTokenRow>, OpError> {
    require_active(actor)?;
    Ok(sqlx::query_as!(
        ApiTokenRow,
        r#"SELECT t.id, t.user_id, u.name AS owner_name, t.name, t.display_prefix,
                t.created_at AS "created_at: DateTime<Utc>",
                t.expires_at AS "expires_at: DateTime<Utc>",
                t.last_used_at AS "last_used_at: DateTime<Utc>",
                t.revoked_at AS "revoked_at: DateTime<Utc>"
         FROM api_tokens t JOIN users u ON u.id = t.user_id
         WHERE t.org_id = $1 AND t.user_id = $2
         ORDER BY t.created_at DESC, t.id DESC"#,
        actor.org_id(),
        actor.id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Every token in the actor's organization, for an administrator.
pub async fn list_org(tx: &mut Tx, actor: &Actor) -> Result<Vec<ApiTokenRow>, OpError> {
    actor.require_administrator()?;
    Ok(sqlx::query_as!(
        ApiTokenRow,
        r#"SELECT t.id, t.user_id, u.name AS owner_name, t.name, t.display_prefix,
                t.created_at AS "created_at: DateTime<Utc>",
                t.expires_at AS "expires_at: DateTime<Utc>",
                t.last_used_at AS "last_used_at: DateTime<Utc>",
                t.revoked_at AS "revoked_at: DateTime<Utc>"
         FROM api_tokens t JOIN users u ON u.id = t.user_id
         WHERE t.org_id = $1
         ORDER BY t.created_at DESC, t.id DESC"#,
        actor.org_id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Who a token's request acts as: the person, their organization and the
/// token itself.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenIdentity {
    pub person_email: String,
    pub organization_name: String,
    /// `owner_name` is the person's name.
    pub token: ApiTokenRow,
}

/// The identity of an actor that came through a token; any other channel has
/// no token to describe.
pub async fn identity(tx: &mut Tx, actor: &Actor) -> Result<TokenIdentity, OpError> {
    require_active(actor)?;
    let Channel::Token { token_id } = actor.channel() else {
        return Err(OpError::not_found(NOT_FOUND));
    };
    let row = sqlx::query!(
        r#"SELECT t.id, t.user_id, u.name AS owner_name, u.email, o.name AS organization_name,
                t.name, t.display_prefix,
                t.created_at AS "created_at: DateTime<Utc>",
                t.expires_at AS "expires_at: DateTime<Utc>",
                t.last_used_at AS "last_used_at: DateTime<Utc>",
                t.revoked_at AS "revoked_at: DateTime<Utc>"
         FROM api_tokens t
         JOIN users u ON u.org_id = t.org_id AND u.id = t.user_id
         JOIN organizations o ON o.id = t.org_id
         WHERE t.id = $1 AND t.org_id = $2 AND t.user_id = $3"#,
        token_id,
        actor.org_id(),
        actor.id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    Ok(TokenIdentity {
        person_email: row.email,
        organization_name: row.organization_name,
        token: ApiTokenRow {
            id: row.id,
            user_id: row.user_id,
            owner_name: row.owner_name,
            name: row.name,
            display_prefix: row.display_prefix,
            created_at: row.created_at,
            expires_at: row.expires_at,
            last_used_at: row.last_used_at,
            revoked_at: row.revoked_at,
        },
    })
}

/// Creates a token for the actor. The [`Secret`] is the only copy there will be.
pub async fn create(
    tx: &mut Tx,
    actor: &Actor,
    name: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Done<(Secret, ApiTokenRow)>, OpError> {
    require_active(actor)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(OpError::invalid("name", NAME_LENGTH));
    }
    if expires_at.is_some_and(|at| at <= Utc::now()) {
        return Err(OpError::invalid("expires_at", EXPIRY_PAST));
    }
    let secret = Secret::generate();
    let row = sqlx::query_as!(
        ApiTokenRow,
        r#"WITH token AS (
             INSERT INTO api_tokens (id, org_id, user_id, name, token_sha256, display_prefix, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *
         )
         SELECT t.id, t.user_id, u.name AS owner_name, t.name, t.display_prefix,
                t.created_at AS "created_at: DateTime<Utc>",
                t.expires_at AS "expires_at: DateTime<Utc>",
                t.last_used_at AS "last_used_at: DateTime<Utc>",
                t.revoked_at AS "revoked_at: DateTime<Utc>"
         FROM token t JOIN users u ON u.id = t.user_id"#,
        Uuid::now_v7(),
        actor.org_id(),
        actor.id(),
        name,
        digest(secret.expose()),
        secret.display_prefix(),
        expires_at as Option<DateTime<Utc>>,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(Done::new((secret, row)))
}

/// Revokes a token of the actor's, or anyone's for an administrator. Revoking
/// a revoked token changes nothing and answers the row. Another person's
/// token reads as not found, so a member learns nothing about it.
pub async fn revoke(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<Done<ApiTokenRow>, OpError> {
    require_active(actor)?;
    let owner = sqlx::query_scalar!(
        "SELECT user_id FROM api_tokens WHERE id = $1 AND org_id = $2 FOR UPDATE",
        id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?;
    if owner.is_none_or(|owner| owner != actor.id() && !actor.administrator()) {
        return Err(OpError::not_found(NOT_FOUND));
    }
    let row = sqlx::query_as!(
        ApiTokenRow,
        r#"WITH token AS (
             UPDATE api_tokens SET revoked_at = COALESCE(revoked_at, now())
             WHERE id = $1
             RETURNING *
         )
         SELECT t.id, t.user_id, u.name AS owner_name, t.name, t.display_prefix,
                t.created_at AS "created_at: DateTime<Utc>",
                t.expires_at AS "expires_at: DateTime<Utc>",
                t.last_used_at AS "last_used_at: DateTime<Utc>",
                t.revoked_at AS "revoked_at: DateTime<Utc>"
         FROM token t JOIN users u ON u.id = t.user_id"#,
        id,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(Done::new(row))
}

/// The caller a presented token stands for: unrevoked, unexpired, and its
/// owner active. `None` for anything else, malformed input included. Runs
/// before the operation's transaction, like `active_caller` for a session;
/// `Actor::load` checks the person again inside it.
///
/// Stamps `last_used_at` at most once a minute, so a busy script does not
/// write on every request.
pub async fn authenticate(pool: &PgPool, token: &str) -> Result<Option<Caller>, sqlx::Error> {
    if !token.starts_with(PREFIX) {
        return Ok(None);
    }
    let Some(found) = sqlx::query!(
        "SELECT t.id, t.org_id, t.user_id
         FROM api_tokens t JOIN users u ON u.id = t.user_id AND u.org_id = t.org_id
         WHERE t.token_sha256 = $1
           AND t.revoked_at IS NULL
           AND (t.expires_at IS NULL OR t.expires_at > now())
           AND u.active",
        digest(token),
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    sqlx::query!(
        "UPDATE api_tokens SET last_used_at = now()
         WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')",
        found.id,
    )
    .execute(pool)
    .await?;
    Ok(Some(Caller::token(found.org_id, found.user_id, found.id)))
}

#[cfg(test)]
mod tests;
