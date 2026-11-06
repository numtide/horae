use dioxus::prelude::*;

#[component]
pub(super) fn StatCell(value: u64, label: String, tone: String) -> Element {
    rsx! {
        div { class: "counter-stat {tone}",
            span { class: "counter-stat-value", "{value}" }
            span { class: "counter-stat-label", "{label}" }
        }
    }
}
