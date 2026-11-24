use dioxus::prelude::*;
use serde::Deserialize;

use super::confirm_dialog::ConfirmDialog;
use super::modal::{Modal, ModalBody, ModalCancel, ModalHeader};

/// A navigation the guard (`assets/js/project-edit-navigation.js`) held: what
/// the editor has (`dirty` or `pending`) and which editor it is.
#[derive(Clone, Deserialize)]
struct Held {
    state: String,
    kind: String,
}

/// Says why leaving an editor was held. Unsaved changes can be discarded,
/// which repeats the navigation; work still being saved cannot.
#[component]
pub fn LeaveDialog() -> Element {
    let tr = crate::i18n::use_tr();
    let mut held = use_signal(|| None::<Held>);
    // Cancelling the event tells the guard it was heard. Should this loop end,
    // the listener goes too, and the guard lets navigation through rather than
    // hold it with nobody to say why. A navigation that went ahead after all
    // sends no detail, and closes the dialog.
    use_future(move || async move {
        let mut events = document::eval(
            "globalThis.horaeHeld = event => { dioxus.send(event.detail); event.preventDefault(); };
             globalThis.horaeMoved = () => dioxus.send(null);
             addEventListener('horae-navigation-held', horaeHeld);
             addEventListener('horae-navigation-moved', horaeMoved);
             await new Promise(() => {});",
        );
        while let Ok(next) = events.recv::<Option<Held>>().await {
            if next.is_some() || held.peek().is_some() {
                held.set(next);
            }
        }
        document::eval(
            "removeEventListener('horae-navigation-held', globalThis.horaeHeld);
             removeEventListener('horae-navigation-moved', globalThis.horaeMoved);",
        );
    });
    let stay = use_callback(move |()| held.set(None));
    let leave = use_callback(move |()| {
        held.set(None);
        document::eval("horaeLeave();");
    });
    let (kind, pending) = held
        .read()
        .as_ref()
        .map(|held| (held.kind.clone(), held.state == "pending"))
        .unzip();
    let kind = kind.unwrap_or_default();
    let pending = pending.unwrap_or_default();
    rsx! {
        ConfirmDialog { id: "leave-discard", labelledby: "leave-discard-title",
            open: held.read().is_some() && !pending, title: tr.leave_discard_title(kind.clone()),
            confirm_label: tr.leave_discard(), cancel_label: tr.leave_keep_editing(),
            on_confirm: leave, on_dismiss: stay,
            p { {tr.leave_discard_body()} }
        }
        Modal { id: "leave-pending", labelledby: "leave-pending-title", open: pending,
            cancel: ModalCancel::new(tr.leave_pending_stay()), on_dismiss: stay,
            ModalHeader { id: "leave-pending-title", title: tr.leave_pending_title() }
            ModalBody { p { {tr.leave_pending_body(kind)} } }
        }
    }
}
