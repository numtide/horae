use super::*;

/// Which person rates a caller may read.
#[derive(Clone, Copy)]
pub(super) struct RateView {
    pub billable: bool,
    pub cost: bool,
}

/// A caller's authority over the users endpoints, held in one transaction
/// with the reads it authorizes.
pub(super) struct UserAccess {
    pub read: ops::ReadTx,
    pub list: bool,
    pub rates: RateView,
}

pub(super) async fn begin(db: &PgPool, caller: &AuthUser) -> Result<UserAccess, ApiError> {
    let read = begin_read(db, caller, "User").await?;
    // Managed-only rate grants stay out of the API: honouring them would
    // need a per-row management lookup, and leaving them out is stricter.
    let people = read.actor.has(Permission::PeopleReadAll);
    let rates = RateView {
        billable: people && read.actor.has(Permission::BillableRateReadAll),
        cost: people && read.actor.has(Permission::CostRateReadAll),
    };
    Ok(UserAccess {
        read,
        list: people,
        rates,
    })
}
