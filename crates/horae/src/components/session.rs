use dioxus::prelude::*;

use crate::models::own_permissions::OwnPermissions;
use crate::models::user::CurrentUser;
use crate::server_fns;

/// The signed-in user and their permissions, fetched once by the app shell and
/// read by every page that renders differently for them.
///
/// This is a display copy: every server function authorizes afresh, so a stale
/// value can only show or hide UI, never grant access. Code that binds browser
/// storage to the identity calls `get_me` itself instead, because this copy would
/// not see a user switched in another tab: invoice recovery on each gate mount,
/// permission recovery on each editor mount and whenever a person is opened.
#[derive(Clone, Copy)]
pub struct Session {
    pub me: Resource<Result<CurrentUser, ServerFnError>>,
    pub permissions: Resource<Result<Option<OwnPermissions>, ServerFnError>>,
    /// Read through [`use_week_start`], which starts the fetch.
    week_start: Resource<Result<chrono::Weekday, ServerFnError>>,
    week_start_wanted: Signal<bool>,
}

impl Session {
    /// Re-read the user and their permissions, after anything that may have
    /// changed the user's own access.
    pub fn refresh(mut self) {
        self.me.restart();
        self.permissions.restart();
        self.retry_failed_week_start();
    }

    /// The week start never changes during a visit, so only a failed read is
    /// worth repeating; without this one error would last until a reload.
    fn retry_failed_week_start(mut self) {
        if matches!(*self.week_start.peek(), Some(Err(_))) {
            self.week_start.restart();
        }
    }
}

/// Call once, above every component that reads the session.
pub fn use_session_provider() {
    let me = use_resource(server_fns::get_me);
    let permissions = use_resource(server_fns::get_my_permissions);
    // Only the week-based pages need it, so it waits for the first of them
    // and is then kept for the rest of the visit.
    let week_start_wanted = use_signal(|| false);
    let week_start = use_resource(move || {
        let wanted = week_start_wanted();
        async move {
            if !wanted {
                std::future::pending::<()>().await;
            }
            server_fns::get_week_start().await
        }
    });
    use_context_provider(|| Session {
        me,
        permissions,
        week_start,
        week_start_wanted,
    });
}

/// The shared session. Panics if no provider is above the caller, which would
/// be a wiring mistake rather than a runtime condition.
pub fn use_session() -> Session {
    use_context::<Session>()
}

/// The organization's first day of the week, fetched on first use and kept,
/// unless it failed: then each new reader tries again.
pub fn use_week_start() -> Resource<Result<chrono::Weekday, ServerFnError>> {
    let session = use_session();
    let mut wanted = session.week_start_wanted;
    use_hook(|| {
        if !*wanted.peek() {
            wanted.set(true);
        } else {
            session.retry_failed_week_start();
        }
    });
    session.week_start
}
