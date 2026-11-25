//! The authorizations people give apps (FR-091 to FR-093): each person lists
//! and revokes their own; an administrator lists and revokes any app's.

use chrono::{DateTime, Utc};
use horae_core::permissions::oauth::ScopeSet;
use uuid::Uuid;

use super::audit::{self, Action};
use crate::ops::{Actor, Done, OpError, Tx};

const NOT_FOUND: &str = "Authorization not found";
const APP_NOT_FOUND: &str = "App not found";

/// An authorization neither revoked nor expired. It holds no token.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantRow {
    pub id: Uuid,
    /// The app's row id.
    pub app_id: Uuid,
    pub app_name: String,
    /// The app's public client id.
    pub client_id: String,
    pub app_built_in: bool,
    pub person_id: Uuid,
    pub person_name: String,
    pub person_email: String,
    pub scopes: ScopeSet,
    pub approved_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

/// The actor's own authorizations, newest first.
pub async fn list_own(tx: &mut Tx, actor: &Actor) -> Result<Vec<GrantRow>, OpError> {
    actor.require_session()?;
    fetch(tx, actor.org_id(), Some(actor.id()), None).await
}

/// Every authorization of one app, for an administrator.
pub async fn list_for_client(
    tx: &mut Tx,
    actor: &Actor,
    app_id: Uuid,
) -> Result<Vec<GrantRow>, OpError> {
    actor.require_session()?;
    actor.require_administrator()?;
    sqlx::query_scalar!(
        "SELECT id FROM oauth_clients WHERE id = $1 AND org_id = $2",
        app_id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(APP_NOT_FOUND))?;
    fetch(tx, actor.org_id(), None, Some(app_id)).await
}

/// Ends an authorization of the actor's, or anyone's for an administrator,
/// and with it every token issued under it. Another person's authorization
/// reads as not found for a member. Revoking an ended one changes nothing.
/// Only a signed-in person revokes here; an app revokes its own token through
/// the revocation endpoint.
pub async fn revoke(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<Done<()>, OpError> {
    actor.require_session()?;
    let grant = sqlx::query!(
        "SELECT g.user_id, g.revoked_at IS NOT NULL AS \"revoked!\", g.scopes,
                c.name AS app_name, c.client_id, u.name AS person_name
         FROM oauth_grants g
         JOIN oauth_clients c ON c.id = g.client_id
         JOIN users u ON u.org_id = g.org_id AND u.id = g.user_id
         WHERE g.id = $1 AND g.org_id = $2
         FOR UPDATE OF g",
        id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    let reason = if grant.user_id == actor.id() {
        "person"
    } else if actor.administrator() {
        "administrator"
    } else {
        return Err(OpError::not_found(NOT_FOUND));
    };
    if grant.revoked {
        return Ok(Done::new(()));
    }
    sqlx::query!(
        "UPDATE oauth_grants
         SET revoked_at = now(), revoked_by = $2, revoke_reason = $3::text::oauth_revoke_reason
         WHERE id = $1",
        id,
        actor.id(),
        reason,
    )
    .execute(&mut **tx)
    .await?;
    let data = serde_json::json!({
        "app_name": grant.app_name,
        "client_id": grant.client_id,
        "person_id": grant.user_id,
        "person_name": grant.person_name,
        "scopes": grant.scopes,
        "reason": reason,
    });
    audit::record(tx, actor, Action::GrantRevoked, id, data).await?;
    Ok(Done::new(()))
}

/// Active authorizations, of one person or of one app.
async fn fetch(
    tx: &mut Tx,
    org_id: Uuid,
    person: Option<Uuid>,
    app: Option<Uuid>,
) -> Result<Vec<GrantRow>, OpError> {
    let rows = sqlx::query!(
        r#"SELECT g.id, c.id AS app_id, c.name AS app_name, c.client_id,
                  c.built_in AS app_built_in, g.user_id, u.name AS person_name, u.email AS person_email,
                  g.scopes, g.approved_at AS "approved_at: DateTime<Utc>",
                  g.last_used_at AS "last_used_at: DateTime<Utc>",
                  g.expires_at AS "expires_at: DateTime<Utc>"
           FROM oauth_grants g
           JOIN oauth_clients c ON c.id = g.client_id
           JOIN users u ON u.org_id = g.org_id AND u.id = g.user_id
           WHERE g.org_id = $1
             AND ($2::uuid IS NULL OR g.user_id = $2)
             AND ($3::uuid IS NULL OR g.client_id = $3)
             AND g.revoked_at IS NULL AND g.expires_at > now()
           ORDER BY g.approved_at DESC, g.id DESC"#,
        org_id,
        person,
        app,
    )
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(GrantRow {
                id: row.id,
                app_id: row.app_id,
                app_name: row.app_name,
                client_id: row.client_id,
                app_built_in: row.app_built_in,
                person_id: row.user_id,
                person_name: row.person_name,
                person_email: row.person_email,
                scopes: ScopeSet::parse_each(row.scopes.iter().map(String::as_str)).map_err(
                    |_| {
                        tracing::error!("Invalid stored OAuth grant scopes");
                        OpError::Internal("Could not complete the request. Please retry.".into())
                    },
                )?,
                approved_at: row.approved_at,
                last_used_at: row.last_used_at,
                expires_at: row.expires_at,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
