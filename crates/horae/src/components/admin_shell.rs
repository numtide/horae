use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::page_header::PageHeader;
use crate::components::screen_state::{ErrorState, Loading};
use crate::components::session::use_session;
use crate::models::own_permissions::OwnPermissions;
use crate::route::Route;
use crate::server_fns;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Settings,
    Audit,
    Importers,
    ApiTokens,
}

impl Section {
    /// The section page's own title, for the header of its no-access state.
    fn title(self) -> &'static str {
        match self {
            Section::Settings => "Workspace settings",
            Section::Audit => "Audit log",
            Section::Importers => "Importers",
            Section::ApiTokens => "API tokens",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorkspaceAccess {
    /// `CompanyWrite`, which `set_workspace_timezone` requires.
    settings: bool,
    audit: bool,
    importers: bool,
    api_tokens: bool,
}

impl WorkspaceAccess {
    fn allows(self, section: Section) -> bool {
        match section {
            Section::Settings => self.settings,
            Section::Audit => self.audit,
            Section::Importers => self.importers,
            Section::ApiTokens => self.api_tokens,
        }
    }
}

/// Which workspace sections to offer, from the session's stored permissions;
/// `None` until they have loaded and `Err` when access cannot be determined.
/// These are navigation hints; every reader/command authorizes afresh.
fn workspace_access(
    own: Option<&Result<OwnPermissions, ServerFnError>>,
) -> Option<Result<WorkspaceAccess, ()>> {
    Some(match own? {
        Ok(own) if own.catalog_version == PERMISSION_CATALOG_VERSION => Ok(WorkspaceAccess {
            settings: own.grants.contains(&Permission::CompanyWrite),
            audit: own.is_administrator,
            importers: own.is_administrator,
            api_tokens: own.is_administrator,
        }),
        _ => Err(()),
    })
}

/// The first section the person may open, in sub-navigation order, so an entry
/// into the admin area never lands on "Access unavailable". `None` while loading
/// or when no section is open.
pub(crate) fn first_admin_route(
    own: Option<&Result<OwnPermissions, ServerFnError>>,
) -> Option<Route> {
    let access = workspace_access(own)?.ok()?;
    [
        (access.settings, Route::WorkspaceSettings {}),
        (access.importers, Route::ImporterIndex {}),
        (access.audit, Route::PermissionAudit {}),
        (access.api_tokens, Route::AdminApiTokens {}),
    ]
    .into_iter()
    .find_map(|(allowed, route)| allowed.then_some(route))
}

/// The admin area shell: a secondary sub-navigation (Workspace / Data) beside the
/// active admin panel, rendered through an `Outlet`. Layered inside `AppLayout`,
/// so the main rail stays put and this owns only the content panel (per the
/// design's admin settings shell). Each section has its own access boundary.
///
/// Only sections with a real destination are listed — Settings, API tokens,
/// Importers (Harvest), and the permission audit log. People and Tasks have
/// their own pages in the main rail, as in Harvest. Other sections are deferred
/// until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let section = match use_route::<Route>() {
        Route::WorkspaceSettings {} => Section::Settings,
        Route::PermissionAudit {} => Section::Audit,
        Route::AdminApiTokens {} => Section::ApiTokens,
        _ => Section::Importers,
    };
    let session = use_session();
    let retry = use_callback(move |_| session.refresh());
    // Mount the workspace and its outlet once the session has loaded. A refresh
    // keeps a resolved answer until the new one arrives, so switching sections
    // or re-reading permissions never unmounts the open panel; server functions
    // still independently enforce access to every operation. A retry after a
    // failure shows as pending rather than repeating the old error.
    let refreshing = session.permissions.state()() != UseResourceState::Ready;
    let access = workspace_access(session.permissions.read().as_ref())
        .filter(|access| access.is_ok() || !refreshing);
    match access {
        Some(Ok(access)) if access.allows(section) => {
            rsx! { AdminWorkspace { access } }
        }
        // Non-disclosing: it names who can grant access, not what the section holds.
        Some(Ok(_)) => rsx! {
            PageHeader { title: section.title() }
            ErrorState {
                title: "Access unavailable",
                body: "Your account cannot view this workspace section. Ask an Administrator if you need it.",
            }
        },
        Some(Err(())) => rsx! {
            ErrorState {
                title: "Could not verify workspace access",
                body: "Try again, or sign in again if it keeps failing.",
                onretry: retry,
            }
        },
        None => rsx! {
            Loading {}
        },
    }
}

#[component]
fn AdminWorkspace(access: WorkspaceAccess) -> Element {
    // The workspace's real name for the header chip (no slug — the schema has no
    // such field, so we show the name only rather than inventing a URL).
    let org = use_resource(|| async move { server_fns::get_org_name().await });
    let org_name = match &*org.read() {
        Some(Ok(name)) => name.clone(),
        _ => "Workspace".to_string(),
    };
    let org_initial = org_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "·".to_string());

    rsx! {
        div {
            div { class: "flex flex-wrap items-start gap-12",
                aside { class: "adm-nav",
                    div { class: "adm-head flex items-center gap-3",
                        span { class: "adm-head-mark", "{org_initial}" }
                        div { class: "min-w-0",
                            div { class: "text-sm font-semibold truncate mb-1", "{org_name}" }
                            if access.audit { span { class: "badge badge-info badge-sm", "Administrator" } }
                        }
                    }

                    if access.settings || access.api_tokens {
                        div { class: "adm-group-label", "Workspace" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.settings { AdmLink { to: Route::WorkspaceSettings {}, label: "Settings" } }
                            if access.api_tokens { AdmLink { to: Route::AdminApiTokens {}, label: "API tokens" } }
                        }
                    }

                    if access.importers || access.audit {
                        div { class: "adm-group-label", "Data" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.importers { AdmLink { to: Route::ImporterIndex {}, label: "Importers" } }
                            if access.audit { AdmLink { to: Route::PermissionAudit {}, label: "Audit log" } }
                        }
                    }
                }
                div { class: "adm-main flex-1 min-w-0",
                    Outlet::<Route> {}
                }
            }
        }
    }
}

