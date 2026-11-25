//! OAuth apps in administration (spec 024, PR D): the list, each app's page
//! with its authorizations, and the dialogs both share. Horae's own Android
//! app is listed first; it can be enabled and disabled, never edited or
//! deleted.

use dioxus::prelude::*;
use horae_core::permissions::oauth::Scope;
use uuid::Uuid;

use crate::components::badge::Badge;
use crate::components::menu::{MenuDivider, MenuItem, RowActions};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::LoadError;
use crate::components::table::{DataTable, NameCell, TableMessage, TableSkeleton};
use crate::i18n::Tr;
use crate::models::oauth_apps::{AppKind, AppRow};
use crate::route::Route;
use crate::server_fns;

// Explicit paths, so the UI tests can include this file with `#[path]`.
#[path = "apps/detail.rs"]
mod detail;
#[path = "apps/form.rs"]
mod form;

pub use detail::AdminApp;
use form::{AppDialog, AppDialogs};

#[component]
pub fn AdminApps() -> Element {
    let tr = crate::i18n::use_tr();
    let mut apps = use_resource(server_fns::list_oauth_apps);
    let can_connect = use_resource(server_fns::oauth_apps_can_connect);
    let retry = use_callback(move |_| apps.restart());
    let pending = apps.state()() != UseResourceState::Ready;
    let mut dialog = use_signal(|| None::<AppDialog>);
    let mut change_error = use_signal(|| None::<String>);
    let toggling = use_signal(|| None::<Uuid>);
    let on_toggle = use_callback(move |(id, enabled): (Uuid, bool)| {
        change_error.set(None);
        toggle(id, enabled, toggling, change_error, tr, move || {
            apps.restart()
        });
    });
    let on_changed = use_callback(move |_| apps.restart());
    let open_register = move |_| dialog.set(Some(AppDialog::Register));

    rsx! {
        PageHeader { title: tr.apps_title(), title_id: "apps-title",
            actions: rsx! {
                button { id: "apps-register", r#type: "button", class: "btn btn-primary", onclick: open_register,
                    {tr.apps_register()}
                }
            },
        }
        p { class: "text-secondary mb-4", {tr.apps_intro()} }
        if let Some(Ok(false)) = *can_connect.read() {
            div { id: "apps-not-ready", class: "alert alert-warning mb-4", role: "status",
                p { class: "font-semibold mb-1", {tr.apps_not_ready_title()} }
                p { class: "text-sm m-0", {tr.apps_not_ready_body()} }
            }
        }
        if let Some(message) = change_error() {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        DataTable { label: tr.apps_table_label(), busy: pending && apps.read().is_some(),
            table { aria_label: tr.apps_table_label(),
                thead {
                    tr {
                        th { scope: "col", {tr.apps_column_app()} }
                        th { scope: "col", {tr.apps_column_type()} }
                        th { scope: "col", {tr.apps_column_access()} }
                        th { scope: "col", {tr.apps_column_status()} }
                        th { scope: "col", class: "text-right", {tr.apps_column_authorizations()} }
                        th { scope: "col", aria_label: tr.action_menu() }
                    }
                }
                tbody {
                    match &*apps.read() {
                        None => rsx! { TableSkeleton { columns: 6, label: tr.apps_loading() } },
                        Some(Err(error)) => rsx! {
                            TableMessage { columns: 6,
                                LoadError { error: error.clone(), what: tr.apps_what(), retry, pending, inset: true,
                                    focus_on_recovery: "apps-title",
                                }
                            }
                        },
                        Some(Ok(rows)) => rsx! {
                            for app in rows.iter() {
                                tr { key: "{app.id}",
                                    NameCell { secondary: rsx! { {app_secondary(tr, app)} },
                                        Link { to: Route::AdminApp { id: app.id }, id: "app-link-{app.id}", "{app.name}" }
                                    }
                                    td { class: "text-secondary", {kind_label(tr, app.kind)} }
                                    td { {access_label(tr, &app.scopes)} }
                                    td { StatusBadge { enabled: app.enabled } }
                                    td { class: "text-right text-mono", "{app.active_grants}" }
                                    td { class: "text-right",
                                        RowActions { id: "app-actions-{app.id}", name: app.name.clone(),
                                            busy: toggling() == Some(app.id),
                                            AppMenuItems { app: app.clone(), dialog, on_toggle, toggling: toggling().is_some(),
                                                with_open: true,
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
        AppDialogs { dialog, focus_fallback: "apps-register", on_changed, on_deleted: on_changed }
    }
}

/// The actions both the list's row menu and the app page's menu offer:
/// Edit, Replace secret, Enable or Disable, then Delete, after Open on the
/// list. Horae's own app has only Enable or Disable.
#[component]
fn AppMenuItems(
    app: AppRow,
    dialog: Signal<Option<AppDialog>>,
    on_toggle: Callback<(Uuid, bool)>,
    toggling: bool,
    #[props(default)] with_open: bool,
) -> Element {
    let tr = crate::i18n::use_tr();
    let id = app.id;
    let enabled = app.enabled;
    let open = use_callback(move |_: MouseEvent| {
        navigator().push(Route::AdminApp { id });
    });
    let edit = {
        let app = app.clone();
        use_callback(move |_: MouseEvent| dialog.set(Some(AppDialog::Edit(app.clone()))))
    };
    let replace = {
        let app = app.clone();
        use_callback(move |_: MouseEvent| dialog.set(Some(AppDialog::ReplaceSecret(app.clone()))))
    };
    let delete = {
        let app = app.clone();
        use_callback(move |_: MouseEvent| dialog.set(Some(AppDialog::Delete(app.clone()))))
    };
    let toggle = use_callback(move |_: MouseEvent| on_toggle.call((id, !enabled)));
    rsx! {
        if with_open {
            MenuItem { id: "app-open-{id}", onclick: open, {tr.apps_open()} }
        }
        if !app.built_in {
            MenuItem { id: "app-edit-{id}", onclick: edit, {tr.apps_edit()} }
        }
        if app.kind == AppKind::Confidential {
            MenuItem { id: "app-replace-secret-{id}", onclick: replace, {tr.apps_replace_secret()} }
        }
        MenuItem { id: "app-toggle-{id}", disabled: toggling, onclick: toggle,
            if enabled { {tr.apps_disable()} } else { {tr.apps_enable()} }
        }
        if !app.built_in {
            MenuDivider {}
            MenuItem { id: "app-delete-{id}", danger: true, onclick: delete, {tr.apps_delete()} }
        }
    }
}

/// Enables or disables an app, then reloads. One change at a time.
fn toggle(
    id: Uuid,
    enabled: bool,
    mut toggling: Signal<Option<Uuid>>,
    mut error: Signal<Option<String>>,
    tr: Tr,
    reload: impl FnOnce() + 'static,
) {
    if toggling.peek().is_some() {
        return;
    }
    toggling.set(Some(id));
    spawn(async move {
        let result = server_fns::set_oauth_app_enabled(id.to_string(), enabled).await;
        toggling.set(None);
        match result {
            Ok(_) => reload(),
            Err(_) => error.set(Some(tr.apps_change_failed())),
        }
    });
}

#[component]
fn StatusBadge(enabled: bool) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        if enabled {
            Badge { variant: "success", {tr.apps_status_enabled()} }
        } else {
            Badge { variant: "neutral", {tr.apps_status_disabled()} }
        }
    }
}

/// Under the name: whose app it is or when it was registered, then the start
/// of its client id.
fn app_secondary(tr: Tr, app: &AppRow) -> String {
    let origin = if app.built_in {
        tr.apps_built_in()
    } else {
        tr.apps_registered_on(date(app.created_at))
    };
    let client_id: String = app.client_id.chars().take(14).collect();
    let ellipsis = if client_id.len() < app.client_id.len() {
        "…"
    } else {
        ""
    };
    format!("{origin} · {client_id}{ellipsis}")
}

fn kind_label(tr: Tr, kind: AppKind) -> String {
    match kind {
        AppKind::Public => tr.apps_kind_public(),
        AppKind::Confidential => tr.apps_kind_confidential(),
    }
}

/// The access an app may ask for, in short words ("Time, projects").
fn access_label(tr: Tr, scopes: &[String]) -> String {
    let has = |scope: Scope| scopes.iter().any(|name| name == scope.as_str());
    let time = if has(Scope::TimeWrite) {
        "write"
    } else if has(Scope::TimeRead) {
        "read"
    } else {
        "none"
    };
    let catalog = if has(Scope::CatalogRead) { "yes" } else { "no" };
    tr.apps_access(catalog, time)
}

/// The sentence that says what a scope lets an app do, shared with the
/// approval page.
fn scope_sentence(tr: Tr, scope: Scope) -> String {
    match scope {
        Scope::TimeRead => tr.oauth_scope_time_read(),
        Scope::TimeWrite => tr.oauth_scope_time_write(),
        Scope::CatalogRead => tr.oauth_scope_catalog_read(),
    }
}

fn date(at: chrono::DateTime<chrono::Utc>) -> String {
    at.format("%d %b %Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn names(scopes: &[Scope]) -> Vec<String> {
        scopes
            .iter()
            .map(|scope| scope.as_str().to_owned())
            .collect()
    }

    #[test]
    fn access_is_put_in_short_words() {
        let tr = Tr::new(Lang::En);
        let cases = [
            (
                vec![Scope::TimeRead, Scope::TimeWrite, Scope::CatalogRead],
                "Time, projects",
            ),
            (
                vec![Scope::TimeRead, Scope::CatalogRead],
                "Time (read), projects",
            ),
            (vec![Scope::TimeRead], "Time (read)"),
            (vec![Scope::CatalogRead], "Projects"),
        ];
        for (scopes, words) in cases {
            assert_eq!(access_label(tr, &names(&scopes)), words);
        }
    }
}
