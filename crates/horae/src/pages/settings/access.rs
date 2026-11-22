//! Your access: what the signed-in person may see and do, read from the
//! session's stored permissions. Display only; every operation authorizes
//! afresh on the server.

use dioxus::prelude::*;
use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;
use uuid::Uuid;

use crate::components::access_table::{AccessCell, AccessTable, OTHER, access_rows};
use crate::components::card::MetricCard;
use crate::components::page_header::PageHeader;
use crate::components::permission_description::permission_description;
use crate::components::screen_state::{ErrorState, LOAD_FAILED};
use crate::components::session::use_session;
use crate::i18n::{Tr, split_tag};
use crate::models::own_permissions::OwnPermissions;
use crate::models::permission_audit::AuditFilter;
use crate::route::Route;

#[component]
pub fn SettingsAccess() -> Element {
    let tr = crate::i18n::use_tr();
    let session = use_session();
    let retry = use_callback(move |_| session.refresh());
    let pending = (session.refreshing)();
    let me = session.me.read().id;
    access_page(tr, &session.permissions.read(), me, retry, pending)
}

/// The header and body for the session's permission read. `pending` is true
/// while a refresh runs, so "Try again" waits for it. `me` is the signed-in
/// person, whom the link to their history filters the audit log by.
fn access_page(
    tr: Tr,
    response: &Result<OwnPermissions, ServerFnError>,
    me: Uuid,
    retry: Callback<()>,
    pending: bool,
) -> Element {
    // The badge shows only above the access it describes, not above an error.
    let badge = response
        .as_ref()
        .ok()
        .filter(|own| own.catalog_version == PERMISSION_CATALOG_VERSION)
        .map(|own| {
            if own.is_administrator {
                (
                    "badge badge-info badge-sm",
                    tr.settings_access_administrator_badge(),
                )
            } else {
                (
                    "badge badge-neutral badge-sm",
                    own.profile
                        .clone()
                        .unwrap_or_else(|| tr.settings_access_individual_badge()),
                )
            }
        });
    rsx! {
        PageHeader { title: tr.settings_access_title(),
            meta: rsx! {
                if let Some((class, label)) = badge { span { class, "{label}" } }
            },
        }
        {access_content(tr, response, me, retry, pending)}
    }
}

/// The page body for the session's permission read. A session refresh keeps
/// the last answer on screen until the new one arrives.
fn access_content(
    tr: Tr,
    response: &Result<OwnPermissions, ServerFnError>,
    me: Uuid,
    retry: Callback<()>,
    pending: bool,
) -> Element {
    const UNAUTHORIZED: u16 = 401;
    const FORBIDDEN: u16 = 403;
    let own = match response {
        Ok(own) if own.catalog_version == PERMISSION_CATALOG_VERSION => own,
        Err(ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        }) => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: tr.common_session_ended() }
            };
        }
        // No stored permission state: only an administrator can repair it.
        Err(ServerFnError::ServerError {
            code: FORBIDDEN, ..
        }) => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: tr.settings_access_repair() }
            };
        }
        _ => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: LOAD_FAILED, onretry: retry, retry_disabled: pending }
            };
        }
    };
    let rows: Vec<_> = access_rows(&own.grants)
        .into_iter()
        .map(|(area, cells)| (area, cells.map(|now| AccessCell { now, was: None })))
        .collect();
    let others: Vec<_> = OTHER
        .iter()
        .copied()
        .filter(|grant| own.grants.contains(grant))
        .collect();
    let administrator = tr.settings_access_administrator();
    let (people_before, people_link, people_after) = split_tag(&administrator, "link");
    let history = AuditFilter {
        person: Some(me),
        ..AuditFilter::default()
    };
    rsx! {
        div { class: "flex flex-col gap-5 wrap-anywhere",
            p { class: "text-secondary",
                {tr.settings_access_intro()}
                // Only those who may read the audit log get a way into it.
                if own.is_administrator {
                    {" "}
                    Link { id: "access-history", to: Route::PermissionAudit { filter: history }, {tr.settings_access_history()} }
                }
            }
            if own.is_administrator {
                p { class: "card",
                    "{people_before}"
                    Link { to: Route::People {}, "{people_link}" }
                    "{people_after}"
                }
            } else {
                AccessTable { label: tr.settings_access_table_label(), rows }
                p { class: "text-xs text-muted", {tr.settings_access_legend()} }
                if !others.is_empty() {
                    section { class: "card", aria_labelledby: "access-other-title",
                        h2 { id: "access-other-title", class: "card-title", {tr.settings_access_other_title()} }
                        div { class: "text-sm",
                            for (index, grant) in others.into_iter().enumerate() {
                                div { key: "{grant:?}", class: if index == 0 { "py-2" } else { "py-2 border-t" },
                                    {permission_description(grant)}
                                }
                            }
                        }
                    }
                }
                p { class: "text-secondary", {tr.settings_access_change()} }
            }
            div { class: "grid md:grid-cols-2 gap-3",
                MetricCard { label: tr.settings_access_people_managed(), value: own.managed_person_ids.len().to_string() }
                MetricCard { label: tr.settings_access_projects_managed(), value: own.managed_project_ids.len().to_string() }
            }
        }
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "access/tests.rs"]
mod tests;
