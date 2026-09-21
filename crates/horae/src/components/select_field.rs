use dioxus::prelude::*;

/// A full-width form selector using the shared menu surface and native popover.
/// An optional query signal connects the search field to the caller's catalog.
#[component]
pub fn SelectField(
    id: String,
    label: String,
    options: Vec<(String, String)>,
    selected: String,
    #[props(default)] disabled_values: Vec<String>,
    #[props(default)] query: Option<Signal<String>>,
    #[props(default)] pending: bool,
    onselect: EventHandler<String>,
    #[props(default)] children: Element,
) -> Element {
    let selected_label = options
        .iter()
        .find(|(value, _)| value == &selected)
        .map(|(_, text)| text.as_str())
        .unwrap_or("Choose…")
        .to_owned();
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
            popover: "auto", role: "dialog", aria_label: "Choose {label}",
            "data-select": "true", "data-popover-trigger": "{id}",
            if let Some(mut query) = query {
                input {
                    class: "form-input mb-2", r#type: "search",
                    aria_label: "Search {label}", placeholder: "Search…", value: query(),
                    oninput: move |event| query.set(event.value()),
                }
            }
            div { role: "listbox", aria_label: "Choose {label}", aria_busy: pending,
                for (value, text) in options.into_iter().filter(|(value, text)| value.is_empty() || search.is_empty() || text.to_lowercase().contains(&search)) {
                    button {
                        key: "{value}", r#type: "button", role: "option",
                        class: if pending || disabled_values.contains(&value) { "menu-item disabled py-3" } else if value == selected { "menu-item selected py-3" } else { "menu-item py-3" },
                        aria_selected: value == selected,
                        disabled: pending || disabled_values.contains(&value),
                        onclick: move |_| onselect.call(value.clone()),
                        span { class: "block truncate", title: "{text}", "{text}" }
                    }
                }
            }
            if pending { p { class: "text-sm text-subtle p-2 m-0", role: "status", "Loading choices…" } }
            {children}
        }
    }
}
