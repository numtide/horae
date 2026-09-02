use chrono::{Datelike, Duration, Months, NaiveDate};
use dioxus::prelude::*;

const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Six full weeks are always drawn so the panel keeps its height from month to
/// month and the grid never reflows under the cursor.
const CELLS: i64 = 42;

/// Monday of the ISO week containing `date`.
fn week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// The first of the month one step either side of `month`.
fn shift_month(month: NaiveDate, forward: bool) -> NaiveDate {
    let step = Months::new(1);
    let moved = if forward {
        month.checked_add_months(step)
    } else {
        month.checked_sub_months(step)
    };
    moved.unwrap_or(month)
}

/// A month calendar panel: the surface a period stepper opens onto.
///
/// The panel is a plain block — the caller owns where it sits (wrap it in
/// `.menu-anchor` + `.dp-pop` for a popover) and when it closes. Days run
/// Monday-first, matching the app's ISO weeks.
#[component]
pub fn DatePicker(
    /// The picked day: the month the calendar opens on, and what it highlights.
    selected: NaiveDate,
    /// Highlight the whole ISO week of `selected` rather than the single day —
    /// for callers that page a week at a time.
    #[props(default)]
    week: bool,
    onpick: EventHandler<NaiveDate>,
) -> Element {
    let today = chrono::Utc::now().date_naive();
    // The month on screen. The arrows move it without changing what is picked,
    // so it is local state, seeded from `selected` when the panel mounts.
    let mut month = use_signal(|| selected.with_day(1).unwrap_or(selected));

    let visible = month();
    // Days between these two carry the wash; the two ends themselves go solid.
    // In day mode both collapse onto `selected`, which is then simply solid.
    let (band_start, band_end) = if week {
        let start = week_monday(selected);
        (start, start + Duration::days(6))
    } else {
        (selected, selected)
    };
    let grid_start = week_monday(visible);

    rsx! {
        div { class: "menu dp",
            div { class: "flex items-center gap-3 mb-4",
                button {
                    r#type: "button",
                    class: "dp-nav",
                    "aria-label": "Previous month",
                    onclick: move |_| month.set(shift_month(month(), false)),
                    "←"
                }
                div { class: "flex-1 text-center font-display text-lg font-semibold text-strong",
                    "{visible.format(\"%B %Y\")}"
                }
                button {
                    r#type: "button",
                    class: "dp-nav",
                    "aria-label": "Next month",
                    onclick: move |_| month.set(shift_month(month(), true)),
                    "→"
                }
            }

            div { class: "grid grid-cols-7 pb-2 border-b mb-2",
                for day in WEEKDAYS {
                    div { class: "text-center text-xs text-label", "{day}" }
                }
            }

            div { class: "grid grid-cols-7 rounded overflow-hidden",
                for offset in 0..CELLS {
                    {
                        let day = grid_start + Duration::days(offset);
                        let mut class = String::from("dp-day");
                        if day.month() != visible.month() {
                            class.push_str(" outside");
                        }
                        if (band_start..=band_end).contains(&day) {
                            if day == band_start || day == band_end {
                                class.push_str(" picked");
                            } else {
                                class.push_str(" band");
                            }
                        }
                        if day == today {
                            class.push_str(" today");
                        }
                        rsx! {
                            button {
                                key: "{day}",
                                r#type: "button",
                                class: "{class}",
                                "aria-label": "{day.format(\"%-d %B %Y\")}",
                                onclick: move |_| onpick.call(day),
                                "{day.day()}"
                            }
                        }
                    }
                }
            }

            div { class: "flex items-center gap-3 mt-4 pt-3 border-t",
                button {
                    r#type: "button",
                    class: "btn btn-ghost btn-sm text-sm font-semibold",
                    onclick: move |_| onpick.call(today),
                    if week {
                        "This week"
                    } else {
                        "Today"
                    }
                }
                div { class: "flex-1" }
                span { class: "text-xs text-faint",
                    if week {
                        "Picks the whole week"
                    } else {
                        "Picks a single day"
                    }
                }
            }
        }
    }
}
