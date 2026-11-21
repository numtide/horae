use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;

use super::date_picker::DatePicker;
use crate::i18n::use_tr;

/// The week pager: previous and next arrows around a label that opens a
/// `DatePicker`. With `day_paged` it names one day instead of the week. The
/// caller owns the dates; `on_step` gets `true` for next, and `on_pick` the
/// date chosen in the picker.
#[component]
pub fn PeriodNav(
    /// Prefix of the ids of its arrows and label: `{id}-prev`, `{id}-label`, `{id}-next`.
    id: &'static str,
    week_start: NaiveDate,
    /// The day the Day face names and the picker highlights.
    day: NaiveDate,
    today: NaiveDate,
    #[props(default)] day_paged: bool,
    on_step: Callback<bool>,
    on_pick: Callback<NaiveDate>,
) -> Element {
    let tr = use_tr();
    let mut picker_open = use_signal(|| false);
    // A new period, from the arrows or from elsewhere (the browser's back
    // button), closes a picker still showing the old one.
    use_effect(use_reactive!(|(week_start, day)| {
        let _ = (week_start, day);
        if *picker_open.peek() {
            picker_open.set(false);
        }
    }));
    let pick = use_callback(move |picked: NaiveDate| {
        picker_open.set(false);
        on_pick.call(picked);
    });
    let week_end = week_start + Duration::days(6);
    let is_this_week = (week_start..=week_end).contains(&today);
    let range_label = format!(
        "{} – {}",
        week_start.format("%d %b"),
        week_end.format("%d %b %Y")
    );
    rsx! {
        // The label is the date picker's trigger. The popover is a sibling of
        // .ts-pager, which clips its children to keep the arrows inside its
        // rounded border.
        div { class: "menu-anchor",
            div { class: "ts-pager",
                button {
                    r#type: "button",
                    id: "{id}-prev",
                    class: "ts-pager-btn prev",
                    "aria-label": if day_paged { tr.period_nav_previous_day() } else { tr.period_nav_previous_week() },
                    onclick: move |_| on_step.call(false),
                    "←"
                }
                button {
                    r#type: "button",
                    id: "{id}-label",
                    class: "ts-pager-label",
                    "aria-haspopup": "dialog",
                    "aria-expanded": "{picker_open}",
                    onclick: move |_| {
                        let next = !picker_open();
                        picker_open.set(next);
                    },
                    // Both labels share one grid cell and the other view's is only
                    // hidden, so the pager is as wide in Day view as in Week, never
                    // narrower than its picker, and holds still as the weekday changes.
                    span { class: "ts-pager-faces",
                        span { class: if day_paged { "ts-pager-face" } else { "ts-pager-face reserve" },
                            span { class: "text-faint", "▦" }
                            span { class: "cur", if day == today { {tr.period_nav_today()} } else { "{day.format(\"%A\")}" } }
                            span { class: "ts-pager-range", "{day.format(\"%d %b %Y\")}" }
                        }
                        span { class: if day_paged { "ts-pager-face reserve" } else { "ts-pager-face" },
                            span { class: "text-faint", "▦" }
                            // "Week" keeps the room "This week" takes, so leaving the current
                            // week does not narrow the pager either.
                            span { class: "cur ts-pager-faces",
                                span { class: if is_this_week { "ts-pager-face" } else { "ts-pager-face reserve" }, {tr.period_nav_this_week()} }
                                span { class: if is_this_week { "ts-pager-face reserve" } else { "ts-pager-face" }, {tr.period_nav_week()} }
                            }
                            span { class: "ts-pager-range", "{range_label}" }
                        }
                    }
                }
                button {
                    r#type: "button",
                    id: "{id}-next",
                    class: "ts-pager-btn next",
                    "aria-label": if day_paged { tr.period_nav_next_day() } else { tr.period_nav_next_week() },
                    onclick: move |_| on_step.call(true),
                    "→"
                }
            }
            if picker_open() {
                div { class: "menu-overlay", onclick: move |_| picker_open.set(false) }
                div { class: "dp-pop",
                    DatePicker {
                        first_day: week_start.weekday(),
                        selected: day,
                        week: !day_paged,
                        onpick: pick,
                    }
                }
            }
        }
    }
}
