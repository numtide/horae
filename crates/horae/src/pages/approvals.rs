use chrono::{Datelike, Days, NaiveDate, Weekday};
use dioxus::prelude::*;
use horae_core::duration::format_hhmm;
use horae_core::types::EntryState;
use horae_core::week::week_bounds;

use super::{is_manager, loaded};
use crate::components::avatar::{Avatar, first_initial};
use crate::components::badge::Badge;
use crate::components::controls::Segmented;
use crate::components::table::DataTable;
use crate::route::Route;
use crate::server_fns;

/// The `list_approvals` status argument for a Segmented label.
fn status_arg(label: &str) -> Option<String> {
    match label {
        "Approved" => Some("approved".to_string()),
        "All" => None,
        _ => Some("submitted".to_string()), // "Pending"
    }
}

/// A week's dates, e.g. "13 Jul – 19 Jul 2026", naming both years when the
/// week spans two.
fn period_label(start: NaiveDate, end: NaiveDate) -> String {
    let first = if start.year() == end.year() {
        start.format("%d %b")
    } else {
        start.format("%d %b %Y")
    };
    format!("{first} – {}", end.format("%d %b %Y"))
}

/// Today's date, by the same UTC convention as the timesheet and reports.
fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// The configured week containing the address's date, or today's when the
/// address names no valid date.
fn selected_week(week: &str, first_day: Weekday) -> Option<(NaiveDate, NaiveDate)> {
    week_bounds(week.parse().unwrap_or_else(|_| today()), first_day)
}

/// The start of the week before or after `start`, when that whole week is
/// representable.
fn adjacent(start: NaiveDate, forward: bool, first_day: Weekday) -> Option<NaiveDate> {
    let date = if forward {
        start.checked_add_days(Days::new(7))
    } else {
        start.checked_sub_days(Days::new(7))
    }?;
    week_bounds(date, first_day).map(|(start, _)| start)
}

fn week_route(start: NaiveDate) -> Route {
    Route::Approvals {
        week: start.to_string(),
    }
}

/// A pager arrow: a link to the adjacent week, or disabled at the range's end.
fn pager_step(target: Option<NaiveDate>, side: &str, label: &str, glyph: &str) -> Element {
    match target {
        Some(start) => rsx! {
            Link { to: week_route(start), class: "ts-pager-btn {side}", aria_label: "{label}", "{glyph}" }
        },
        None => rsx! {
            button { class: "ts-pager-btn {side}", "aria-label": "{label}", disabled: true, "{glyph}" }
        },
    }
}

/// Run a mutating server action, then clear/report the error and bump `refresh`
/// so the list re-loads. Every action button is disabled while `busy` is set,
/// so one action cannot be submitted twice or overlap another.
fn spawn_action(
    fut: impl std::future::Future<Output = Result<(), ServerFnError>> + 'static,
    mut refresh: Signal<u32>,
    mut error: Signal<Option<String>>,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<(NaiveDate, String)>>,
) {
    busy.set(true);
    notice.set(None);
    spawn(async move {
        match fut.await {
            Ok(()) => error.set(None),
            Err(e) => error.set(Some(e.to_string())),
        }
        busy.set(false);
        refresh.set(refresh() + 1);
    });
}

