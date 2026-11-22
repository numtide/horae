//! The apps an administrator registers (FR-001 to FR-005), and the built-in
//! client of Horae's Android app that every organization has.

use chrono::{DateTime, Utc};
use horae_core::permissions::oauth::{ClientKind, RedirectUri, ScopeSet};
use uuid::Uuid;

use super::audit::{self, Action};
use super::{CLIENT_SECRET_PREFIX, Secret, generate_client_id};
use crate::ops::{Actor, Done, OpError, Tx};

/// The built-in client's fixed fields (spec D-1). Migration `0008_oauth`
/// creates it, disabled, for every organization.
pub const BUILT_IN_CLIENT_ID: &str = "horae-android";
pub const BUILT_IN_REDIRECT_URI: &str = "dev.horae.android:/oauth/callback";

const MAX_NAME: usize = 100;
const MAX_REDIRECT_URIS: usize = 10;

const NOT_FOUND: &str = "App not found";
const NAME_LENGTH: &str = "Name the app in 1 to 100 characters";
const NAME_TAKEN: &str = "Another app already has this name";
const REDIRECT_COUNT: &str = "Add between 1 and 10 redirect URIs";
const REDIRECT_DUPLICATE: &str = "Each redirect URI can be listed once";
const SCOPES_EMPTY: &str = "Choose at least one scope";
const BUILT_IN_FIXED: &str = "The redirect URIs of Horae's own app cannot change";
const BUILT_IN_KEPT: &str = "Horae's own app cannot be deleted; disable it instead";
const PUBLIC_NO_SECRET: &str = "A public app has no secret";

