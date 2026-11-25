//! One app's page: its details, the Actions menu, and the authorizations
//! people gave it, each revocable.

use dioxus::prelude::*;
use horae_core::permissions::oauth::Scope;
use uuid::Uuid;

use super::form::{AppDialog, AppDialogs};
use super::{AppMenuItems, StatusBadge, access_label, date, scope_sentence};
use crate::components::back_link::BackLink;
use crate::components::card::Card;
use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::menu::{Menu, MenuItem, RowActions};
use crate::components::page_header::PageHeader;
use crate::components::revealed_secret::RevealedSecret;
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::components::table::{DataTable, NameCell, TableMessage, TableSkeleton};
use crate::models::oauth_apps::{AppKind, GrantRow};
use crate::route::Route;
use crate::server_fns;

#[component]
pub fn AdminApp(id: Uuid) -> Element {
    // Route reuse must discard one app's dialogs and errors with it.
    rsx! { for id in [id] { AppPage { key: "{id}", id } } }
}

#[component]
fn AppPage(id: Uuid) -> Element {
    let tr = crate::i18n::use_tr();
    let mut app = use_resource(move || server_fns::get_oauth_app(id.to_string()));
    let mut grants = use_resource(move || server_fns::list_oauth_app_grants(id.to_string()));
    let retry = use_callback(move |_| app.restart());
    let dialog = use_signal(|| None::<AppDialog>);
    let mut change_error = use_signal(|| None::<String>);
    let toggling = use_signal(|| None::<Uuid>);
    let on_toggle = use_callback(move |(id, enabled): (Uuid, bool)| {
        change_error.set(None);
        super::toggle(id, enabled, toggling, change_error, tr, move || {
            app.restart()
        });
    });
    let on_changed = use_callback(move |_| {
        app.restart();
        grants.restart();
    });
    let on_deleted = use_callback(move |_| {
        navigator().replace(Route::AdminApps {});
    });
    let ready = app.state()() == UseResourceState::Ready;

    rsx! {
        BackLink { to: Route::AdminApps {}, label: tr.apps_back() }
        match &*app.read() {
            None => rsx! { Loading { label: tr.apps_loading_app() } },
            Some(Err(error)) => rsx! {
                LoadError { error: error.clone(), what: tr.apps_what_app(), retry, pending: !ready,
                    focus_on_recovery: "page-back",
                }
            },
            Some(Ok(row)) => rsx! {
                PageHeader { title: row.name.clone(), title_id: "app-title",
                    meta: rsx! { StatusBadge { enabled: row.enabled } },
                    actions: rsx! {
                        Menu { id: "app-actions", align_right: true, trigger_class: "text-sm py-3 px-5",
                            label: tr.action_menu(), busy: toggling().is_some(),
                            AppMenuItems { app: row.clone(), dialog, on_toggle, toggling: toggling().is_some() }
                        }
                    },
                }
                if let Some(message) = change_error() {
                    p { class: "alert alert-danger", role: "alert", "{message}" }
                }
                Card { class: "mb-6 max-w-form", id: "app-details",
                    dl { class: "text-sm m-0 flex flex-col gap-4", aria_label: tr.apps_details_label(),
                        div {
                            dt { class: "text-secondary mb-1", {tr.apps_client_id()} }
                            dd { class: "m-0",
                                RevealedSecret { id: "app-client-id", value: row.client_id.clone(),
                                    label: tr.apps_client_id(), copy_label: tr.apps_copy_client_id(),
                                }
                            }
                        }
                        div {
                            dt { class: "text-secondary mb-1", {tr.apps_type()} }
                            dd { class: "m-0",
                                match row.kind {
                                    AppKind::Public => tr.apps_kind_public(),
                                    AppKind::Confidential => tr.apps_kind_confidential_detail(),
                                }
                            }
                        }
                        div {
                            dt { class: "text-secondary mb-1", {tr.apps_redirects()} }
                            dd { class: "m-0",
                                ul { class: "data-list flex flex-col gap-1",
                                    for uri in row.redirect_uris.iter() {
                                        li { key: "{uri}", class: "text-mono text-xs wrap-anywhere", "{uri}" }
                                    }
                                }
                            }
                        }
                        div {
                            dt { class: "text-secondary mb-1", {tr.apps_scopes()} }
                            dd { class: "m-0",
                                ul { class: "data-list flex flex-col gap-1",
                                    for scope in Scope::ALL.into_iter().filter(|scope| row.scopes.iter().any(|name| name == scope.as_str())) {
                                        li { key: "{scope.as_str()}", {scope_sentence(tr, scope)} }
                                    }
                                }
                            }
                        }
                        if !row.built_in {
                            div {
                                dt { class: "text-secondary mb-1", {tr.apps_registered()} }
                                dd { class: "m-0",
                                    match &row.created_by_name {
                                        Some(name) => tr.apps_registered_by(date(row.created_at), name.as_str()),
                                        None => date(row.created_at),
                                    }
                                }
                            }
                        }
                    }
                }
                Authorizations { grants, count: row.active_grants, on_changed }
            },
        }
        AppDialogs { dialog, focus_fallback: "page-back", on_changed, on_deleted }
    }
}

