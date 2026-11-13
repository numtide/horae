//! The calendar view. It is its own component so that the hover hint and each
//! drag step re-render the calendar alone, not the page around it.

use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::html::geometry::PixelsVector2D;
use dioxus::prelude::*;
use uuid::Uuid;

use horae_core::duration::format_hhmm;
use horae_core::types::EntryState;

use super::{
    CAL_HOUR, CalDrag, DragKind, cal_time_label, cal_y_to_min, day_col_class, timed_lanes,
    today_offset, tracking, untimed_ordered,
};
use crate::models::scoped_time::{TimesheetTrackingOption, VisibleTimeEntry};

/// A calendar event's pre-computed placement and labels, plus the id of the
/// entry it came from so a press can find it to drag or edit.
#[derive(PartialEq)]
struct CalEvent {
    id: Uuid,
    top: i32,
    height: i32,
    /// True when the entry has a start time (positioned at its hour); false when
    /// untimed (stacked from the top of the day).
    timed: bool,
    /// Column and column-count for laying overlapping timed blocks side by side.
    lane: i32,
    lanes: i32,
    project: String,
    task: String,
    duration: String,
    /// Start–end clock label (e.g. "9:00–10:30") for timed entries; empty when
    /// untimed.
    time_label: String,
    client: String,
    state: EntryState,
    /// Whether the loaded sheet and choices allow changing it; `busy` locks it too.
    editable: bool,
}

/// One day's events, and the minute ranges they occupy so the "+ Add time"
/// hint hides over existing blocks and a click there edits instead of adding.
#[derive(PartialEq)]
struct CalDay {
    events: Vec<CalEvent>,
    occupied: Vec<(i32, i32)>,
}

/// Place entries: timed ones (with a start time) at their hour; untimed ones
/// stacked from the top of the day by cumulative duration (Harvest does the
/// same for duration-only entries).
fn layout(
    by_day: &[Vec<VisibleTimeEntry>; 7],
    projects: &HashMap<Uuid, String>,
    tasks: &HashMap<Uuid, String>,
    clients: &HashMap<Uuid, String>,
    loaded: bool,
    tracking: &[TimesheetTrackingOption],
) -> Vec<CalDay> {
    by_day
        .iter()
        .map(|day| {
            // Lay out timed entries side by side where they overlap: greedily assign
            // each a lane, then give every entry in an overlap cluster the same
            // column count so none is hidden (SC-005). Keyed by index into `day`.
            let (lane_of, lanes_of) = timed_lanes(day);

            // Untimed blocks stack from the top in reorder-aware order; precompute
            // each one's top so the render loop can stay in the entries' own order.
            let mut untimed_top: HashMap<Uuid, i32> = HashMap::new();
            let mut cum = 0i32;
            for e in untimed_ordered(day) {
                untimed_top.insert(e.id, cum);
                cum += e.minutes;
            }
            let mut events = Vec::new();
            let mut occupied = Vec::new();
            for (idx, e) in day.iter().enumerate() {
                let (top_min, timed, lane, lanes) = match e.start_minute {
                    Some(sm) => (sm, true, lane_of[idx], lanes_of[idx].max(1)),
                    None => (untimed_top.get(&e.id).copied().unwrap_or(0), false, 0, 1),
                };
                occupied.push((top_min, top_min + e.minutes));
                events.push(CalEvent {
                    id: e.id,
                    top: top_min * CAL_HOUR / 60,
                    height: (e.minutes * CAL_HOUR / 60).max(20),
                    timed,
                    lane,
                    lanes,
                    project: projects
                        .get(&e.project_id)
                        .cloned()
                        .unwrap_or_else(|| "Untitled".into()),
                    task: tasks.get(&e.task_id).cloned().unwrap_or_default(),
                    duration: format_hhmm(e.minutes.into()),
                    time_label: match e.start_minute {
                        Some(sm) => cal_time_label(sm, sm + e.minutes),
                        None => String::new(),
                    },
                    client: clients.get(&e.project_id).cloned().unwrap_or_default(),
                    state: e.state,
                    editable: tracking::editable(loaded, tracking, e),
                });
            }
            CalDay { events, occupied }
        })
        .collect()
}

