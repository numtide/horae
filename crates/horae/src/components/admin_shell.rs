use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::session::use_session;
use crate::models::own_permissions::OwnPermissions;
use crate::models::user::CurrentUser;
use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;
use crate::server_fns;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    People,
    Settings,
    Audit,
    Importers,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorkspaceAccess {
    people: bool,
    /// `CompanyWrite`, which `set_workspace_timezone` requires.
    settings: bool,
    audit: bool,
    importers: bool,
}

impl WorkspaceAccess {
    fn allows(self, section: Section) -> bool {
        match section {
            Section::People => self.people,
            Section::Settings => self.settings,
            Section::Audit => self.audit,
            Section::Importers => self.importers,
        }
    }
}

/// Which workspace sections to offer, from the shared session; `None` until both
/// halves have loaded and `Err` when access cannot be determined. These are
/// navigation hints; every reader/command authorizes afresh.
fn workspace_access(
    section: Section,
    own: Option<&Result<Option<OwnPermissions>, ServerFnError>>,
    me: Option<&Result<CurrentUser, ServerFnError>>,
) -> Option<Result<WorkspaceAccess, ()>> {
    let (own, me) = (own?, me?);
    let Ok(me) = me else {
        return Some(Err(()));
    };
    let legacy_admin = me.is_admin();
    let own = match own {
        Ok(Some(own)) if own.catalog_version != PERMISSION_CATALOG_VERSION => Err(()),
        Ok(own) => Ok(own.as_ref()),
        Err(_) => Err(()),
    };
    // Importers stay reachable for legacy admins when permission state is
    // unavailable.
    if section == Section::Importers && own.is_err() {
        return Some(Ok(WorkspaceAccess {
            people: false,
            settings: false,
            audit: false,
            importers: legacy_admin,
        }));
    }
    Some(own.map(|own| match own {
        Some(own) => WorkspaceAccess {
            people: own.grants.contains(&Permission::PeopleReadAll)
                || own.grants.contains(&Permission::PeopleReadManaged),
            settings: own.grants.contains(&Permission::CompanyWrite),
            audit: own.is_administrator,
            importers: legacy_admin,
        },
        None => WorkspaceAccess {
            people: legacy_admin,
            settings: false,
            audit: false,
            importers: legacy_admin,
        },
    }))
}

/// The first section the person may open, in sub-navigation order, so an entry
/// into the admin area never lands on "Access unavailable". `None` while loading
/// or when no section is open.
pub(crate) fn first_admin_route(
    own: Option<&Result<Option<OwnPermissions>, ServerFnError>>,
    me: Option<&Result<CurrentUser, ServerFnError>>,
) -> Option<Route> {
    // Asking as Importers keeps the shell's fallback for legacy admins whose
    // permission state is unavailable.
    let access = workspace_access(Section::Importers, own, me)?.ok()?;
    [
        (access.people, Route::AdminUsers {}),
        (access.settings, Route::WorkspaceSettings {}),
        (access.importers, Route::ImporterIndex {}),
        (access.audit, Route::PermissionAudit {}),
    ]
    .into_iter()
    .find_map(|(allowed, route)| allowed.then_some(route))
}