/// Who has allowed the app, with "Revoke…" in each row's menu.
#[component]
fn Authorizations(
    grants: Resource<Result<Vec<GrantRow>, ServerFnError>>,
    count: i64,
    on_changed: Callback<()>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let retry = use_callback(move |_| grants.restart());
    let pending = grants.state()() != UseResourceState::Ready;
    // The authorization whose revocation is being confirmed, with its person.
    let mut revoking = use_signal(|| None::<(Uuid, String)>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let revoke = use_callback(move |_| {
        let Some((id, _)) = revoking.peek().clone() else {
            return;
        };
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = server_fns::revoke_oauth_grant(id.to_string()).await;
            busy.set(false);
            match result {
                Ok(()) => {
                    revoking.set(None);
                    on_changed.call(());
                }
                Err(_) => error.set(Some(tr.apps_revoke_failed())),
            }
        });
    });
    let keep = use_callback(move |_| {
        if !busy() {
            error.set(None);
            revoking.set(None);
        }
    });
    let person = revoking
        .read()
        .as_ref()
        .map(|(_, name)| name.clone())
        .unwrap_or_default();

    rsx! {
        section { aria_labelledby: "app-authorizations-title",
            div { class: "flex items-baseline gap-3 mb-3",
                h2 { id: "app-authorizations-title", class: "card-title m-0", {tr.apps_authorizations_title()} }
                span { class: "text-sm text-secondary", {tr.apps_authorizations_count(count)} }
            }
            DataTable { label: tr.apps_authorizations_title(), busy: pending && grants.read().is_some(),
                table { class: "name-table", aria_labelledby: "app-authorizations-title",
                    thead {
                        tr {
                            th { scope: "col", {tr.apps_column_person()} }
                            th { scope: "col", {tr.apps_column_access()} }
                            th { scope: "col", {tr.apps_column_allowed()} }
                            th { scope: "col", {tr.apps_column_last_used()} }
                            th { scope: "col", aria_label: tr.action_menu() }
                        }
                    }
                    tbody {
                        match &*grants.read() {
                            None => rsx! { TableSkeleton { columns: 5, label: tr.apps_authorizations_loading() } },
                            Some(Err(error)) => rsx! {
                                TableMessage { columns: 5,
                                    LoadError { error: error.clone(), what: tr.apps_authorizations_what(), retry, pending,
                                        inset: true, focus_on_recovery: "app-authorizations-title",
                                    }
                                }
                            },
                            Some(Ok(rows)) if rows.is_empty() => rsx! {
                                TableMessage { columns: 5,
                                    EmptyState { icon: "users", title: tr.apps_authorizations_empty(),
                                        body: tr.apps_authorizations_empty_hint(), inset: true,
                                    }
                                }
                            },
                            Some(Ok(rows)) => rsx! {
                                for grant in rows.iter() {
                                    tr { key: "{grant.id}",
                                        NameCell { secondary: rsx! { "{grant.person_email}" }, "{grant.person_name}" }
                                        td { {access_label(tr, &grant.scopes)} }
                                        td { class: "whitespace-nowrap", {date(grant.approved_at)} }
                                        td { class: "whitespace-nowrap",
                                            {grant.last_used_at.map_or_else(|| tr.apps_never(), date)}
                                        }
                                        td { class: "text-right",
                                            GrantActions { id: grant.id, person: grant.person_name.clone(), revoking }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
        ConfirmDialog { id: "grant-revoke-dialog", labelledby: "grant-revoke-title", open: revoking.read().is_some(),
            title: tr.apps_revoke_title(person.as_str()), confirm_label: tr.apps_revoke_confirm(),
            tone: ConfirmTone::Danger, busy: busy(), focus_fallback: Some("app-authorizations-title"),
            on_confirm: revoke, on_dismiss: keep,
            p { {tr.apps_revoke_body(person.as_str())} }
            if let Some(message) = error() {
                p { class: "alert alert-danger mt-3 mb-0", role: "alert", "{message}" }
            }
        }
    }
}

/// One authorization's ⋮ menu: "Revoke…", which asks first.
#[component]
fn GrantActions(id: Uuid, person: String, revoking: Signal<Option<(Uuid, String)>>) -> Element {
    let tr = crate::i18n::use_tr();
    let ask = {
        let person = person.clone();
        use_callback(move |_: MouseEvent| revoking.set(Some((id, person.clone()))))
    };
    rsx! {
        RowActions { id: "grant-actions-{id}", name: person,
            MenuItem { id: "grant-revoke-{id}", danger: true, onclick: ask, {tr.apps_revoke()} }
        }
    }
}