/// An app as the administration pages show it. It never holds a secret.
#[derive(Debug, Clone, PartialEq)]
pub struct OAuthClient {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub kind: ClientKind,
    pub redirect_uris: Vec<String>,
    pub scopes: ScopeSet,
    pub built_in: bool,
    pub enabled: bool,
    /// The first characters of a confidential app's secret.
    pub secret_prefix: Option<String>,
    /// Authorizations neither revoked nor expired.
    pub active_grants: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What an administrator enters to register an app.
#[derive(Debug, Clone)]
pub struct NewClient {
    pub name: String,
    pub kind: ClientKind,
    pub redirect_uris: Vec<String>,
    pub scopes: ScopeSet,
}

/// What an administrator may change on an app; its kind stays as registered.
#[derive(Debug, Clone)]
pub struct ClientChanges {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub scopes: ScopeSet,
}

/// Every app of the organization, the built-in client first.
pub async fn list(tx: &mut Tx, actor: &Actor) -> Result<Vec<OAuthClient>, OpError> {
    actor.require_administrator()?;
    fetch(tx, actor.org_id(), None).await
}

pub async fn get(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<OAuthClient, OpError> {
    actor.require_administrator()?;
    fetch_one(tx, actor.org_id(), id).await
}

/// Registers an app. A confidential app's [`Secret`] is the only copy there
/// will be.
pub async fn register(
    tx: &mut Tx,
    actor: &Actor,
    input: NewClient,
) -> Result<Done<(OAuthClient, Option<Secret>)>, OpError> {
    actor.require_administrator()?;
    let name = valid_name(&input.name)?;
    let redirect_uris = valid_redirect_uris(&input.redirect_uris, input.kind)?;
    let scopes = valid_scopes(input.scopes)?;
    let secret =
        (input.kind == ClientKind::Confidential).then(|| Secret::generate(CLIENT_SECRET_PREFIX));
    let id = Uuid::now_v7();
    let inserted = sqlx::query!(
        "INSERT INTO oauth_clients
           (id, org_id, client_id, name, kind, secret_sha256, secret_prefix, redirect_uris,
            scopes, created_by)
         VALUES ($1, $2, $3, $4, $5::text::oauth_client_kind, $6, $7, $8, $9, $10)",
        id,
        actor.org_id(),
        generate_client_id(),
        name,
        input.kind.as_str(),
        secret.as_ref().map(Secret::digest),
        secret
            .as_ref()
            .map(|secret| secret.display_prefix().to_owned()),
        &redirect_uris,
        &scopes.names() as &[&str],
        actor.id(),
    )
    .execute(&mut **tx)
    .await;
    name_taken(inserted)?;
    let client = fetch_one(tx, actor.org_id(), id).await?;
    audit::record(tx, actor, Action::ClientRegistered, id, details(&client)).await?;
    Ok(Done::new((client, secret)))
}

/// Changes an app's name, redirect URIs and scopes. The built-in client
/// keeps its redirect URIs. Narrowing the scopes leaves existing
/// authorizations as they were approved.
pub async fn update(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    changes: ClientChanges,
) -> Result<Done<OAuthClient>, OpError> {
    actor.require_administrator()?;
    let current = lock(tx, actor.org_id(), id).await?;
    let name = valid_name(&changes.name)?;
    let redirect_uris = valid_redirect_uris(&changes.redirect_uris, current.kind)?;
    let scopes = valid_scopes(changes.scopes)?;
    if current.built_in && redirect_uris != current.redirect_uris {
        return Err(OpError::conflict(BUILT_IN_FIXED));
    }
    let updated = sqlx::query!(
        "UPDATE oauth_clients SET name = $2, redirect_uris = $3, scopes = $4 WHERE id = $1",
        id,
        name,
        &redirect_uris,
        &scopes.names() as &[&str],
    )
    .execute(&mut **tx)
    .await;
    name_taken(updated)?;
    let client = fetch_one(tx, actor.org_id(), id).await?;
    audit::record(tx, actor, Action::ClientUpdated, id, details(&client)).await?;
    Ok(Done::new(client))
}

/// Enables or disables an app. A disabled app's tokens stop working and it
/// cannot start new authorizations, but its authorizations are kept and work
/// again once it is enabled. Setting the current state changes nothing.
pub async fn set_enabled(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    enabled: bool,
) -> Result<Done<OAuthClient>, OpError> {
    actor.require_administrator()?;
    let current = lock(tx, actor.org_id(), id).await?;
    if current.enabled != enabled {
        sqlx::query!(
            "UPDATE oauth_clients SET enabled = $2 WHERE id = $1",
            id,
            enabled
        )
        .execute(&mut **tx)
        .await?;
    }
    let client = fetch_one(tx, actor.org_id(), id).await?;
    if current.enabled != enabled {
        let action = if enabled {
            Action::ClientEnabled
        } else {
            Action::ClientDisabled
        };
        audit::record(tx, actor, action, id, details(&client)).await?;
    }
    Ok(Done::new(client))
}

/// Gives a confidential app a new secret, which ends the old one at once.
pub async fn replace_secret(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
) -> Result<Done<(OAuthClient, Secret)>, OpError> {
    actor.require_administrator()?;
    let current = lock(tx, actor.org_id(), id).await?;
    if current.kind != ClientKind::Confidential {
        return Err(OpError::conflict(PUBLIC_NO_SECRET));
    }
    let secret = Secret::generate(CLIENT_SECRET_PREFIX);
    sqlx::query!(
        "UPDATE oauth_clients SET secret_sha256 = $2, secret_prefix = $3 WHERE id = $1",
        id,
        secret.digest(),
        secret.display_prefix(),
    )
    .execute(&mut **tx)
    .await?;
    let client = fetch_one(tx, actor.org_id(), id).await?;
    audit::record(
        tx,
        actor,
        Action::ClientSecretReplaced,
        id,
        details(&client),
    )
    .await?;
    Ok(Done::new((client, secret)))
}

/// Deletes an app with its authorizations, codes and tokens. The built-in
/// client cannot be deleted.
pub async fn delete(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<Done<()>, OpError> {
    actor.require_administrator()?;
    let current = lock(tx, actor.org_id(), id).await?;
    if current.built_in {
        return Err(OpError::conflict(BUILT_IN_KEPT));
    }
    sqlx::query!("DELETE FROM oauth_clients WHERE id = $1", id)
        .execute(&mut **tx)
        .await?;
    audit::record(tx, actor, Action::ClientDeleted, id, details(&current)).await?;
    Ok(Done::new(()))
}

/// What the audit log records about an app: never a secret or its digest.
fn details(client: &OAuthClient) -> serde_json::Value {
    serde_json::json!({
        "name": client.name,
        "client_id": client.client_id,
        "kind": client.kind.as_str(),
        "scopes": client.scopes.names(),
        "redirect_uris": client.redirect_uris,
        "enabled": client.enabled,
    })
}

fn valid_name(name: &str) -> Result<&str, OpError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(OpError::invalid("name", NAME_LENGTH));
    }
    Ok(name)
}

fn valid_redirect_uris(uris: &[String], kind: ClientKind) -> Result<Vec<String>, OpError> {
    if uris.is_empty() || uris.len() > MAX_REDIRECT_URIS {
        return Err(OpError::invalid("redirect_uris", REDIRECT_COUNT));
    }
    let mut valid: Vec<String> = Vec::with_capacity(uris.len());
    for uri in uris {
        let uri = RedirectUri::parse(uri.trim(), kind)
            .map_err(|error| OpError::invalid("redirect_uris", error))?;
        if valid.iter().any(|seen| seen == uri.as_str()) {
            return Err(OpError::invalid("redirect_uris", REDIRECT_DUPLICATE));
        }
        valid.push(uri.as_str().to_owned());
    }
    Ok(valid)
}

fn valid_scopes(scopes: ScopeSet) -> Result<ScopeSet, OpError> {
    if scopes.is_empty() {
        return Err(OpError::invalid("scopes", SCOPES_EMPTY));
    }
    Ok(scopes.normalize())
}

fn name_taken(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) -> Result<(), OpError> {
    match result {
        Err(error)
            if error
                .as_database_error()
                .is_some_and(|db| db.is_unique_violation()) =>
        {
            Err(OpError::conflict(NAME_TAKEN))
        }
        other => other.map(drop).map_err(OpError::from),
    }
}

/// Locks an app for a change and reads it.
async fn lock(tx: &mut Tx, org_id: Uuid, id: Uuid) -> Result<OAuthClient, OpError> {
    sqlx::query_scalar!(
        "SELECT id FROM oauth_clients WHERE id = $1 AND org_id = $2 FOR UPDATE",
        id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    fetch_one(tx, org_id, id).await
}

async fn fetch_one(tx: &mut Tx, org_id: Uuid, id: Uuid) -> Result<OAuthClient, OpError> {
    fetch(tx, org_id, Some(id))
        .await?
        .pop()
        .ok_or_else(|| OpError::not_found(NOT_FOUND))
}

/// One app, or all of them with `id` `None`.
async fn fetch(tx: &mut Tx, org_id: Uuid, id: Option<Uuid>) -> Result<Vec<OAuthClient>, OpError> {
    let rows = sqlx::query!(
        r#"SELECT c.id, c.client_id, c.name, c.kind::text AS "kind!", c.redirect_uris,
                  c.scopes, c.built_in, c.enabled, c.secret_prefix,
                  c.created_at AS "created_at: DateTime<Utc>",
                  c.updated_at AS "updated_at: DateTime<Utc>",
                  (SELECT count(*) FROM oauth_grants g
                   WHERE g.client_id = c.id AND g.revoked_at IS NULL AND g.expires_at > now())
                    AS "active_grants!"
           FROM oauth_clients c
           WHERE c.org_id = $1 AND ($2::uuid IS NULL OR c.id = $2)
           ORDER BY c.built_in DESC, lower(c.name), c.id"#,
        org_id,
        id,
    )
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(OAuthClient {
                id: row.id,
                client_id: row.client_id,
                name: row.name,
                kind: ClientKind::parse(&row.kind).ok_or_else(stored_invalid)?,
                scopes: ScopeSet::parse_each(row.scopes.iter().map(String::as_str))
                    .map_err(|_| stored_invalid())?,
                redirect_uris: row.redirect_uris,
                built_in: row.built_in,
                enabled: row.enabled,
                secret_prefix: row.secret_prefix,
                active_grants: row.active_grants,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}

/// A stored value the schema's checks should have kept out.
fn stored_invalid() -> OpError {
    tracing::error!("Invalid stored OAuth client");
    OpError::Internal("Could not complete the request. Please retry.".into())
}

#[cfg(test)]
mod tests;
