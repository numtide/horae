//! The current actor, loaded once per request inside the operation's
//! transaction and answering authorization questions from the person's stored
//! permission state.

use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::permissions::oauth::{Scope, ScopeSet};
use sqlx::PgConnection;

use horae_core::types::RoundDir;

use super::permissions::{PermissionStorageError, StoredPersonPermissions};
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
        /// The scope the endpoint declares, which the adapter checked the
        /// token holds ([`Caller::unlock`](super::Caller::unlock)): the actor
        /// loads already unlocked, with the grants of all the token's scopes.
        unlocked: Option<Scope>,
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
    /// The organization's settings, read with its lock; `None` for a missing
    /// organization.
    organization: Option<OrganizationSettings>,
    /// The stored grants and administrator flag; `None` for a missing
    /// organization or a foreign or inactive person.
    permissions: Option<PersonPermissions>,
    /// An OAuth actor's narrowed state, held back until
    /// [`Actor::unlock_scope`] moves it into `permissions`.
    locked: Option<PersonPermissions>,
    /// Whether the stored grants, before any narrowing, let the person
    /// track their own time. Read only through
    /// [`Actor::may_track_own_time`].
    tracks_own_time: bool,
}

/// The organization's settings as the actor's transaction sees them, so an
/// operation need not read the row again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationSettings {
    /// The workspace's IANA zone name.
    pub timezone: String,
    /// The first weekday, 1 (Monday) to 7 (Sunday).
    pub week_start: i16,
    pub default_currency: String,
    pub access_revision: i64,
    /// CHECK-constrained non-negative.
    pub round_minutes: i16,
    pub round_dir: RoundDir,
}

impl OrganizationSettings {
    /// The organization's today, for workspace-wide periods such as budget
    /// months.
    pub fn today(&self) -> chrono::NaiveDate {
        crate::timezone::today_in(&self.timezone, chrono::Utc::now())
    }
}

/// One row of the actor's load: the organization's settings, whether the
/// person is an active member, and their stored permission state, if any.
struct Authority {
    person_found: bool,
    timezone: String,
    week_start: i16,
    default_currency: String,
    access_revision: i64,
    round_minutes: i16,
    round_dir: RoundDir,
    catalog_version: Option<i32>,
    grants: Option<Vec<String>>,
    is_administrator: Option<bool>,
    source: Option<String>,
    built_in_profile: Option<String>,
    template_id: Option<uuid::Uuid>,
    applied_template_revision: Option<i64>,
    revision: Option<i64>,
}

impl Authority {
    fn organization(&self) -> OrganizationSettings {
        OrganizationSettings {
            timezone: self.timezone.clone(),
            week_start: self.week_start,
            default_currency: self.default_currency.clone(),
            access_revision: self.access_revision,
            round_minutes: self.round_minutes,
            round_dir: self.round_dir,
        }
    }

