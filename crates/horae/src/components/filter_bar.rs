use dioxus::prelude::*;

use super::icons::NavIcon;

/// The row under a list's page header: its filters together on the left, its
/// search on the right (`DESIGN.md`, *Selects, filters and search*).
#[component]
pub fn FilterBar(filters: Element, search: Option<Element>) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-4 mb-5",
            div { class: "flex flex-wrap items-center gap-3", {filters} }
            if let Some(search) = search {
                div { class: "flex-1" }
                {search}
            }
        }
    }
}

/// A list's search box, with the magnifier inset on the left. `label` is its
/// accessible name and placeholder.
#[component]
pub fn SearchField(id: &'static str, label: &'static str, query: Signal<String>) -> Element {
    let mut query = query;
    rsx! {
        div { class: "search-field w-form-select max-w-full",
            NavIcon { name: "search", class: "search-field-icon" }
            input {
                id,
                class: "form-input",
                r#type: "search",
                aria_label: label,
                placeholder: label,
                value: "{query}",
                oninput: move |event| query.set(event.value()),
            }
        }
    }
}