/// One sub-nav row: a client-side `Link` that marks itself active for its route
/// variant (matching the rail's `SideLink` behaviour).
#[component]
fn AdmLink(to: Route, label: String) -> Element {
    let active = crate::route::route_is_active(&to);
    rsx! {
        Link { to, class: if active { "adm-link active" } else { "adm-link" }, "{label}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(
        grants: Vec<Permission>,
        is_administrator: bool,
        catalog_version: u32,
    ) -> Result<OwnPermissions, ServerFnError> {
        Ok(OwnPermissions {
            catalog_version,
            grants,
            is_administrator,
            profile: None,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        })
    }

    /// Access without workspace settings, which only `CompanyWrite` opens.
    fn access(audit: bool, importers: bool) -> WorkspaceAccess {
        WorkspaceAccess {
            settings: false,
            audit,
            importers,
            // Like importers, API tokens follow the administrator identity.
            api_tokens: importers,
        }
    }

    #[test]
    fn waits_for_the_permissions_to_load() {
        assert_eq!(workspace_access(None), None);
    }

    #[test]
    fn grants_and_the_administrator_identity_choose_the_sections() {
        let administrator = canonical(
            vec![Permission::PeopleReadManaged, Permission::TaskReadAll],
            true,
            PERMISSION_CATALOG_VERSION,
        );
        assert_eq!(
            workspace_access(Some(&administrator)),
            Some(Ok(access(true, true)))
        );
        // People is a main screen, so reading people opens no admin section.
        let people = canonical(
            vec![Permission::PeopleReadAll],
            false,
            PERMISSION_CATALOG_VERSION,
        );
        assert_eq!(
            workspace_access(Some(&people)),
            Some(Ok(access(false, false)))
        );
    }

    /// Importers and the audit log follow the stored identity, not the grants
    /// an administrator usually holds.
    #[test]
    fn importers_need_the_administrator_identity() {
        let everything = canonical(Permission::ALL.to_vec(), false, PERMISSION_CATALOG_VERSION);
        let access = workspace_access(Some(&everything)).unwrap().unwrap();
        assert!(!access.allows(Section::Importers));
        assert!(!access.allows(Section::Audit));
        assert!(!access.allows(Section::ApiTokens));
    }

    #[test]
    fn unknown_or_failed_state_fails_closed() {
        let newer = canonical(
            vec![Permission::PeopleReadAll],
            true,
            PERMISSION_CATALOG_VERSION + 1,
        );
        let failed = Err(ServerFnError::new("unavailable"));
        for own in [&newer, &failed] {
            assert_eq!(workspace_access(Some(own)), Some(Err(())));
        }
    }

    #[test]
    fn workspace_settings_need_company_write() {
        let version = PERMISSION_CATALOG_VERSION;
        let allowed = |own: &Result<OwnPermissions, ServerFnError>| {
            workspace_access(Some(own))
                .and_then(Result::ok)
                .is_some_and(|access| access.allows(Section::Settings))
        };
        let company = canonical(vec![Permission::CompanyWrite], false, version);
        assert!(allowed(&company));
        for own in [
            // An administrator without the grant, and every other grant.
            canonical(Vec::new(), true, version),
            canonical(
                vec![Permission::PeopleWriteAll, Permission::InvoiceWriteAll],
                false,
                version,
            ),
            canonical(vec![Permission::CompanyWrite], false, version + 1),
            Err(ServerFnError::new("unavailable")),
        ] {
            assert!(!allowed(&own), "{own:?}");
        }
    }

    #[test]
    fn first_admin_route_follows_sub_navigation_order() {
        let version = PERMISSION_CATALOG_VERSION;
        // `Route` has no `Debug`, so compare the paths it renders to.
        let first = |own: &Result<OwnPermissions, ServerFnError>| {
            first_admin_route(Some(own)).map(|route| route.to_string())
        };
        let path = |p: &str| Some(p.to_string());
        let people = canonical(vec![Permission::PeopleReadAll], true, version);
        assert_eq!(first(&people), path("/admin/importers"));
        // People and Tasks are in the main rail, so their grants alone open no
        // admin section.
        for grant in [Permission::PeopleReadAll, Permission::TaskReadAll] {
            assert_eq!(first(&canonical(vec![grant], false, version)), None);
        }
        let floor = canonical(Vec::new(), false, version);
        assert_eq!(first(&floor), None);
        let company = canonical(vec![Permission::CompanyWrite], false, version);
        assert_eq!(first(&company), path("/admin/workspace"));
        let administrator = canonical(Vec::new(), true, version);
        assert_eq!(first(&administrator), path("/admin/importers"));
        let failed = Err(ServerFnError::new("unavailable"));
        assert_eq!(first(&failed), None);
        assert!(first_admin_route(None).is_none(), "loading");
    }
}