/// The admin area shell: a secondary sub-navigation (Workspace / Data) beside the
/// active admin panel, rendered through an `Outlet`. Layered inside `AppLayout`,
/// so the main rail stays put and this owns only the content panel (per the
/// design's admin settings shell). Each section has its own access boundary.
///
/// Only sections with a real destination are listed — Team, Importers
/// (Harvest), and the permission audit log. Tasks has its own page in the main
/// rail, as in Harvest. Other sections are deferred
/// until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let section = match use_route::<Route>() {
        Route::AdminUsers {} => Section::People,
        Route::WorkspaceSettings {} => Section::Settings,
        Route::PermissionAudit {} => Section::Audit,
        _ => Section::Importers,
    };
    let session = use_session();
    // Mount the workspace and its outlet once the session has loaded. A refresh
    // keeps a resolved answer until the new one arrives, so switching sections
    // or re-reading permissions never unmounts the open panel; server functions
    // still independently enforce access to every operation. A retry after a
    // failure shows as pending rather than repeating the old error.
    let refreshing = session.me.state()() != UseResourceState::Ready
        || session.permissions.state()() != UseResourceState::Ready;
    let access = workspace_access(
        section,
        session.permissions.read().as_ref(),
        session.me.read().as_ref(),
    )
    .filter(|access| access.is_ok() || !refreshing);
    match access {
        Some(Ok(access)) if access.allows(section) => {
            rsx! { AdminWorkspace { access } }
        }
        Some(Ok(_)) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                h1 { class: "page-title", "Access unavailable" }
                p { class: "text-secondary", "Your account cannot view this workspace section." }
                Link {
                    to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() },
                    class: "btn btn-secondary",
                    "Back to Timesheet"
                }
            }
            if section == Section::People { crate::pages::admin::PermissionRecovery { on_saved: move |_| session.refresh() } }
        },
        Some(Err(())) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                div { class: "alert alert-danger", role: "alert",
                    "Could not verify workspace access. Sign in again or retry."
                }
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| session.refresh(),
                    "Retry"
                }
            }
            if section == Section::People { crate::pages::admin::PermissionRecovery { on_saved: move |_| session.refresh() } }
        },
        None => rsx! {
            div { class: "text-muted text-sm", role: "status", "Loading…" }
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
            Link {
                to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() },
                class: "adm-back inline-flex items-center gap-2 text-sm text-secondary mb-5",
                span { "←" }
                "Back to Timesheet"
            }
            div { class: "flex flex-wrap items-start gap-12",
                aside { class: "adm-nav",
                    div { class: "adm-head flex items-center gap-3",
                        span { class: "adm-head-mark", "{org_initial}" }
                        div { class: "min-w-0",
                            div { class: "text-sm font-semibold truncate mb-1", "{org_name}" }
                            if access.audit { span { class: "badge badge-info badge-sm", "Administrator" } }
                        }
                    }

                    if access.people || access.settings {
                        div { class: "adm-group-label", "Workspace" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.people { AdmLink { to: Route::AdminUsers {}, label: "Team" } }
                            if access.settings { AdmLink { to: Route::WorkspaceSettings {}, label: "Settings" } }
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
    use horae_core::types::OrgRole;
    use uuid::Uuid;

    use super::*;

    fn user(org_role: OrgRole) -> Result<CurrentUser, ServerFnError> {
        Ok(CurrentUser {
            id: Uuid::nil(),
            org_id: Uuid::nil(),
            email: "a@example.com".into(),
            name: "A".into(),
            org_role,
            timezone: None,
            workspace_timezone: "UTC".into(),
            utc_offset_seconds: 0,
        })
    }

    fn canonical(
        grants: Vec<Permission>,
        is_administrator: bool,
        catalog_version: u32,
    ) -> Result<Option<OwnPermissions>, ServerFnError> {
        Ok(Some(OwnPermissions {
            catalog_version,
            grants,
            is_administrator,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        }))
    }

    /// Access without workspace settings, which only `CompanyWrite` opens.
    fn access(people: bool, audit: bool, importers: bool) -> WorkspaceAccess {
        WorkspaceAccess {
            people,
            settings: false,
            audit,
            importers,
        }
    }

    #[test]
    fn waits_for_both_halves_of_the_session() {
        let admin = user(OrgRole::Admin);
        let own = Ok(None);
        assert_eq!(workspace_access(Section::People, None, Some(&admin)), None);
        assert_eq!(workspace_access(Section::People, Some(&own), None), None);
    }

    #[test]
    fn policy_zero_follows_the_legacy_role() {
        let own = Ok(None);
        let admin = user(OrgRole::Admin);
        let manager = user(OrgRole::Manager);
        assert_eq!(
            workspace_access(Section::People, Some(&own), Some(&admin)),
            Some(Ok(access(true, false, true)))
        );
        assert_eq!(
            workspace_access(Section::People, Some(&own), Some(&manager)),
            Some(Ok(access(false, false, false)))
        );
    }

    #[test]
    fn canonical_grants_choose_the_sections() {
        let member = user(OrgRole::Member);
        let own = canonical(
            vec![Permission::PeopleReadManaged, Permission::TaskReadAll],
            true,
            PERMISSION_CATALOG_VERSION,
        );
        assert_eq!(
            workspace_access(Section::People, Some(&own), Some(&member)),
            Some(Ok(access(true, true, false)))
        );
    }

    #[test]
    fn unknown_or_failed_state_fails_closed_except_importers_for_admins() {
        let admin = user(OrgRole::Admin);
        let newer = canonical(
            vec![Permission::PeopleReadAll],
            true,
            PERMISSION_CATALOG_VERSION + 1,
        );
        let failed = Err(ServerFnError::new("unavailable"));
        for own in [&newer, &failed] {
            assert_eq!(
                workspace_access(Section::People, Some(own), Some(&admin)),
                Some(Err(()))
            );
            assert_eq!(
                workspace_access(Section::Importers, Some(own), Some(&admin)),
                Some(Ok(access(false, false, true)))
            );
        }
        let no_identity = Err(ServerFnError::new("unauthorized"));
        assert_eq!(
            workspace_access(Section::Importers, Some(&Ok(None)), Some(&no_identity)),
            Some(Err(()))
        );
    }

    #[test]
    fn workspace_settings_need_company_write() {
        let version = PERMISSION_CATALOG_VERSION;
        let allowed = |own: &Result<Option<OwnPermissions>, ServerFnError>| {
            // The legacy role must not matter.
            [user(OrgRole::Admin), user(OrgRole::Member)].map(|me| {
                workspace_access(Section::Settings, Some(own), Some(&me))
                    .and_then(Result::ok)
                    .is_some_and(|access| access.allows(Section::Settings))
            })
        };
        let company = canonical(vec![Permission::CompanyWrite], false, version);
        assert_eq!(allowed(&company), [true, true]);
        for own in [
            // An administrator without the grant, and every other grant.
            canonical(Vec::new(), true, version),
            canonical(
                vec![Permission::PeopleWriteAll, Permission::InvoiceWriteAll],
                false,
                version,
            ),
            canonical(vec![Permission::CompanyWrite], false, version + 1),
            Ok(None),
            Err(ServerFnError::new("unavailable")),
        ] {
            assert_eq!(allowed(&own), [false, false], "{own:?}");
        }
    }

    #[test]
    fn first_admin_route_follows_sub_navigation_order() {
        let admin = user(OrgRole::Admin);
        let member = user(OrgRole::Member);
        let version = PERMISSION_CATALOG_VERSION;
        // `Route` has no `Debug`, so compare the paths it renders to.
        let first = |own: &Result<Option<OwnPermissions>, ServerFnError>, me| {
            first_admin_route(Some(own), Some(me)).map(|route| route.to_string())
        };
        let path = |p: &str| Some(p.to_string());
        assert_eq!(first(&Ok(None), &admin), path("/admin/users"));
        assert_eq!(first(&Ok(None), &member), None);
        // Tasks is in the main rail, so the task grant alone opens no admin section.
        let tasks = canonical(vec![Permission::TaskReadAll], false, version);
        assert_eq!(first(&tasks, &member), None);
        let floor = canonical(Vec::new(), false, version);
        assert_eq!(first(&floor, &admin), path("/admin/importers"));
        assert_eq!(first(&floor, &member), None);
        let company = canonical(vec![Permission::CompanyWrite], false, version);
        assert_eq!(first(&company, &member), path("/admin/workspace"));
        let administrator = canonical(Vec::new(), true, version);
        assert_eq!(first(&administrator, &member), path("/admin/audit"));
        let failed = Err(ServerFnError::new("unavailable"));
        assert_eq!(first(&failed, &admin), path("/admin/importers"));
        assert_eq!(first(&failed, &member), None);
        assert!(first_admin_route(None, Some(&admin)).is_none(), "loading");
    }
}
