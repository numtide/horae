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
    GrantApproved,
    GrantRevoked,
    RefreshReused,
    CodeReused,
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
            Self::GrantApproved => "grant_approved",
            Self::GrantRevoked => "grant_revoked",
            Self::RefreshReused => "refresh_reused",
            Self::CodeReused => "code_reused",
        }
    }

    const fn entity_type(self) -> &'static str {
        match self {
            Self::GrantApproved | Self::GrantRevoked | Self::RefreshReused | Self::CodeReused => {
                "oauth_grant"
            }
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
    let person = actor.is_active().then(|| actor.id());
    record_for(tx, actor.org_id(), person, action, entity_id, data).await
}

/// [`record`] for what an app or Horae did at the token or revocation
/// endpoint, where no person acts.
pub(crate) async fn record_for(
    tx: &mut Tx,
    org_id: Uuid,
    person: Option<Uuid>,
    action: Action,
    entity_id: Uuid,
    data: serde_json::Value,
) -> Result<(), OpError> {
    sqlx::query!(
        "INSERT INTO audit_log (id, org_id, actor_user_id, action, entity_type, entity_id, data)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        Uuid::now_v7(),
        org_id,
        person,
        action.as_str(),
        action.entity_type(),
        entity_id,
        data,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
