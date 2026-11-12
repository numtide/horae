//! The current actor, loaded once per request inside the operation's
//! transaction and answering authorization questions from the person's stored
//! permission state.

use horae_core::permissions::catalog::Permission;

use super::{Caller, OpError, Tx};
use crate::models::permissions::PersonPermissions;
use crate::server_fns::PermissionStorageError;

/// The entry point a request came through. Tokens and the CLI join later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    Session,
}

#[derive(Debug)]
pub(crate) struct Actor {
    identity: horae_core::permissions::Actor,
    channel: Channel,
    /// The stored grants and administrator flag; `None` for a missing
    /// organization or a foreign or inactive person.
    permissions: Option<PersonPermissions>,
}

const STATE_UNAVAILABLE: &str = "Permission state is unavailable";

impl Actor {
    /// Locks the organization, then the person (`FOR SHARE`, in that order, as
    /// every access change does), and loads the permission state valid for the
    /// rest of the transaction. A concurrent access change either commits first
    /// and is seen here, or waits for this transaction to finish.
    ///
    /// A missing organization or a foreign or inactive person gives an actor
    /// with no authority, so each operation chooses whether that reads as not
    /// found or forbidden. Missing or invalid permission state fails closed
    /// with `Forbidden`; a storage error maps like any other.
    pub async fn load(tx: &mut Tx, caller: Caller) -> Result<Self, OpError> {
        let mut actor = Self {
            identity: horae_core::permissions::Actor {
                id: caller.user_id,
                org_id: caller.org_id,
                active: false,
            },
            channel: caller.channel,
            permissions: None,
        };
        let organization = sqlx::query_scalar!(
            "SELECT id FROM organizations WHERE id = $1 FOR SHARE",
            caller.org_id,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if organization.is_none() {
            return Ok(actor);
        }
        let person = sqlx::query_scalar!(
            "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
            caller.org_id,
            caller.user_id,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if person.is_none() {
            return Ok(actor);
        }
        let state = crate::server_fns::load_person_permissions(tx, caller.org_id, caller.user_id)
            .await
            .map_err(|error| match error {
                // A lost race or a storage fault, not missing authority.
                PermissionStorageError::Database(error) => OpError::from(error),
                error => {
                    tracing::error!(%error, "Invalid stored permission state");
                    OpError::forbidden(STATE_UNAVAILABLE)
                }
            })?
            .ok_or_else(|| OpError::forbidden(STATE_UNAVAILABLE))?;
        actor.identity.active = true;
        actor.permissions = Some(state);
        Ok(actor)
    }

    pub fn id(&self) -> uuid::Uuid {
        self.identity.id
    }

    pub fn org_id(&self) -> uuid::Uuid {
        self.identity.org_id
    }

    /// The identity `horae-core` scope checks (`AccessScope::covers`) take.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "foundation for the next ops/ domains")
    )]
    pub fn identity(&self) -> &horae_core::permissions::Actor {
        &self.identity
    }

    /// An active person of this organization; `false` means no authority.
    pub fn is_active(&self) -> bool {
        self.identity.active
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "foundation for the next ops/ domains")
    )]
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// Whether the actor's stored state grants `permission`.
    pub fn has(&self, permission: Permission) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.grants.contains(permission))
    }

    /// The stored administrator identity. Holding every grant does not make
    /// someone an administrator.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "foundation for the next ops/ domains")
    )]
    pub fn administrator(&self) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.is_administrator)
    }
}

#[cfg(test)]
mod tests;
