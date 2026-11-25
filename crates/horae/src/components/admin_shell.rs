use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::page_header::PageHeader;
use crate::components::screen_state::{ErrorState, NoAccess};
use crate::components::session::use_session;
use crate::i18n::Tr;
use crate::models::own_permissions::OwnPermissions;
use crate::route::Route;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    Settings,
    Plugins,
    Audit,
    Importers,
    ApiTokens,
    Apps,
}

impl Section {
    /// The title of the no-access state. It has no subnav to give context, so
    /// Settings takes its full name.
    fn title(self, tr: Tr) -> String {
        match self {
            Section::Settings => tr.workspace_settings_form_label(),
            Section::Plugins => tr.workspace_plugins_title(),
            Section::Audit => "Audit log".into(),
            Section::Importers => "Importers".into(),
            Section::ApiTokens => "API tokens".into(),
            Section::Apps => tr.apps_title(),
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
            // The plugins were part of the settings page, so the same people see them.
            Section::Settings | Section::Plugins => self.settings,
            Section::Audit => self.audit,
            Section::Importers => self.importers,
            // Apps, like API tokens, are for the administrator identity.
            Section::ApiTokens | Section::Apps => self.api_tokens,
        }
    }
}

/// Which workspace sections to offer, from the session's stored permissions;
/// `Err` when access cannot be determined. These are navigation hints; every
/// reader/command authorizes afresh.
fn workspace_access(own: &Result<OwnPermissions, ServerFnError>) -> Result<WorkspaceAccess, ()> {
    match own {
        Ok(own) if own.catalog_version == PERMISSION_CATALOG_VERSION => Ok(WorkspaceAccess {
            settings: own.grants.contains(&Permission::CompanyWrite),
            audit: own.is_administrator,
            importers: own.is_administrator,
            api_tokens: own.is_administrator,
        }),
        _ => Err(()),
    }
}

/// The first section the person may open, in sub-navigation order, so an entry
/// into the admin area never lands on "Access unavailable". `None` when no
/// section is open.
pub(crate) fn first_admin_route(own: &Result<OwnPermissions, ServerFnError>) -> Option<Route> {
    let access = workspace_access(own).ok()?;
    [
        (access.settings, Route::WorkspaceSettings {}),
        (access.importers, Route::ImporterIndex {}),
        (
            access.audit,
            Route::PermissionAudit {
                filter: Default::default(),
            },
        ),
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
/// Only sections with a real destination are listed — Settings, Plugins, API
/// tokens, Importers (Harvest), and the permission audit log. People and Tasks have
/// their own pages in the main rail, as in Harvest. Other sections are deferred
/// until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let section = match use_route::<Route>() {
        Route::WorkspaceSettings {} => Section::Settings,
        Route::WorkspacePlugins {} => Section::Plugins,
        Route::PermissionAudit { .. } => Section::Audit,
        Route::AdminApiTokens {} => Section::ApiTokens,
        Route::AdminApps {} | Route::AdminApp { .. } => Section::Apps,
        _ => Section::Importers,
    };
    let tr = crate::i18n::use_tr();
    let session = use_session();
    let retry = use_callback(move |_| session.refresh());
    // The session arrives with the page, so the shell decides at once. A
    // refresh keeps the last answer until the new one arrives, so switching
    // sections or re-reading permissions never unmounts the open panel; server
    // functions still independently enforce access to every operation.
    let pending = (session.refreshing)();
    match workspace_access(&session.permissions.read()) {
        Ok(access) if access.allows(section) => {
            rsx! { AdminWorkspace { access } }
        }
        // Non-disclosing: it names who can grant access, not what the section holds.
        Ok(_) => rsx! {
            PageHeader { title: section.title(tr) }
            NoAccess {}
        },
        Err(()) => rsx! {
            ErrorState {
                title: tr.workspace_access_failed(),
                body: tr.workspace_access_failed_hint(),
                onretry: retry,
                retry_disabled: pending,
            }
        },
    }
}

#[component]
fn AdminWorkspace(access: WorkspaceAccess) -> Element {
    let tr = crate::i18n::use_tr();
    // The workspace's real name for the header chip (no slug — the schema has no
    // such field, so we show the name only rather than inventing a URL).
    let org_name = use_session().org_name.cloned();
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
                            if access.settings {
                                AdmLink { to: Route::WorkspaceSettings {}, label: tr.workspace_settings_title() }
                                AdmLink { to: Route::WorkspacePlugins {}, label: tr.workspace_plugins_title() }
                            }
                            if access.api_tokens {
                                AdmLink { to: Route::AdminApiTokens {}, label: "API tokens" }
                                AdmLink { to: Route::AdminApps {}, label: tr.apps_title() }
                            }
                        }
                    }

                    if access.importers || access.audit {
                        div { class: "adm-group-label", "Data" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.importers { AdmLink { to: Route::ImporterIndex {}, label: "Importers" } }
                            if access.audit { AdmLink { to: Route::PermissionAudit { filter: Default::default() }, label: "Audit log" } }
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
pub(crate) fn AdmLink(to: Route, label: String) -> Element {
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
    fn grants_and_the_administrator_identity_choose_the_sections() {
        let administrator = canonical(
            vec![Permission::PeopleReadManaged, Permission::TaskReadAll],
            true,
            PERMISSION_CATALOG_VERSION,
        );
        assert_eq!(workspace_access(&administrator), Ok(access(true, true)));
        // People is a main screen, so reading people opens no admin section.
        let people = canonical(
            vec![Permission::PeopleReadAll],
            false,
            PERMISSION_CATALOG_VERSION,
        );
        assert_eq!(workspace_access(&people), Ok(access(false, false)));
    }

    /// Importers and the audit log follow the stored identity, not the grants
    /// an administrator usually holds.
    #[test]
    fn importers_need_the_administrator_identity() {
        let everything = canonical(Permission::ALL.to_vec(), false, PERMISSION_CATALOG_VERSION);
        let access = workspace_access(&everything).unwrap();
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
            assert_eq!(workspace_access(own), Err(()));
        }
    }

    #[test]
    fn workspace_settings_need_company_write() {
        let version = PERMISSION_CATALOG_VERSION;
        let allowed = |own: &Result<OwnPermissions, ServerFnError>| {
            workspace_access(own).is_ok_and(|access| {
                // Plugins open to exactly the people who open the settings.
                assert_eq!(
                    access.allows(Section::Settings),
                    access.allows(Section::Plugins)
                );
                access.allows(Section::Settings)
            })
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
            first_admin_route(own).map(|route| route.to_string())
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
    }
}
