use dioxus::prelude::*;

use crate::components::screen_state::Loading;

/// A full-width form selector using the shared menu surface and native popover.
/// An optional query signal connects the search field to the caller's catalog.
/// `error_id` links a rejected selection to its existing visible error message.
/// `label` completes the popover's accessible names ("Choose {label}").
#[component]
pub fn SelectField(
    id: String,
    label: String,
    options: Vec<(String, String)>,
    selected: String,
    #[props(default)] error_id: Option<String>,
    #[props(default)] disabled_values: Vec<String>,
    #[props(default)] query: Option<Signal<String>>,
    #[props(default)] pending: bool,
    #[props(default)] disabled: bool,
    onselect: EventHandler<String>,
    #[props(default)] children: Element,
) -> Element {
    let tr = crate::i18n::use_tr();
    let selected_label = options
        .iter()
        .find(|(value, _)| value == &selected)
        .map(|(_, text)| text.clone())
        .unwrap_or_else(|| tr.select_field_choose());
    let search = query
        .map(|query| query().trim().to_lowercase())
        .unwrap_or_default();
    rsx! {
        document::Script { src: asset!("/assets/js/menu.js") }
        button {
            id: "{id}", r#type: "button",
            class: "form-input flex items-center justify-between gap-2 text-left",
            popovertarget: "{id}-options", aria_haspopup: "dialog",
            aria_controls: "{id}-options", aria_expanded: "false",
            aria_invalid: error_id.as_ref().map(|_| "true"),
            aria_describedby: error_id,
            disabled,
            "data-select-trigger": "true",
            onclick: move |_| {
                if let Some(mut query) = query && !query.peek().is_empty() {
                    query.set(String::new());
                }
            },
            span { class: "truncate", "{selected_label}" }
            span { class: "text-subtle", aria_hidden: "true", "▾" }
        }
        div {
            id: "{id}-options", class: "menu menu-popover p-2",
            popover: "auto", role: "dialog", aria_label: tr.select_field_choose_label(&label),
            "data-select": "true", "data-popover-trigger": "{id}",
            if let Some(mut query) = query {
                input {
                    class: "form-input mb-2", r#type: "search",
                    aria_label: tr.select_field_search_label(&label), placeholder: tr.select_field_search_placeholder(), value: query(),
                    oninput: move |event| query.set(event.value()),
                }
            }
            div { role: "listbox", aria_label: tr.select_field_choose_label(&label), aria_busy: pending,
                for (value, text) in options.into_iter().filter(|(value, text)| value.is_empty() || search.is_empty() || text.to_lowercase().contains(&search)) {
                    button {
                        key: "{value}", id: "{id}-option-{value}", r#type: "button", role: "option",
                        class: if pending || disabled_values.contains(&value) { "menu-item disabled py-3" } else if value == selected { "menu-item selected py-3" } else { "menu-item py-3" },
                        aria_selected: value == selected,
                        disabled: pending || disabled_values.contains(&value),
                        onclick: move |_| onselect.call(value.clone()),
                        span { class: "block truncate", title: "{text}", "{text}" }
                    }
                }
            }
            if pending { Loading { label: tr.select_field_loading(), class: "text-subtle p-2" } }
            {children}
        }
    }
}
