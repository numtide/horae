use dioxus::core::CapturedError;
use dioxus::prelude::dioxus_fullstack::Loader;
use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use serde::{Deserialize, Serialize};

use crate::components::screen_state::PageFailed;
use crate::models::own_permissions::OwnPermissions;
use crate::models::user::CurrentUser;
use crate::server_fns;

/// The signed-in user, their permissions and their organization's settings,
/// loaded once by the app shell and read by every page that renders
/// differently for them.
///
/// This is a display copy: every server function authorizes afresh, so a stale
/// value can only show or hide UI, never grant access. Code that binds browser
/// storage to the identity calls `get_me` itself instead, because this copy would
/// not see a user switched in another tab: invoice recovery on each gate mount,
/// permission recovery on each editor mount and whenever a person is opened.
#[derive(Clone, Copy)]
pub struct Session {
    pub me: Memo<CurrentUser>,
    /// The answer as it came, refusals included: a person without stored
    /// permission state, or one the server cannot read, still gets the shell,
    /// and each screen explains what it cannot show.
    pub permissions: Memo<Result<OwnPermissions, ServerFnError>>,
    /// The organization's first day of the week.
    pub week_start: Memo<chrono::Weekday>,
    pub org_name: Memo<String>,
    /// Whether a re-read is running, so "Try again" can wait for it.
    pub refreshing: Signal<bool>,
    loaded: Loader<Loaded>,
}

/// What the session reads, as one value so one `use_loader` carries it.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Loaded {
    me: CurrentUser,
    permissions: Result<OwnPermissions, ServerFnError>,
    week_start: chrono::Weekday,
    org_name: String,
}

impl Session {
    /// Re-read the session, after anything that may have changed the user's
    /// own access or profile. The values stay until the new ones arrive.
    pub fn refresh(mut self) {
        self.loaded.restart();
    }

    /// Whether to offer project creation, by the same rule the projects page
    /// uses: the `ProjectCreateAll` grant.
    pub fn can_create_projects(&self) -> bool {
        matches!(
            &*self.permissions.read(),
            Ok(access) if access.catalog_version == PERMISSION_CATALOG_VERSION
                && access.grants.contains(&Permission::ProjectCreateAll)
        )
    }
}

/// The error the session raises when the server no longer knows the person
/// (401): the session expired, or the account was deactivated. It crosses
/// hydration as text, so the error boundary compares the text.
pub const SESSION_ENDED: &str = "The session has ended";
const UNAUTHORIZED: u16 = 401;

/// Call once, above every component that reads the session, and propagate
/// with `?`. The server render waits for the session and sends it with the
/// HTML, so the first page already shows the signed-in shell. A failed read
/// goes to the nearest `ErrorBoundary`.
pub fn use_session_provider() -> Result<(), RenderError> {
    let mut refreshing = use_signal(|| false);
    // One loader for all four reads: while the server render waits, Dioxus
    // polls only the task it is suspended on, so a second loader started
    // beside it would never be woken.
    let loaded = use_loader(move || async move {
        refreshing.set(true);
        let (me, permissions, week_start, org_name) = futures_util::join!(
            server_fns::get_me(),
            server_fns::get_my_permissions(),
            server_fns::get_week_start(),
            server_fns::get_org_name(),
        );
        refreshing.set(false);
        let me = me.map_err(|error| match error {
            ServerFnError::ServerError {
                code: UNAUTHORIZED, ..
            } => CapturedError::from_display(SESSION_ENDED),
            error => error.into(),
        });
        Ok::<_, CapturedError>(Loaded {
            me: me?,
            permissions,
            week_start: week_start?,
            org_name: org_name?,
        })
    })?;
    let me = use_memo(move || loaded.read().me.clone());
    let permissions = use_memo(move || loaded.read().permissions.clone());
    let week_start = use_memo(move || loaded.read().week_start);
    let org_name = use_memo(move || loaded.read().org_name.clone());
    use_context_provider(|| Session {
        me,
        permissions,
        week_start,
        org_name,
        refreshing,
        loaded,
    });
    Ok(())
}

/// The app's `ErrorBoundary` handler: the page error, or "Session ended"
/// when the session raised [`SESSION_ENDED`].
pub fn page_failed(errors: ErrorContext) -> Element {
    let ended = errors
        .error()
        .is_some_and(|error| error.to_string() == SESSION_ENDED);
    rsx! {
        main { id: "app-main", class: "p-5", PageFailed { ended } }
    }
}

/// The shared session. Panics if no provider is above the caller, which would
/// be a wiring mistake rather than a runtime condition.
pub fn use_session() -> Session {
    use_context::<Session>()
}

/// The organization's first day of the week.
pub fn use_week_start() -> chrono::Weekday {
    *use_session().week_start.read()
}
