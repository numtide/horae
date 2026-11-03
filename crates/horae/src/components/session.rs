use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

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
}

impl Session {
    /// Re-read both, after anything that may have changed the user's own access.
    pub fn refresh(mut self) {
        self.me.restart();
        self.permissions.restart();
    }

    /// Whether to offer project creation, by the same rule the projects page
    /// uses: the canonical grant, or manager and above under legacy roles.
    pub fn can_create_projects(&self) -> bool {
        match &*self.permissions.read() {
            Some(Ok(Some(access))) => {
                access.catalog_version == PERMISSION_CATALOG_VERSION
                    && access.grants.contains(&Permission::ProjectCreateAll)
            }
            Some(Ok(None)) => matches!(&*self.me.read(), Some(Ok(me)) if me.is_manager_or_above()),
            _ => false,
        }
    }
}

/// Call once, above every component that reads the session.
pub fn use_session_provider() {
    let me = use_resource(server_fns::get_me);
    let permissions = use_resource(server_fns::get_my_permissions);
    use_context_provider(|| Session { me, permissions });
}

/// The shared session. Panics if no provider is above the caller, which would
/// be a wiring mistake rather than a runtime condition.
pub fn use_session() -> Session {
    use_context::<Session>()
}
