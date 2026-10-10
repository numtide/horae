use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::menu::{Menu, MenuDivider};
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns;

const NEXT: &str = "permission-subject-next";
const PREVIOUS: &str = "permission-subject-previous";
const RETRY: &str = "permission-subject-retry";

/// Mount for one editor/requester generation so no page survives a target change.
#[component]
pub(super) fn SubjectPicker(
    requester: PermissionRequester,
    selected: Uuid,
    disabled: bool,
    on_selected: EventHandler<(Uuid, PermissionRequester)>,
) -> Element {
    let mut cursors = use_signal(|| vec![None::<Uuid>]);
    let mut page = use_resource(move || async move {
        let after = cursors.read().last().copied().flatten();
        let result = server_fns::list_permission_subjects(after)
            .await
            .map_err(|_| "Could not load people. Retry to check your current access.")?;
        if result.requester != requester {
            return Err(
                "Your session changed. Close and reopen permissions before choosing another person.",
            );
        }
        Ok(result)
    });
    let ready = page.state()() == UseResourceState::Ready;
    rsx! {
        div { class: "mb-4",
            if !ready {
                p { role: "status", class: "text-sm text-secondary", "Loading people…" }
            } else if let Some(Err(message)) = &*page.read() {
                p { role: "alert", class: "text-sm text-danger mb-2", "{message}" }
                button { id: RETRY, r#type: "button", class: "btn btn-secondary btn-sm",
                    disabled, onclick: move |_| { if !disabled { page.restart(); } }, "Retry people"
                }
            }
            Menu { id: "permission-subjects", label: "Change person",
                disabled: disabled || !ready || !matches!(&*page.read(), Some(Ok(_))),
                if ready {
                if let Some(Ok(value)) = &*page.read() {
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