#[component]
pub fn Approvals(week: String) -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let first_day = use_resource(|| async move { server_fns::get_week_start().await });
    let mut status_label = use_signal(|| "Pending".to_string());
    let refresh = use_signal(|| 0u32);
    let action_error = use_signal(|| None::<String>);
    let busy = use_signal(|| false);
    // The bulk result, kept with the week it belongs to.
    let notice = use_signal(|| None::<(NaiveDate, String)>);

    let selected = use_memo(use_reactive!(|(week,)| {
        match &*first_day.read() {
            Some(Ok(first)) => selected_week(&week, *first),
            _ => None,
        }
    }));

    // Each response carries the week it was requested for, so a slower answer
    // for a week no longer selected is never shown under the current one.
    let approvals = use_resource(move || {
        let f = status_arg(&status_label.read());
        let _tick = *refresh.read();
        let start = selected()
            .filter(|_| is_manager(&me))
            .map(|(start, _)| start);
        async move {
            let start = start?;
            Some((
                start,
                server_fns::list_approvals(f, start.to_string()).await,
            ))
        }
    });

    let is_manager = is_manager(&me);
    let this_week = match &*first_day.read() {
        Some(Ok(first)) => week_bounds(today(), *first).map(|(start, _)| start),
        _ => None,
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Approvals" }
            }

            if !is_manager {
                div { class: "card p-8 text-center",
                    p { class: "text-muted", "Manager or admin access is required to review approvals." }
                }
            } else {
                match (&*first_day.read(), selected()) {
                    (None, _) => rsx! {
                        div { class: "text-muted text-sm", "Loading…" }
                    },
                    (Some(Err(e)), _) => rsx! {
                        div { class: "alert alert-danger", "{e}" }
                    },
                    (Some(Ok(_)), None) => rsx! {
                        div { class: "alert alert-danger mb-4", "This week is outside the supported date range." }
                        if let Some(current) = this_week {
                            Link { to: week_route(current), class: "btn btn-ghost btn-sm", "This week" }
                        }
                    },
                    (Some(Ok(first)), Some((start, end))) => {
                        let first = *first;
                        let current = this_week == Some(start);
                        let shown = match &*approvals.read() {
                            Some(Some((loaded_start, rows))) if *loaded_start == start => Some(rows.clone()),
                            _ => None,
                        };
                        let notice_text = match &*notice.read() {
                            Some((notice_start, text)) if *notice_start == start => Some(text.clone()),
                            _ => None,
                        };
                        rsx! {
                            div { class: "flex items-center gap-3 flex-wrap mb-4",
                                div { class: "ts-pager",
                                    {pager_step(adjacent(start, false, first), "prev", "Previous week", "←")}
                                    span { class: "ts-pager-label",
                                        span { class: "cur", if current { "This week" } else { "Week" } }
                                        span { class: "ts-pager-range", "{period_label(start, end)}" }
                                    }
                                    {pager_step(adjacent(start, true, first), "next", "Next week", "→")}
                                }
                                if !current {
                                    if let Some(current) = this_week {
                                        Link { to: week_route(current), class: "btn btn-ghost btn-sm", "This week" }
                                    }
                                }
                            }

                            div { class: "mb-6",
                                Segmented {
                                    items: vec!["Pending".to_string(), "Approved".to_string(), "All".to_string()],
                                    active: status_label(),
                                    onselect: move |v| status_label.set(v),
                                }
                            }

                            if let Some(err) = &*action_error.read() {
                                div { class: "alert alert-danger mb-4", "{err}" }
                            }
                            if let Some(text) = notice_text {
                                div { class: "alert alert-success mb-4", "{text}" }
                            }

                            {loaded(&shown, |items| {
                                    if items.is_empty() {
                                        return rsx! {
                                            div { class: "card p-8 text-center",
                                                p { class: "text-muted", "No approvals found." }
                                            }
                                        };
                                    }
                                    let total: i64 = items.iter().map(|s| s.total_minutes).sum();
                                    let billable: i64 = items.iter().map(|s| s.billable_minutes).sum();
                                    let nonbill = (total - billable).max(0);
                                    let bill_pct = if total > 0 { billable * 100 / total } else { 0 };
                                    let nonbill_pct = if total > 0 { 100 - bill_pct } else { 0 };

                                    let pending_ids: Vec<String> = items
                                        .iter()
                                        .filter(|s| s.approval.state == EntryState::Submitted)
                                        .map(|s| s.approval.id.to_string())
                                        .collect();
                                    let pending_count = pending_ids.len();

                                    rsx! {
                                        // Total time (billable / non-billable). The design's second
                                        // "Total expenses" card is omitted — Horae has no expenses.
                                        div { class: "card flex items-center gap-8 mb-6",
                                            div {
                                                div { class: "text-muted text-sm", "Total time" }
                                                div { class: "appr-total", "{format_hhmm(total)}" }
                                            }
                                            div { class: "flex-1 flex flex-col gap-2",
                                                div { class: "flex items-center gap-3",
                                                    span { class: "appr-dot appr-dot-billable" }
                                                    span { class: "flex-1 text-sm", "Billable" }
                                                    span { class: "text-mono", "{format_hhmm(billable)}" }
                                                    span { class: "appr-legend-pct", "({bill_pct}%)" }
                                                }
                                                div { class: "flex items-center gap-3",
                                                    span { class: "appr-dot appr-dot-nonbillable" }
                                                    span { class: "flex-1 text-sm", "Non-billable" }
                                                    span { class: "text-mono", "{format_hhmm(nonbill)}" }
                                                    span { class: "appr-legend-pct", "({nonbill_pct}%)" }
                                                }
                                            }
                                        }

                                        DataTable {
                                            table {
                                                thead {
                                                    tr {
                                                        th { "Teammate" }
                                                        th { class: "text-right", "Hours" }
                                                        th { class: "text-center", "Status" }
                                                        th { "Submitted" }
                                                        th { class: "text-right", "Action" }
                                                    }
                                                }
                                                tbody {
                                                    for s in items.iter() {
                                                        {
                                                            let a = s.approval.clone();
                                                            let name = &s.user_name;
                                                            let period = period_label(a.period_start, a.period_end);
                                                            let submitted = a.submitted_at.format("%d %b, %H:%M").to_string();
                                                            let hours = format_hhmm(s.total_minutes);
                                                            let is_pending = a.state == EntryState::Submitted;
                                                            let can_reopen =
                                                                is_pending || a.state == EntryState::Approved;
                                                            let aid = a.id.to_string();
                                                            rsx! {
                                                                tr {
                                                                    td {
                                                                        div { class: "flex items-center gap-3",
                                                                            Avatar { initials: first_initial(name) }
                                                                            div { class: "flex flex-col",
                                                                                span { class: "font-medium", "{name}" }
                                                                                span { class: "text-mono text-sm text-muted", "{period}" }
                                                                            }
                                                                        }
                                                                    }
                                                                    td { class: "text-mono text-right", "{hours}" }
                                                                    td { class: "text-center",
                                                                        match a.state {
                                                                            EntryState::Submitted => rsx! { Badge { variant: "warning", "Awaiting" } },
                                                                            EntryState::Approved => rsx! { Badge { variant: "success", "Approved" } },
                                                                            other => rsx! { Badge { variant: "neutral", "{other}" } },
                                                                        }
                                                                    }
                                                                    td { class: "text-mono text-sm text-muted", "{submitted}" }
                                                                    td { class: "text-right",
                                                                        if can_reopen {
                                                                            div { class: "flex gap-2 justify-end",
                                                                                button {
                                                                                    class: "btn btn-secondary btn-sm",
                                                                                    disabled: busy(),
                                                                                    onclick: {
                                                                                        let aid = aid.clone();
                                                                                        move |_| spawn_action(
                                                                                            { let aid = aid.clone(); async move { server_fns::reject_submission(aid).await } },
                                                                                            refresh, action_error, busy, notice,
                                                                                        )
                                                                                    },
                                                                                    "Reopen"
                                                                                }
                                                                                if is_pending {
                                                                                    button {
                                                                                        class: "btn btn-solid btn-sm",
                                                                                        disabled: busy(),
                                                                                        onclick: {
                                                                                            let aid = aid.clone();
                                                                                            move |_| spawn_action(
                                                                                                { let aid = aid.clone(); async move { server_fns::approve_submission(aid).await.map(|_| ()) } },
                                                                                                refresh, action_error, busy, notice,
                                                                                            )
                                                                                        },
                                                                                        "Approve"
                                                                                    }
                                                                                }
                                                                            }
                                                                        } else {
                                                                            span { class: "text-muted text-sm", "\u{2014}" }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if pending_count > 0 {
                                            div { class: "flex items-center mt-5 gap-4",
                                                span { class: "text-muted text-sm",
                                                    "{pending_count} timesheet(s) awaiting your approval"
                                                }
                                                div { class: "flex-1" }
                                                button {
                                                    class: "btn btn-solid",
                                                    disabled: busy(),
                                                    onclick: move |_| {
                                                        let ids = pending_ids.clone();
                                                        let mut notice = notice;
                                                        spawn_action(
                                                            async move {
                                                                let requested = ids.len();
                                                                let approved = server_fns::approve_submissions(ids).await?;
                                                                notice.set(Some((start, format!("Approved {approved} of {requested} timesheets"))));
                                                                Ok(())
                                                            },
                                                            refresh, action_error, busy, notice,
                                                        )
                                                    },
                                                    "Approve visible timesheets"
                                                }
                                            }
                                        }
                                    }
                            })}
                        }
                    }
                }
            }
        }
    }
}
