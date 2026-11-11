//! The current actor, loaded once per request inside the operation's
//! transaction and answering authorization questions under both permission
//! policies until policy zero is deleted.

use horae_core::permissions::catalog::Permission;
use horae_core::types::OrgRole;

use super::{Caller, OpError, Tx};
use crate::models::permissions::PersonPermissions;

/// The entry point a request came through. Tokens and the CLI join later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    Session,
}

/// Where the actor's authority comes from.
#[derive(Debug)]
enum Authority {
    /// Missing organization, foreign or inactive person: no authority at all.
    None,
    /// Policy zero: the legacy `users.org_role`.
    Legacy(OrgRole),
    /// Policy one: the stored permission state (grants and administrator flag).
    Scoped(PersonPermissions),
}

#[derive(Debug)]
pub(crate) struct Actor {
    identity: horae_core::permissions::Actor,
    channel: Channel,
    authority: Authority,
}

const STATE_UNAVAILABLE: &str = "Permission state is unavailable";

impl Actor {
    /// Locks the organization, then the person (`FOR SHARE`, in that order, as
    /// every access change does), and loads the authority valid for the rest of
    /// the transaction. A concurrent access change either commits first and is
    /// seen here, or waits for this transaction to finish.
    ///
    /// A missing organization or a foreign or inactive person gives an actor
    /// with no authority, so each operation chooses whether that reads as not
    /// found or forbidden. Unknown policies and missing or invalid permission
    /// state fail closed with `Forbidden`.
    pub async fn load(tx: &mut Tx, caller: Caller) -> Result<Self, OpError> {
        let none = |caller: Caller| Self {
            identity: horae_core::permissions::Actor {
                id: caller.user_id,
                org_id: caller.org_id,
                active: false,
            },
            channel: caller.channel,
            authority: Authority::None,
        };
        let Some(policy) = sqlx::query_scalar!(
            "SELECT permission_policy_version FROM organizations WHERE id = $1 FOR SHARE",
            caller.org_id,
        )
        .fetch_optional(&mut **tx)
        .await?
        else {
            return Ok(none(caller));
        };
        if !matches!(policy, 0 | 1) {
            return Err(OpError::forbidden(STATE_UNAVAILABLE));
        }
        let Some(role) = sqlx::query_scalar!(
            r#"SELECT org_role AS "org_role: OrgRole" FROM users
               WHERE org_id = $1 AND id = $2 AND active FOR SHARE"#,
            caller.org_id,
            caller.user_id,
        )
        .fetch_optional(&mut **tx)
        .await?
        else {
            return Ok(none(caller));
        };
        let authority = if policy == 0 {
            Authority::Legacy(role)
        } else {
            let state =
                crate::server_fns::load_person_permissions(tx, caller.org_id, caller.user_id)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Unable to load the actor's permission state");
                        OpError::forbidden(STATE_UNAVAILABLE)
                    })?
                    .ok_or_else(|| OpError::forbidden(STATE_UNAVAILABLE))?;
            Authority::Scoped(state)
        };
        Ok(Self {
            identity: horae_core::permissions::Actor {
                id: caller.user_id,
                org_id: caller.org_id,
                active: true,
            },
            channel: caller.channel,
            authority,
        })
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

    /// Whether the organization is still on policy zero (legacy roles).
    pub fn is_legacy(&self) -> bool {
        matches!(self.authority, Authority::Legacy(_))
    }

    /// The legacy role under policy zero, for a rule `has` cannot express.
    /// Goes away with policy zero.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "foundation for the next ops/ domains")
    )]
    pub fn legacy_role(&self) -> Option<OrgRole> {
        match self.authority {
            Authority::Legacy(role) => Some(role),
            _ => None,
        }
    }

    /// Whether the actor holds `permission`: the stored grant under policy one,
    /// the legacy role's equivalent (see [`legacy_has`]) under policy zero.
    pub fn has(&self, permission: Permission) -> bool {
        match &self.authority {
            Authority::None => false,
            Authority::Legacy(role) => legacy_has(*role, permission),
            Authority::Scoped(state) => state.grants.contains(permission),
        }
    }

    /// The administrator identity: the legacy Admin role, or the stored flag.
    /// Holding every grant does not make someone an administrator.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "foundation for the next ops/ domains")
    )]
    pub fn administrator(&self) -> bool {
        match &self.authority {
            Authority::None => false,
            Authority::Legacy(role) => *role == OrgRole::Admin,
            Authority::Scoped(state) => state.is_administrator,
        }
    }
}

/// What each legacy role could do, expressed as grants, from the inventory in
/// `specs/015-scoped-permissions/contracts/current-access.md`. It is the
/// policy-zero answer of [`Actor::has`] and is exact for the clients domain;
/// before relying on a row for another domain, check it against that domain's
/// current legacy code and correct the row here, or branch on
/// [`Actor::legacy_role`] where one grant cannot express the old rule.
pub(crate) fn legacy_has(role: OrgRole, permission: Permission) -> bool {
    use Permission::*;
    match role {
        OrgRole::Admin => true,
        OrgRole::Manager => match permission {
            // Own time, everyone's time in reports and approvals, but no
            // editing of other people's timesheets. Expenses mirror time.
            TimeReadOwn
            | TimeWriteOwn
            | TimeReadManaged
            | TimeReadAll
            | TimeApproveManaged
            | TimeApproveAll
            | ApprovalWithdrawManaged => true,
            TimeWriteManaged | TimeWriteAll => false,
            ExpenseReadOwn | ExpenseWriteOwn | ExpenseReadManaged | ExpenseReadAll => true,
            ExpenseWriteManaged | ExpenseWriteAll => false,
            // Organization-wide project, client and task management.
            ProjectReadManaged | ProjectWriteManaged | ProjectCreateAll | ProjectReadAll
            | ProjectWriteAll | ClientReadAll | ClientWriteAll | TaskReadAll | TaskWriteAll => true,
            // Billable rates (project and client defaults) and invoices, but
            // not costs or people administration.
            BillableRateReadManaged
            | BillableRateWriteManaged
            | BillableRateReadAll
            | BillableRateWriteAll
            | InvoiceReadManaged
            | InvoiceDraftWriteManaged
            | InvoiceWriteManaged
            | InvoiceReadAll
            | InvoiceWriteAll => true,
            CostRateReadAll | CostRateWriteAll => false,
            PeopleReadManaged | PeopleWriteManaged | PeopleReadAll | PeopleWriteAll => false,
            ReportProfitabilityRead | ReportContractorRead | ReportInvoicingRead => true,
            EstimateReadAll
            | EstimateWriteAll
            | SavedReportReadInactive
            | SavedReportWriteInactive
            | CompanyRead
            | CompanyWrite
            | BillingRead
            | BillingWrite => false,
        },
        // Own time, plus the client catalog every active person could read.
        OrgRole::Member => matches!(
            permission,
            TimeReadOwn | TimeWriteOwn | ExpenseReadOwn | ExpenseWriteOwn | ClientReadAll
        ),
    }
}

#[cfg(test)]
mod tests;
