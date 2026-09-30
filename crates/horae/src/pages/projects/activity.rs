use chrono::{Days, Months, NaiveDate, Weekday};
use dioxus::prelude::*;
use horae_core::project_activity::{ActivityError, ActivityRange};
use uuid::Uuid;

use crate::components::date_picker::DatePicker;
use crate::components::form::{FormGroup, Input};
use crate::components::menu::{Menu, MenuItem};
use crate::components::table::DataTable;
use crate::models::project::{ProjectActivity, ProjectActivityInterval, ProjectActivityWeek};
use crate::server_fns;

use super::hours;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Period {
    AllTime,
    ThisMonth,
    LastMonth,
    ThisQuarter,
    ThisYear,
}

impl Period {
    const ALL: [Self; 5] = [
        Self::AllTime,
        Self::ThisMonth,
        Self::LastMonth,
        Self::ThisQuarter,
        Self::ThisYear,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::AllTime => "All time",
            Self::ThisMonth => "This month",
            Self::LastMonth => "Last month",
            Self::ThisQuarter => "This quarter",
            Self::ThisYear => "This year",
        }
    }

    fn interval(self, today: NaiveDate) -> Result<Option<ProjectActivityInterval>, ActivityError> {
        let range = match self {
            Self::AllTime => return Ok(None),
            Self::ThisMonth => ActivityRange::month(today)?,
            Self::LastMonth => ActivityRange::month(
                today
                    .checked_sub_months(Months::new(1))
                    .ok_or(ActivityError::DateOutOfRange)?,
            )?,
            Self::ThisQuarter => ActivityRange::quarter(today)?,
            Self::ThisYear => ActivityRange::year(today)?,
        };
        Ok(Some(ProjectActivityInterval {
            from: range.from(),
            to: range.to(),
        }))
    }
}

fn custom_interval(from: &str, to: &str) -> Result<ProjectActivityInterval, String> {
    let from = from
        .parse::<NaiveDate>()
        .map_err(|_| "Choose a valid start date.".to_string())?;
    let to = to
        .parse::<NaiveDate>()
        .map_err(|_| "Choose a valid end date.".to_string())?;
    ActivityRange::new(from, to).map_err(|error| error.to_string())?;
    Ok(ProjectActivityInterval { from, to })
}

fn chart_interval(anchor: NaiveDate, first_day: Weekday) -> Result<ActivityRange, ActivityError> {
    let start =
        horae_core::week::week_start(anchor, first_day).ok_or(ActivityError::DateOutOfRange)?;
    let from = start
        .checked_sub_days(Days::new(25 * 7))
        .ok_or(ActivityError::DateOutOfRange)?;
    let to = start
        .checked_add_days(Days::new(6))
        .ok_or(ActivityError::DateOutOfRange)?;
    ActivityRange::new(from, to)
}

#[derive(Debug, PartialEq)]
struct Plot {
    maximum: i64,
    line: String,
    bars: String,
}

fn plot(weeks: &[ProjectActivityWeek], cumulative: bool) -> Result<Plot, ActivityError> {
    let values: Vec<i64> = weeks
        .iter()
        .map(|week| {
            if week.billable_minutes < 0
                || week.non_billable_minutes < 0
                || week.cumulative_minutes < 0
            {
                return Err(ActivityError::NegativeMinutes);
            }
            let total = week
                .billable_minutes
                .checked_add(week.non_billable_minutes)
                .ok_or(ActivityError::Overflow)?;
            Ok(if cumulative {
                week.cumulative_minutes
            } else {
                total
            })
        })
        .collect::<Result<_, _>>()?;
    let maximum = values.iter().copied().max().unwrap_or(0).max(60);
    // Geometry alone is scaled: source minutes and displayed totals stay exact.
    // A wide viewBox keeps the scaled bar edges distinct without floating point.
    let count = values.len().max(1) as i128;
    let origin = if cumulative {
        weeks.first().map_or(Ok(0), |week| {
            week.cumulative_minutes
                .checked_sub(week.billable_minutes)
                .and_then(|value| value.checked_sub(week.non_billable_minutes))
                .filter(|value| *value >= 0)
                .ok_or(ActivityError::NegativeMinutes)
        })?
    } else {
        0
    };
    let mut line = format!(
        "M0 {}",
        2000 - i128::from(origin) * 2000 / i128::from(maximum)
    );
    let mut bars = String::new();
    for (index, value) in values.into_iter().enumerate() {
        let left = index as i128 * 10000 / count;
        let right = (index as i128 + 1) * 10000 / count;
        let height = i128::from(value) * 2000 / i128::from(maximum);
        let y = 2000 - height;
        line.push_str(&format!(" L{right} {y}"));
        let width = ((right - left) * 3 / 5).max(1);
        let x = left + (right - left - width) / 2;
        bars.push_str(&format!("M{x} 2000 v-{height} h{width} v{height} Z "));
    }
    Ok(Plot {
        maximum,
        line,
        bars,
    })
}

