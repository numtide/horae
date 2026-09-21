use dioxus::prelude::*;

/// Card chrome for an inline create/edit form: uppercase title, the form's
/// error banner, then the fields.
#[component]
pub fn FormCard(title: String, error: ReadSignal<Option<String>>, children: Element) -> Element {
    rsx! {
        div { class: "card",
            div { class: "p-5",
                h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint", "{title}" }
                if let Some(err) = &*error.read() {
                    div { class: "alert alert-danger", "{err}" }
                }
                {children}
            }
        }
    }
}

/// A labelled field wrapper with an optional hint line below the control.
/// `id` becomes the label's `for`, pairing it with a control given the same id.
#[component]
pub fn FormGroup(
    label: String,
    #[props(default)] id: String,
    #[props(default)] hint: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "form-group",
            label { class: "form-label", r#for: if !id.is_empty() { "{id}" }, "{label}" }
            {children}
            if !hint.is_empty() {
                p { class: "form-hint", "{hint}" }
            }
        }
    }
}

/// A text input. `kind` maps to the HTML `type` (e.g. `text`, `number`, `email`).
/// `error_id` marks invalid input and links to an existing visible error message.
#[component]
pub fn Input(
    #[props(default = "text".to_string())] kind: String,
    #[props(default)] class: String,
    #[props(default)] id: String,
    #[props(default)] label: String,
    #[props(default)] error_id: Option<String>,
    #[props(default)] value: String,
    #[props(default)] placeholder: String,
    #[props(default)] disabled: bool,
    #[props(default)] readonly: bool,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        input {
            class: if class.is_empty() { "form-input".into() } else { format!("form-input {class}") },
            id: if !id.is_empty() { "{id}" },
            aria_label: if !label.is_empty() { "{label}" },
            aria_invalid: error_id.as_ref().map(|_| "true"),
            aria_describedby: error_id,
            r#type: "{kind}",
            value: "{value}",
            placeholder: "{placeholder}",
            disabled,
            readonly,
            oninput: move |e| oninput.call(e),
        }
    }
}

/// A multi-line text input.
#[component]
pub fn Textarea(
    #[props(default)] id: String,
    #[props(default)] value: String,
    #[props(default)] placeholder: String,
    #[props(default = 3)] rows: i64,
    #[props(default)] disabled: bool,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        textarea {
            class: "form-textarea",
            id: if !id.is_empty() { "{id}" },
            rows: "{rows}",
            placeholder: "{placeholder}",
            disabled,
            oninput: move |e| oninput.call(e),
            "{value}"
        }
    }
}

/// A select built from `(value, label)` option pairs; the option matching
/// `selected` is pre-selected.
#[component]
pub fn Select(
    options: Vec<(String, String)>,
    #[props(default)] id: String,
    #[props(default)] selected: String,
    #[props(default)] disabled: bool,
    #[props(default)] onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        select {
            class: "form-select",
            id: if !id.is_empty() { "{id}" },
            disabled,
            onchange: move |e| onchange.call(e),
            for (value , label) in options {
                option { value: "{value}", selected: value == selected, "{label}" }
            }
        }
    }
}
