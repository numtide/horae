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
    use_future(move || async move {
        let mut events = document::eval(
            "addEventListener('horae-navigation-held', event => dioxus.send(event.detail));
             await new Promise(() => {});",
        );
        while let Ok(next) = events.recv::<Held>().await {
            held.set(Some(next));
        }
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
