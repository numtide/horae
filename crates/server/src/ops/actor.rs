//! The current actor, loaded once per request inside the operation's
//! transaction and answering authorization questions from the person's stored
//! permission state.

use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::permissions::oauth::{Scope, ScopeSet};
use sqlx::PgConnection;

use super::permissions::{PermissionStorageError, load_person_permissions};
use super::{Caller, OpError};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::permissions::PersonPermissions;

/// The entry point a request came through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// A signed-in person.
    Session,
    /// A person's personal access token: the person and their permissions,
    /// exactly as for a session.
    Token { token_id: uuid::Uuid },
    /// An app's OAuth access token: the person, with their grants narrowed to
    /// what the token's scopes let through and never the Administrator
    /// identity (spec 024 R-6). The actor has no authority at all until the
    /// adapter unlocks it for a scope the token holds
    /// ([`Actor::unlock_scope`]), so an operation reached without a declared
    /// scope refuses it like an inactive person.
    OAuth {
        grant_id: uuid::Uuid,
        token_id: uuid::Uuid,
        scopes: ScopeSet,
    },
    /// The server's operator at its terminal: no person, and no grants. Each
    /// operation that accepts it says what it may do.
    Operator,
    /// Horae acting on its own, such as an OIDC first sign-in: no person, and
    /// no grants. Each operation that accepts it says what it may do.
    System,
}

#[derive(Debug)]
pub struct Actor {
    identity: horae_core::permissions::Actor,
    channel: Channel,
    /// The stored grants and administrator flag; `None` for a missing
    /// organization or a foreign or inactive person.
    permissions: Option<PersonPermissions>,
    /// An OAuth actor's narrowed state, held back until
    /// [`Actor::unlock_scope`] moves it into `permissions`.
    locked: Option<PersonPermissions>,
}

const STATE_UNAVAILABLE: &str = "Permission state is unavailable";
const ADMINISTRATOR_REQUIRED: &str = "Active administrator access required";
const SESSION_REQUIRED: &str = "Sign in to the web app to do this";
const SCOPE_REQUIRED: &str = "The token's scopes do not cover this request";

