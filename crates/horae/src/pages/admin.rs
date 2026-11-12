use dioxus::prelude::*;

use crate::components::session::use_session;

mod add_person;
mod people;
mod permission_editor;
mod rates;
mod workspace;

pub use workspace::WorkspaceSettings;

#[component]
pub fn AdminUsers() -> Element {
    let session = use_session();
    let own = session.permissions;
    let mut notice = use_signal(|| None::<String>);
    let on_saved = use_callback(move |changed: bool| {
        notice.set(Some(
            if changed {
                "Permissions saved."
            } else {
                "Permissions are unchanged."
            }
            .into(),
        ));
        // The save may have changed the user's own access, which the rail and
        // the admin navigation render from too.
        session.refresh();
    });
    // The permissions are shared with the rail, which refreshes them when its
    // account menu opens; keep a resolved answer meanwhile so the directory does
    // not remount and lose its filter and page. A retry after a failure shows as
    // pending rather than repeating the old error.
    let resolved = matches!(&*own.read(), Some(Ok(_)));
    if !resolved && own.state()() != UseResourceState::Ready {
        return rsx! { p { role: "status", "Loading people access…" } };
    }
    let content = match &*own.read() {
        Some(Ok(Some(access)))
            if access.catalog_version
                == horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION
                && (access
                    .grants
                    .contains(&horae_core::permissions::catalog::Permission::PeopleReadAll)
                    || access.grants.contains(
                        &horae_core::permissions::catalog::Permission::PeopleReadManaged,
                    )) =>
        {
            use horae_core::permissions::catalog::Permission::{
                BillableRateReadAll, BillableRateReadManaged, CostRateReadAll, PeopleWriteAll,
            };
            // Per-person access is decided when the dialog loads; with a
            // managed-only grant, other people's rows show a refusal there.
            let can_view_rates = [
                BillableRateReadManaged,
                BillableRateReadAll,
                CostRateReadAll,
            ]
            .iter()
            .any(|grant| access.grants.contains(grant));
            rsx! {
                people::CanonicalPeople {
                    can_edit_permissions: access.is_administrator,
                    can_manage_people: access.grants.contains(&PeopleWriteAll),
                    can_view_rates,
                    on_saved,
                }
            }
        }
        _ => rsx! {
            p { class: "alert alert-danger", role: "alert", "People access is unavailable. Sign in again or retry." }
            button { r#type: "button", class: "btn btn-secondary", onclick: move |_| session.refresh(), "Retry" }
            PermissionRecovery { on_saved }
        },
    };
    rsx! {
        if let Some(message) = notice() { p { class: "text-sm text-secondary mb-4", role: "status", "{message}" } }
        {content}
    }
}

/// Retained requests remain recoverable without mounting a directory or selecting a person.
#[component]
pub(crate) fn PermissionRecovery(on_saved: EventHandler<bool>) -> Element {
    let person = use_signal(|| None);
    rsx! { permission_editor::PermissionEditorDialog { person, on_saved } }
}
