//! User server functions: session adapters over `ops::people`.

use super::*;
use crate::models::people::{PeopleActivity, PeopleCursor, PeoplePage};
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
#[cfg(feature = "server")]
use crate::ops::permissions::provisioning;
#[cfg(feature = "server")]
use crate::ops::{self, Caller, people::Existing};
#[cfg(feature = "server")]
use horae_core::permissions::catalog::BuiltInProfile;

mod rates;
pub use rates::{load_person_rates, update_person_rates};

// ── Users ─────────────────────────────────────────────────────────────────────

/// Read a bounded people-directory page under current canonical permissions.
/// This is not a report/approval identity resolver or project assignment picker.
#[server]
pub async fn list_people(
    activity: PeopleActivity,
    after: Option<PeopleCursor>,
) -> Result<PeoplePage, ServerFnError> {
    let caller = session_caller_or("People directory is unavailable").await?;
    let state = crate::state::global_state().await;
    Ok(people_page(&state.db, caller, activity, after.as_ref()).await?)
}

#[cfg(feature = "server")]
pub(crate) async fn people_page(
    db: &sqlx::PgPool,
    caller: Caller,
    activity: PeopleActivity,
    after: Option<&PeopleCursor>,
) -> Result<PeoplePage, ops::OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = ops::people::directory::page(&mut read.tx, &read.actor, activity, after).await;
    read.finish(result).await
}

/// List identities for the directory and workflow pickers, without financial
/// or authentication metadata. Any active person can list active people;
/// including archived people requires permission to view everyone.
#[server]
pub async fn list_users(include_inactive: bool) -> Result<Vec<UserListItem>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::people::list(&mut read.tx, &read.actor, include_inactive).await;
    Ok(read.finish(result).await?)
}

/// Add a person with a built-in profile, named as stored (`member`,
/// `project_manager`, …).
#[server]
pub async fn create_user(
    email: String,
    name: String,
    profile: String,
) -> Result<UserListItem, ServerFnError> {
    let caller = session_caller().await?;
    let profile = provisioning::parse_profile(&profile)
        .ok_or_else(|| err(BAD_REQUEST, "Choose one of the built-in profiles"))?;
    let state = crate::state::global_state().await;
    insert_user(&state.db, &*state.plugins, caller, &email, &name, profile).await
}

/// Add members, deactivated unless the caller chose otherwise. The Harvest run
/// page uses this for people a run could not match. An email that already has
/// an account (added meanwhile, or in another tab) is reported as present
/// rather than failing the rest.
#[server]
pub async fn add_harvest_people(
    people: Vec<NewPerson>,
    activation: Activation,
) -> Result<AddedPeople, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    add_people(
        &state.db,
        &*state.plugins,
        caller,
        &people,
        BuiltInProfile::Member,
        activation,
        Existing::Skip,
    )
    .await
}

/// Archive or restore a person. Archived people cannot sign in but their
/// time entries are kept (FR-002).
#[server]
pub async fn set_user_active(user_id: String, active: bool) -> Result<UserListItem, ServerFnError> {
    let caller = session_caller().await?;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let state = crate::state::global_state().await;
    change_user_active(&state.db, &*state.plugins, caller, user_id, active).await
}

/// Add one active person, refusing an email already in use.
#[cfg(feature = "server")]
pub(crate) async fn insert_user(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    email: &str,
    name: &str,
    profile: BuiltInProfile,
) -> Result<UserListItem, ServerFnError> {
    let person = NewPerson {
        email: email.to_owned(),
        name: name.to_owned(),
    };
    let mut added = add_people(
        db,
        events,
        caller,
        &[person],
        profile,
        Activation::Active,
        Existing::Refuse,
    )
    .await?;
    Ok(added.added.remove(0))
}

#[cfg(feature = "server")]
async fn add_people(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    people: &[NewPerson],
    profile: BuiltInProfile,
    activation: Activation,
    existing: Existing,
) -> Result<AddedPeople, ServerFnError> {
    let mut write = ops::begin(db, caller).await?;
    let result = ops::people::add(
        &mut write.tx,
        &write.actor,
        people,
        profile,
        activation,
        existing,
    )
    .await;
    Ok(write.commit(result, events).await?)
}

#[cfg(feature = "server")]
pub(crate) async fn change_user_active(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    user_id: uuid::Uuid,
    active: bool,
) -> Result<UserListItem, ServerFnError> {
    let mut write = ops::begin(db, caller).await?;
    let result = ops::people::set_active(&mut write.tx, &write.actor, user_id, active).await;
    Ok(write.commit(result, events).await?)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    mod authority;
    mod concurrency;
    mod people;
    mod rates;
}