#[derive(Clone, Copy)]
enum Lock {
    Share,
    None,
}

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
    pub async fn load(connection: &mut PgConnection, caller: Caller) -> Result<Self, OpError> {
        Self::load_with(connection, caller, Lock::Share).await
    }

    /// [`Actor::load`] without the locks: for refusing early, before the
    /// operation's own transaction checks again, or for a read that must not
    /// queue behind an access change. The answer holds only when it was read.
    pub async fn load_unlocked(
        connection: &mut PgConnection,
        caller: Caller,
    ) -> Result<Self, OpError> {
        Self::load_with(connection, caller, Lock::None).await
    }

    async fn load_with(
        connection: &mut PgConnection,
        caller: Caller,
        lock: Lock,
    ) -> Result<Self, OpError> {
        let mut actor = Self {
            identity: horae_core::permissions::Actor {
                id: caller.user_id,
                org_id: caller.org_id,
                active: false,
            },
            channel: caller.channel,
            permissions: None,
            locked: None,
        };
        let organization = match lock {
            Lock::Share => {
                sqlx::query_scalar!(
                    "SELECT id FROM organizations WHERE id = $1 FOR SHARE",
                    caller.org_id,
                )
                .fetch_optional(&mut *connection)
                .await?
            }
            Lock::None => {
                sqlx::query_scalar!("SELECT id FROM organizations WHERE id = $1", caller.org_id)
                    .fetch_optional(&mut *connection)
                    .await?
            }
        };
        // Only the organization is locked for a channel with no person.
        let person_channel = matches!(
            caller.channel,
            Channel::Session | Channel::Token { .. } | Channel::OAuth { .. }
        );
        if organization.is_none() || !person_channel {
            return Ok(actor);
        }
        let person = match lock {
            Lock::Share => {
                sqlx::query_scalar!(
                    "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
                    caller.org_id,
                    caller.user_id,
                )
                .fetch_optional(&mut *connection)
                .await?
            }
            Lock::None => {
                sqlx::query_scalar!(
                    "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active",
                    caller.org_id,
                    caller.user_id,
                )
                .fetch_optional(&mut *connection)
                .await?
            }
        };
        if person.is_none() {
            return Ok(actor);
        }
        let mut state = load_person_permissions(connection, caller.org_id, caller.user_id)
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
        if let Channel::OAuth { scopes, .. } = caller.channel {
            // Scopes only narrow: whatever an operation checks, it checks
            // against a subset of the person's grants, and no scope covers
            // administration.
            state.grants = state.grants.intersection(&scopes.ceiling());
            state.is_administrator = false;
            actor.locked = Some(state);
            return Ok(actor);
        }
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
    pub fn identity(&self) -> &horae_core::permissions::Actor {
        &self.identity
    }

    /// The identity a page reports back, so the client can tell a stale session.
    pub fn requester(&self) -> PermissionRequester {
        PermissionRequester {
            org_id: self.org_id(),
            user_id: self.id(),
        }
    }

    /// [`Actor::requester`], refused with `Forbidden(refusal)` when the page
    /// bound its request to someone else. `None` binds to no one.
    pub fn expect_requester(
        &self,
        expected: Option<PermissionRequester>,
        refusal: &str,
    ) -> Result<PermissionRequester, OpError> {
        expect(self.requester(), expected, refusal)
    }

    /// An active person of this organization; `false` means no authority.
    pub fn is_active(&self) -> bool {
        self.identity.active
    }

    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// The scopes of an OAuth access token; `None` for every other channel,
    /// which no scope limits.
    pub fn scopes(&self) -> Option<ScopeSet> {
        match self.channel {
            Channel::OAuth { scopes, .. } => Some(scopes),
            _ => None,
        }
    }

    /// Gives an OAuth actor its narrowed authority for a request whose
    /// endpoint declares `scope`, if the token holds it (`time:write`
    /// includes `time:read`). Until then the actor answers like an inactive
    /// person. Every other channel is unaffected.
    pub fn unlock_scope(&mut self, scope: Scope) -> Result<(), OpError> {
        let Channel::OAuth { scopes, .. } = self.channel else {
            return Ok(());
        };
        if !scopes.normalize().contains(scope) {
            return Err(OpError::forbidden(SCOPE_REQUIRED));
        }
        if let Some(state) = self.locked.take() {
            self.permissions = Some(state);
            self.identity.active = true;
        }
        Ok(())
    }

    /// For a write whose own guard would let any unlocked OAuth actor
    /// through: an OAuth caller needs `scope` in its token, whichever scope
    /// the endpoint unlocked. Every other channel is unaffected.
    pub fn require_scope(&self, scope: Scope) -> Result<(), OpError> {
        match self.channel {
            Channel::OAuth { scopes, .. } if !scopes.normalize().contains(scope) => {
                Err(OpError::forbidden(SCOPE_REQUIRED))
            }
            _ => Ok(()),
        }
    }

    /// Refuses an OAuth caller: no scope covers this write (FR-063).
    pub fn refuse_oauth(&self) -> Result<(), OpError> {
        match self.channel {
            Channel::OAuth { .. } => Err(OpError::forbidden(SCOPE_REQUIRED)),
            _ => Ok(()),
        }
    }

    /// Refuses anyone but a signed-in person: the guard for managing tokens,
    /// apps and authorizations, which only the web app does.
    pub fn require_session(&self) -> Result<(), OpError> {
        if self.channel == Channel::Session && self.is_active() {
            Ok(())
        } else {
            Err(OpError::forbidden(SESSION_REQUIRED))
        }
    }

    /// Whether the actor's stored state grants `permission`.
    pub fn has(&self, permission: Permission) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.grants.contains(permission))
    }

    /// The stored grants, for the `horae-core` record checks that take the
    /// whole selection; `None` for an actor with no authority.
    pub fn grants(&self) -> Option<&PermissionSelection> {
        self.permissions.as_ref().map(|state| &state.grants)
    }

    /// The whole stored state (grants, administrator identity, source and
    /// revision); `None` for an actor with no authority.
    pub fn permissions(&self) -> Option<&PersonPermissions> {
        self.permissions.as_ref()
    }

    /// The stored administrator identity. Holding every grant does not make
    /// someone an administrator.
    pub fn administrator(&self) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.is_administrator)
    }

    /// Refuses anyone but an active administrator: the guard for actions no
    /// grant covers, such as the Harvest importer and its jobs.
    pub fn require_administrator(&self) -> Result<(), OpError> {
        if self.administrator() {
            Ok(())
        } else {
            Err(OpError::forbidden(ADMINISTRATOR_REQUIRED))
        }
    }
}

fn expect(
    requester: PermissionRequester,
    expected: Option<PermissionRequester>,
    refusal: &str,
) -> Result<PermissionRequester, OpError> {
    if expected.is_some_and(|expected| expected != requester) {
        return Err(OpError::forbidden(refusal));
    }
    Ok(requester)
}

#[cfg(test)]
mod tests;
