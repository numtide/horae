use dioxus::prelude::*;
use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;

use crate::components::permission_description::permission_description;
use crate::models::own_permissions::OwnPermissions;
use crate::server_fns;

#[component]
pub(super) fn OwnPermissionSection() -> Element {
    let mut permissions = use_resource(server_fns::get_my_permissions);
    let pending = permissions.state()() != UseResourceState::Ready;
    rsx! {
        section { class: "mt-6 mb-6 wrap-anywhere", aria_labelledby: "own-permissions-title",
            div { class: "flex flex-wrap items-center justify-between gap-3 mb-2",
                h2 { id: "own-permissions-title", class: "text-2xl font-semibold", "Your permissions" }
                button { r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending,
                    onclick: move |_| {
                        if permissions.state()() == UseResourceState::Ready {
                            permissions.restart();
                        }
                    },
                    "Refresh permissions"
                }
            }
            p { class: "text-sm text-secondary mb-5",
                "Your permissions determine what you can see and do in this workspace."
            }
            {permission_content(pending, &permissions.read())}
        }
    }
}

fn permission_content(
    pending: bool,
    response: &Option<Result<Option<OwnPermissions>, ServerFnError>>,
) -> Element {
    if pending || response.is_none() {
        return rsx! { p { class: "text-sm text-secondary", role: "status", "Loading your permissions…" } };
    }
    const UNAUTHORIZED: u16 = 401;
    const FORBIDDEN: u16 = 403;
    let own = match response {
        Some(Ok(Some(own))) if own.catalog_version == PERMISSION_CATALOG_VERSION => own,
        Some(Ok(None)) => {
            return rsx! {
                p { class: "text-sm text-secondary",
                    "Detailed permissions are not enabled for this workspace. Your existing access is unchanged."
                }
            };
        }
        other => {
            let message = match other {
                Some(Err(ServerFnError::ServerError {
                    code: UNAUTHORIZED, ..
                })) => "Sign in again to view your permissions.",
                Some(Err(ServerFnError::ServerError {
                    code: FORBIDDEN, ..
                })) => {
                    "Your permissions cannot be viewed with the current account. Ask an Administrator to check your access."
                }
                _ => "Could not load your permissions. Refresh permissions to try again.",
            };
            return rsx! { p { class: "text-sm text-danger", role: "alert", "{message}" } };
        }
    };
    rsx! {
        div { class: "banner banner-warning mb-5",
            p { class: "banner-title", "Only an Administrator can edit your permissions." }
        }
        if own.is_administrator {
            p { class: "text-sm font-semibold mb-4", "Administrator access" }
            a { class: "inline-flex mb-4", href: "/admin/audit", "View permission audit log" }
        }
        h3 { class: "text-lg mb-2", "Configured permissions" }
        p { class: "text-sm text-secondary mb-4",
            "These permissions do not enable unavailable features or remove approval and invoice locks."
        }
        if own.grants.is_empty() {
            p { class: "text-sm text-secondary", "No configured permissions were returned. Ask an Administrator to check your access." }
        } else {
            ul { class: "flex flex-col gap-2 pl-5 text-sm",
                for grant in &own.grants {
                    li { {permission_description(*grant)} }
                }
            }
        }
        h3 { class: "text-lg mt-5 mb-2", "Management responsibilities" }
        dl { class: "grid md:grid-cols-2 gap-3 text-sm",
            div {
                dt { class: "text-secondary", "Managed people" }
                dd { class: "font-mono", "{own.managed_person_ids.len()}" }
            }
            div {
                dt { class: "text-secondary", "Managed projects" }
                dd { class: "font-mono", "{own.managed_project_ids.len()}" }
            }
        }
        p { class: "text-sm text-secondary mt-3",
            "These relationships do not grant access on their own. Each action still requires its matching permission and scope."
        }
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "own_permissions/tests.rs"]
mod tests;
