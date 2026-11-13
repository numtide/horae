use dioxus::prelude::*;
use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;

use crate::models::own_permissions::OwnPermissions;

pub mod admin;
pub mod approvals;
pub mod clients;
pub mod gallery;
pub mod importers;
pub mod invoices;
pub mod new_project;
pub mod permission_audit;
pub mod projects;
pub mod reports;
pub mod settings;
pub mod tasks;
pub mod timesheet;

/// Whether the session's stored permissions carry the Administrator identity.
/// `false` while they are loading, failed, or on a catalog this bundle does
/// not know.
pub(crate) fn is_administrator(own: &Resource<Result<OwnPermissions, ServerFnError>>) -> bool {
    matches!(
        &*own.read(),
        Some(Ok(own)) if own.catalog_version == PERMISSION_CATALOG_VERSION && own.is_administrator
    )
}

/// Render a resource's loaded value, with the standard loading placeholder and
/// error banner for the pending and failed states.
pub(crate) fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(v)) => render(v),
        Some(Err(e)) => rsx! {
            div { class: "alert alert-danger", "{e}" }
        },
        None => rsx! {
            div { class: "text-muted text-sm", "Loading…" }
        },
    }
}

/// Run a mutating server action: on success clear `error`, run `on_ok` (form
/// reset / close), and restart `resource` so the list re-loads; on failure
/// surface the error.
pub(crate) fn run_action<O: 'static, T: 'static>(
    fut: impl std::future::Future<Output = Result<O, ServerFnError>> + 'static,
    mut resource: Resource<T>,
    mut error: Signal<Option<String>>,
    on_ok: impl FnOnce() + 'static,
) {
    spawn(async move {
        match fut.await {
            Ok(_) => {
                error.set(None);
                on_ok();
                resource.restart();
            }
            Err(e) => error.set(Some(e.to_string())),
        }
    });
}
