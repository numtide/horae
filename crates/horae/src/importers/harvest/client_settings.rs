//! The Harvest OAuth client an organization's administrators store from the
//! importer, and the client each request actually uses.
//!
//! The environment's client ID and secret take precedence as a pair; the
//! environment's redirect URL takes precedence on its own; the encryption key
//! comes only from the environment. The stored secret is sealed with the same
//! AEAD as the OAuth tokens. Changes share the connection-change transaction:
//! the import lock, an administrator recheck, the connection gate, refusal
//! while an import is active, and a revision bump that invalidates OAuth
//! attempts started before the change.

use chrono::{DateTime, Utc};
use horae_core::importers::harvest::client::{
    self as rules, ClientSettings, ClientSource, Existing, InvalidClient,
};
use sqlx::{Acquire, PgPool};
use uuid::Uuid;

use super::{account_switch, credentials};
use crate::config::{HarvestConfig, HarvestDeployment};

/// Refusals that are safe to show an administrator.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(
        "Horae cannot store a client secret until the operator sets HORAE_HARVEST_ENC_KEY (64 hex characters) and restarts the server"
    )]
    NoEncryptionKey,
    #[error(
        "The Harvest OAuth client is configured by the environment (HORAE_HARVEST_CLIENT_ID and HORAE_HARVEST_CLIENT_SECRET)"
    )]
    Environment,
    /// The change would delete the tokens of a connection the caller did not
    /// confirm disconnecting, for example from a page loaded before it existed.
    #[error("{}", rules::DISCONNECT_UNCONFIRMED)]
    DisconnectUnconfirmed,
    #[error(transparent)]
    Invalid(#[from] InvalidClient),
}

struct Stored {
    id: Uuid,
    client_id: String,
    client_secret_enc: Vec<u8>,
    redirect_url: Option<String>,
    updated_at: DateTime<Utc>,
}

async fn stored<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
) -> anyhow::Result<Option<Stored>> {
    Ok(sqlx::query_as!(
        Stored,
        r#"SELECT id, client_id, client_secret_enc, redirect_url,
                  updated_at AS "updated_at: DateTime<Utc>"
           FROM harvest_oauth_clients WHERE org_id = $1"#,
        org_id,
    )
    .fetch_optional(exec)
    .await?)
}

/// The client this organization's Harvest requests use, or `None` when the
/// environment and the stored client together do not supply one.
pub async fn resolve<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    deployment: &HarvestDeployment,
    org_id: Uuid,
) -> anyhow::Result<Option<HarvestConfig>> {
    let Some(key) = &deployment.encryption_key_hex else {
        return Ok(None);
    };
    // An environment client ignores the stored row entirely, which cannot be
    // edited while it is in effect.
    let (client_id, client_secret, stored_redirect) = match &deployment.client {
        Some(client) => (client.client_id.clone(), client.client_secret.clone(), None),
        None => {
            let Some(row) = stored(exec, org_id).await? else {
                return Ok(None);
            };
            let Ok(secret) = credentials::decrypt(key, &row.client_secret_enc) else {
                tracing::warn!(%org_id, "stored Harvest client secret does not open with HORAE_HARVEST_ENC_KEY");
                return Ok(None);
            };
            (row.client_id, secret, row.redirect_url)
        }
    };
    let Some(redirect_url) = deployment.redirect_url.clone().or(stored_redirect) else {
        return Ok(None);
    };
    Ok(Some(HarvestConfig {
        client_id,
        client_secret,
        redirect_url,
        encryption_key_hex: key.clone(),
    }))
}

/// What the importer shows administrators. Never carries the secret.
pub async fn view<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    deployment: &HarvestDeployment,
    org_id: Uuid,
) -> anyhow::Result<ClientSettings> {
    let row = stored(exec, org_id).await?;
    let key = deployment.encryption_key_hex.as_deref();
    let mut settings = ClientSettings {
        redirect_url: deployment.redirect_url.clone(),
        redirect_from_environment: deployment.redirect_url.is_some(),
        encryption_key_set: key.is_some(),
        ..Default::default()
    };
    if let Some(client) = &deployment.client {
        settings.source = ClientSource::Environment;
        settings.client_id = Some(client.client_id.clone());
        settings.secret_set = true;
        settings.secret_readable = true;
        settings.stored_ignored = row.is_some();
    } else if let Some(row) = row {
        settings.source = ClientSource::Stored;
        settings.secret_set = true;
        settings.secret_readable =
            key.is_some_and(|key| credentials::decrypt(key, &row.client_secret_enc).is_ok());
        settings.stored_at = Some(row.updated_at.to_rfc3339());
        settings.redirect_url = settings.redirect_url.or(row.redirect_url);
        settings.client_id = Some(row.client_id);
    }
    Ok(settings)
}

