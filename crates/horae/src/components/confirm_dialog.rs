use dioxus::prelude::*;

use super::button::{Button, ButtonVariant};
use super::icons::NavIcon;
use super::modal::{Modal, ModalBody, ModalFooter};

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
/// The buttons are `{id}-confirm` and `{id}-cancel`.
///
/// With `confirm_word`, the confirm button stays disabled until that word is
/// typed; keep it for the most serious actions.
///
/// `cancel_first` keeps the footer order a dialog had before it moved here,
/// until the footer order is decided (`DESIGN.md`, *Dialogs*).
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
    #[props(default)] confirm_word: Option<String>,
    #[props(default)] cancel_first: bool,
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
    let on_confirm_click = use_callback(move |_: MouseEvent| {
        if !busy && word_matches {
            on_confirm.call(());
        }
    });
    let on_cancel_click = use_callback(move |_: MouseEvent| on_dismiss.call(()));
    let confirm = rsx! {
        Button { id: "{id}-confirm", variant, label: confirm_label,
            disabled: busy || !word_matches, onclick: on_confirm_click,
        }
    };
    let cancel = rsx! {
        Button { id: "{id}-cancel", variant: ButtonVariant::Secondary, label: cancel_label,
            disabled: busy, onclick: on_cancel_click,
        }
    };

    rsx! {
        Modal { id, labelledby, open, busy, focus_fallback, on_dismiss,
            // Closed, it renders nothing a page or a test could mistake for content.
            if open {
                ModalBody {
                    div { class: "flex gap-4",
                        span { class: tile, aria_hidden: "true", NavIcon { name: icon } }
                        div { class: "min-w-0 flex-1 wrap-anywhere",
                            h2 { id: labelledby, class: "text-xl font-semibold", "{title}" }
                            div { class: "confirm-body", {children} }
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
                ModalFooter {
                    if cancel_first {
                        {cancel}
                        {confirm}
                    } else {
                        {confirm}
                        {cancel}
                    }
                }
            }
        }
    }
}