    /// The stored state; `None` when the person has none.
    fn permissions(self) -> Result<Option<PersonPermissions>, PermissionStorageError> {
        let (
            Some(catalog_version),
            Some(grants),
            Some(is_administrator),
            Some(source),
            Some(revision),
        ) = (
            self.catalog_version,
            self.grants,
            self.is_administrator,
            self.source,
            self.revision,
        )
        else {
            return Ok(None);
        };
        StoredPersonPermissions {
            catalog_version,
            grants,
            is_administrator,
            source,
            built_in_profile: self.built_in_profile,
            template_id: self.template_id,
            applied_template_revision: self.applied_template_revision,
            revision,
        }
        .restore()
        .map(Some)
    }
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
            organization: None,
            permissions: None,
            locked: None,
            tracks_own_time: false,
        };
        // Only the organization is locked for a channel with no person.
        let person_channel = matches!(
            caller.channel,
            Channel::Session | Channel::Token { .. } | Channel::OAuth { .. }
        );
        // One round trip. The locked load is `horae_actor_authority` (migration
        // 0017), which keeps the lock order and the per-statement snapshots of
        // separate statements; changing how authority is locked means changing
        // that function and the access writers, nothing here.
        let authority = match lock {
            Lock::Share => {
                sqlx::query_as!(
                    Authority,
                    r#"SELECT person_found AS "person_found!",
                              timezone AS "timezone!", week_start AS "week_start!",
                              default_currency AS "default_currency!",
                              access_revision AS "access_revision!",
                              round_minutes AS "round_minutes!",
                              round_dir AS "round_dir!: RoundDir",
                              catalog_version, grants, is_administrator, source,
                              built_in_profile, template_id, applied_template_revision, revision
                         FROM horae_actor_authority($1, $2, $3)"#,
                    caller.org_id,
                    caller.user_id,
                    person_channel,
                )
                .fetch_optional(&mut *connection)
                .await?
            }
            Lock::None => {
                sqlx::query_as!(
                    Authority,
                    r#"SELECT u.id IS NOT NULL AS "person_found!",
                              o.timezone, o.week_start, o.default_currency, o.access_revision,
                              o.round_minutes, o.round_dir::text AS "round_dir!: RoundDir",
                              s.catalog_version AS "catalog_version?", s.grants AS "grants?",
                              s.is_administrator AS "is_administrator?", s.source AS "source?",
                              s.built_in_profile, s.template_id,
                              s.applied_template_revision, s.revision AS "revision?"
                         FROM organizations o
                         LEFT JOIN users u
                           ON $3 AND u.org_id = o.id AND u.id = $2 AND u.active
                         LEFT JOIN person_permission_states s
                           ON s.org_id = o.id AND s.user_id = u.id
                        WHERE o.id = $1"#,
                    caller.org_id,
                    caller.user_id,
                    person_channel,
                )
                .fetch_optional(&mut *connection)
                .await?
            }
        };
        let Some(authority) = authority else {
            return Ok(actor);
        };
        actor.organization = Some(authority.organization());
        if !authority.person_found {
            return Ok(actor);
        }
        let mut state = authority
            .permissions()
            .map_err(|error| {
                tracing::error!(%error, "Invalid stored permission state");
                OpError::forbidden(STATE_UNAVAILABLE)
            })?
            .ok_or_else(|| OpError::forbidden(STATE_UNAVAILABLE))?;
        actor.tracks_own_time = state.grants.contains(Permission::TimeWriteOwn)
            || state.grants.contains(Permission::TimeWriteAll);
        if let Channel::OAuth {
            scopes, unlocked, ..
        } = caller.channel
        {
            // Scopes only narrow: whatever an operation checks, it checks
            // against a subset of the person's grants, and no scope covers
            // administration.
            state.grants = state.grants.intersection(&scopes.ceiling());
            state.is_administrator = false;
            actor.locked = Some(state);
            if let Some(scope) = unlocked {
                actor.unlock_scope(scope)?;
            }
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

    /// The organization's settings, read with its lock, which holds them
    /// unchanged until the transaction ends unless it changes them itself.
    /// Not found when the organization does not exist.
    pub fn organization(&self) -> Result<&OrganizationSettings, OpError> {
        self.organization
            .as_ref()
            .ok_or_else(|| OpError::not_found("Organization not found"))
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
    /// includes `time:read`). The authority is the person's grants within
    /// the ceiling of all the token's scopes, not only `scope`'s. Until then
    /// the actor answers like an inactive person. Every other channel is
    /// unaffected.
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

    /// Whether the person may track their own time, from their stored
    /// grants before an OAuth token's scopes narrowed them. Only the reads of
    /// what the person may track ask this, so an app with `catalog:read` sees
    /// the projects and tasks they track on (FR-060); every write checks the
    /// narrowed grants. `false` for an actor with no authority.
    pub fn may_track_own_time(&self) -> bool {
        self.is_active() && self.tracks_own_time
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