/// Save or replace the organization's client. A blank secret keeps the stored
/// one for the same client ID; a different client ID disconnects Harvest,
/// which only a caller passing `confirm_disconnect` may do.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors the save request's fields"
)]
pub async fn save(
    pool: &PgPool,
    deployment: &HarvestDeployment,
    org_id: Uuid,
    actor_id: Uuid,
    client_id: &str,
    client_secret: Option<&str>,
    redirect_url: &str,
    confirm_disconnect: bool,
) -> anyhow::Result<()> {
    stored_in_effect(deployment)?;
    let key = deployment
        .encryption_key_hex
        .as_deref()
        .ok_or(ClientError::NoEncryptionKey)?;
    change(pool, org_id, actor_id, async |tx| {
        let existing = stored(&mut **tx, org_id).await?;
        let input = rules::validate_save(
            client_id,
            client_secret,
            redirect_url,
            deployment.redirect_url.is_some(),
            existing.as_ref().map(|row| Existing {
                client_id: &row.client_id,
                secret_readable: credentials::decrypt(key, &row.client_secret_enc).is_ok(),
            }),
        )
        .map_err(ClientError::from)?;
        let secret_enc = match (&input.client_secret, &existing) {
            (Some(secret), _) => credentials::encrypt(key, secret)?,
            (None, Some(row)) => row.client_secret_enc.clone(),
            (None, None) => unreachable!("validate_save requires a secret for a new client"),
        };
        let disconnects = rules::save_disconnects(
            existing.as_ref().map(|row| row.client_id.as_str()),
            &input.client_id,
        );
        let disconnected = disconnects && delete_tokens(tx, org_id, confirm_disconnect).await?;
        let id = sqlx::query_scalar!(
            r#"INSERT INTO harvest_oauth_clients
                 (id, org_id, client_id, client_secret_enc, redirect_url, updated_by)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (org_id) DO UPDATE SET
                 client_id = EXCLUDED.client_id,
                 client_secret_enc = EXCLUDED.client_secret_enc,
                 redirect_url = EXCLUDED.redirect_url,
                 updated_by = EXCLUDED.updated_by
               RETURNING id"#,
            Uuid::now_v7(),
            org_id,
            input.client_id,
            secret_enc,
            input.redirect_url,
            actor_id,
        )
        .fetch_one(&mut **tx)
        .await?;
        record(
            tx,
            org_id,
            actor_id,
            "harvest.oauth_client.save",
            id,
            &input.client_id,
            disconnected,
        )
        .await
    })
    .await
}

/// Remove the organization's stored client, disconnecting Harvest when
/// `confirm_disconnect` allows it. Removing a client that does not exist
/// changes nothing. It needs no encryption key, so a client whose secret no
/// longer opens can still be removed.
pub async fn remove(
    pool: &PgPool,
    deployment: &HarvestDeployment,
    org_id: Uuid,
    actor_id: Uuid,
    confirm_disconnect: bool,
) -> anyhow::Result<()> {
    stored_in_effect(deployment)?;
    change(pool, org_id, actor_id, async |tx| {
        let Some(row) = stored(&mut **tx, org_id).await? else {
            return Ok(());
        };
        let disconnected = delete_tokens(tx, org_id, confirm_disconnect).await?;
        sqlx::query!(
            "DELETE FROM harvest_oauth_clients WHERE org_id = $1",
            org_id
        )
        .execute(&mut **tx)
        .await?;
        record(
            tx,
            org_id,
            actor_id,
            "harvest.oauth_client.remove",
            row.id,
            &row.client_id,
            disconnected,
        )
        .await
    })
    .await
}

/// The stored client may change only while the environment does not override it.
fn stored_in_effect(deployment: &HarvestDeployment) -> Result<(), ClientError> {
    match deployment.client {
        Some(_) => Err(ClientError::Environment),
        None => Ok(()),
    }
}

type Tx<'c> = sqlx::Transaction<'c, sqlx::Postgres>;

/// Run one client change under the same guards as connecting or changing the
/// Harvest account. `write` returns without writing when nothing changes.
async fn change(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    write: impl AsyncFnOnce(&mut Tx<'_>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut connection = super::lock_import(pool, org_id).await?;
    let result = async {
        let mut tx = connection.begin().await?;
        credentials::authorize_change(&mut tx, org_id, actor_id).await?;
        account_switch::gate(&mut tx, org_id).await?;
        if account_switch::status(&mut *tx, org_id, true)
            .await?
            .active_imports
            > 0
        {
            return Err(account_switch::ChangeError::Blocked(
                "Finish or cancel the active Harvest import before changing the OAuth client",
            )
            .into());
        }
        write(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    super::release_import(connection).await?;
    result
}

/// Disconnect as `credentials::disconnect` does: tokens go, the account
/// binding stays. Returns whether a connection existed. Deleting tokens the
/// caller did not confirm fails, which rolls the whole change back.
async fn delete_tokens(tx: &mut Tx<'_>, org_id: Uuid, confirmed: bool) -> anyhow::Result<bool> {
    let deleted = sqlx::query!("DELETE FROM harvest_credentials WHERE org_id = $1", org_id)
        .execute(&mut **tx)
        .await?;
    let disconnected = deleted.rows_affected() > 0;
    if disconnected && !confirmed {
        return Err(ClientError::DisconnectUnconfirmed.into());
    }
    Ok(disconnected)
}

/// Fence OAuth attempts started under the previous client and audit the change.
async fn record(
    tx: &mut Tx<'_>,
    org_id: Uuid,
    actor_id: Uuid,
    action: &str,
    client_row: Uuid,
    client_id: &str,
    disconnected: bool,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE harvest_connection_generations SET connection_revision = connection_revision + 1 WHERE org_id = $1",
        org_id,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO audit_log (id, org_id, actor_user_id, action, entity_type, entity_id, data)
         VALUES ($1, $2, $3, $4, 'harvest_oauth_client', $5, $6)",
        Uuid::now_v7(),
        org_id,
        actor_id,
        action,
        client_row,
        serde_json::json!({ "client_id": client_id, "disconnected": disconnected }),
    )
    .execute(&mut **tx)
    .await?;
    tracing::info!(%org_id, %actor_id, action, disconnected, "Harvest OAuth client changed");
    Ok(())
}

#[cfg(test)]
mod tests;
