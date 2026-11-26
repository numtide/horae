use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::menu::{Menu, MenuDivider};
use crate::components::screen_state::Loading;
use crate::models::permission_editor::{PermissionRequester, PermissionSubjectPage};
use crate::server_fns;

const NEXT: &str = "permission-subject-next";
const PREVIOUS: &str = "permission-subject-previous";
const RETRY: &str = "permission-subject-retry";

/// The people a dialog's picker lists, one page per cursor in `cursors`.
pub(super) type SubjectPages = Resource<Option<Result<PermissionSubjectPage, &'static str>>>;

/// The dialog's subject pages, loaded while a person is open so the list
/// starts beside the editor instead of after it. Closing returns to the
/// first page.
pub(super) fn use_subject_pages(open: Memo<bool>) -> (SubjectPages, Signal<Vec<Option<Uuid>>>) {
    let mut cursors = use_signal(|| vec![None::<Uuid>]);
    use_effect(move || {
        if !open() && cursors.peek().len() > 1 {
            cursors.set(vec![None]);
        }
    });
    let page = use_resource(move || async move {
        if !open() {
            return None;
        }
        let after = cursors.read().last().copied().flatten();
        Some(
            server_fns::list_permission_subjects(after)
                .await
                .map_err(|_| "Could not load people. Try again to check your current access."),
        )
    });
    (page, cursors)
}

/// Lists `page` for the editor's `requester`; a page another account answered
/// is refused, so no page survives a target change.
#[component]
pub(super) fn SubjectPicker(
    requester: PermissionRequester,
    selected: Uuid,
    disabled: bool,
    on_selected: EventHandler<(Uuid, PermissionRequester)>,
    mut page: SubjectPages,
    mut cursors: Signal<Vec<Option<Uuid>>>,
) -> Element {
    let read = page.read();
    let current = read.as_ref().and_then(Option::as_ref).map(|result| match result {
        Ok(value) if value.requester != requester => Err(
            "Your session changed. Close and reopen permissions before choosing another person.",
        ),
        Ok(value) => Ok(value),
        Err(message) => Err(*message),
    });
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    rsx! {
        div { class: "mb-4",
            if !ready {
                Loading { label: "Loading people…" }
            } else if let Some(Err(message)) = current {
                p { role: "alert", class: "text-sm text-danger mb-2", "{message}" }
                button { id: RETRY, r#type: "button", class: "btn btn-secondary btn-sm",
                    disabled, onclick: move |_| { if !disabled { page.restart(); } }, "Try again"
                }
            }
            Menu { id: "permission-subjects", label: "Change person",
                disabled: disabled || !ready || !matches!(current, Some(Ok(_))),
                if ready {
                if let Some(Ok(value)) = current {
                    if value.subjects.is_empty() {
                        p { role: "status", class: "text-sm text-secondary px-3 py-2", "No people on this page." }
                    }
                    for subject in &value.subjects {
                        button {
                            key: "{subject.id}", id: "permission-subject-{subject.id}",
                            r#type: "button", role: "menuitem", tabindex: "-1",
                            class: if subject.id == selected { "menu-item selected wrap-anywhere" } else { "menu-item wrap-anywhere" },
                            aria_current: (subject.id == selected).then_some("true"), disabled,
                            onclick: {
                                let id = subject.id;
                                move |_| { if !disabled && id != selected { on_selected.call((id, requester)); } }
                            },
                            if subject.active { "{subject.name}" } else { "{subject.name} (inactive)" }
                        }
                    }
                    if cursors.read().len() > 1 || value.next_after.is_some() {
                        MenuDivider {}
                    }
                    if cursors.read().len() > 1 {
                        button { id: PREVIOUS, r#type: "button", role: "menuitem",
                            tabindex: "-1", class: "menu-item", disabled,
                            onclick: move |_| { if !disabled && cursors.peek().len() > 1 { cursors.write().pop(); } },
                            "Previous people"
                        }
                    }
                    if let Some(next) = value.next_after {
                        button { id: NEXT, r#type: "button", role: "menuitem",
                            tabindex: "-1", class: "menu-item", disabled,
                            onclick: move |_| { if !disabled { cursors.write().push(Some(next)); } },
                            "Next people"
                        }
                    }
                }
                }
            }
        }
    }
}
