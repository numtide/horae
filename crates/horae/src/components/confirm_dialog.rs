use dioxus::prelude::*;

use super::button::ButtonVariant;
use super::icons::NavIcon;
use super::modal::{Modal, ModalAction, ModalBody, ModalCancel};

/// Whether confirming loses data (`DESIGN.md`, *Destructive actions*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmTone {
    /// An irreversible action: a red tile and the Danger solid button.
    #[default]
    Danger,
    /// A significant action that can be undone, such as a bulk archive: a
    /// green tile and the Primary button.
    Neutral,
}

/// Confirms one destructive, irreversible or otherwise significant action.
/// `children` is the body: what will happen, plus any error of the last try.
/// The buttons are `{id}-confirm` and `{id}-cancel`, drawn by `Modal` in its
/// fixed order.
///
/// The Danger tone opens with Cancel focused, so a stray Enter does not
/// destroy anything; the body (`{id}-body`) describes the dialog.
///
/// With `confirm_word`, the confirm button stays disabled until that word is
/// typed; keep it for the most serious actions. No screen needs it yet, and it
/// stays as part of the component (`DESIGN.md`, *Dialogs*). `confirm_disabled`
/// holds the button while what it confirms is not ready, such as a preview
/// still loading.
#[component]
pub fn ConfirmDialog(
    id: &'static str,
    labelledby: &'static str,
    open: bool,
    title: String,
    confirm_label: String,
    #[props(default = "Cancel".to_string())] cancel_label: String,
    #[props(default)] tone: ConfirmTone,
    #[props(default)] busy: bool,
    #[props(default)] confirm_disabled: bool,
    #[props(default)] confirm_word: Option<String>,
    #[props(default)] focus_fallback: Option<&'static str>,
    on_confirm: EventHandler<()>,
    on_dismiss: EventHandler<()>,
    children: Element,
) -> Element {
    let mut typed = use_signal(String::new);
    // Each opening starts with an empty confirmation word.
    use_effect(use_reactive!(|open| {
        if !open {
            typed.set(String::new());
        }
    }));
    let word_matches = confirm_word
        .as_deref()
        .is_none_or(|word| typed.read().trim() == word);
    let (icon, variant) = match tone {
        ConfirmTone::Danger => ("warning", ButtonVariant::DangerSolid),
        ConfirmTone::Neutral => ("question", ButtonVariant::Primary),
    };
    let tile = match tone {
        ConfirmTone::Danger => "confirm-icon confirm-icon-danger",
        ConfirmTone::Neutral => "confirm-icon confirm-icon-neutral",
    };
    let word_id = format!("{id}-word");
    let ready = !busy && !confirm_disabled && word_matches;
    let confirm = use_callback(move |()| {
        if ready {
            on_confirm.call(());
        }
    });
    let primary = ModalAction::click(confirm_label, confirm)
        .variant(variant)
        .id(format!("{id}-confirm"))
        .disabled(!ready);
    let cancel = match tone {
        ConfirmTone::Danger => ModalCancel::new(cancel_label).autofocus(),
        ConfirmTone::Neutral => ModalCancel::new(cancel_label),
    };
    let body_id = format!("{id}-body");

    rsx! {
        Modal { id, labelledby, open, busy, focus_fallback, describedby: body_id.clone(), primary, cancel, on_dismiss,
            if open {
                ModalBody {
                    div { class: "flex gap-4",
                        span { class: tile, aria_hidden: "true", NavIcon { name: icon } }
                        div { class: "min-w-0 flex-1 wrap-anywhere",
                            h2 { id: labelledby, class: "text-xl font-semibold", "{title}" }
                            div { id: body_id, class: "confirm-body", {children} }
                            if let Some(word) = &confirm_word {
                                div { class: "mt-4",
                                    label { r#for: "{word_id}", class: "form-label",
                                        "Type " span { class: "text-mono", "{word}" } " to confirm"
                                    }
                                    input { id: "{word_id}", class: "form-input text-mono", value: "{typed}",
                                        autocomplete: "off", disabled: busy,
                                        oninput: move |event| typed.set(event.value()),
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
