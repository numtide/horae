use dioxus::prelude::*;
use std::collections::BTreeSet;
use uuid::Uuid;

use crate::components::badge::Badge;
use crate::components::button::{Button, ButtonVariant};
use crate::components::debounce::{SEARCH_DEBOUNCE, use_debounced};
use crate::components::filter_bar::{FilterBar, SearchField};
use crate::components::menu::{Menu, MenuDivider, MenuItem, RowActions};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, ErrorState, Loading, NoAccess};
use crate::components::session::use_session;
use crate::components::toast::{Toast, ToastContainer};
use crate::models::permission_editor::PermissionRequester;
use crate::route::Route;
use crate::server_fns;

#[path = "clients/filters.rs"]
mod filters;
pub use filters::ClientFilter;
use filters::{ClientScope, matches_query_currency};

#[path = "clients/detail.rs"]
mod detail;
pub use detail::ClientDetail;

#[path = "clients/form.rs"]
mod form;

#[path = "clients/status.rs"]
mod status;

/// The account a Clients page first loaded under.
type Binding = PermissionRequester;

/// Rejects a response for another account or organization than the page is
/// bound to, so a refresh never shows the previous identity's data.
fn bind(mut binding: Signal<Option<Binding>>, current: Binding) -> Result<(), ServerFnError> {
    if binding.peek().is_some_and(|expected| expected != current) {
        return Err(ServerFnError::new(
            "Your session changed. Reload this page to continue.",
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
pub fn ClientList(filter: ClientFilter) -> Element {
    let tr = crate::i18n::use_tr();
    let navigator = use_navigator();
    // Management view: include inactive clients so managers can reactivate them.
    // The search box answers at once; the URL follows when typing pauses.
    let mut query = use_signal(|| filter.search.clone());
    let settled = use_debounced(query, SEARCH_DEBOUNCE);
    // Only a settled search moves the URL. The filter is read from the
    // router at that moment, so Back to another search is never undone.
    use_effect(move || {
        let search = settled();
        if let Route::ClientList { filter: current } = dioxus::router::router().current::<Route>()
            && search != current.search
        {
            navigator.replace(Route::ClientList {
                filter: ClientFilter { search, ..current },
            });
        }
    });
    // Back, or a shared link, brings another search: show it in the box.
    let external = filter.search.clone();
    use_effect(use_reactive!(|external| {
        if external != *settled.peek() {
            query.set(external);
        }
    }));
    let scope = filter.scope;
    let currency_filter = filter.currency.clone();
    let go = {
        let current = filter.clone();
        move |next: ClientFilter| {
            if next != current {
                navigator.push(Route::ClientList { filter: next });
            }
        }
    };
    let pick_scope = {
        let go = go.clone();
        let current = filter.clone();
        use_callback(move |scope: ClientScope| {
            go(ClientFilter {
                scope,
                ..current.clone()
            })
        })
    };
    let pick_currency = {
        let go = go.clone();
        let current = filter.clone();
        use_callback(move |currency: String| {
            go(ClientFilter {
                currency,
                ..current.clone()
            })
        })
    };
    let clear_filters = use_callback(move |_: MouseEvent| {
        query.set(String::new());
        go(ClientFilter::cleared());
    });

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
        let overview = server_fns::client_overview(*binding.peek()).await?;
        bind(binding, overview.requester)?;
        Ok::<_, ServerFnError>(overview)
    });
    let retry = use_callback(move |_| clients.restart());
    let own = use_session().permissions;
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
    // `can_manage` reflects `ClientWriteAll`; every write rechecks it.
    let can_manage = overview.is_some_and(|overview| overview.can_manage);
    // The Harvest importer is for administrators; it rechecks that itself.
    let can_import = can_manage && super::is_administrator(&own.read());
    let denied = overview.is_some_and(|overview| !overview.can_read);
    let list = overview
        .filter(|overview| overview.can_read)
        .map(|overview| &overview.clients);
    let matching: Vec<_> = list
        .into_iter()
        .flatten()
        .filter(|row| matches_query_currency(row, &query(), &currency_filter))
        .collect();
    let count_scope = |status: ClientScope| {
        matching
            .iter()
            .filter(|row| status.matches(row.client.active))
            .count()
    };
    let scope_label = |status: ClientScope| {
        let count = count_scope(status) as i64;
        match status {
            ClientScope::Active => tr.clients_status_active(count),
            ClientScope::Inactive => tr.clients_status_archived(count),
            ClientScope::All => tr.clients_status_all(count),
        }
    };
    let scope_labels = ClientScope::ALL.map(|status| (status, scope_label(status)));
    let scope_shown = scope_label(scope);
    let currencies: BTreeSet<String> = list
        .into_iter()
        .flatten()
        .flat_map(|row| std::iter::once(&row.client.currency).chain(row.project_currencies.iter()))
        .map(|currency| currency.trim().to_owned())
        .collect();
    let currency_label = if currency_filter.is_empty() {
        tr.clients_currency_any()
    } else {
        currency_filter.clone()
    };
    let visible: Vec<_> = matching
        .into_iter()
        .filter(|row| scope.matches(row.client.active))
        .collect();

    let mut reset_form = move || {
        editing_id.set(None);
        show_form.set(false);
    };
    let on_edit = use_callback(move |id: Uuid| {
        editing_id.set(Some(id));
        show_form.set(true);
    });
    // One archive request at a time, even after its confirmation was closed by
    // a failed reload.
    let on_archive = use_callback(move |target: status::ArchiveTarget| {
        if archive_pending.peek().is_none() {
            archiving.set(Some(target));
        }
    });
    let on_restore = use_callback(move |(id, name): (Uuid, String)| {
        if !restoring.write().insert(id) {
            return;
        }
        spawn(async move {
            let result = server_fns::set_client_active(id.to_string(), true).await;
            restoring.write().remove(&id);
            match result {
                Ok(_) => {
                    action_error.set(None);
                    saved.set(Some(tr.clients_restored_toast(&name)));
                    refocus.set(Some(actions_trigger(id)));
                    clients.restart();
                }
                Err(error) => action_error.set(Some(error.to_string())),
            }
        });
    });

    rsx! {
        div {
            PageHeader { title: "Clients", title_id: "clients-heading",
                actions: rsx! {
                    if can_manage {
                        button {
                            id: "clients-new",
                            class: "btn btn-primary",
                            onclick: move |_| { editing_id.set(None); show_form.set(true); },
                            "New client"
                        }
                    }
                    if can_import {
                        Link { to: Route::ImporterIndex {}, class: "btn btn-secondary", "Import" }
                    }
                },
            }

            // Without access the screen keeps no controls. The menus wait for
            // the list, whose counts and currencies they show.
            if !denied {
                FilterBar {
                    filters: rsx! {
                        Menu { id: "client-scope-menu", label: scope_shown, trigger_class: "text-sm px-4", disabled: list.is_none(),
                            for (status, label) in scope_labels {
                                MenuItem { key: "{status:?}", selected: scope == status, onclick: move |_| pick_scope.call(status), "{label}" }
                            }
                        }
                        Menu { id: "client-currency-menu", label: currency_label, trigger_class: "text-sm px-4", disabled: list.is_none(),
                            MenuItem { selected: currency_filter.is_empty(), onclick: move |_| pick_currency.call(String::new()),
                                {tr.clients_currency_any()}
                            }
                            for code in currencies {
                                MenuItem { key: "{code}", selected: currency_filter == code,
                                    onclick: { let code = code.clone(); move |_| pick_currency.call(code.clone()) },
                                    "{code}"
                                }
                            }
                        }
                    },
                    search: rsx! { SearchField { id: "clients-search", label: tr.clients_search_label(), query } },
                }
            }

            form::ClientEditor { open: show_form() && can_manage, client_id: editing_id(), focus_fallback: "clients-heading",
                on_dismiss: move |_| reset_form(),
                on_saved: move |_| {
                    refocus.set(Some(match *editing_id.peek() {
                        Some(id) => actions_trigger(id),
                        None => "clients-new".to_owned(),
                    }));
                    reset_form();
                    clients.restart();
                },
            }

            status::ArchiveDialog { target: archiving, pending: archive_pending, focus_fallback: "clients-heading",
                on_archived: move |client: status::ArchiveTarget| {
                    action_error.set(None);
                    saved.set(Some(tr.clients_archived_toast(&client.name)));
                    refocus.set(Some(actions_trigger(client.id)));
                    clients.restart();
                },
            }
            ToastContainer {
                // Only beside the settled, bound list it describes.
                if let (true, Some(message)) = (can_manage, saved()) {
                    Toast { message, variant: "success", dismissible: true, ondismiss: move |_| saved.set(None) }
                }
            }

            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change client status: {message}" }
            }

            // A failed list stays up, its retry disabled, while the retry runs.
            if !ready && !matches!(&*data, Some(Err(_))) {
                Loading { label: "Loading clients…" }
            } else if let Some(Err(_)) = &*data {
                ErrorState {
                    title: "Could not load clients",
                    body: "Try again, or reload this page if your account changed.",
                    retry_id: "clients-retry",
                    retry_disabled: !ready,
                    focus_on_recovery: "clients-heading",
                    onretry: retry,
                }
            } else if denied {
                NoAccess {}
            } else if let Some(list) = list {
                if visible.is_empty() {
                    if list.is_empty() {
                        EmptyState {
                            icon: "clients",
                            title: "No clients yet",
                            body: "Clients group your projects and billing information.",
                            actions: rsx! {
                                if can_manage {
                                    button { class: "btn btn-primary", onclick: move |_| { reset_form(); show_form.set(true); }, "New client" }
                                }
                                if can_import {
                                    Link { to: Route::ImporterIndex {}, class: "btn btn-secondary", "Import from Harvest" }
                                }
                            },
                        }
                    } else {
                        EmptyState {
                            icon: "clients",
                            title: tr.clients_none_match(),
                            body: tr.clients_none_match_hint(),
                            actions: rsx! {
                                Button { id: "clients-clear-filters", variant: ButtonVariant::Secondary, label: tr.action_clear_filters(), onclick: clear_filters }
                            },
                        }
                    }
                } else {
                    div { class: "table-container rounded-xl bg-secondary", role: "region", aria_label: "Clients", tabindex: "0",
                        table { class: "name-table",
                            thead {
                                tr {
                                    th { scope: "col", "Client" }
                                    th { scope: "col", class: "text-right", "Currency" }
                                    th { scope: "col", class: "text-right", "Projects" }
                                    th { scope: "col", "Status" }
                                    if can_manage { th { scope: "col", class: "text-right", "Actions" } }
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
                                                td { class: "text-right text-mono text-secondary", "{row_currencies}" }
                                                td { class: "text-right text-mono whitespace-nowrap",
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
                                                if can_manage {
                                                    td { class: "text-right",
                                                        ClientActions { id: c.id, name: c.name.clone(), active: c.active,
                                                            restoring: restoring.read().contains(&c.id),
                                                            archive_pending: archive_pending().is_some(),
                                                            on_edit, on_archive, on_restore,
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

/// The id of a row's "Actions" trigger, where focus returns after a change.
fn actions_trigger(id: Uuid) -> String {
    format!("client-actions-{id}-trigger")
}

/// One client's "Actions" menu: Edit, then Archive or Restore.
#[component]
fn ClientActions(
    id: Uuid,
    name: String,
    active: bool,
    restoring: bool,
    archive_pending: bool,
    on_edit: Callback<Uuid>,
    on_archive: Callback<status::ArchiveTarget>,
    on_restore: Callback<(Uuid, String)>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let edit = use_callback(move |_: MouseEvent| on_edit.call(id));
    let label = name.clone();
    let change = use_callback(move |_: MouseEvent| {
        if active {
            on_archive.call(status::ArchiveTarget {
                id,
                name: name.clone(),
            });
        } else {
            on_restore.call((id, name.clone()));
        }
    });
    rsx! {
        RowActions { id: "client-actions-{id}", name: label, busy: restoring,
            MenuItem { id: "client-edit-{id}", onclick: edit, {tr.action_edit()} }
            MenuDivider {}
            MenuItem { id: "client-activity-{id}", disabled: if active { archive_pending } else { restoring }, onclick: change,
                if !active && restoring { {tr.action_restoring()} }
                else if active { {tr.action_archive()} }
                else { {tr.action_restore()} }
            }
        }
    }
}