#[component]
pub(super) fn ProjectActivityPanel(
    project_id: Uuid,
    mut interval: Signal<Option<ProjectActivityInterval>>,
) -> Element {
    let today = use_hook(|| chrono::Utc::now().date_naive());
    let mut anchor = use_signal(|| today);
    let mut label = use_signal(|| Period::AllTime.label());
    let mut cumulative = use_signal(|| true);
    let mut custom_open = use_signal(|| false);
    let mut custom_from = use_signal(|| today.to_string());
    let mut custom_to = use_signal(|| today.to_string());
    let mut error = use_signal(|| None::<String>);
    let mut activity = use_resource(move || {
        let requested = interval();
        async move {
            let response =
                server_fns::activity::get_project_activity(project_id.to_string(), requested).await;
            (requested, response)
        }
    });
    let loading = activity.state()() != UseResourceState::Ready
        || activity
            .read()
            .as_ref()
            .is_none_or(|(requested, _)| *requested != interval());

    rsx! {
        section { class: "card mt-6 p-5 min-w-0", aria_label: "Project activity",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div { class: "segmented flex-wrap", role: "group", aria_label: "Chart view",
                    for (is_cumulative, title) in [(true, "Project progress"), (false, "Hours per week")] {
                        button {
                            r#type: "button",
                            class: if cumulative() == is_cumulative { "segmented-item active py-3 min-h-control" } else { "segmented-item py-3 min-h-control" },
                            aria_pressed: cumulative() == is_cumulative,
                            onclick: move |_| cumulative.set(is_cumulative),
                            "{title}"
                        }
                    }
                }
                if !loading {
                    if let Some((_, Ok(data))) = &*activity.read() {
                        ChartNavigation { anchor, today, first_day: data.week_start }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-3 mt-4",
                span { class: "text-sm text-muted", "Reporting period" }
                Menu { id: "project-activity-period", label: label(), trigger_class: "py-3 min-h-control",
                    for preset in Period::ALL {
                        MenuItem { selected: label() == preset.label(), onclick: move |_| {
                            match preset.interval(today) {
                                Ok(value) => { interval.set(value); anchor.set(value.map_or(today, |range| range.to.min(today))); label.set(preset.label()); custom_open.set(false); error.set(None); }
                                Err(message) => error.set(Some(message.to_string())),
                            }
                        }, "{preset.label()}" }
                    }
                    MenuItem { selected: label() == "Custom period", onclick: move |_| { custom_open.set(true); error.set(None); }, "Custom…" }
                }
            }
            if custom_open() {
                form { class: "mt-4", onsubmit: move |event| {
                    event.prevent_default();
                    match custom_interval(&custom_from(), &custom_to()) {
                        Ok(value) => { interval.set(Some(value)); anchor.set(value.to.min(today)); label.set("Custom period"); custom_open.set(false); error.set(None); }
                        Err(message) => error.set(Some(message)),
                    }
                },
                    div { class: "grid sm:grid-cols-2 gap-3",
                        FormGroup { label: "Start date", id: "activity-from",
                            Input { id: "activity-from", kind: "date", value: custom_from(), error_id: error().map(|_| "activity-period-error".into()),
                                oninput: move |event: FormEvent| custom_from.set(event.value()) }
                        }
                        FormGroup { label: "End date", id: "activity-to",
                            Input { id: "activity-to", kind: "date", value: custom_to(), error_id: error().map(|_| "activity-period-error".into()),
                                oninput: move |event: FormEvent| custom_to.set(event.value()) }
                        }
                    }
                    div { class: "flex flex-wrap gap-3",
                        button { r#type: "submit", class: "btn btn-primary min-h-control", "Apply period" }
                        button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| { custom_open.set(false); error.set(None); }, "Cancel" }
                    }
                }
            }
            if let Some(message) = error() {
                p { id: "activity-period-error", class: "text-danger", role: "alert", "{message}" }
            }
            div { aria_busy: loading,
                if loading {
                    p { class: "py-12 text-muted", role: "status", "Loading project activity…" }
                } else {
                    match &*activity.read() {
                        Some((_, Ok(data))) => render_activity(data, cumulative(), label(), anchor()),
                        Some((_, Err(message))) => rsx! {
                            p { class: "text-danger mt-4", role: "alert", "Project activity is unavailable: {message}" }
                            button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| activity.restart(), "Retry activity" }
                        },
                        None => rsx! {},
                    }
                }
            }
        }
    }
}

