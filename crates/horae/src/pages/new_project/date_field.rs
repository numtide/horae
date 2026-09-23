use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::components::date_picker::DatePicker;
use crate::components::icons::NavIcon;

/// Advisory planning dates keep ISO values in the draft and use the shared calendar.
#[component]
pub(super) fn DateField(
    id: String,
    label: String,
    placeholder: String,
    value: String,
    #[props(default)] error_id: Option<String>,
    onchange: EventHandler<String>,
) -> Element {
    let mut opening = use_signal(|| 0_u64);
    let selected = NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok();
    let display = selected
        .map(|date| date.format("%d %b %Y").to_string())
        // Preserve invalid recovered input so replacing it remains an explicit edit.
        .unwrap_or_else(|| {
            if value.is_empty() {
                placeholder
            } else {
                value.clone()
            }
        });
    rsx! {
        document::Script { src: asset!("/assets/js/menu.js") }
        div { class: "flex items-center gap-1",
            button {
                id: "{id}", r#type: "button",
                class: "np-date-field form-input flex items-center justify-between gap-3 font-mono text-left",
                popovertarget: "{id}-calendar",
                aria_haspopup: "dialog", aria_expanded: "false", aria_controls: "{id}-calendar",
                aria_invalid: error_id.as_ref().map(|_| "true"),
                aria_describedby: error_id,
                onclick: move |_| opening += 1,
                span { class: if value.is_empty() { "text-faint" } else { "" }, "{display}" }
                span { class: "text-label inline-flex", aria_hidden: "true", NavIcon { name: "timesheet" } }
            }
            if !value.is_empty() {
                button {
                    r#type: "button", class: "btn btn-ghost btn-sm",
                    aria_label: "Clear {label}", "data-clear-date": "{id}",
                    onclick: move |_| onchange.call(String::new()), "×"
                }
            }
        }
        div {
            id: "{id}-calendar", class: "menu-popover np-date-popover p-0 border-0",
            popover: "auto", role: "dialog", aria_label: "Choose {label}",
            "data-popover-trigger": "{id}", "data-calendar": "true",
            // A keyed fragment remounts the calendar on reopening, discarding only
            // its browsed month, never the date held by the draft.
            for generation in [opening()] {
                DatePicker {
                    key: "{generation}",
                    selected: selected.unwrap_or_else(|| chrono::Utc::now().date_naive()),
                    onpick: move |day: NaiveDate| onchange.call(day.to_string()),
                }
            }
        }
    }
}
