use dioxus::prelude::*;
use std::collections::BTreeSet;
use uuid::Uuid;

use super::is_admin;
use crate::components::badge::Badge;
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuItem};
use crate::components::toast::{Toast, ToastContainer};
use crate::models::permission_editor::PermissionRequester;
use crate::route::Route;
use crate::server_fns;

#[path = "clients/filters.rs"]
mod filters;
use filters::{ClientScope, matches_query_currency};

#[path = "clients/detail.rs"]
mod detail;
pub use detail::ClientDetail;

#[path = "clients/form.rs"]
mod form;

#[path = "clients/status.rs"]
mod status;

/// The account and permission policy a Clients page first loaded under.
type Binding = (PermissionRequester, bool);

/// Rejects a response for another account, organization or policy than the
/// page is bound to, so a refresh never shows the previous identity's data.
fn bind(mut binding: Signal<Option<Binding>>, current: Binding) -> Result<(), ServerFnError> {
    if binding.peek().is_some_and(|expected| expected != current) {
        return Err(ServerFnError::new(
            "Your session or client policy changed. Reload this page to continue.",
        ));
    }
    binding.set(Some(current));
    Ok(())
}

/// A save reloads the page, unmounting the opener before focus can return to
/// it. Focus the rebuilt opener, or the fallback when the opener is gone (for
/// example a row that left the current filter), unless the user has already
/// moved focus beyond the fallback during the reload.
fn restore_opener_focus(opener: &str, fallback: &str) {
    let opener = serde_json::json!(opener);
    let fallback = serde_json::json!(fallback);
    document::eval(&format!(
        "requestAnimationFrame(() => {{
            const active = document.activeElement;
            if (!active || active === document.body || active.id === {fallback}) {{
                (document.getElementById({opener}) ?? document.getElementById({fallback}))?.focus();
            }}
        }});"
    ));
}

