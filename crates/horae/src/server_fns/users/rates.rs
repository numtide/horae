//! A person's default billable and cost rates: session adapters over
//! `ops::people::rates`.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::user::PersonRates;
use horae_core::permissions::rates::RateEdit;

/// Read a person's rates, each withheld unless the viewer may read it.
#[server]
pub async fn load_person_rates(
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
) -> Result<PersonRates, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    read_person_rates(&state.db, caller, person_id, expected_requester).await
}

/// Change a person's rates. `currency` is the one the form showed, so an
/// amount typed for another currency is refused rather than reinterpreted.
#[server]
pub async fn update_person_rates(
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
    currency: String,
    billable: RateEdit,
    cost: RateEdit,
) -> Result<PersonRates, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    write_person_rates(
        &state.db,
        &*state.plugins,
        caller,
        person_id,
        expected_requester,
        &currency,
        billable,
        cost,
    )
    .await
}

#[cfg(feature = "server")]
pub(crate) async fn read_person_rates(
    db: &sqlx::PgPool,
    caller: Caller,
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
) -> Result<PersonRates, ServerFnError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result =
        ops::people::rates::load(&mut read.tx, &read.actor, person_id, expected_requester).await;
    Ok(read.finish(result).await?)
}

#[cfg(feature = "server")]
#[expect(
    clippy::too_many_arguments,
    reason = "a session adapter passes each server function argument through"
)]
pub(crate) async fn write_person_rates(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
    currency: &str,
    billable: RateEdit,
    cost: RateEdit,
) -> Result<PersonRates, ServerFnError> {
    let mut write = ops::begin(db, caller).await?;
    let result = ops::people::rates::update(
        &mut write.tx,
        &write.actor,
        person_id,
        expected_requester,
        currency,
        billable,
        cost,
    )
    .await;
    Ok(write.commit(result, events).await?)
}
