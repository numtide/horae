//! The `audit_log` rows of OAuth (research R-15): names, scopes and reasons
//! in `data`, never a credential.

use uuid::Uuid;

use crate::ops::{Actor, OpError, Tx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    ClientRegistered,
    ClientUpdated,
    ClientEnabled,
    ClientDisabled,
    ClientDeleted,
    ClientSecretReplaced,
    GrantRevoked,
}

impl Action {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ClientRegistered => "client_registered",
            Self::ClientUpdated => "client_updated",
            Self::ClientEnabled => "client_enabled",
            Self::ClientDisabled => "client_disabled",
            Self::ClientDeleted => "client_deleted",
            Self::ClientSecretReplaced => "client_secret_replaced",
            Self::GrantRevoked => "grant_revoked",
        }
    }

    const fn entity_type(self) -> &'static str {
        match self {
            Self::GrantRevoked => "oauth_grant",
            _ => "oauth_client",
        }
    }
}

pub(crate) async fn record(
    tx: &mut Tx,
    actor: &Actor,
    action: Action,
    entity_id: Uuid,
    data: serde_json::Value,
) -> Result<(), OpError> {
    sqlx::query!(
        "INSERT INTO audit_log (id, org_id, actor_user_id, action, entity_type, entity_id, data)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        Uuid::now_v7(),
        actor.org_id(),
        actor.is_active().then(|| actor.id()),
        action.as_str(),
        action.entity_type(),
        entity_id,
        data,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