#[component]
pub fn ClientList() -> Element {
    // Management view: include inactive clients so managers can reactivate them.
    let mut query = use_signal(String::new);
    let mut scope = use_signal(ClientScope::default);
    let mut currency_filter = use_signal(String::new);

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing client, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut action_error = use_signal(|| None::<String>);
    let mut archiving = use_signal(|| None::<status::ArchiveTarget>);
    // The client whose archive is in flight, shared with the confirmation.
    let archive_pending = use_signal(|| None::<Uuid>);
    // Clients whose restore is in flight; each control ignores its own repeats.
    let mut restoring = use_signal(BTreeSet::<Uuid>::new);
    let mut saved = use_signal(|| None::<String>);
    // The opener to refocus after a save's reload; see `restore_opener_focus`.
    let mut refocus = use_signal(|| None::<String>);

    let binding = use_signal(|| None::<Binding>);
    let mut clients = use_resource(move || async move {
        // An editor or idle confirmation opened under the previous authority must
        // not survive a refresh. An archive already in flight keeps its
        // confirmation through an ordinary refresh; the settled result decides.
        editing_id.set(None);
        show_form.set(false);
        if archive_pending.peek().is_none() {
            archiving.set(None);
        }
        let expected = binding.peek().as_ref().map(|(requester, _)| *requester);
        let overview = server_fns::client_overview(expected).await?;
        bind(
            binding,
            (overview.requester, overview.canonical_permissions),
        )?;
        Ok::<_, ServerFnError>(overview)
    });
    let me = use_resource(|| async move { server_fns::get_me().await });
    use_effect(move || {
        if clients.state()() != UseResourceState::Ready {
            return;
        }
        let managed = matches!(&*clients.peek(), Some(Ok(overview)) if overview.can_manage);
        // A notice or confirmation names a client of the bound account; a failed,
        // denied or rebound reload must not keep it, even while it is pending.
        if !managed {
            saved.set(None);
            archiving.set(None);
        }
        // One-shot: a failed, denied or rebound reload consumes it without focusing.
        if let Some(opener) = refocus.take()
            && managed
        {
            restore_opener_focus(&opener, "clients-heading");
        }
    });

    let ready = clients.state()() == UseResourceState::Ready;
    let data = clients.read();
    // A reloading resource keeps its previous value; never act on it.
    let overview = ready
        .then(|| data.as_ref().and_then(|result| result.as_ref().ok()))
        .flatten();
    // Client writes are not yet integrated with canonical permissions, so the
    // server grants `can_manage` only to legacy-policy managers.
    let is_manager = overview.is_some_and(|overview| overview.can_manage);
    let can_import = is_manager && is_admin(&me);
    let denied = overview.is_some_and(|overview| !overview.can_read);
    let list = overview
        .filter(|overview| overview.can_read)
        .map(|overview| &overview.clients);
    let matching: Vec<_> = list
        .into_iter()
        .flatten()
        .filter(|row| matches_query_currency(row, &query(), &currency_filter()))
        .collect();
    let count_scope = |status: ClientScope| {
        matching
            .iter()
            .filter(|row| status.matches(row.client.active))
            .count()
    };
    let active_count = count_scope(ClientScope::Active);
    let inactive_count = count_scope(ClientScope::Inactive);
    let all_count = matching.len();
    let scope_label = format!("{} ({})", scope().label(), count_scope(scope()));
    let currencies: BTreeSet<String> = list
        .into_iter()
        .flatten()
        .flat_map(|row| std::iter::once(&row.client.currency).chain(row.project_currencies.iter()))
        .map(|currency| currency.trim().to_owned())
        .collect();
    let currency_label = if currency_filter().is_empty() {
        "All currencies".to_owned()
    } else {
        currency_filter()
    };
    let visible: Vec<_> = matching
        .into_iter()
        .filter(|row| scope().matches(row.client.active))
        .collect();

    let mut reset_form = move || {
        editing_id.set(None);
        show_form.set(false);
    };

    rsx! {
        div {
            div { class: "page-header mb-5",
                h1 { id: "clients-heading", tabindex: "-1", class: "page-title text-4xl font-semibold text-strong tracking-tight", "Clients" }
                div { class: "page-actions items-center gap-4",
                    if is_manager {
                        button {
                            id: "clients-new",
                            class: "btn btn-primary",
                            onclick: move |_| { editing_id.set(None); show_form.set(true); },
                            "New client"
                        }
                    }
                    if can_import {
                        Link { to: Route::HarvestImport {}, class: "btn btn-secondary", "Import" }
                    }
                    input {
                        class: "form-input client-search w-form-select max-w-full",
                        r#type: "search",
                        aria_label: "Search clients by name",
                        placeholder: "Search clients by name",
                        value: "{query}",
                        oninput: move |event| query.set(event.value()),
                    }
                }
            }

            if ready && list.is_some() {
                div { class: "flex flex-wrap items-center gap-4 mb-5",
                    Menu { id: "client-scope-menu", label: scope_label, trigger_class: "text-sm px-4",
                        MenuItem { selected: scope() == ClientScope::Active,
                            onclick: move |_| scope.set(ClientScope::Active), "Active clients ({active_count})" }
                        MenuItem { selected: scope() == ClientScope::Inactive,
                            onclick: move |_| scope.set(ClientScope::Inactive), "Archived clients ({inactive_count})" }
                        MenuItem { selected: scope() == ClientScope::All,
                            onclick: move |_| scope.set(ClientScope::All), "All clients ({all_count})" }
                    }
                    div { class: "flex-1" }
                    Menu { id: "client-currency-menu", label: currency_label, align_right: true, trigger_class: "text-sm px-4",
                        MenuItem { selected: currency_filter().is_empty(),
                            onclick: move |_| currency_filter.set(String::new()), "All currencies" }
                        for code in currencies {
                            MenuItem { key: "{code}", selected: currency_filter() == code,
                                onclick: { let code = code.clone(); move |_| currency_filter.set(code.clone()) },
                                "{code}"
                            }
                        }
                    }
                }
            }

            form::ClientEditor { open: show_form() && is_manager, client_id: editing_id(), focus_fallback: "clients-heading",
                on_dismiss: move |_| reset_form(),
                on_saved: move |_| {
                    refocus.set(Some(match *editing_id.peek() {
                        Some(id) => format!("client-edit-{id}"),
                        None => "clients-new".to_owned(),
                    }));
                    reset_form();
                    clients.restart();
                },
            }

            status::ArchiveDialog { target: archiving, pending: archive_pending, focus_fallback: "clients-heading",
                on_archived: move |client: status::ArchiveTarget| {
                    action_error.set(None);
                    saved.set(Some(format!("{} archived", client.name)));
                    refocus.set(Some(format!("client-activity-{}", client.id)));
                    clients.restart();
                },
            }
            ToastContainer {
                // Only beside the settled, bound list it describes.
                if let (true, Some(message)) = (is_manager, saved()) {
                    Toast { message, variant: "success", dismissible: true, ondismiss: move |_| saved.set(None) }
                }
            }

            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change client status: {message}" }
            }

            if !ready {
                p { class: "text-sm text-secondary", role: "status", aria_busy: "true", "Loading clients…" }
            } else if let Some(Err(_)) = &*data {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load clients. Retry, or reload this page if your account changed. "
                    button { id: "clients-retry", r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| clients.restart(), "Retry clients" }
                }
            } else if denied {
                div { class: "empty-state rounded-xl",
                    div { class: "empty-state-icon-tile", aria_hidden: "true", NavIcon { name: "clients" } }
                    h2 { class: "empty-state-title text-xl m-0", "Client access is unavailable" }
                    p { class: "empty-state-text m-0", "Your permissions do not include the client list." }
                }
            } else if let Some(list) = list {
                if visible.is_empty() {
                    div { class: "empty-state rounded-xl",
                        div { class: "empty-state-icon-tile", aria_hidden: "true", NavIcon { name: "clients" } }
                        if list.is_empty() {
                            h2 { class: "empty-state-title text-xl m-0", "No clients yet" }
                            p { class: "empty-state-text m-0", "Clients group your projects and billing information." }
                            div { class: "flex flex-wrap justify-center gap-3",
                                if is_manager {
                                    button { class: "btn btn-primary", onclick: move |_| { reset_form(); show_form.set(true); }, "New client" }
                                }
                                if can_import {
                                    Link { to: Route::HarvestImport {}, class: "btn btn-secondary", "Import from Harvest" }
                                }
                            }
                        } else {
                            h2 { class: "empty-state-title text-xl m-0", "No clients match your filters" }
                            p { class: "empty-state-text m-0", "Try another name, status or currency." }
                            button { class: "btn btn-secondary", onclick: move |_| {
                                query.set(String::new()); currency_filter.set(String::new()); scope.set(ClientScope::All);
                            }, "Clear filters" }
                        }
                    }
                } else {
                    div { class: "table-container rounded-xl bg-secondary", role: "region", aria_label: "Clients", tabindex: "0",
                        table { class: "client-table",
                            thead {
                                tr {
                                    th { scope: "col", class: "text-secondary", "Client" }
                                    th { scope: "col", class: "text-right text-secondary", "Currency" }
                                    th { scope: "col", class: "text-right text-secondary", "Projects" }
                                    th { scope: "col", class: "text-secondary", "Status" }
                                    if is_manager { th { scope: "col", class: "text-secondary", "Actions" } }
                                }
                            }
                            tbody {
                                for summary in visible {
                                    {
                                        let c = summary.client.clone();
                                        let row_currencies: BTreeSet<_> = std::iter::once(c.currency.trim())
                                            .chain(summary.project_currencies.iter().map(|c| c.trim())).collect();
                                        let row_currencies = row_currencies.into_iter().collect::<Vec<_>>().join(" / ");
                                        rsx! {
                                            tr { key: "{c.id}",
                                                td { Link { to: Route::ClientDetail { id: c.id }, class: "font-semibold text-strong wrap-anywhere", "{c.name}" } }
                                                td { class: "text-right font-mono text-secondary", "{row_currencies}" }
                                                td { class: "text-right font-mono whitespace-nowrap",
                                                    span { aria_label: "{summary.active_projects} active of {summary.total_projects} visible projects",
                                                        "{summary.active_projects} / {summary.total_projects}"
                                                    }
                                                }
                                                td {
                                                    if c.active {
                                                        Badge { variant: "success", "Active" }
                                                    } else {
                                                        Badge { variant: "neutral", "Archived" }
                                                    }
                                                }
                                                if is_manager {
                                                    td {
                                                        div { class: "flex items-center gap-2",
                                                        button {
                                                            id: "client-edit-{c.id}",
                                                            class: "btn btn-secondary btn-sm",
                                                            onclick: {
                                                                let c = c.clone();
                                                                move |_| {
                                                                    editing_id.set(Some(c.id));
                                                                    show_form.set(true);
                                                                }
                                                            },
                                                            "Edit"
                                                        }
                                                        if c.active {
                                                            button {
                                                                id: "client-activity-{c.id}",
                                                                r#type: "button",
                                                                class: "btn btn-secondary btn-sm",
                                                                // One archive request at a time, even after its
                                                                // confirmation was closed by a failed reload.
                                                                disabled: archive_pending().is_some(),
                                                                onclick: {
                                                                    let target = status::ArchiveTarget { id: c.id, name: c.name.clone() };
                                                                    move |_| {
                                                                        if archive_pending().is_none() {
                                                                            archiving.set(Some(target.clone()));
                                                                        }
                                                                    }
                                                                },
                                                                "Archive"
                                                            }
                                                        } else {
                                                            button {
                                                                id: "client-activity-{c.id}",
                                                                r#type: "button",
                                                                class: "btn btn-secondary btn-sm",
                                                                disabled: restoring.read().contains(&c.id),
                                                                aria_busy: restoring.read().contains(&c.id),
                                                                onclick: {
                                                                    let (id, name) = (c.id, c.name.clone());
                                                                    move |_| {
                                                                        if !restoring.write().insert(id) { return; }
                                                                        let name = name.clone();
                                                                        spawn(async move {
                                                                            let result = server_fns::set_client_active(id.to_string(), true).await;
                                                                            restoring.write().remove(&id);
                                                                            match result {
                                                                                Ok(_) => {
                                                                                    action_error.set(None);
                                                                                    saved.set(Some(format!("{name} restored")));
                                                                                    refocus.set(Some(format!("client-activity-{id}")));
                                                                                    clients.restart();
                                                                                }
                                                                                Err(error) => action_error.set(Some(error.to_string())),
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                if restoring.read().contains(&c.id) { "Restoring…" } else { "Restore" }
                                                            }
                                                        }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