#[component]
fn ChartNavigation(mut anchor: Signal<NaiveDate>, today: NaiveDate, first_day: Weekday) -> Element {
    let mut opening = use_signal(|| 0_u64);
    let Some(current) = horae_core::week::week_start(today, first_day) else {
        return rsx! {};
    };
    let Some(selected) = horae_core::week::week_start(anchor(), first_day) else {
        return rsx! {};
    };
    let Some(last_allowed_day) = current.checked_add_days(Days::new(6)) else {
        return rsx! { p { role: "alert", class: "text-danger", "This week is outside the supported date range." } };
    };
    let previous = selected
        .checked_sub_days(Days::new(7))
        .filter(|day| chart_interval(*day, first_day).is_ok());
    let next = selected
        .checked_add_days(Days::new(7))
        .filter(|day| *day <= current);
    rsx! {
        document::Script { src: asset!("/assets/js/menu.js") }
        div { class: "flex flex-wrap items-center gap-2", role: "group", aria_label: "Chart week navigation",
            button { r#type: "button", class: "btn btn-secondary min-h-control", aria_label: "Previous week",
                disabled: previous.is_none(), onclick: move |_| { if let Some(day) = previous { anchor.set(day); } }, "←" }
            button { r#type: "button", class: "btn btn-secondary min-h-control", disabled: selected == current,
                onclick: move |_| anchor.set(today), "This week" }
            button { r#type: "button", class: "btn btn-secondary min-h-control", aria_label: "Next week",
                disabled: next.is_none(), onclick: move |_| { if let Some(day) = next { anchor.set(day); } }, "→" }
            button { id: "project-chart-week", r#type: "button", class: "btn btn-secondary min-h-control font-mono text-xs",
                aria_label: "Choose chart week, ending week of {selected.format(\"%d %b %Y\")}",
                popovertarget: "project-chart-calendar", aria_haspopup: "dialog", aria_expanded: "false", aria_controls: "project-chart-calendar",
                onclick: move |_| opening += 1,
                "Week of {selected.format(\"%d %b %Y\")}" }
            div { id: "project-chart-calendar", class: "menu-popover calendar-popover p-0 border-0",
                popover: "auto", role: "dialog", aria_label: "Choose chart week",
                "data-popover-trigger": "project-chart-week", "data-calendar": "true",
                for generation in [opening()] {
                    DatePicker { key: "{generation}", selected, week: true, first_day, max_date: Some(last_allowed_day),
                        onpick: move |day| {
                            if let Some(week) = horae_core::week::week_start(day, first_day)
                                .filter(|week| *week <= current && chart_interval(*week, first_day).is_ok()) {
                                anchor.set(week);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_activity(
    data: &ProjectActivity,
    cumulative: bool,
    label: &str,
    anchor: NaiveDate,
) -> Element {
    let Some(interval) = data.interval else {
        return rsx! { p { class: "py-12 text-muted", "No time has been tracked on this project." } };
    };
    let total = data.weeks.last().map_or(0, |week| week.cumulative_minutes);
    rsx! {
        p { class: "text-sm text-muted mt-4 mb-2", "{label}: {interval.from.format(\"%d %b %Y\")} – {interval.to.format(\"%d %b %Y\")}" }
        {render_chart(data, cumulative, anchor)}
        p { class: "text-sm mt-4", "Selected period: " strong { class: "font-mono", "{hours(total)}" } }
        if total == 0 { p { class: "text-sm text-muted", "No time tracked in this period." } }
        details { class: "mt-4",
            summary { class: "text-sm text-primary cursor-pointer py-3", "View weekly data" }
            p { class: "text-xs text-subtle", "Actual minutes, without invoice rounding. Weeks start on {data.week_start}; first and last weeks are clipped to the selected dates." }
            DataTable {
                table {
                    caption { class: "text-left text-sm py-3", "Weekly activity — {label}" }
                    thead { tr {
                        th { scope: "col", "Dates (inclusive)" }
                        th { scope: "col", class: "text-right", "Billable minutes" }
                        th { scope: "col", class: "text-right", "Non-billable minutes" }
                        th { scope: "col", class: "text-right", "Cumulative minutes" }
                    } }
                    tbody {
                        for week in &data.weeks {
                            tr { key: "{week.from}",
                                th { scope: "row", "{week.from} – {week.to}" }
                                td { class: "text-right font-mono", "{week.billable_minutes}" }
                                td { class: "text-right font-mono", "{week.non_billable_minutes}" }
                                td { class: "text-right font-mono", "{week.cumulative_minutes}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_chart(data: &ProjectActivity, cumulative: bool, anchor: NaiveDate) -> Element {
    let window = match chart_interval(anchor, data.week_start) {
        Ok(window) => window,
        Err(error) => {
            return rsx! { p { role: "alert", class: "text-danger", "Cannot display activity: {error}" } };
        }
    };
    let start = data.weeks.partition_point(|week| week.to < window.from());
    let end = data.weeks.partition_point(|week| week.from <= window.to());
    let visible = &data.weeks[start..end];
    let chart = match plot(visible, cumulative) {
        Ok(chart) => chart,
        Err(error) => {
            return rsx! { p { role: "alert", class: "text-danger", "Cannot display activity: {error}" } };
        }
    };
    let (Some(first), Some(last)) = (visible.first(), visible.last()) else {
        return rsx! { p { class: "py-12 text-muted", "No selected-period dates in this chart window. Use the week controls or choose another reporting period." } };
    };
    let legend = if cumulative {
        "Cumulative hours in selected period"
    } else {
        "Hours tracked per week"
    };
    let total = data.weeks.last().map_or(0, |week| week.cumulative_minutes);
    rsx! {
        p { class: "text-xs text-subtle m-0", "{legend}" }
        p { class: "text-xs text-muted mt-2", "Chart window: {first.from} – {last.to}. Up to 26 weeks; reporting totals and the table below cover the full selected period." }
        if visible.iter().all(|week| week.billable_minutes == 0 && week.non_billable_minutes == 0) {
            p { class: "text-sm text-muted", "No time tracked in this chart window." }
        }
        div { class: "project-activity-plot grid gap-3 mt-4",
            div { class: "flex flex-col justify-between items-end text-xs font-mono text-subtle", aria_hidden: "true",
                span { "{hours(chart.maximum)}" }
                span { "{hours(chart.maximum / 2)}" }
                span { "0h" }
            }
            svg { class: "project-activity-svg w-full h-full", view_box: "0 0 10000 2000", preserve_aspect_ratio: "none",
                role: "img", "aria-label": "{legend}. Chart window: {first.from} to {last.to}. Selected period total: {hours(total)}. Exact minutes are available in the weekly data table below.",
                path { class: "project-activity-grid", d: "M0 0 H10000 M0 1000 H10000 M0 2000 H10000", vector_effect: "non-scaling-stroke" }
                if cumulative {
                    path { class: "project-activity-area", d: "{chart.line} L10000 2000 L0 2000 Z" }
                    path { class: "project-activity-line", d: "{chart.line}", vector_effect: "non-scaling-stroke" }
                } else {
                    path { class: "project-activity-bars", d: "{chart.bars}" }
                }
            }
            span {}
            div { class: "flex justify-between gap-3 text-xs text-subtle font-mono", aria_hidden: "true",
                span { "{first.from.format(\"%d %b %Y\")}" }
                span { "{last.to.format(\"%d %b %Y\")}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        value.parse().unwrap()
    }

    #[test]
    fn chart_window_contains_26_configured_weeks_across_year_and_leap_boundaries() {
        for first_day in [Weekday::Mon, Weekday::Sun, Weekday::Wed] {
            let range = chart_interval(date("2024-03-01"), first_day).unwrap();
            assert_eq!((range.to() - range.from()).num_days(), 26 * 7 - 1);
            assert_eq!(chrono::Datelike::weekday(&range.from()), first_day);
            assert!((range.from()..=range.to()).contains(&date("2024-02-29")));
            let previous = chart_interval(date("2024-02-23"), first_day).unwrap();
            assert_eq!((range.from() - previous.from()).num_days(), 7);
            assert_eq!((range.to() - previous.to()).num_days(), 7);
        }
    }

    #[test]
    fn chart_window_rejects_unrepresentable_bounds_without_panicking() {
        for anchor in [NaiveDate::MIN, NaiveDate::MAX] {
            assert_eq!(
                chart_interval(anchor, chrono::Datelike::weekday(&anchor)),
                Err(ActivityError::DateOutOfRange)
            );
        }
    }

    #[test]
    fn cropped_cumulative_plot_preserves_hours_before_the_visible_window() {
        assert_eq!(
            plot(&[week(60, 0, 120)], true).unwrap().line,
            "M0 1000 L10000 0"
        );
        assert_eq!(
            plot(&[week(60, 0, 120)], false).unwrap().line,
            "M0 2000 L10000 0"
        );
    }

    #[test]
    fn chart_window_crops_the_plot_but_keeps_the_complete_reporting_table_and_total() {
        let range = ActivityRange::new(date("2026-01-01"), date("2026-09-30")).unwrap();
        let weeks = horae_core::project_activity::weekly_activity(
            range,
            Weekday::Mon,
            &[
                horae_core::project_activity::DailyActivity {
                    date: range.from(),
                    billable_minutes: 60,
                    non_billable_minutes: 0,
                },
                horae_core::project_activity::DailyActivity {
                    date: range.to(),
                    billable_minutes: 120,
                    non_billable_minutes: 0,
                },
            ],
            100,
        )
        .unwrap()
        .into_iter()
        .map(|week| ProjectActivityWeek {
            from: week.from,
            to: week.to,
            billable_minutes: week.billable_minutes,
            non_billable_minutes: week.non_billable_minutes,
            cumulative_minutes: week.cumulative_minutes,
        })
        .collect();
        let data = ProjectActivity {
            interval: Some(ProjectActivityInterval {
                from: range.from(),
                to: range.to(),
            }),
            week_start: Weekday::Mon,
            weeks,
        };
        let html =
            dioxus::ssr::render_element(render_activity(&data, true, "This year", range.to()));
        for expected in [
            "Chart window: 2026-04-06 – 2026-09-30",
            "2026-01-01 – 2026-01-04",
            "Selected period total: 3h",
            "M0 1334",
            "L10000 2000 L0 2000 Z",
        ] {
            assert!(html.contains(expected), "missing {expected}: {html}");
        }
        let outside = dioxus::ssr::render_element(render_activity(
            &data,
            true,
            "This year",
            date("2025-01-01"),
        ));
        assert!(
            outside.contains("No selected-period dates in this chart window"),
            "{outside}"
        );
        assert!(outside.contains("2026-01-01 – 2026-01-04"), "{outside}");
        assert!(outside.contains("3h"), "{outside}");
        assert!(!outside.contains("<svg"), "{outside}");
    }

    fn week(billable: i64, non_billable: i64, cumulative: i64) -> ProjectActivityWeek {
        ProjectActivityWeek {
            from: date("2026-09-21"),
            to: date("2026-09-27"),
            billable_minutes: billable,
            non_billable_minutes: non_billable,
            cumulative_minutes: cumulative,
        }
    }

    #[test]
    fn presets_use_complete_calendar_periods_including_leap_days() {
        let today = date("2024-03-31");
        for (preset, expected) in [
            (Period::AllTime, None),
            (Period::ThisMonth, Some(("2024-03-01", "2024-03-31"))),
            (Period::LastMonth, Some(("2024-02-01", "2024-02-29"))),
            (Period::ThisQuarter, Some(("2024-01-01", "2024-03-31"))),
            (Period::ThisYear, Some(("2024-01-01", "2024-12-31"))),
        ] {
            assert_eq!(
                preset.interval(today).unwrap(),
                expected.map(|(from, to)| ProjectActivityInterval {
                    from: date(from),
                    to: date(to)
                })
            );
        }
    }

    #[test]
    fn last_month_crosses_the_year_boundary() {
        assert_eq!(
            Period::LastMonth.interval(date("2026-01-01")).unwrap(),
            Some(ProjectActivityInterval {
                from: date("2025-12-01"),
                to: date("2025-12-31")
            })
        );
    }

    #[test]
    fn custom_period_accepts_one_day_and_rejects_missing_invalid_or_reversed_dates() {
        assert!(custom_interval("2026-09-29", "2026-09-29").is_ok());
        for (from, to) in [
            ("", ""),
            ("2026-02-30", "2026-03-01"),
            ("2026-09-30", "2026-09-29"),
        ] {
            assert!(custom_interval(from, to).is_err());
        }
    }

    #[test]
    fn plot_scales_weekly_and_cumulative_minutes_independently() {
        let weeks = [week(60, 60, 120), week(30, 30, 180)];
        assert_eq!(plot(&weeks, true).unwrap().maximum, 180);
        assert_eq!(plot(&weeks, false).unwrap().maximum, 120);
        assert_eq!(
            plot(&weeks, true).unwrap().line,
            "M0 2000 L5000 667 L10000 0"
        );
    }

    #[test]
    fn empty_and_zero_series_have_a_nonzero_axis_without_invented_hours() {
        assert_eq!(
            plot(&[], true).unwrap(),
            Plot {
                maximum: 60,
                line: "M0 2000".into(),
                bars: String::new()
            }
        );
        assert_eq!(plot(&[week(0, 0, 0)], false).unwrap().maximum, 60);
    }

    #[test]
    fn plot_handles_maximum_integer_minutes_without_float_conversion_or_overflow() {
        let chart = plot(&[week(i64::MAX, 0, i64::MAX)], true).unwrap();
        assert_eq!(chart.maximum, i64::MAX);
        assert_eq!(chart.line, "M0 2000 L10000 0");
    }

    #[test]
    fn plot_rejects_negative_or_overflowing_values() {
        assert_eq!(
            plot(&[week(-1, 0, 0)], false),
            Err(ActivityError::NegativeMinutes)
        );
        assert_eq!(
            plot(&[week(i64::MAX, 1, i64::MAX)], false),
            Err(ActivityError::Overflow)
        );
    }

    #[test]
    fn activity_table_exposes_exact_minutes_and_clipped_dates_in_both_chart_modes() {
        let data = ProjectActivity {
            interval: Some(ProjectActivityInterval {
                from: date("2026-09-21"),
                to: date("2026-09-27"),
            }),
            week_start: chrono::Weekday::Sun,
            weeks: vec![week(59, 2, 61)],
        };
        for cumulative in [true, false] {
            let html = dioxus::ssr::render_element(render_activity(
                &data,
                cumulative,
                "Custom period",
                date("2026-09-27"),
            ));
            for expected in [
                "59",
                "61",
                "Billable minutes",
                "Non-billable minutes",
                "Cumulative minutes",
                "2026-09-21 – 2026-09-27",
                "Weeks start on Sun",
                "Selected period total: 1.02h",
                "scope=\"row\"",
            ] {
                assert!(html.contains(expected), "missing {expected}: {html}");
            }
        }
    }

    #[test]
    fn empty_all_time_is_distinct_from_an_explicit_zero_period() {
        let mut data = ProjectActivity {
            interval: None,
            week_start: chrono::Weekday::Mon,
            weeks: Vec::new(),
        };
        let empty = dioxus::ssr::render_element(render_activity(
            &data,
            true,
            "All time",
            date("2026-09-27"),
        ));
        assert!(empty.contains("No time has been tracked on this project."));
        assert!(!empty.contains("<svg"));
        data.interval = Some(ProjectActivityInterval {
            from: date("2026-09-21"),
            to: date("2026-09-27"),
        });
        data.weeks.push(week(0, 0, 0));
        let zero = dioxus::ssr::render_element(render_activity(
            &data,
            false,
            "Custom period",
            date("2026-09-27"),
        ));
        assert!(zero.contains("No time tracked in this period."));
        assert!(zero.contains("Weekly activity — Custom period"));
    }
}