#[component]
pub(super) fn CalendarView(
    by_day: Memo<[Vec<VisibleTimeEntry>; 7]>,
    daily_totals: Memo<Vec<i32>>,
    visible_days: Vec<usize>,
    week_start: NaiveDate,
    today: NaiveDate,
    project_names: Memo<HashMap<Uuid, String>>,
    task_names: Memo<HashMap<Uuid, String>>,
    /// project_id -> authorized client name.
    project_client: Memo<HashMap<Uuid, String>>,
    mut cal_drag: Signal<Option<CalDrag>>,
    drag_commit: Callback<CalDrag>,
    tracking: Memo<Vec<TimesheetTrackingOption>>,
    busy: Memo<bool>,
    /// Whether the sheet's write context has arrived.
    loaded: Memo<bool>,
) -> Element {
    // The free calendar slot the cursor is over: (column, snapped minute). Drives
    // the cursor-following "+ Add time" hint.
    let mut add_hint = use_signal(|| None::<(usize, i32)>);
    // Another week or span leaves no slot under the cursor.
    let shown = (week_start, visible_days.clone());
    use_effect(use_reactive!(|(shown,)| {
        let _ = shown;
        if add_hint.peek().is_some() {
            add_hint.set(None);
        }
    }));
    let placed = use_memo(move || {
        layout(
            &by_day.read(),
            &project_names.read(),
            &task_names.read(),
            &project_client.read(),
            loaded(),
            &tracking.read(),
        )
    });
    let days = placed.read();
    let daily_totals = daily_totals.read();
    let busy = busy();
    let drag = cal_drag.read();
    let drag = drag.as_ref();
    let hint = *add_hint.read();

    let can_create = !busy && !tracking.read().is_empty();
    let today_off = today_offset(today, week_start);
    let col_class = |i: usize| {
        day_col_class(
            "ts-cal-col",
            today_off,
            i,
            (week_start + Duration::days(i as i64)).weekday(),
        )
    };
    let head_class = |i: usize| {
        day_col_class(
            "ts-cal-dayhead",
            today_off,
            i,
            (week_start + Duration::days(i as i64)).weekday(),
        )
    };

    // Show the full 24-hour day (the scroll container clips it); the grid scrolls
    // vertically and, on mount, jumps to the earliest block across the visible
    // days — or to the working hours (7am) when the range is empty.
    let max_hours = 24;
    let first_min = visible_days
        .iter()
        .flat_map(|&i| days.get(i).into_iter().flat_map(|d| &d.occupied))
        .map(|&(s, _)| s)
        .min()
        .unwrap_or(7 * 60);
    let scroll_px = (first_min - 30).max(0) * CAL_HOUR / 60;

    // The grid spans a variable number of days; size the columns to match and
    // sum only the visible days for the header total.
    let n = visible_days.len().max(1);
    let shown_total: i32 = visible_days.iter().map(|&i| daily_totals[i]).sum();
    let total_label = if n == 1 { "Day total" } else { "Week total" };
    let grid_style = format!(
        "grid-template-columns: 56px repeat({n}, 1fr) 100px; min-width: {}px;",
        156 + n * 106
    );
    // The entry a press grabs, as it is now.
    let entry_at =
        move |day: usize, id: Uuid| by_day.peek()[day].iter().find(|e| e.id == id).cloned();

    rsx! {
        div { class: "ts-cal",
            div {
                class: "ts-cal-scroll",
                // Open on the earliest block (or the working hours), not midnight.
                onmounted: move |evt: MountedEvent| {
                    spawn(async move {
                        let _ = evt
                            .data()
                            .scroll(
                                PixelsVector2D::new(0.0, f64::from(scroll_px)),
                                ScrollBehavior::Instant,
                            )
                            .await;
                    });
                },
                div { class: "ts-cal-head", style: "{grid_style}",
                    span {}
                    for i in visible_days.iter().copied() {
                        {
                            let d = week_start + Duration::days(i as i64);
                            rsx! {
                                div { class: "{head_class(i)}",
                                    div { class: "ts-cal-dayname", "{d.format(\"%a\")} {d.day()}" }
                                    div { class: "ts-cal-daytotal", "{format_hhmm(daily_totals[i].into())}" }
                                }
                            }
                        }
                    }
                    div { class: "ts-cal-weektot",
                        div { class: "ts-cal-weektot-label", "{total_label}" }
                        div { class: "ts-cal-weektot-value", "{format_hhmm(shown_total.into())}" }
                    }
                }

                div {
                    class: if drag.is_some() { "ts-cal-grid dragging" } else { "ts-cal-grid" },
                    style: "{grid_style}",
                    onmouseleave: move |_| {
                        if cal_drag.read().is_some() {
                            cal_drag.set(None);
                        }
                        if add_hint.read().is_some() {
                            add_hint.set(None);
                        }
                    },
                    div { class: "ts-cal-rail",
                        for h in 0..max_hours {
                            div { class: "ts-cal-hour",
                                span { class: "ts-cal-hour-label", "{h + 1}hr" }
                            }
                        }
                    }
                    for i in visible_days.iter().copied() {
                        div {
                            class: "{col_class(i)}",
                            // Press-drag on an empty column draws a slot; release
                            // opens the entry form (a plain click has no start).
                            onmousedown: move |e: MouseEvent| {
                                if !can_create { return; }
                                let m = cal_y_to_min(e.element_coordinates().y);
                                cal_drag.set(Some(CalDrag {
                                    kind: DragKind::Create,
                                    day: i,
                                    start_min: m,
                                    cur_min: m,
                                    grab_min: m,
                                    entry: None,
                                    orig_dur: 0,
                                    orig_day: i,
                                }));
                            },
                            onmousemove: move |e: MouseEvent| {
                                if busy || (!can_create && cal_drag.read().is_none()) { return; }
                                let m = cal_y_to_min(e.element_coordinates().y);
                                if cal_drag.peek().is_some() {
                                    // Writing only real changes keeps a mousemove
                                    // within one snap step from re-rendering the
                                    // whole calendar.
                                    let next = cal_drag.peek().as_ref().and_then(|d| d.follow(m, i));
                                    if let Some((cur_min, day)) = next {
                                        cal_drag.with_mut(|d| {
                                            if let Some(d) = d {
                                                d.cur_min = cur_min;
                                                d.day = day;
                                            }
                                        });
                                    }
                                } else {
                                    // Track the free slot under the cursor for the
                                    // "+ Add time" hint; hide it over a block.
                                    let free = !placed.peek()[i].occupied.iter().any(|&(lo, hi)| m >= lo && m < hi);
                                    let next = free.then_some((i, m));
                                    if *add_hint.peek() != next {
                                        add_hint.set(next);
                                    }
                                }
                            },
                            onmouseup: move |_| {
                                let drag = cal_drag.read().clone();
                                if let Some(d) = drag {
                                    cal_drag.set(None);
                                    drag_commit.call(d);
                                }
                            },
                            if let Some(d) = drag.filter(|d| d.day == i && d.kind == DragKind::Create) {
                                {
                                    let a = d.start_min.min(d.cur_min);
                                    let top = a * CAL_HOUR / 60;
                                    let h = ((d.cur_min - d.start_min).abs() * CAL_HOUR / 60).max(2);
                                    rsx! {
                                        div { class: "ts-cal-ghost", style: "top: {top}px; height: {h}px;" }
                                    }
                                }
                            }
                            // While reordering/moving an untimed block, a ghost of it
                            // follows the cursor in the hovered column for feedback.
                            if let Some((e, cur)) = drag
                                .filter(|d| d.day == i && d.kind == DragKind::Reorder)
                                .and_then(|d| d.entry.as_ref().map(|e| (e, d.cur_min)))
                            {
                                {
                                    let h = (e.minutes * CAL_HOUR / 60).max(20);
                                    let top = (cur * CAL_HOUR / 60 - h / 2).max(0);
                                    let name = project_names
                                        .read()
                                        .get(&e.project_id)
                                        .cloned()
                                        .unwrap_or_default();
                                    rsx! {
                                        div { class: "ts-cal-ghost drag", style: "top: {top}px; height: {h}px;", "{name}" }
                                    }
                                }
                            }
                            // Cursor-following "+ Add time" over a free slot; a click
                            // there seeds a timed entry at that hour (drag_commit).
                            if let Some(top) = drag
                                .is_none()
                                .then_some(hint)
                                .flatten()
                                .filter(|&(hd, _)| hd == i)
                                .map(|(_, m)| m * CAL_HOUR / 60)
                            {
                                div { class: "ts-cal-add-hint", style: "top: {top}px;", "+ Add time" }
                            }
                            for ev in days[i].events.iter() {
                                {
                                // Live preview: while this entry is being moved or
                                // resized in its own column, drive its box from the
                                // in-progress drag so you can see it grow/shrink and
                                // read its new time (Create has its own ghost above).
                                let dragged = drag.filter(|d| d.entry.as_ref().map(|e| e.id) == Some(ev.id));
                                let live = dragged.and_then(|d| match d.kind {
                                    DragKind::Resize => Some((d.start_min, d.resize_end())),
                                    DragKind::Move if d.day == i => {
                                        let s = d.move_start();
                                        Some((s, s + d.orig_dur))
                                    }
                                    _ => None,
                                });
                                let (top_px, height_px, time_label) = match live {
                                    Some((s, e)) => (
                                        s * CAL_HOUR / 60,
                                        ((e - s) * CAL_HOUR / 60).max(20),
                                        cal_time_label(s, e),
                                    ),
                                    None => (ev.top, ev.height, ev.time_label.clone()),
                                };
                                // Dim the untimed block being reordered — its ghost
                                // follows the cursor instead.
                                let reordering = dragged.is_some_and(|d| d.kind == DragKind::Reorder);
                                let base = if reordering {
                                    "ts-cal-event dragging"
                                } else if live.is_some() {
                                    "ts-cal-event timed live"
                                } else if ev.timed {
                                    "ts-cal-event timed"
                                } else {
                                    "ts-cal-event"
                                };
                                // Locked entries (submitted/approved/invoiced) can't be
                                // dragged; mark them and explain why on hover.
                                let locked = busy || !ev.editable;
                                let ev_class = if locked {
                                    format!("{base} locked")
                                } else {
                                    base.to_string()
                                };
                                let lock_title = match ev.state {
                                    EntryState::Submitted => "Submitted for approval — can't be moved",
                                    EntryState::Approved => "Approved — can't be moved",
                                    EntryState::Invoiced => "Invoiced — can't be moved",
                                    EntryState::Open if locked => "Not currently editable",
                                    EntryState::Open => "",
                                };
                                let id = ev.id;
                                let timed = ev.timed;
                                rsx! {
                                div {
                                    key: "{id}",
                                    class: "{ev_class}",
                                    title: "{lock_title}",
                                    style: "top: {top_px}px; height: {height_px}px; left: calc(4px + {ev.lane} * (100% - 8px) / {ev.lanes}); width: calc((100% - 8px) / {ev.lanes} - 2px); right: auto;",
                                    // Over a block there's no free slot — clear the
                                    // "+ Add time" hint (the column's mousemove can't
                                    // fire while the block captures the pointer).
                                    onmouseenter: move |_| {
                                        if add_hint.peek().is_some() {
                                            add_hint.set(None);
                                        }
                                    },
                                    // Pressing a timed entry starts a move drag (its
                                    // body); a plain click with no move opens it for
                                    // editing. Untimed entries just open for editing.
                                    onmousedown: move |e: MouseEvent| {
                                        e.stop_propagation();
                                        if locked { return; }
                                        let Some(entry) = entry_at(i, id) else { return; };
                                        let start = entry.start_minute.unwrap_or(0);
                                        let dur = entry.minutes;
                                        if timed {
                                            let off =
                                                (e.element_coordinates().y * 60.0 / CAL_HOUR as f64) as i32;
                                            let g = start + off;
                                            cal_drag.set(Some(CalDrag {
                                                kind: DragKind::Move,
                                                day: i,
                                                start_min: start,
                                                cur_min: g,
                                                grab_min: g,
                                                entry: Some(entry),
                                                orig_dur: dur,
                                                orig_day: i,
                                            }));
                                        } else {
                                            // Untimed: drag to reorder within the
                                            // day's stack; cur_min tracks the drop.
                                            let m = cal_y_to_min(e.element_coordinates().y);
                                            cal_drag.set(Some(CalDrag {
                                                kind: DragKind::Reorder,
                                                day: i,
                                                start_min: 0,
                                                cur_min: m,
                                                grab_min: m,
                                                entry: Some(entry),
                                                orig_dur: dur,
                                                orig_day: i,
                                            }));
                                        }
                                    },
                                    div { class: "ts-cal-ev-project",
                                        span { class: "ts-cal-ev-name", "{ev.project}" }
                                        span { class: "ts-cal-ev-dur", "{ev.duration}" }
                                    }
                                    if ev.timed {
                                        div { class: "ts-cal-ev-time", "{time_label}" }
                                    }
                                    if !ev.task.is_empty() {
                                        div { class: "ts-cal-ev-task", "{ev.task}" }
                                    }
                                    if !ev.client.is_empty() {
                                        div { class: "ts-cal-ev-client", "{ev.client}" }
                                    }
                                    if ev.timed && !locked {
                                        div {
                                            class: "ts-cal-resize",
                                            onmousedown: move |e: MouseEvent| {
                                                e.stop_propagation();
                                                let Some(entry) = entry_at(i, id) else { return; };
                                                let start = entry.start_minute.unwrap_or(0);
                                                let dur = entry.minutes;
                                                cal_drag.set(Some(CalDrag {
                                                    kind: DragKind::Resize,
                                                    day: i,
                                                    start_min: start,
                                                    cur_min: start + dur,
                                                    grab_min: start + dur,
                                                    entry: Some(entry),
                                                    orig_dur: dur,
                                                    orig_day: i,
                                                }));
                                            },
                                        }
                                    }
                                }
                                }
                                }
                            }
                        }
                    }
                    div { class: "ts-cal-tail" }
                }
            }
        }
    }
}
