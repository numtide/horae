use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::modal::Modal;
use crate::route::Route;
use crate::server_fns;

/// The server refuses with this status while the client has active projects.
const CONFLICT: u16 = 409;

/// The client whose archive confirmation is open.
#[derive(Clone, PartialEq)]
pub(super) struct ArchiveTarget {
    pub(super) id: Uuid,
    pub(super) name: String,
}

/// A failed archive of one client: its message, and whether it is the
/// active-projects refusal.
#[derive(Clone, PartialEq)]
struct ArchiveError {
    client: Uuid,
    message: String,
    refused: bool,
}

/// Confirms archiving one client. Only the client changes, so the text never
/// counts or names projects; a refusal stays here with a way to its projects.
/// `pending` is the client whose archive is in flight; the page reads it so an
/// ordinary refresh keeps that confirmation open. A result reaches only the
/// confirmation it was sent from, and any change of target discards its error.
#[component]
pub(super) fn ArchiveDialog(
    target: Signal<Option<ArchiveTarget>>,
    pending: Signal<Option<Uuid>>,
    focus_fallback: &'static str,
    on_archived: EventHandler<ArchiveTarget>,
) -> Element {
    let mut error = use_signal(|| None::<ArchiveError>);
    use_effect(move || {
        // Closing, an authority reset or another client discards the error.
        let _ = target.read();
        error.set(None);
    });
    let mut close = move || {
        if pending().is_none() {
            target.set(None);
        }
    };
    let current = target();
    let busy = pending().is_some();
    let shown_error = error().filter(|error| Some(error.client) == current.as_ref().map(|c| c.id));

    rsx! {
        Modal { id: "client-archive-dialog", labelledby: "client-archive-title",
            open: current.is_some(), busy, focus_fallback: Some(focus_fallback),
            on_dismiss: move |_| close(),
            if let Some(client) = current {
                h2 { id: "client-archive-title", class: "modal-title m-0 wrap-anywhere", "Archive {client.name}?" }
                // Only prose may break anywhere; button labels keep their words.
                div { class: "modal-body px-3 sm:px-6",
                    p { class: "text-sm text-secondary wrap-anywhere",
                        "Only the client is archived. Its projects must already be archived. Tracked time, invoices and import history are kept."
                    }
                    if let Some(failure) = shown_error {
                        div { class: "alert alert-danger", role: "alert",
                            p { class: "m-0 wrap-anywhere", "{failure.message}" }
                            if failure.refused {
                                Link { to: Route::ProjectsForClient { client: client.id.to_string() }, class: "font-semibold",
                                    "View this client's projects"
                                }
                            }
                        }
                    }
                    div { class: "modal-actions flex-wrap",
                        button { id: "client-archive-confirm", r#type: "button", class: "btn btn-primary",
                            disabled: busy,
                            onclick: move |_| {
                                if pending().is_some() { return; }
                                pending.set(Some(client.id));
                                error.set(None);
                                let client = client.clone();
                                spawn(async move {
                                    let result = server_fns::set_client_active(client.id.to_string(), false).await;
                                    pending.set(None);
                                    // An authority reset closes it before the result arrives.
                                    let still_open = target.peek().as_ref().is_some_and(|open| open.id == client.id);
                                    match result {
                                        Ok(_) => {
                                            if still_open {
                                                target.set(None);
                                            }
                                            on_archived.call(client);
                                        }
                                        Err(failure) if still_open => {
                                            let (message, refused) = match failure {
                                                ServerFnError::ServerError { message, code, .. } => (message, code == CONFLICT),
                                                other => (other.to_string(), false),
                                            };
                                            error.set(Some(ArchiveError { client: client.id, message, refused }));
                                        }
                                        Err(_) => {}
                                    }
                                });
                            },
                            if busy { "Archiving client…" } else { "Archive client" }
                        }
                        button { r#type: "button", class: "btn btn-secondary", disabled: busy,
                            onclick: move |_| close(), "Cancel"
                        }
                    }
                }
            }
        }
    }
}
