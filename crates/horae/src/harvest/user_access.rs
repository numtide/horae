use super::*;
use crate::server_fns::ProjectReadAccess;
use horae_core::permissions::catalog::Permission;

/// Which person rates a caller may read.
#[derive(Clone, Copy)]
pub(super) struct RateView {
    pub billable: bool,
    pub cost: bool,
}

/// A caller's authority over the users endpoints under the active policy,
/// held in one transaction with the reads it authorizes.
pub(super) struct UserAccess<'a> {
    pub access: ProjectReadAccess<'a>,
    pub list: bool,
    pub rates: RateView,
}

pub(super) async fn begin<'a>(
    db: &'a PgPool,
    caller: &AuthUser,
) -> Result<UserAccess<'a>, ApiError> {
    let access = ProjectReadAccess::begin(db, caller.org_id, caller.user_id)
        .await
        .map_err(|error| match error {
            dioxus::prelude::ServerFnError::ServerError { code, .. }
                if code == StatusCode::FORBIDDEN.as_u16() =>
            {
                forbidden("User permission state is unavailable")
            }
            error => internal(error),
        })?
        .ok_or_else(|| forbidden("User access is unavailable"))?;
    let (list, rates) = if access.legacy() {
        let manager = caller.org_role.is_manager_or_above();
        let rates = RateView {
            billable: manager,
            cost: caller.org_role == OrgRole::Admin,
        };
        (manager, rates)
    } else {
        // Managed-only rate grants stay out of the API: honouring them would
        // need a per-row management lookup, and leaving them out is stricter.
        let people = access.has(Permission::PeopleReadAll);
        let rates = RateView {
            billable: people && access.has(Permission::BillableRateReadAll),
            cost: people && access.has(Permission::CostRateReadAll),
        };
        (people, rates)
    };
    Ok(UserAccess {
        access,
        list,
        rates,
    })
}
