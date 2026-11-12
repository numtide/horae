//! User server functions: session adapters over `ops::people`, plus the
//! people directory.

use super::*;
use crate::models::people::{PeopleActivity, PeopleCursor, PeoplePage};
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
#[cfg(feature = "server")]
use crate::ops::{self, Caller, people::Existing};
#[cfg(feature = "server")]
use horae_core::permissions::catalog::BuiltInProfile;
#[cfg(feature = "server")]
use permissions::provisioning;

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
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate directory request");
            server_err("People directory is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::directory::read(&state.db, user.org_id, user.id, activity, after.as_ref())
        .await
        .map_err(|error| match error {
            permissions::directory::DirectoryError::Forbidden => {
                forbidden("Current people-read authority is required")
            }
            permissions::directory::DirectoryError::InvalidCursor => {
                err(BAD_REQUEST, "Invalid directory cursor")
            }
            error => {
                tracing::error!(%error, "People directory read failed");
                server_err("People directory is unavailable")
            }
        })
}

/// List identities for the directory and workflow pickers, without financial
/// or authentication metadata. Any active person can list active people;
/// including archived people requires permission to view everyone.
#[server]
pub async fn list_users(include_inactive: bool) -> Result<Vec<UserListItem>, ServerFnError> {
    let viewer = require_user().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(viewer.org_id, viewer.id)).await?;
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
) -> Result<User, ServerFnError> {
    let user = require_user().await?;
    let profile = provisioning::parse_profile(&profile)
        .ok_or_else(|| err(BAD_REQUEST, "Choose one of the built-in profiles"))?;
    let state = crate::state::global_state().await;
    insert_user(
        &state.db,
        &*state.plugins,
        Caller::session(user.org_id, user.id),
        &email,
        &name,
        profile,
    )
    .await
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
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    add_people(
        &state.db,
        &*state.plugins,
        Caller::session(user.org_id, user.id),
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
pub async fn set_user_active(user_id: String, active: bool) -> Result<User, ServerFnError> {
    let user = require_user().await?;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let state = crate::state::global_state().await;
    change_user_active(
        &state.db,
        &*state.plugins,
        Caller::session(user.org_id, user.id),
        user_id,
        active,
    )
    .await
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
) -> Result<User, ServerFnError> {
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
) -> Result<User, ServerFnError> {
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
