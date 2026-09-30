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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HourBudget {
    pub minutes: i64,
    pub label: &'static str,
    pub configured: bool,
}

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
    budget_y: Option<i128>,
}

fn current_week_band(
    weeks: &[ProjectActivityWeek],
    first_day: Weekday,
    today: NaiveDate,
) -> Option<(i128, i128)> {
    let current = horae_core::week::week_start(today, first_day)?;
    let index = weeks
        .iter()
        .position(|week| horae_core::week::week_start(week.from, first_day) == Some(current))?
        as i128;
    let count = weeks.len() as i128;
    Some((index * 10000 / count, (index + 1) * 10000 / count))
}

fn plot(
    weeks: &[ProjectActivityWeek],
    cumulative: bool,
    budget: Option<i64>,
) -> Result<Plot, ActivityError> {
    let budget = budget.filter(|_| cumulative);
    if budget.is_some_and(|value| value < 0) {
        return Err(ActivityError::NegativeMinutes);
    }
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
    let maximum = values
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .max(budget.unwrap_or(0))
        .max(60);
    let budget_y = budget.map(|value| 2000 - i128::from(value) * 2000 / i128::from(maximum));
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
        budget_y,
    })
}

#[component]
pub(super) fn ProjectActivityPanel(
    project_id: Uuid,
    interval: ReadSignal<Option<ProjectActivityInterval>>,
    #[props(default)] budget: Option<HourBudget>,
) -> Element {
    let today = use_hook(|| chrono::Utc::now().date_naive());
    let mut anchor = use_signal(|| today);
    let mut cumulative = use_signal(|| true);
    use_effect(move || anchor.set(interval().map_or(today, |range| range.to.min(today))));
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
            div { aria_busy: loading,
                if loading {
                    p { class: "py-12 text-muted", role: "status", "Loading project activity…" }
                } else {
                    match &*activity.read() {
                        Some((_, Ok(data))) => render_activity(data, cumulative(), if interval().is_none() { "All time" } else { "Selected period" }, anchor(), today, budget),
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
pub(super) fn ProjectReportingPeriod(
    mut interval: Signal<Option<ProjectActivityInterval>>,
    #[props(default)] onchange: EventHandler<Option<ProjectActivityInterval>>,
    #[props(default)] children: Element,
) -> Element {
    let today = use_hook(|| chrono::Utc::now().date_naive());
    let mut label = use_signal(|| {
        if interval().is_none() {
            "All time"
        } else {
            "Custom period"
        }
    });
    let mut custom_open = use_signal(|| false);
    let mut custom_from = use_signal(|| interval().map_or(today, |range| range.from).to_string());
    let mut custom_to = use_signal(|| interval().map_or(today, |range| range.to).to_string());
    let mut error = use_signal(|| None::<String>);
    let mut labelled_interval = use_signal(&*interval);
    use_effect(move || {
        let current = interval();
        if *labelled_interval.peek() != current {
            labelled_interval.set(current);
            label.set(if current.is_none() {
                "All time"
            } else {
                "Custom period"
            });
            custom_from.set(current.map_or(today, |range| range.from).to_string());
            custom_to.set(current.map_or(today, |range| range.to).to_string());
            custom_open.set(false);
            error.set(None);
        }
    });
    let close_custom = use_callback(move |()| {
        custom_open.set(false);
        error.set(None);
        spawn(async move {
            let _ =
                document::eval("document.getElementById('project-report-period-trigger')?.focus()")
                    .await;
        });
    });
    rsx! {
        section { aria_label: "Project reporting period", class: "mt-6 mb-4",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                h2 { class: "text-2xl font-semibold text-strong m-0", "{label}" }
                div { class: "flex flex-wrap items-center gap-3",
                Menu { id: "project-report-period", label: label(), trigger_class: "py-3 min-h-control",
                    for preset in Period::ALL {
                        MenuItem { selected: label() == preset.label(), onclick: move |_| {
                            match preset.interval(today) {
                                Ok(value) => { interval.set(value); labelled_interval.set(value); onchange.call(value); label.set(preset.label()); custom_open.set(false); error.set(None); }
                                Err(message) => error.set(Some(message.to_string())),
                            }
                        }, "{preset.label()}" }
                    }
                    MenuItem { selected: label() == "Custom period", onclick: move |_| { custom_open.set(true); error.set(None); }, "Custom…" }
                }
                {children}
                }
            }
            if let Some(range) = interval() {
                p { class: "text-xs text-muted mt-2 mb-0", "{range.from.format(\"%d %b %Y\")} – {range.to.format(\"%d %b %Y\")}" }
            }
            if custom_open() {
                form { class: "mt-4", onsubmit: move |event| {
                    event.prevent_default();
                    match custom_interval(&custom_from(), &custom_to()) {
                        Ok(value) => { interval.set(Some(value)); labelled_interval.set(Some(value)); onchange.call(Some(value)); label.set("Custom period"); close_custom.call(()); }
                        Err(message) => error.set(Some(message)),
                    }
                },
                    div { class: "grid sm:grid-cols-2 gap-3",
                        FormGroup { label: "Start date", id: "project-report-from",
                            Input { id: "project-report-from", kind: "date", value: custom_from(), error_id: error().map(|_| "project-period-error".into()),
                                oninput: move |event: FormEvent| custom_from.set(event.value()) }
                        }
                        FormGroup { label: "End date", id: "project-report-to",
                            Input { id: "project-report-to", kind: "date", value: custom_to(), error_id: error().map(|_| "project-period-error".into()),
                                oninput: move |event: FormEvent| custom_to.set(event.value()) }
                        }
                    }
                    div { class: "flex flex-wrap gap-3",
                        button { r#type: "submit", class: "btn btn-primary min-h-control", "Apply period" }
                        button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| close_custom.call(()), "Cancel" }
                    }
                }
            }
            if let Some(message) = error() {
                p { id: "project-period-error", class: "text-danger", role: "alert", "{message}" }
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
    let Some(selected_end) = selected.checked_add_days(Days::new(6)) else {
        return rsx! { p { role: "alert", class: "text-danger", "Selected week is outside the supported date range." } };
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
        div { class: "flex flex-wrap items-center gap-2 max-w-full", role: "group", aria_label: "Chart week navigation",
            div { class: "ts-pager bg-base max-w-full",
            button { r#type: "button", class: "ts-pager-btn prev min-h-control", aria_label: "Previous week",
                disabled: previous.is_none(), onclick: move |_| { if let Some(day) = previous { anchor.set(day); } }, "←" }
            button { id: "project-chart-week", r#type: "button", class: "project-chart-week btn btn-ghost min-h-control min-w-0 border-0 px-3 py-2 flex-wrap justify-center text-strong",
                aria_label: "Choose chart week, ending week of {selected.format(\"%d %b %Y\")}",
                popovertarget: "project-chart-calendar", aria_haspopup: "dialog", aria_expanded: "false", aria_controls: "project-chart-calendar",
                onclick: move |_| opening += 1,
                span { class: "inline-flex text-muted", aria_hidden: "true", super::NavIcon { name: "timesheet", class: "size-4" } }
                span { class: "text-sm font-semibold", if selected == current { "This week" } else { "Week" } }
                span { class: "font-mono text-xs text-secondary wrap-anywhere", "{selected.format(\"%d %b\")} – {selected_end.format(\"%d %b %Y\")}" }
            }
            button { r#type: "button", class: "ts-pager-btn next min-h-control", aria_label: "Next week",
                disabled: next.is_none(), onclick: move |_| { if let Some(day) = next { anchor.set(day); } }, "→" }
            }
            button { r#type: "button", class: "btn btn-ghost min-h-control text-muted", disabled: selected == current,
                onclick: move |_| anchor.set(today), "This week" }
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
    today: NaiveDate,
    budget: Option<HourBudget>,
) -> Element {
    let Some(interval) = data.interval else {
        return rsx! { p { class: "py-12 text-muted", "No time has been tracked on this project." } };
    };
    let total = data.weeks.last().map_or(0, |week| week.cumulative_minutes);
    rsx! {
        p { class: "text-sm text-muted mt-4 mb-2", "{label}: {interval.from.format(\"%d %b %Y\")} – {interval.to.format(\"%d %b %Y\")}" }
        {render_chart(data, cumulative, anchor, today, budget)}
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

fn render_chart(
    data: &ProjectActivity,
    cumulative: bool,
    anchor: NaiveDate,
    today: NaiveDate,
    budget: Option<HourBudget>,
) -> Element {
    let budget = budget.filter(|_| cumulative);
    let window = match chart_interval(anchor, data.week_start) {
        Ok(window) => window,
        Err(error) => {
            return rsx! { p { role: "alert", class: "text-danger", "Cannot display activity: {error}" } };
        }
    };
    let start = data.weeks.partition_point(|week| week.to < window.from());
    let end = data.weeks.partition_point(|week| week.from <= window.to());
    let visible = &data.weeks[start..end];
    let current_band = current_week_band(visible, data.week_start, today);
    let current_label = if current_band.is_some() {
        " Current week is highlighted."
    } else {
        ""
    };
    let chart = match plot(visible, cumulative, budget.map(|value| value.minutes)) {
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
        if let Some(budget) = budget {
            p { class: "text-xs text-warning mt-2 mb-0", "Dashed reference — {budget.label}: {hours(budget.minutes)} ({budget.minutes} minutes)." }
            if budget.configured {
                p { class: "text-xs text-muted mt-1 mb-0", "Current allowance, not rounded consumption. The chart shows actual minutes; Budget remaining applies configured rounding and included work. Combined allowances do not rule out individual overruns." }
            }
        }
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
            div { class: "relative min-w-0 h-full",
            svg { class: "project-activity-svg w-full h-full", view_box: "0 0 10000 2000", preserve_aspect_ratio: "none",
                role: "img", "aria-label": "{legend}. Chart window: {first.from} to {last.to}. Selected period total: {hours(total)}.{current_label} Exact minutes are available in the weekly data table below.",
                if let Some((left, right)) = current_band {
                    rect { class: "project-activity-current", x: "{left}", y: "0", width: "{right - left}", height: "2000" }
                    path { class: "project-activity-current-edges", d: "M{left} 0 V2000 M{right} 0 V2000", vector_effect: "non-scaling-stroke" }
                }
                path { class: "project-activity-grid", d: "M0 0 H10000 M0 1000 H10000 M0 2000 H10000", vector_effect: "non-scaling-stroke" }
                if cumulative {
                    path { class: "project-activity-area", d: "{chart.line} L10000 2000 L0 2000 Z" }
                    path { class: "project-activity-line", d: "{chart.line}", vector_effect: "non-scaling-stroke" }
                } else {
                    path { class: "project-activity-bars", d: "{chart.bars}" }
                }
                if let Some(y) = chart.budget_y {
                    path { class: "project-activity-budget", d: "M0 {y} H10000", vector_effect: "non-scaling-stroke" }
                }
            }
            if current_band.is_some() {
                div { class: "absolute inset-0 flex items-start justify-end p-2", aria_hidden: "true",
                    span { class: "project-activity-current-label badge badge-info font-mono bg-secondary min-w-0 wrap-anywhere", "This week" }
                }
            }
            }
            span {}
            div { class: "flex flex-wrap justify-between gap-3 text-xs text-subtle font-mono", aria_hidden: "true",
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
            plot(&[week(60, 0, 120)], true, None).unwrap().line,
            "M0 1000 L10000 0"
        );
        assert_eq!(
            plot(&[week(60, 0, 120)], false, None).unwrap().line,
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
        let html = dioxus::ssr::render_element(render_activity(
            &data,
            true,
            "This year",
            range.to(),
            range.to(),
            None,
        ));
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
            range.to(),
            None,
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
    fn current_week_band_uses_configured_boundaries_and_clipped_buckets() {
        let weeks = [
            ProjectActivityWeek {
                from: date("2026-09-23"),
                to: date("2026-09-27"),
                ..week(0, 0, 0)
            },
            ProjectActivityWeek {
                from: date("2026-09-28"),
                to: date("2026-09-30"),
                ..week(0, 0, 0)
            },
        ];
        assert_eq!(
            current_week_band(&weeks, Weekday::Mon, date("2026-09-27")),
            Some((0, 5000))
        );
        assert_eq!(
            current_week_band(&weeks, Weekday::Mon, date("2026-09-28")),
            Some((5000, 10000))
        );
        let sunday = [
            ProjectActivityWeek {
                to: date("2026-09-26"),
                ..weeks[0].clone()
            },
            ProjectActivityWeek {
                from: date("2026-09-27"),
                ..weeks[1].clone()
            },
        ];
        assert_eq!(
            current_week_band(&sunday, Weekday::Sun, date("2026-09-27")),
            Some((5000, 10000))
        );
        let year_boundary = [ProjectActivityWeek {
            from: date("2026-12-31"),
            to: date("2027-01-02"),
            ..week(0, 0, 0)
        }];
        assert_eq!(
            current_week_band(&year_boundary, Weekday::Mon, date("2027-01-01")),
            Some((0, 10000))
        );
    }

    #[test]
    fn current_week_band_does_not_label_old_future_or_empty_series_as_current() {
        for today in ["2026-09-14", "2026-09-30"] {
            assert_eq!(
                current_week_band(&[week(60, 0, 60)], Weekday::Mon, date(today)),
                None
            );
        }
        assert_eq!(
            current_week_band(&[], Weekday::Mon, date("2026-09-23")),
            None
        );
    }

    #[test]
    fn current_week_rendering_tracks_visibility_in_both_modes_without_changing_totals() {
        let data = ProjectActivity {
            interval: Some(ProjectActivityInterval {
                from: date("2026-09-21"),
                to: date("2026-09-30"),
            }),
            week_start: Weekday::Mon,
            weeks: vec![
                week(60, 0, 60),
                ProjectActivityWeek {
                    from: date("2026-09-28"),
                    to: date("2026-09-30"),
                    ..week(0, 0, 60)
                },
            ],
        };
        for cumulative in [true, false] {
            for (anchor, today, highlighted) in [
                ("2026-09-30", "2026-09-30", true),
                ("2026-09-23", "2026-09-30", false),
                ("2026-09-30", "2026-10-07", false),
            ] {
                let html = dioxus::ssr::render_element(render_activity(
                    &data,
                    cumulative,
                    "All time",
                    date(anchor),
                    date(today),
                    None,
                ));
                assert_eq!(
                    html.contains("project-activity-current-edges"),
                    highlighted,
                    "{html}"
                );
                assert_eq!(
                    html.contains("Current week is highlighted."),
                    highlighted,
                    "{html}"
                );
                assert!(html.contains("Selected period total: 1h"), "{html}");
                assert!(html.contains("2026-09-28 – 2026-09-30"), "{html}");
            }
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
    fn budget_reference_scales_cumulative_geometry_but_not_weekly_bars() {
        let weeks = [week(60, 0, 60)];
        let chart = plot(&weeks, true, Some(120)).unwrap();
        assert_eq!(chart.maximum, 120);
        assert_eq!(chart.line, "M0 2000 L10000 1000");
        assert_eq!(chart.budget_y, Some(0));
        assert_eq!(plot(&weeks, true, Some(0)).unwrap().budget_y, Some(2000));
        assert_eq!(plot(&weeks, true, Some(30)).unwrap().budget_y, Some(1000));
        let weekly = plot(&weeks, false, Some(i64::MAX)).unwrap();
        assert_eq!(weekly.maximum, 60);
        assert_eq!(weekly.budget_y, None);
        let large = plot(&weeks, true, Some(i64::MAX)).unwrap();
        assert_eq!(large.maximum, i64::MAX);
        assert_eq!(large.budget_y, Some(0));
        assert_eq!(
            plot(&weeks, true, Some(-1)),
            Err(ActivityError::NegativeMinutes)
        );
    }

    #[test]
    fn budget_reference_has_exact_text_and_does_not_change_tracked_totals() {
        let data = ProjectActivity {
            interval: Some(ProjectActivityInterval {
                from: date("2026-09-21"),
                to: date("2026-09-27"),
            }),
            week_start: Weekday::Mon,
            weeks: vec![week(59, 2, 61)],
        };
        let budget = Some(HourBudget {
            minutes: 120,
            label: "Project hours budget",
            configured: true,
        });
        let html = dioxus::ssr::render_element(render_activity(
            &data,
            true,
            "All time",
            date("2026-09-27"),
            date("2026-09-27"),
            budget,
        ));
        for expected in [
            "project-activity-budget",
            "Project hours budget: 2h (120 minutes)",
            "Current allowance, not rounded consumption",
            "Selected period total: 1.02h",
        ] {
            assert!(html.contains(expected), "Missing {expected}: {html}");
        }
        let weekly = dioxus::ssr::render_element(render_activity(
            &data,
            false,
            "All time",
            date("2026-09-27"),
            date("2026-09-27"),
            budget,
        ));
        assert!(!weekly.contains("project-activity-budget"));
        assert!(!weekly.contains("Dashed reference"));
        assert!(weekly.contains("Selected period total: 1.02h"));
    }

    #[test]
    fn plot_scales_weekly_and_cumulative_minutes_independently() {
        let weeks = [week(60, 60, 120), week(30, 30, 180)];
        assert_eq!(plot(&weeks, true, None).unwrap().maximum, 180);
        assert_eq!(plot(&weeks, false, None).unwrap().maximum, 120);
        assert_eq!(
            plot(&weeks, true, None).unwrap().line,
            "M0 2000 L5000 667 L10000 0"
        );
    }

    #[test]
    fn empty_and_zero_series_have_a_nonzero_axis_without_invented_hours() {
        assert_eq!(
            plot(&[], true, None).unwrap(),
            Plot {
                maximum: 60,
                line: "M0 2000".into(),
                bars: String::new(),
                budget_y: None,
            }
        );
        assert_eq!(plot(&[week(0, 0, 0)], false, None).unwrap().maximum, 60);
    }

    #[test]
    fn plot_handles_maximum_integer_minutes_without_float_conversion_or_overflow() {
        let chart = plot(&[week(i64::MAX, 0, i64::MAX)], true, None).unwrap();
        assert_eq!(chart.maximum, i64::MAX);
        assert_eq!(chart.line, "M0 2000 L10000 0");
    }

    #[test]
    fn plot_rejects_negative_or_overflowing_values() {
        assert_eq!(
            plot(&[week(-1, 0, 0)], false, None),
            Err(ActivityError::NegativeMinutes)
        );
        assert_eq!(
            plot(&[week(i64::MAX, 1, i64::MAX)], false, None),
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
                date("2026-09-27"),
                None,
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
            date("2026-09-27"),
            None,
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
            date("2026-09-27"),
            None,
        ));
        assert!(zero.contains("No time tracked in this period."));
        assert!(zero.contains("Weekly activity — Custom period"));
    }
}
