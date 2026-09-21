use dioxus::prelude::*;

/// A segmented control: one active item out of several. Emits the chosen label.
#[component]
pub fn Segmented(
    items: Vec<String>,
    active: String,
    #[props(default)] onselect: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "segmented", role: "tablist",
            for item in items {
                button {
                    r#type: "button",
                    class: if item == active { "segmented-item active" } else { "segmented-item" },
                    "aria-selected": item == active,
                    onclick: {
                        let item = item.clone();
                        move |_| onselect.call(item.clone())
                    },
                    "{item}"
                }
            }
        }
    }
}

/// An on/off switch with a trailing label. A `<button>` so it is focusable and
/// toggles on Enter/Space without extra key handling.
#[component]
pub fn Toggle(
    on: bool,
    #[props(default)] label: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if on { "toggle on" } else { "toggle" },
            role: "switch",
            "aria-checked": "{on}",
            onclick: move |e| onclick.call(e),
            span { class: "toggle-track", span { class: "toggle-thumb" } }
            if !label.is_empty() {
                span { "{label}" }
            }
        }
    }
}

/// A checkbox with a label. Controlled via `checked`.
/// Optional `id` and `error_id` associate a field-specific validation message.
#[component]
pub fn Checkbox(
    checked: bool,
    label: String,
    #[props(default)] id: String,
    #[props(default)] error_id: Option<String>,
    #[props(default)] mixed: bool,
    #[props(default)] compact: bool,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if checked || mixed { "choice checked" } else { "choice" },
            role: "checkbox",
            id: if !id.is_empty() { "{id}" },
            aria_invalid: error_id.as_ref().map(|_| "true"),
            aria_describedby: error_id,
            aria_label: label.clone(),
            "aria-checked": if mixed { "mixed" } else if checked { "true" } else { "false" },
            disabled,
            onclick: move |e| onclick.call(e),
            span { class: "choice-box checkbox", aria_hidden: "true",
                if mixed { "−" } else if checked { "✓" }
            }
            if !compact { span { "{label}" } }
        }
    }
}

/// A radio option with a label. Controlled via `selected`.
#[component]
pub fn Radio(
    selected: bool,
    label: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if selected { "choice checked" } else { "choice" },
            role: "radio",
            "aria-checked": "{selected}",
            onclick: move |e| onclick.call(e),
            span { class: "choice-box radio" }
            span { "{label}" }
        }
    }
}
