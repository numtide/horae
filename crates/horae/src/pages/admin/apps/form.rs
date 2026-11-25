//! The dialogs of the apps pages: register or edit an app, confirm replacing
//! its secret or deleting it, and show a new secret once.

use dioxus::prelude::*;
use horae_core::permissions::oauth::{Scope, ScopeSet};

use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::controls::{Checkbox, Radio};
use crate::components::form::{FormGroup, Input, Textarea};
use crate::components::modal::{Modal, ModalAction, ModalBody, ModalCancel, ModalHeader};
use crate::components::revealed_secret::RevealedSecret;
use crate::models::oauth_apps::{AppInput, AppKind, AppRow, CreatedApp};
use crate::route::Route;
use crate::server_fns;

const BAD_REQUEST: u16 = 400;
const CONFLICT: u16 = 409;

/// The dialog open on an apps page, if any.
#[derive(Clone, PartialEq)]
pub(super) enum AppDialog {
    Register,
    Edit(AppRow),
    ReplaceSecret(AppRow),
    Delete(AppRow),
    /// A new secret, shown once. `registered` is a new app, whose page opens
    /// once the secret is stored.
    Secret {
        created: CreatedApp,
        registered: bool,
    },
}

/// Every dialog of an apps page. `on_changed` runs after a save, a new
/// secret or a deletion; `on_deleted` after a deletion only.
#[component]
pub(super) fn AppDialogs(
    dialog: Signal<Option<AppDialog>>,
    focus_fallback: &'static str,
    on_changed: Callback<()>,
    on_deleted: Callback<()>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let current = dialog.read().clone();
    let editing = match &current {
        Some(AppDialog::Register) => Some(None),
        Some(AppDialog::Edit(app)) => Some(Some(app.clone())),
        _ => None,
    };
    let replacing = match &current {
        Some(AppDialog::ReplaceSecret(app)) => Some(app.clone()),
        _ => None,
    };
    let deleting = match &current {
        Some(AppDialog::Delete(app)) => Some(app.clone()),
        _ => None,
    };
    let secret = match current {
        Some(AppDialog::Secret {
            created,
            registered,
        }) => Some((created, registered)),
        _ => None,
    };
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let close = use_callback(move |_| {
        if !busy() {
            error.set(None);
            dialog.set(None);
        }
    });
    let on_saved = use_callback(move |created: CreatedApp| {
        on_changed.call(());
        let registered = matches!(*dialog.peek(), Some(AppDialog::Register));
        match (registered, created.secret.is_some()) {
            (true, true) => dialog.set(Some(AppDialog::Secret {
                created,
                registered,
            })),
            // An app installed on devices has no secret: its page opens.
            (true, false) => {
                dialog.set(None);
                navigator().push(Route::AdminApp { id: created.app.id });
            }
            (false, _) => dialog.set(None),
        }
    });
    let replace = use_callback(move |_| {
        let Some(AppDialog::ReplaceSecret(app)) = dialog.peek().clone() else {
            return;
        };
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = server_fns::replace_oauth_app_secret(app.id.to_string()).await;
            busy.set(false);
            match result {
                Ok(created) => {
                    on_changed.call(());
                    dialog.set(Some(AppDialog::Secret {
                        created,
                        registered: false,
                    }));
                }
                Err(_) => error.set(Some(tr.apps_change_failed())),
            }
        });
    });
    let delete = use_callback(move |_| {
        let Some(AppDialog::Delete(app)) = dialog.peek().clone() else {
            return;
        };
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = server_fns::delete_oauth_app(app.id.to_string()).await;
            busy.set(false);
            match result {
                Ok(()) => {
                    dialog.set(None);
                    on_changed.call(());
                    on_deleted.call(());
                }
                Err(_) => error.set(Some(tr.apps_change_failed())),
            }
        });
    });
    let stored = use_callback(move |_| {
        let Some(AppDialog::Secret {
            created,
            registered,
        }) = dialog.peek().clone()
        else {
            return;
        };
        dialog.set(None);
        if registered {
            navigator().push(Route::AdminApp { id: created.app.id });
        }
    });

    rsx! {
        Modal { id: "app-editor-dialog", labelledby: "app-editor-title", open: editing.is_some(),
            busy: busy(), focus_fallback: Some(focus_fallback), large: true,
            primary: ModalAction::submit(
                match (&editing, busy()) {
                    (Some(None), true) => tr.apps_registering(),
                    (Some(None), false) => tr.apps_register(),
                    (_, true) => tr.apps_saving(),
                    (_, false) => tr.apps_save(),
                },
                "app-editor-form",
            ).id("app-editor-save").disabled(busy()),
            on_dismiss: close,
            ModalHeader { id: "app-editor-title",
                title: if matches!(editing, Some(Some(_))) { tr.apps_edit_title() } else { tr.apps_register_title() },
            }
            if let Some(initial) = editing {
                AppEditor { key: "{initial.as_ref().map(|app| app.id.to_string()).unwrap_or_default()}",
                    initial, busy, on_saved,
                }
            }
        }
        ConfirmDialog { id: "app-replace-dialog", labelledby: "app-replace-title", open: replacing.is_some(),
            title: tr.apps_replace_title(replacing.as_ref().map(|app| app.name.as_str()).unwrap_or_default()),
            confirm_label: tr.apps_replace_confirm(), tone: ConfirmTone::Danger, busy: busy(),
            focus_fallback: Some(focus_fallback),
            on_confirm: replace, on_dismiss: close,
            p { {tr.apps_replace_body()} }
            if let Some(message) = error() {
                p { class: "alert alert-danger mt-3 mb-0", role: "alert", "{message}" }
            }
        }
        ConfirmDialog { id: "app-delete-dialog", labelledby: "app-delete-title", open: deleting.is_some(),
            title: tr.apps_delete_title(deleting.as_ref().map(|app| app.name.as_str()).unwrap_or_default()),
            confirm_label: tr.apps_delete_confirm(), tone: ConfirmTone::Danger, busy: busy(),
            focus_fallback: Some(focus_fallback),
            on_confirm: delete, on_dismiss: close,
            p { {tr.apps_delete_body()} }
            if let Some(message) = error() {
                p { class: "alert alert-danger mt-3 mb-0", role: "alert", "{message}" }
            }
        }
        Modal { id: "app-secret-dialog", labelledby: "app-secret-title", open: secret.is_some(),
            focus_fallback: Some(focus_fallback),
            primary: ModalAction::click(tr.apps_secret_done(), stored).id("app-secret-done"),
            cancel: None::<ModalCancel>,
            on_dismiss: stored,
            if let Some((created, registered)) = secret {
                ModalHeader { id: "app-secret-title",
                    title: if registered {
                        tr.apps_secret_registered_title(created.app.name.as_str())
                    } else {
                        tr.apps_secret_replaced_title(created.app.name.as_str())
                    },
                }
                ModalBody {
                    p { class: "text-sm mb-4", {tr.apps_secret_body()} }
                    FormGroup { label: tr.apps_client_id(), id: "app-secret-client-id",
                        p { id: "app-secret-client-id", class: "text-mono text-sm m-0 wrap-anywhere", "{created.app.client_id}" }
                    }
                    FormGroup { label: tr.apps_secret(), id: "app-secret-value",
                        RevealedSecret { id: "app-secret-value", value: created.secret.clone().unwrap_or_default(),
                            label: tr.apps_secret(), copy_label: tr.apps_copy_secret(),
                        }
                    }
                }
            } else {
                ModalHeader { id: "app-secret-title", title: String::new() }
            }
        }
    }
}

