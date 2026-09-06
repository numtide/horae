use dioxus::prelude::*;

use crate::models::User;

pub mod admin;
pub mod approvals;
pub mod auth;
pub mod clients;
pub mod gallery;
pub mod importers;
pub mod invoices;
pub mod projects;
pub mod reports;
pub mod settings;
pub mod timesheet;

/// Whether the session user (from a `get_me` resource) is a manager or admin.
/// `false` while the resource is loading or errored.
pub(crate) fn is_manager(me: &Resource<Result<User, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(u)) if u.is_manager_or_above())
}

/// Whether the session user (from a `get_me` resource) is an admin.
/// `false` while the resource is loading or errored.
pub(crate) fn is_admin(me: &Resource<Result<User, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(u)) if u.is_admin())
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