/// One scope's checkbox, with its sentence; adding and changing time entries
/// says it includes seeing them.
#[component]
fn ScopeChoice(
    scope: Scope,
    checked: bool,
    invalid: bool,
    disabled: bool,
    on_toggle: Callback<Scope>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let toggle = use_callback(move |_: MouseEvent| on_toggle.call(scope));
    rsx! {
        div {
            Checkbox { id: format!("app-scope-{}", scope.as_str().replace(':', "-")),
                checked, label: super::scope_sentence(tr, scope),
                error_id: invalid.then(|| "app-scopes-error".to_string()),
                disabled, onclick: toggle,
            }
            if scope == Scope::TimeWrite {
                p { class: "form-hint ml-8 mt-0", {tr.apps_scope_write_hint()} }
            }
        }
    }
}

/// The register or edit form. A new app starts installed on devices, with
/// the time and projects access an app like Horae's own needs.
#[component]
fn AppEditor(
    initial: Option<AppRow>,
    busy: Signal<bool>,
    on_saved: Callback<CreatedApp>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let editing = initial.as_ref().map(|app| app.id);
    let mut name = use_signal(|| {
        initial
            .as_ref()
            .map(|app| app.name.clone())
            .unwrap_or_default()
    });
    let mut kind = use_signal(|| initial.as_ref().map_or(AppKind::Public, |app| app.kind));
    let mut redirects = use_signal(|| {
        initial
            .as_ref()
            .map(|app| app.redirect_uris.join("\n"))
            .unwrap_or_default()
    });
    let mut scopes = use_signal(|| match &initial {
        Some(app) => {
            ScopeSet::parse_each(app.scopes.iter().map(String::as_str)).unwrap_or_default()
        }
        None => ScopeSet::of(&[Scope::TimeWrite, Scope::CatalogRead]).normalize(),
    });
    // The server's message, and the field it names when it names one.
    let mut error = use_signal(|| None::<(Option<String>, String)>);
    let field_error = move |field: &str| match &*error.read() {
        Some((Some(name), message)) if name == field => Some(message.clone()),
        _ => None,
    };
    let toggle_scope = use_callback(move |scope: Scope| {
        let current = *scopes.peek();
        let next = if current.contains(scope) {
            // Without seeing entries an app cannot change them either.
            Scope::ALL
                .into_iter()
                .filter(|other| {
                    *other != scope && !(scope == Scope::TimeRead && *other == Scope::TimeWrite)
                })
                .filter(|other| current.contains(*other))
                .fold(ScopeSet::EMPTY, ScopeSet::with)
        } else {
            current.with(scope).normalize()
        };
        scopes.set(next);
    });
    let edit_name = use_callback(move |e: FormEvent| name.set(e.value()));
    let edit_redirects = use_callback(move |e: FormEvent| redirects.set(e.value()));
    let pick_public = use_callback(move |_: MouseEvent| kind.set(AppKind::Public));
    let pick_confidential = use_callback(move |_: MouseEvent| kind.set(AppKind::Confidential));
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if *busy.peek() {
            return;
        }
        busy.set(true);
        error.set(None);
        let input = AppInput {
            name: name.peek().clone(),
            kind: *kind.peek(),
            redirect_uris: redirects
                .peek()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect(),
            scopes: scopes
                .peek()
                .names()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };
        spawn(async move {
            let result = match editing {
                Some(id) => server_fns::update_oauth_app(id.to_string(), input)
                    .await
                    .map(|app| CreatedApp { app, secret: None }),
                None => server_fns::create_oauth_app(input).await,
            };
            busy.set(false);
            match result {
                Ok(created) => on_saved.call(created),
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST | CONFLICT,
                    message,
                    details,
                }) => {
                    let field = details
                        .as_ref()
                        .and_then(|details| details.get("field")?.as_str().map(str::to_owned));
                    error.set(Some((field, message)));
                }
                Err(_) => error.set(Some((None, tr.apps_save_failed()))),
            }
        });
    };
    let general_error = match &*error.read() {
        Some((field, message))
            if !matches!(field.as_deref(), Some("name" | "redirect_uris" | "scopes")) =>
        {
            Some(message.clone())
        }
        _ => None,
    };

    rsx! {
        ModalBody {
            form { id: "app-editor-form", class: "flex flex-col gap-4", novalidate: true, onsubmit: submit,
                if let Some(message) = general_error {
                    p { id: "app-editor-error", class: "alert alert-danger m-0", role: "alert", "{message}" }
                }
                FormGroup { label: tr.apps_name(), id: "app-name", hint: tr.apps_name_hint(),
                    Input { id: "app-name", value: name(), disabled: busy(),
                        error_id: field_error("name").map(|_| "app-name-error".to_string()),
                        oninput: edit_name,
                    }
                    if let Some(message) = field_error("name") {
                        p { id: "app-name-error", class: "text-sm text-danger mt-1 mb-0", "{message}" }
                    }
                }
                div { class: "form-group", role: "radiogroup", aria_labelledby: "app-kind-label",
                    span { id: "app-kind-label", class: "form-label", {tr.apps_type()} }
                    if editing.is_some() {
                        p { id: "app-kind", class: "m-0", {super::kind_label(tr, kind())} }
                        p { class: "form-hint", {tr.apps_type_fixed_hint()} }
                    } else {
                        div { class: "flex flex-col gap-2",
                            Radio { id: "app-kind-public", selected: kind() == AppKind::Public,
                                label: tr.apps_kind_public_option(), hint: tr.apps_kind_public_hint(),
                                onclick: pick_public,
                            }
                            Radio { id: "app-kind-confidential", selected: kind() == AppKind::Confidential,
                                label: tr.apps_kind_confidential_option(), hint: tr.apps_kind_confidential_hint(),
                                onclick: pick_confidential,
                            }
                        }
                    }
                }
                FormGroup { label: tr.apps_redirects(), id: "app-redirects", hint: tr.apps_redirects_hint(),
                    Textarea { id: "app-redirects", value: redirects(), rows: 3, disabled: busy(),
                        error_id: field_error("redirect_uris").map(|_| "app-redirects-error".to_string()),
                        oninput: edit_redirects,
                    }
                    if let Some(message) = field_error("redirect_uris") {
                        p { id: "app-redirects-error", class: "text-sm text-danger mt-1 mb-0", "{message}" }
                    }
                }
                div { class: "form-group", role: "group", aria_labelledby: "app-scopes-label",
                    span { id: "app-scopes-label", class: "form-label", {tr.apps_scopes()} }
                    div { class: "flex flex-col gap-2",
                        for scope in Scope::ALL {
                            ScopeChoice { key: "{scope.as_str()}", scope, checked: scopes().contains(scope),
                                invalid: field_error("scopes").is_some(), disabled: busy(), on_toggle: toggle_scope,
                            }
                        }
                    }
                    if let Some(message) = field_error("scopes") {
                        p { id: "app-scopes-error", class: "text-sm text-danger mt-1 mb-0", "{message}" }
                    }
                }
            }
        }
    }
}
