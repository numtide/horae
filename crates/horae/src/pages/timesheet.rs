use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use dioxus::html::geometry::PixelsVector2D;
use dioxus::prelude::*;
use uuid::Uuid;

use horae_core::duration::format_hhmm;
use horae_core::week::week_start as start_of_week;

use super::loaded;
use crate::components::controls::Segmented;
use crate::components::date_picker::DatePicker;
use crate::components::menu::{Menu, MenuItem};
use crate::components::modal::Modal;
use crate::components::timer_widget::use_running_timer;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{
    TimesheetCommand, TimesheetEntryInput, TimesheetPolicy, TimesheetWriteContext, VisibleTimeEntry,
};
use crate::route::Route;
use crate::server_fns;

mod data;
mod people;
mod tracking;

#[cfg(test)]
mod refresh_tests;

/// Offset (0..=6) of `today` within the week starting `week_start`,
/// or `None` when today falls outside that week.
fn today_offset(today: NaiveDate, week_start: NaiveDate) -> Option<usize> {
    let o = (today - week_start).num_days();
    (0..7).contains(&o).then_some(o as usize)
}

/// A weekday column's CSS class: `base`, plus a `today`/`weekend` modifier.
fn day_col_class(base: &str, today_off: Option<usize>, i: usize, weekday: Weekday) -> String {
    if today_off == Some(i) {
        format!("{base} today")
    } else if matches!(weekday, Weekday::Sat | Weekday::Sun) {
        format!("{base} weekend")
    } else {
        base.to_string()
    }
}

/// A week-grid value cell's class: `base`, plus `empty` when zero or `today`
/// when it's today's column.
fn value_cell_class(base: &str, minutes: i32, today_off: Option<usize>, i: usize) -> String {
    if minutes == 0 {
        format!("{base} empty")
    } else if today_off == Some(i) {
        format!("{base} today")
    } else {
        base.to_string()
    }
}

/// Empty is an intentional clear/start action, not a failed parse.
fn entry_minutes(input: &str) -> Result<Option<i32>, &'static str> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    match horae_core::duration::parse(input) {
        Ok(m) if m <= 24 * 60 => Ok(Some(m as i32)),
        Ok(_) => Err("Duration can't exceed 24 hours."),
        Err(_) => Err("Enter a duration like 1:30 or 1.5 (at most 24 hours)."),
    }
}

/// Only a blank new entry on today, without a start time, may start a timer.
/// A typed zero is a duration, and invalid text must never start a timer.
fn modal_minutes(input: &str, can_start_timer: bool) -> Result<Option<i32>, &'static str> {
    match entry_minutes(input)? {
        None if can_start_timer => Ok(None),
        None | Some(0) => Err("Duration must be greater than zero."),
        minutes => Ok(minutes),
    }
}

struct CellFields {
    minutes: i32,
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,
}

fn cell_fields(
    input: &str,
    existing: Option<&VisibleTimeEntry>,
) -> Result<CellFields, &'static str> {
    Ok(CellFields {
        minutes: entry_minutes(input)?.unwrap_or(0),
        notes: existing.and_then(|e| e.notes.clone()),
        billable: existing.is_none_or(|e| e.billable),
        start_minute: existing.and_then(|e| e.start_minute),
    })
}

/// Create, update, or (when `minutes` is 0) delete a time entry — `existing` is
/// the entry to change, or `None` to create one. Shared by the week grid cells
/// and the entry dialog so both save the same way.
async fn persist_entry(
    context: TimesheetWriteContext,
    existing: Option<Uuid>,
    entry: TimesheetEntryInput,
) -> Result<(), ServerFnError> {
    let command = match (existing, entry.minutes) {
        (Some(entry_id), 0) => TimesheetCommand::Delete {
            entry_ids: vec![entry_id],
        },
        (Some(entry_id), _) => TimesheetCommand::Update { entry_id, entry },
        (None, m) if m > 0 => TimesheetCommand::Create { entry },
        _ => return Ok(()),
    };
    server_fns::apply_timesheet_command(context, command).await
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    Day,
    #[default]
    Week,
    Calendar,
}

impl std::fmt::Display for ViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ViewMode::Day => "day",
            ViewMode::Week => "week",
            ViewMode::Calendar => "calendar",
        })
    }
}

impl std::str::FromStr for ViewMode {
    type Err = std::convert::Infallible;
    // Unknown values fall back to Week so a stray URL never fails to route.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "day" => ViewMode::Day,
            "calendar" => ViewMode::Calendar,
            _ => ViewMode::Week,
        })
    }
}

/// The timesheet's anchor day, carried in the URL path
/// (`/timesheet/<view>/YYYY-MM-DD`). The week shown is the ISO week containing
/// it; in Day view it is the selected day.
#[derive(Clone, Copy, PartialEq)]
pub struct Anchor(pub NaiveDate);

impl Default for Anchor {
    fn default() -> Self {
        Anchor(chrono::Utc::now().date_naive())
    }
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl std::str::FromStr for Anchor {
    type Err = std::convert::Infallible;
    // A malformed date falls back to today rather than failing the route.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Anchor)
            .unwrap_or_default())
    }
}

/// How many days the Calendar view shows at once. Carried in the URL query
/// (`?span=week|5day|day`) so the chosen span is shareable and survives reload.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum CalSpan {
    /// The organization's full seven-day week.
    #[default]
    Week,
    /// Mon–Fri.
    WorkWeek,
    /// Just the anchor day.
    Day,
}

impl std::fmt::Display for CalSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CalSpan::Week => "week",
            CalSpan::WorkWeek => "5day",
            CalSpan::Day => "day",
        })
    }
}

impl std::str::FromStr for CalSpan {
    type Err = std::convert::Infallible;
    // Unknown values fall back to the week view so a stray URL never fails to route.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "day" => CalSpan::Day,
            "5day" => CalSpan::WorkWeek,
            _ => CalSpan::Week,
        })
    }
}

impl CalSpan {
    /// Offsets within the configured week, given the anchor day's own offset.
    fn visible_days(self, anchor: usize, first_day: Weekday) -> Vec<usize> {
        match self {
            CalSpan::Week => (0..7).collect(),
            CalSpan::WorkWeek => (0..7)
                .filter(|i| (i + first_day.num_days_from_monday() as usize) % 7 < 5)
                .collect(),
            CalSpan::Day => vec![anchor.min(6)],
        }
    }

    /// Dropdown label (Harvest wording).
    fn label(self) -> &'static str {
        match self {
            CalSpan::Week => "Week view",
            CalSpan::WorkWeek => "5-day view",
            CalSpan::Day => "Day view",
        }
    }
}

#[component]
pub fn Timesheet(view: ViewMode, date: Anchor, span: CalSpan, user: String) -> Element {
    let config = use_resource(server_fns::get_week_start);
    // Keep the requester pinned across selected-person/date/view remounts.
    let requester = use_signal(|| None::<PermissionRequester>);
    let subject = if user.is_empty() {
        None
    } else if let Ok(id) = user.parse::<Uuid>() {
        Some(id)
    } else {
        return rsx! { div { class: "alert alert-danger", role: "alert", "Invalid timesheet person. Choose Timesheet from the navigation to return to your own time." } };
    };
    loaded(&config.read(), |first_day| {
        let Some(start) = start_of_week(date.0, *first_day)
            .filter(|start| start.checked_add_days(chrono::Days::new(6)).is_some())
        else {
            return rsx! { div { class: "alert alert-danger", "This week is outside the supported date range." } };
        };
        rsx! { TimesheetContent { key: "{view}:{date}:{span}:{user}", view, date, span, start, subject, requester } }
    })
}

#[component]
fn TimesheetContent(
    view: ViewMode,
    date: Anchor,
    span: CalSpan,
    start: NaiveDate,
    subject: Option<Uuid>,
    mut requester: Signal<Option<PermissionRequester>>,
) -> Element {
    let today = chrono::Utc::now().date_naive();
    // View, week, selected day and calendar span all derive from the URL
    // (/timesheet/<view>/<date>?span=<span>), so switching views, changing the
    // calendar span or navigating is shareable and works with the browser's
    // back/forward. Actions push a new route.
    let view_mode = use_memo(use_reactive!(|(view,)| view));
    let week_start = use_memo(use_reactive!(|(start,)| start));
    // Which day is selected within the configured week for Day view.
    let selected_day_offset = use_memo(use_reactive!(|(date,)| (date.0 - week_start()).num_days()));

    // Push a new view/anchor/span to the URL.
    let go = use_callback(move |(v, anchor, span): (ViewMode, NaiveDate, CalSpan)| {
        navigator().push(Route::Timesheet {
            view: v,
            date: Anchor(anchor),
            span,
            user: subject.map(|id| id.to_string()).unwrap_or_default(),
        });
    });
    // Selecting a day in the Day-view strip navigates to that day (span carried
    // through so returning to the Calendar keeps the chosen span).
    let select_day = use_callback(move |i: i64| {
        go.call((ViewMode::Day, week_start() + Duration::days(i), span));
    });

    // The rail owns the running-timer display, and it is shared: starting a timer
    // here refreshes the rail, and a timer started from the rail invalidates the
    // entries below.
    let running_timer = use_running_timer();

    let mut sheet = use_resource(move || {
        let ws = *week_start.read();
        // Starting or stopping a timer adds or closes an entry in this week, so
        // subscribe to the shared timer: the rail can start one without knowing
        // this page exists.
        let _changes = running_timer.changes();
        let expected = *requester.peek();
        async move {
            let we = ws + chrono::Duration::days(6);
            (
                ws,
                data::load(subject, ws, we, expected, server_fns::load_timesheet_page).await,
            )
        }
    });
    let current_sheet = use_memo(move || {
        data::current(
            sheet.state() == UseResourceState::Ready,
            week_start(),
            &sheet.read(),
        )
        .cloned()
    });
    use_effect(move || {
        if let Some(Ok(page)) = current_sheet.read().as_ref()
            && requester.peek().is_none()
        {
            requester.set(Some(page.requester));
        }
    });
    // Resources retain their previous value during a refresh. Do not render
    // those rows or derive totals until the complete new window is admitted.
    let entries = use_memo(move || {
        current_sheet
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .map(|page| page.entries.clone())
            .unwrap_or_default()
    });
    let context = use_memo(move || {
        current_sheet
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .map(|page| TimesheetWriteContext {
                expected_requester: page.requester,
                subject_id: page.subject.id,
                expected_policy: page.policy,
            })
    });
    let policy = use_memo(move || context().map(|context| context.expected_policy));
    let mut tracking_page = use_resource(move || {
        let expected = context();
        async move {
            let result = match expected {
                Some(context) => server_fns::load_timesheet_tracking(context).await,
                None => Ok(Vec::new()),
            };
            (expected, result)
        }
    });
    let current_tracking = use_memo(move || {
        if tracking_page.state() != UseResourceState::Ready || context().is_none() {
            return None;
        }
        tracking_page
            .read()
            .as_ref()
            .filter(|(expected, _)| expected == &context())
            .map(|(_, result)| result.clone())
    });
    let tracking = use_memo(move || {
        current_tracking
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .cloned()
            .unwrap_or_default()
    });
    let mut pending_action = use_signal(|| false);

    // Lookups and grid data are memoized so they rebuild only when their
    // resources (or the selected week) change — not on every render, e.g. each
    // keystroke in the add-entry modal.
    let project_names = use_memo(move || -> HashMap<Uuid, String> {
        let mut names: HashMap<_, _> = tracking
            .read()
            .iter()
            .map(|p| (p.project_id, p.project_name.clone()))
            .collect();
        names.extend(
            entries
                .read()
                .iter()
                .map(|e| (e.project_id, e.project_name.clone())),
        );
        names
    });
    let task_names = use_memo(move || -> HashMap<Uuid, String> {
        let mut names: HashMap<_, _> = tracking
            .read()
            .iter()
            .map(|t| (t.task_id, t.task_name.clone()))
            .collect();
        names.extend(
            entries
                .read()
                .iter()
                .map(|e| (e.task_id, e.task_name.clone())),
        );
        names
    });
    // Time visibility supplies historical labels, not financial authority.
    let project_client = use_memo(move || -> HashMap<Uuid, String> {
        entries
            .read()
            .iter()
            .map(|e| (e.project_id, e.client_name.clone()))
            .collect()
    });

    let ws = *week_start.read();
    let week_end = ws + Duration::days(6);

    // Entries for the visible week, grouped by weekday, with per-day totals.
    let week_entries = use_memo(move || -> Vec<VisibleTimeEntry> {
        let ws = week_start();
        let we = ws + Duration::days(6);
        entries
            .read()
            .iter()
            .filter(|e| e.spent_date >= ws && e.spent_date <= we)
            .cloned()
            .collect()
    });
    let by_day = use_memo(move || -> [Vec<VisibleTimeEntry>; 7] {
        let ws = week_start();
        let mut by_day: [Vec<VisibleTimeEntry>; 7] = Default::default();
        for entry in week_entries.read().iter() {
            let offset = (entry.spent_date - ws).num_days();
            if (0..7).contains(&offset) {
                by_day[offset as usize].push(entry.clone());
            }
        }
        by_day
    });
    let daily_totals = use_memo(move || -> Vec<i32> {
        by_day
            .read()
            .iter()
            .map(|d| d.iter().map(|e| e.minutes).sum())
            .collect()
    });
    let week_total: i32 = daily_totals.read().iter().sum();

    // A week can mix open and settled entries — invoicing bills one client and
    // leaves another open — and `submit_week` transitions only the open ones.
    let has_open = week_entries
        .read()
        .iter()
        .any(|e| e.state == horae_core::types::EntryState::Open);
    let all_submitted_or_approved = !week_entries.read().is_empty() && !has_open;

    let submit_status = use_signal(|| None::<String>);
    let mut grid_error = use_signal(|| None::<String>);

    // Add–entry modal state. `add_open` holds the date the new entry is for
    // (None = closed); the rest back the form fields.
    let mut add_open = use_signal(|| None::<NaiveDate>);
    let mut add_project = use_signal(String::new);
    let mut add_task = use_signal(String::new);
    let mut add_notes = use_signal(String::new);
    let mut add_duration = use_signal(String::new);
    let mut add_error = use_signal(|| None::<String>);
    let mut add_saving = use_signal(|| false);
    // When set, the modal edits this existing entry instead of creating one; its
    // billable flag is carried through (the modal doesn't expose it).
    let mut editing = use_signal(|| None::<Uuid>);
    let mut edit_billable = use_signal(|| true);
    // Legacy own edits may retain their historical source without making that
    // pair eligible for a new entry, timer, or another person's sheet.
    let modal_tracking = use_memo(move || {
        let mut options = tracking();
        if policy() == Some(TimesheetPolicy::LegacyOwn)
            && let Some(id) = editing()
            && let Some(entry) = entries.read().iter().find(|entry| entry.id == id)
            && !tracking::eligible(&options, entry.project_id, entry.task_id)
        {
            options.push(crate::models::scoped_time::TimesheetTrackingOption {
                project_id: entry.project_id,
                project_name: entry.project_name.clone(),
                task_id: entry.task_id,
                task_name: entry.task_name.clone(),
                billable: entry.billable,
            });
        }
        options
    });
    // The entry's optional start time (minutes since midnight); None = untimed.
    // Set by a calendar drag or when editing a timed entry; carried into save.
    let mut add_start = use_signal(|| None::<i32>);

    // Whether the entry modal's primary action starts a timer (Harvest-style):
    // a new entry on today's column with no duration typed yet. Once a duration
    // or a start time is set the entry is clearly a fixed one, so the primary
    // saves instead of starting a timer.
    let can_start_timer = use_memo(move || {
        let Some(date) = *add_open.read() else {
            return false;
        };
        editing.read().is_none() && date == today && add_start.read().is_none()
    });
    let timer_mode = use_memo(move || {
        matches!(
            modal_minutes(&add_duration.read(), can_start_timer()),
            Ok(None)
        )
    });

    // Open the modal to create a new entry for `date`, defaulting the selects to
    // the first project/task.
    let open_add = use_callback(move |date: NaiveDate| {
        if pending_action() {
            return;
        }
        let Some(first) = tracking.read().first().cloned() else {
            return;
        };
        editing.set(None);
        add_project.set(first.project_id.to_string());
        add_task.set(first.task_id.to_string());
        add_notes.set(String::new());
        add_duration.set(String::new());
        add_start.set(None);
        add_error.set(None);
        add_open.set(Some(date));
    });

    // Opening the modal does not grant write access to its source or destination.
    let open_edit = use_callback(move |e: VisibleTimeEntry| {
        if pending_action() || !tracking::editable(policy(), &tracking.read(), &e) {
            return;
        }
        editing.set(Some(e.id));
        add_project.set(e.project_id.to_string());
        add_task.set(e.task_id.to_string());
        add_notes.set(e.notes.clone().unwrap_or_default());
        add_duration.set(format_hhmm(e.minutes.into()));
        add_start.set(e.start_minute);
        edit_billable.set(e.billable);
        add_error.set(None);
        add_open.set(Some(e.spent_date));
    });

    // Calendar drag: the slot/entry being manipulated, committed on release.
    let cal_drag = use_signal(|| None::<CalDrag>);

    // The free calendar slot the cursor is over: (column, snapped minute). Drives
    // the cursor-following "+ Add time" hint.
    let add_hint = use_signal(|| None::<(usize, i32)>);
    let drag_commit = use_callback(move |d: CalDrag| {
        let ws = *week_start.read();
        let clamp = |start: i32, dur: i32| {
            horae_core::time_of_day::clamp_to_day(start.clamp(0, 1439) as u16, dur.max(0) as u32)
                as i32
        };
        // Refresh after refusals too: another session may have changed the entry,
        // or a response may have been lost after the server committed the move.
        let Some(context) = context() else {
            return;
        };
        if pending_action() {
            return;
        }
        let mut commit = move |command: TimesheetCommand| {
            pending_action.set(true);
            let mut timer = running_timer;
            spawn(async move {
                match server_fns::apply_timesheet_command(context, command).await {
                    Ok(()) => grid_error.set(None),
                    Err(e) => grid_error.set(Some(format!("Could not change entry: {e}"))),
                }
                timer.refresh();
                pending_action.set(false);
            });
        };
        // Submitted edits still need the approval-coverage integration.
        if let Some(entry) = d.entry.clone()
            && !tracking::editable(policy(), &tracking.read(), &entry)
        {
            open_edit.call(entry);
            return;
        }
        match d.kind {
            // Draw a new slot → open the entry form prefilled at that hour. A drag
            // sets the duration to the dragged span; a plain click seeds a default
            // one-hour block starting at the clicked time.
            DragKind::Create => {
                let day = ws + Duration::days(d.day as i64);
                let start = d.start_min.min(d.cur_min).clamp(0, 1439);
                let raw = (d.cur_min - d.start_min).abs();
                let dur = if raw >= i32::from(horae_core::time_of_day::MIN_DURATION) {
                    raw
                } else {
                    60
                };
                open_add.call(day);
                add_start.set(Some(start));
                add_duration.set(format_hhmm(clamp(start, dur).into()));
            }
            // Move an entry → new start follows the pointer (keeping the grab
            // offset), possibly to another day. No movement → open it for editing.
            DragKind::Move => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                let new_start = d.move_start();
                if new_start == d.start_min && d.day == d.orig_day {
                    open_edit.call(entry);
                    return;
                }
                let dur = clamp(new_start, d.orig_dur);
                commit(TimesheetCommand::Reschedule {
                    entry_id: entry.id,
                    spent_date: ws + Duration::days(d.day as i64),
                    start_minute: new_start,
                    minutes: dur,
                });
            }
            // Resize an entry → new duration from its start to the pointer.
            DragKind::Resize => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                let dur = clamp(d.start_min, d.resize_end() - d.start_min);
                commit(TimesheetCommand::Reschedule {
                    entry_id: entry.id,
                    spent_date: ws + Duration::days(d.orig_day as i64),
                    start_minute: d.start_min,
                    minutes: dur,
                });
            }
            // Reorder an untimed entry within its day's stack. No move (or a drop
            // on another day) → treat as a click and open it for editing.
            DragKind::Reorder => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                // Drop column = target day; the same call reorders within a day and
                // moves the entry to another day (its spent_date follows).
                let target_date = ws + Duration::days(d.day as i64);
                let day: Vec<VisibleTimeEntry> = entries
                    .read()
                    .iter()
                    .filter(|e| e.spent_date == target_date)
                    .cloned()
                    .collect();
                let mut ordered = untimed_ordered(&day);
                let before: Vec<Uuid> = ordered.iter().map(|e| e.id).collect();
                // Drop the moved entry from the target list (present only on a
                // same-day reorder) and re-insert it at the drop position.
                ordered.retain(|e| e.id != entry.id);
                let mut cum = 0i32;
                let mut to = ordered.len();
                for (idx, e) in ordered.iter().enumerate() {
                    if d.cur_min < cum + e.minutes / 2 {
                        to = idx;
                        break;
                    }
                    cum += e.minutes;
                }
                ordered.insert(to, entry.clone());
                let after: Vec<Uuid> = ordered.iter().map(|e| e.id).collect();
                // Same day and unchanged order → treat as a click and open editing.
                if d.day == d.orig_day && after == before {
                    open_edit.call(entry);
                    return;
                }
                commit(TimesheetCommand::Reorder {
                    spent_date: target_date,
                    entry_ids: after,
                });
            }
        }
    });

    // Start a timer for an existing entry's project/task (the Day-view "Start"
    // action, Harvest-style resume).
    let start_entry = use_callback(move |e: VisibleTimeEntry| {
        let Some(context) = context() else {
            return;
        };
        if pending_action() || !tracking::eligible(&tracking.read(), e.project_id, e.task_id) {
            return;
        }
        pending_action.set(true);
        let mut timer = running_timer;
        spawn(async move {
            match server_fns::apply_timesheet_command(
                context,
                TimesheetCommand::StartTimer {
                    project_id: e.project_id,
                    task_id: e.task_id,
                    notes: e.notes,
                },
            )
            .await
            {
                Ok(_) => grid_error.set(None),
                Err(e) => grid_error.set(Some(format!("Could not start timer: {e}"))),
            }
            timer.refresh();
            pending_action.set(false);
        });
    });
    let stop_entry = use_callback(move |entry: VisibleTimeEntry| {
        let Some(context) = context() else {
            return;
        };
        if pending_action()
            || !tracking::stoppable(
                &tracking.read(),
                &entry,
                Some(context.expected_requester.user_id),
            )
        {
            return;
        }
        pending_action.set(true);
        let mut timer = running_timer;
        spawn(async move {
            match server_fns::apply_timesheet_command(
                context,
                TimesheetCommand::StopTimer { entry_id: entry.id },
            )
            .await
            {
                Ok(()) => grid_error.set(None),
                Err(error) => grid_error.set(Some(format!("Could not stop timer: {error}"))),
            }
            timer.refresh();
            pending_action.set(false);
        });
    });

    // ── Editable week grid ───────────────────────────────────────────────────
    // Rows added via "Add row" that have no entries yet this week.
    let mut pending_rows = use_signal(Vec::<(Uuid, Uuid)>::new);
    let mut removing_row = use_signal(|| false);
    let mut addrow_open = use_signal(|| false);
    let mut addrow_project = use_signal(String::new);
    let mut addrow_task = use_signal(String::new);
    // Keep keystrokes outside the refreshable grid: another cell's save may
    // unmount it before this cell has emitted blur/change.
    let mut cell_drafts = use_signal(HashMap::<CellKey, String>::new);
    let mut saving_cells = use_signal(HashSet::<CellKey>::new);

    // Commit a grid cell: create, update, or clear the entry behind it, then
    // reload. Notes, billability, and the start time of an update are preserved.
    let commit_cell = use_callback(move |edit: CellEdit| {
        let Some(context) = context() else {
            grid_error.set(Some(
                "Wait for the timesheet to finish loading before saving.".into(),
            ));
            return;
        };
        let key = (edit.project_id, edit.task_id, edit.day);
        if pending_action()
            || removing_row()
            || (edit.existing.is_none()
                && !tracking::eligible(&tracking.read(), edit.project_id, edit.task_id))
            || saving_cells.read().contains(&key)
            || cell_drafts.read().get(&key) != Some(&edit.input)
        {
            return;
        }
        let mut fields = {
            let entries = week_entries.read();
            let existing = edit
                .existing
                .and_then(|id| entries.iter().find(|e| e.id == id));
            if edit.existing.is_some() && existing.is_none() {
                grid_error.set(Some(
                    "Entry is no longer available. Refresh and try again.".to_string(),
                ));
                return;
            }
            if existing.is_some_and(|entry| !tracking::editable(policy(), &tracking.read(), entry))
            {
                return;
            }
            match cell_fields(&edit.input, existing) {
                Ok(fields) => fields,
                Err(message) => {
                    grid_error.set(Some(message.to_string()));
                    return;
                }
            }
        };
        if edit.existing.is_none() {
            fields.billable = tracking
                .read()
                .iter()
                .find(|option| {
                    option.project_id == edit.project_id && option.task_id == edit.task_id
                })
                .is_some_and(|option| option.billable);
        }
        saving_cells.write().insert(key);
        let mut timer = running_timer;
        spawn(async move {
            let res = persist_entry(
                context,
                edit.existing,
                TimesheetEntryInput {
                    project_id: edit.project_id,
                    task_id: edit.task_id,
                    spent_date: edit.day,
                    minutes: fields.minutes,
                    notes: fields.notes,
                    billable: fields.billable,
                    start_minute: fields.start_minute,
                },
            )
            .await;
            match res {
                Ok(()) => {
                    let mut drafts = cell_drafts.write();
                    if drafts.get(&key) == Some(&edit.input) {
                        drafts.remove(&key);
                    }
                    if fields.minutes > 0 {
                        pending_rows
                            .write()
                            .retain(|row| *row != (edit.project_id, edit.task_id));
                    }
                    grid_error.set(None);
                    timer.refresh();
                }
                // Re-read saved facts on failure, but retain the user's input
                // separately so it can be corrected and retried.
                Err(e) => {
                    grid_error.set(Some(format!("Could not save: {e}")));
                    timer.refresh();
                }
            }
            saving_cells.write().remove(&key);
        });
    });

    // Remove a row: drop a pending one, or delete every entry it holds this week.
    let remove_row = use_callback(move |key: (Uuid, Uuid)| {
        let start = week_start();
        let Some(context) = context() else {
            return;
        };
        if pending_action() || removing_row() {
            return;
        }
        if saving_cells.read().iter().any(|&(project, task, day)| {
            (project, task) == key && (0..7).contains(&(day - start).num_days())
        }) {
            grid_error.set(Some(
                "Wait for this row's changes to finish saving before removing it.".into(),
            ));
            return;
        }
        pending_rows.write().retain(|k| *k != key);
        let entries: Vec<VisibleTimeEntry> = week_entries
            .read()
            .iter()
            .filter(|e| e.project_id == key.0 && e.task_id == key.1)
            .cloned()
            .collect();
        if entries.is_empty() {
            discard_row_drafts(&mut cell_drafts.write(), key, start);
            return;
        }
        if entries
            .iter()
            .any(|entry| !tracking::editable(policy(), &tracking.read(), entry))
        {
            grid_error.set(Some(
                "This row contains entries that cannot currently be removed.".into(),
            ));
            return;
        }
        removing_row.set(true);
        let mut timer = running_timer;
        spawn(async move {
            let command = TimesheetCommand::Delete {
                entry_ids: entries.iter().map(|entry| entry.id).collect(),
            };
            match server_fns::apply_timesheet_command(context, command).await {
                Ok(()) => {
                    discard_row_drafts(&mut cell_drafts.write(), key, start);
                    grid_error.set(None);
                }
                Err(error) => grid_error.set(Some(format!("Could not remove row: {error}"))),
            }
            timer.refresh();
            removing_row.set(false);
        });
    });

    let open_add_row = use_callback(move |()| {
        if pending_action() {
            return;
        }
        let Some(first) = tracking.read().first().cloned() else {
            return;
        };
        addrow_project.set(first.project_id.to_string());
        addrow_task.set(first.task_id.to_string());
        addrow_open.set(true);
    });

    let busy = use_memo(move || {
        pending_action() || add_saving() || removing_row() || !saving_cells.read().is_empty()
    });
    let dirty = !cell_drafts.read().is_empty()
        || add_open().is_some()
        || addrow_open()
        || !pending_rows.read().is_empty()
        || cal_drag.read().is_some();
    let week_actions = WeekActions {
        commit: commit_cell,
        remove_row,
        removing_row: removing_row.into(),
        add_row: open_add_row,
        drafts: cell_drafts,
        saving: saving_cells.into(),
        tracking,
        busy,
        policy,
    };

    // Shared validation for the entry dialog's Start-timer / Save actions: a
    // project and task must be picked. Returns the values (notes trimmed) or sets
    // the modal error and yields None.
    let read_pt_notes = use_callback(move |()| -> Option<(Uuid, Uuid, Option<String>)> {
        let (Ok(project_id), Ok(task_id)) =
            (add_project().parse::<Uuid>(), add_task().parse::<Uuid>())
        else {
            add_error.set(Some("Select a project and task.".to_string()));
            return None;
        };
        if !tracking::eligible(&modal_tracking.read(), project_id, task_id) {
            add_error.set(Some(
                "This project and task are not currently available for this person.".into(),
            ));
            return None;
        }
        let notes = {
            let n = add_notes.read().trim().to_string();
            (!n.is_empty()).then_some(n)
        };
        Some((project_id, task_id, notes))
    });

    // The "+" button adds for today when it's in the viewed week, else its first day.
    let add_default_date = if (0..7).contains(&(today - ws).num_days()) {
        today
    } else {
        ws
    };

    let current_mode = *view_mode.read();
    let sel_offset = *selected_day_offset.read();
    // The pager moves a single day in Day view and in the Calendar's single-day
    // span; otherwise it moves a whole week (like Harvest).
    let day_paged = current_mode == ViewMode::Day
        || (current_mode == ViewMode::Calendar && span == CalSpan::Day);

    let mut picker_open = use_signal(|| false);
    // Pager stepping. Moving the anchor date across the week edge rolls the week
    // automatically.
    let step = use_callback(move |forward: bool| {
        let days = if day_paged { 1 } else { 7 };
        let delta = Duration::days(if forward { days } else { -days });
        go.call((current_mode, date.0 + delta, span));
    });
    let is_this_week = today_offset(today, ws).is_some();
    let range_label = format!("{} – {}", ws.format("%d %b"), week_end.format("%d %b %Y"));

    rsx! {
        div {
            "data-editor-kind": "timesheet",
            "data-editor-state": if busy() { "pending" } else if dirty { "dirty" } else { "clean" },
            // Header: title + last-saved + view toggle
            div { class: "ts-header",
                h1 { class: "page-title", "Timesheet" }
                if matches!(&*current_sheet.read(), Some(Ok(_))) {
                    span { class: "ts-saved", "{format_hhmm(week_total.into())} this week" }
                }
                Segmented {
                    items: vec!["Day".to_string(), "Week".to_string(), "Calendar".to_string()],
                    active: match current_mode {
                        ViewMode::Day => "Day",
                        ViewMode::Week => "Week",
                        ViewMode::Calendar => "Calendar",
                    }
                        .to_string(),
                    onselect: move |v: String| {
                        let v = match v.as_str() {
                            "Day" => ViewMode::Day,
                            "Calendar" => ViewMode::Calendar,
                            _ => ViewMode::Week,
                        };
                        go.call((v, date.0, span));
                    },
                }
            }

            // Toolbar: add entry + week pager
            div { class: "ts-toolbar",
                button {
                    class: "ts-add",
                    "aria-label": "Add entry",
                    disabled: busy() || tracking.read().is_empty(),
                    onclick: move |_| open_add.call(add_default_date),
                    "+"
                }
                // The pager's own label is the date picker's trigger. The popover
                // is a sibling of .ts-pager, which clips its children to keep the
                // arrows inside its rounded border.
                div { class: "menu-anchor",
                    div { class: "ts-pager",
                        button {
                            class: "ts-pager-btn prev",
                            "aria-label": if day_paged { "Previous day" } else { "Previous week" },
                            onclick: move |_| step.call(false),
                            "←"
                        }
                        button {
                            r#type: "button",
                            class: "ts-pager-label",
                            "aria-haspopup": "dialog",
                            "aria-expanded": "{picker_open}",
                            onclick: move |_| {
                                let next = !picker_open();
                                picker_open.set(next);
                            },
                            span { class: "text-faint", "▦" }
                            if day_paged {
                                {
                                    let d = ws + Duration::days((*selected_day_offset.read()).clamp(0, 6));
                                    rsx! {
                                        span { class: "cur", if d == today { "Today" } else { "{d.format(\"%A\")}" } }
                                        span { class: "ts-pager-range", "{d.format(\"%d %b %Y\")}" }
                                    }
                                }
                            } else {
                                span { class: "cur", if is_this_week { "This week" } else { "Week" } }
                                span { class: "ts-pager-range", "{range_label}" }
                            }
                        }
                        button {
                            class: "ts-pager-btn next",
                            "aria-label": if day_paged { "Next day" } else { "Next week" },
                            onclick: move |_| step.call(true),
                            "→"
                        }
                    }
                    if picker_open() {
                        div { class: "menu-overlay", onclick: move |_| picker_open.set(false) }
                        div { class: "dp-pop",
                            DatePicker {
                                first_day: ws.weekday(),
                                selected: date.0,
                                week: !day_paged,
                                onpick: move |d| {
                                    picker_open.set(false);
                                    go.call((current_mode, d, span));
                                },
                            }
                        }
                    }
                }
                // Calendar-only: day-range dropdown beside the pager (Harvest-style).
                // Picking a span pushes it to the URL so it's shareable.
                if current_mode == ViewMode::Calendar {
                    Menu { id: "calendar-span-menu", label: span.label().to_string(),
                        MenuItem {
                            selected: span == CalSpan::Day,
                            onclick: move |_| go.call((ViewMode::Calendar, date.0, CalSpan::Day)),
                            "Day view"
                        }
                        MenuItem {
                            selected: span == CalSpan::WorkWeek,
                            onclick: move |_| go.call((ViewMode::Calendar, date.0, CalSpan::WorkWeek)),
                            "5-day view"
                        }
                        MenuItem {
                            selected: span == CalSpan::Week,
                            onclick: move |_| go.call((ViewMode::Calendar, date.0, CalSpan::Week)),
                            "Week view"
                        }
                    }
                }
                if !is_this_week {
                    button {
                        class: "btn btn-ghost btn-sm",
                        onclick: move |_| go.call((current_mode, today, span)),
                        "Today"
                    }
                }
            }

            if let Some(Ok(page)) = current_sheet.read().as_ref() {
                if page.policy == TimesheetPolicy::Scoped {
                    people::PersonPicker {
                        requester: page.requester, selected: page.subject.clone(), disabled: busy(),
                        on_selected: move |id: Uuid| {
                            navigator().push(Route::Timesheet { view, date, span, user: id.to_string() });
                        },
                    }
                }
            }
            if subject.is_some() {
                Link { to: Route::Timesheet { view, date, span, user: String::new() }, class: "btn btn-ghost btn-sm", "My timesheet" }
            }
            if let Some(Err(error)) = current_tracking.read().as_ref() {
                div { class: "alert alert-danger", role: "alert",
                    p { "Could not load available projects and tasks: {error}" }
                    button { class: "btn btn-secondary btn-sm", onclick: move |_| tracking_page.restart(), "Retry choices" }
                }
            }

            // Keep mutation failures visible across the week, day and calendar
            // views, including partial row deletion results.
            if let Some(msg) = grid_error() {
                div { class: "alert alert-danger", role: "alert", "{msg}" }
            }

            // Content
            if current_sheet.read().is_none() {
                div { class: "text-muted text-sm", role: "status", "Loading timesheet…" }
            } else if let Some(Err(error)) = current_sheet.read().as_ref() {
                div { class: "alert alert-danger", role: "alert",
                    p { "Could not load the complete timesheet: {error}" }
                    button { class: "btn btn-secondary btn-sm", onclick: move |_| sheet.restart(), "Retry" }
                }
            } else {
            {loaded(&*current_sheet.read(), |_| match current_mode {
                    ViewMode::Week => rsx! {
                        {render_week_view(&week_entries.read(), &daily_totals.read(), ws, today, &project_names.read(), &task_names.read(), &pending_rows.read(), week_actions)}
                        div { class: "ts-submit-bar",
                            if all_submitted_or_approved {
                                span { class: "badge badge-success", "Submitted" }
                            } else if !week_entries.is_empty() && has_open && context().is_some_and(|context| context.subject_id == context.expected_requester.user_id && context.expected_policy == TimesheetPolicy::LegacyOwn) {
                                div { class: "ts-submit",
                                    button {
                                        class: "ts-submit-main",
                                        disabled: busy(),
                                        onclick: move |_| {
                                            if busy() { return; }
                                            let Some(command_context) = context() else { return; };
                                            pending_action.set(true);
                                            let ws_str = ws.to_string();
                                            let mut timer = running_timer;
                                            let mut submit_status = submit_status;
                                            spawn(async move {
                                                match server_fns::submit_week(ws_str, command_context).await {
                                                    Ok(_) => {
                                                        submit_status.set(None);
                                                    }
                                                    Err(e) => submit_status.set(Some(format!("{e}"))),
                                                }
                                                sheet.restart();
                                                timer.refresh();
                                                pending_action.set(false);
                                            });
                                        },
                                        "Submit week for approval"
                                    }
                                    button { class: "ts-submit-caret", "aria-label": "More", "▾" }
                                }
                            }
                            if let Some(err) = &*submit_status.read() {
                                span { class: "text-danger text-sm ml-3",
                                    "{err}"
                                }
                            }
                        }
                    },
                    ViewMode::Day => rsx! {
                        {render_day_view(&by_day.read(), &daily_totals.read(), ws, sel_offset, select_day, &project_names.read(), &task_names.read(), open_edit, start_entry, stop_entry, &tracking.read(), busy(), context())}
                    },
                    ViewMode::Calendar => rsx! {
                        {
                            let visible = span.visible_days(*selected_day_offset.read() as usize, ws.weekday());
                            render_calendar_view(&by_day.read(), &daily_totals.read(), &visible, ws, today, &CalLabels { projects: &project_names.read(), tasks: &task_names.read(), clients: &project_client.read() }, cal_drag, add_hint, drag_commit, &tracking.read(), busy(), policy())
                        }
                    },
                })}
            }

            // Add–entry modal (opened by "+" or by clicking a calendar day).
            Modal {
                id: "time-entry-dialog",
                labelledby: "time-entry-title",
                open: add_open.read().is_some(),
                busy: add_saving(),
                large: true,
                on_dismiss: move |_| add_open.set(None),
                if let Some(date) = *add_open.read() {
                    div { id: "time-entry-title", class: "ts-modal-title",
                        if editing.read().is_some() { "Edit time entry" } else { "New time entry" }
                        " for {date.format(\"%A, %d %b\")}"
                    }
                    div { class: "ts-modal-body",
                        tracking::TrackingPicker {
                            project: add_project, task: add_task, options: modal_tracking(),
                            disabled: add_saving(),
                        }
                        div { class: "ts-modal-row",
                            input {
                                class: "form-input ts-modal-notes",
                                placeholder: "Notes (optional)",
                                value: "{add_notes}",
                                disabled: add_saving(),
                                oninput: move |e| add_notes.set(e.value()),
                            }
                            input {
                                class: "form-input ts-modal-duration",
                                "aria-label": "Duration",
                                placeholder: "0:00",
                                value: "{add_duration}",
                                disabled: add_saving(),
                                oninput: move |e| add_duration.set(e.value()),
                            }
                        }
                        input {
                            class: "form-input",
                            "aria-label": "Start time",
                            placeholder: "Start time, e.g. 9:00 (optional)",
                            disabled: add_saving(),
                            value: add_start().map(|m| horae_core::time_of_day::format(m as u16)).unwrap_or_default(),
                            oninput: move |e| {
                                let v = e.value();
                                let v = v.trim();
                                if v.is_empty() {
                                    add_start.set(None);
                                } else if let Some(m) = horae_core::time_of_day::parse(v) {
                                    add_start.set(Some(i32::from(m)));
                                }
                            },
                        }
                        if let Some(err) = &*add_error.read() {
                            div { class: "ts-modal-error", role: "alert", "{err}" }
                        }
                        div { class: "ts-modal-actions",
                            // Harvest-style single primary: with no duration on
                            // today's column it starts a running timer; once a
                            // duration is typed (or on a past day / when editing)
                            // it saves a fixed entry.
                            button {
                                class: "btn btn-primary",
                                disabled: busy() || context().is_none() || modal_tracking.read().is_empty(),
                                onclick: move |_| {
                                    if busy() { return; }
                                    let Some(context) = context() else { return; };
                                    let Some((project_id, task_id, notes)) = read_pt_notes.call(()) else {
                                        return;
                                    };
                                    let mut running_timer = running_timer;
                                    let minutes = match modal_minutes(&add_duration.read(), can_start_timer()) {
                                        Ok(minutes) => minutes,
                                        Err(message) => {
                                            add_error.set(Some(message.to_string()));
                                            return;
                                        }
                                    };
                                    let Some(minutes) = minutes else {
                                        add_saving.set(true);
                                        add_error.set(None);
                                        spawn(async move {
                                            match server_fns::apply_timesheet_command(context, TimesheetCommand::StartTimer { project_id, task_id, notes }).await {
                                                Ok(_) => {
                                                    add_open.set(None);
                                                }
                                                Err(e) => add_error
                                                    .set(Some(format!("Could not start timer: {e}"))),
                                            }
                                            running_timer.refresh();
                                            add_saving.set(false);
                                        });
                                        return;
                                    };
                                    let start_minute = *add_start.read();
                                    let editing_id = *editing.read();
                                    let billable = if editing_id.is_some() {
                                        edit_billable()
                                    } else {
                                        tracking.read().iter().find(|option| option.project_id == project_id && option.task_id == task_id).is_some_and(|option| option.billable)
                                    };
                                    let mut modal_timer = running_timer;
                                    add_saving.set(true);
                                    add_error.set(None);
                                    spawn(async move {
                                        let result = persist_entry(
                                            context, editing_id,
                                            TimesheetEntryInput { project_id, task_id, spent_date: date, minutes, notes, billable, start_minute },
                                        )
                                        .await;
                                        match result {
                                            Ok(()) => {
                                                add_open.set(None);
                                            }
                                            Err(e) => add_error.set(Some(format!("Could not save: {e}"))),
                                        }
                                        modal_timer.refresh();
                                        add_saving.set(false);
                                    });
                                },
                                if add_saving() {
                                    "Saving…"
                                } else if timer_mode() {
                                    "Start timer"
                                } else {
                                    "Save entry"
                                }
                            }
                            if editing.read().is_some() {
                                button {
                                    class: "btn btn-danger",
                                    disabled: busy() || context().is_none(),
                                    onclick: move |_| {
                                        if busy() { return; }
                                        let Some(context) = context() else { return; };
                                        let Some(id) = *editing.read() else {
                                            return;
                                        };
                                        let mut modal_timer = running_timer;
                                        add_saving.set(true);
                                        add_error.set(None);
                                        spawn(async move {
                                            match server_fns::apply_timesheet_command(context, TimesheetCommand::Delete { entry_ids: vec![id] }).await {
                                                Ok(()) => {
                                                    add_open.set(None);
                                                }
                                                Err(e) => {
                                                    add_error.set(Some(format!("Could not delete: {e}")))
                                                }
                                            }
                                            modal_timer.refresh();
                                            add_saving.set(false);
                                        });
                                    },
                                    "Delete"
                                }
                            }
                            button {
                                class: "btn btn-ghost",
                                disabled: add_saving(),
                                onclick: move |_| add_open.set(None),
                                "Cancel"
                            }
                        }
                    }
                }
            }

            // Add-row picker: choose a project/task to add an empty grid row.
            Modal {
                id: "add-row-dialog",
                labelledby: "add-row-title",
                open: addrow_open(),
                on_dismiss: move |_| addrow_open.set(false),
                if addrow_open() {
                    div { id: "add-row-title", class: "ts-modal-title", "Add a row" }
                    div { class: "ts-modal-body",
                        tracking::TrackingPicker {
                            project: addrow_project, task: addrow_task, options: tracking(), disabled: busy(),
                        }
                        div { class: "ts-modal-actions",
                            button {
                                class: "btn btn-primary",
                                disabled: busy() || tracking.read().is_empty(),
                                onclick: move |_| {
                                    if busy() { return; }
                                    let p = addrow_project.read().parse::<Uuid>();
                                    let t = addrow_task.read().parse::<Uuid>();
                                    if let (Ok(pid), Ok(tid)) = (p, t) {
                                        if !tracking::eligible(&tracking.read(), pid, tid) { return; }
                                        let key = (pid, tid);
                                        if !pending_rows.read().contains(&key) && !week_entries.read().iter().any(|entry| (entry.project_id, entry.task_id) == key) {
                                            pending_rows.write().push(key);
                                        }
                                    }
                                    addrow_open.set(false);
                                },
                                "Add row"
                            }
                            button {
                                class: "btn btn-ghost",
                                onclick: move |_| addrow_open.set(false),
                                "Cancel"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Label lookups shared by the calendar renderer.
struct CalLabels<'a> {
    projects: &'a HashMap<Uuid, String>,
    tasks: &'a HashMap<Uuid, String>,
    /// project_id -> authorized client name.
    clients: &'a HashMap<Uuid, String>,
}

/// A calendar event's pre-computed placement and labels, plus the entry it came
/// from so a click can open it for editing.
struct CalEvent {
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
    entry: VisibleTimeEntry,
}

/// Calendar grid pixels per hour.
const CAL_HOUR: i32 = 48;

/// Pointer Y (px within a day column) → snapped minutes since midnight.
fn cal_y_to_min(y: f64) -> i32 {
    horae_core::time_of_day::snap(
        (y * 60.0 / CAL_HOUR as f64) as i32,
        horae_core::time_of_day::SNAP_STEP,
    )
}

/// Assign overlapping timed entries to side-by-side lanes. Returns, per index of
/// `day` (parallel to the slice), the entry's lane and the number of lanes in its
/// overlap cluster; untimed entries get `(0, 1)`.
fn timed_lanes(day: &[VisibleTimeEntry]) -> (Vec<i32>, Vec<i32>) {
    let n = day.len();
    let mut lane_of = vec![0i32; n];
    let mut lanes_of = vec![1i32; n];

    // (index, start, end) for timed entries, sorted by start then end.
    let mut timed: Vec<(usize, i32, i32)> = day
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.start_minute.map(|s| (i, s, s + e.minutes)))
        .collect();
    timed.sort_by_key(|&(_, s, e)| (s, e));

    // Greedy: put each entry in the first lane free by its start time.
    let mut lane_end: Vec<i32> = Vec::new();
    for &(i, s, e) in &timed {
        let lane = match lane_end.iter().position(|&end| end <= s) {
            Some(l) => {
                lane_end[l] = e;
                l
            }
            None => {
                lane_end.push(e);
                lane_end.len() - 1
            }
        };
        lane_of[i] = lane as i32;
    }

    // Every entry in a maximal overlap run shares the run's lane count so all
    // stay the same width and none is hidden.
    let mut k = 0;
    while k < timed.len() {
        let mut cluster_end = timed[k].2;
        let mut max_lane = lane_of[timed[k].0];
        // Start the scan past k. A zero-length entry — a timer stopped inside
        // the same minute it started — does not start before its own end, so
        // scanning from k would leave j at k and spin here forever.
        let mut j = k + 1;
        while j < timed.len() && timed[j].1 < cluster_end {
            cluster_end = cluster_end.max(timed[j].2);
            max_lane = max_lane.max(lane_of[timed[j].0]);
            j += 1;
        }
        let count = max_lane + 1;
        for &(i, _, _) in &timed[k..j] {
            lanes_of[i] = count;
        }
        k = j;
    }

    (lane_of, lanes_of)
}

/// What a calendar drag is doing: drawing a new slot, or moving/resizing an
/// existing timed entry.
#[derive(Clone, Copy, PartialEq)]
enum DragKind {
    Create,
    Move,
    Resize,
    /// Drag an untimed entry within its day's stack to reorder it.
    Reorder,
}

/// In-progress calendar drag (minutes are snapped). `day` is the column under the
/// pointer; for Move/Resize the target entry and its original span come along so
/// the release can reschedule it (or, if it didn't move, open it for editing).
#[derive(Clone)]
struct CalDrag {
    kind: DragKind,
    day: usize,
    /// Create: the slot's anchor. Move/Resize: the entry's original start minute.
    start_min: i32,
    cur_min: i32,
    /// Move: the pointer minute where the block was grabbed.
    grab_min: i32,
    /// Move/Resize target (None for Create).
    entry: Option<VisibleTimeEntry>,
    orig_dur: i32,
    orig_day: usize,
}

impl CalDrag {
    /// Move: the entry's new start, following the pointer while keeping the grab
    /// offset, clamped into the day. Shared by the commit and the live preview so
    /// the two can't drift.
    fn move_start(&self) -> i32 {
        (self.cur_min - (self.grab_min - self.start_min)).clamp(0, 1439)
    }

    /// Resize: the entry's new end — the bottom edge follows the pointer but stays
    /// at least one snap step below the start. Shared by the commit and preview.
    fn resize_end(&self) -> i32 {
        self.cur_min
            .max(self.start_min + i32::from(horae_core::time_of_day::MIN_DURATION))
    }
}

/// A calendar block's start–end clock label, e.g. "9:00–10:30". `end` is capped at
/// the end of day for display.
fn cal_time_label(start: i32, end: i32) -> String {
    format!(
        "{}–{}",
        horae_core::time_of_day::format(start as u16),
        horae_core::time_of_day::format(end.min(1440) as u16),
    )
}

/// A day's untimed (duration-only) entries in stacking order: by explicit
/// `sort_order`, then newest-first (the pre-reorder default). Shared by the
/// calendar placement and the reorder commit so both agree on the order.
fn untimed_ordered(day: &[VisibleTimeEntry]) -> Vec<VisibleTimeEntry> {
    let mut u: Vec<VisibleTimeEntry> = day
        .iter()
        .filter(|e| e.start_minute.is_none())
        .cloned()
        .collect();
    u.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then(b.created_at.cmp(&a.created_at))
    });
    u
}

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, and the add/edit/drag actions"
)]
fn render_calendar_view(
    by_day: &[Vec<VisibleTimeEntry>; 7],
    daily_totals: &[i32],
    visible_days: &[usize],
    week_start: NaiveDate,
    today: NaiveDate,
    labels: &CalLabels,
    mut cal_drag: Signal<Option<CalDrag>>,
    mut add_hint: Signal<Option<(usize, i32)>>,
    drag_commit: Callback<CalDrag>,
    tracking: &[crate::models::scoped_time::TimesheetTrackingOption],
    busy: bool,
    policy: Option<TimesheetPolicy>,
) -> Element {
    let can_create = !busy && !tracking.is_empty();
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

    // Place entries: timed ones (with a start time) at their hour; untimed ones
    // stacked from the top of the day by cumulative duration (Harvest does the
    // same for duration-only entries). Track the latest bottom so the grid is
    // tall enough to show every block.
    let mut day_events: Vec<Vec<CalEvent>> = Vec::with_capacity(7);
    // Per-column occupied minute ranges, so the "+ Add time" hint hides over
    // existing blocks and a click there edits instead of adding.
    let mut occupied: Vec<Vec<(i32, i32)>> = Vec::with_capacity(7);
    for day in by_day.iter() {
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
        let mut evs = Vec::new();
        let mut occ: Vec<(i32, i32)> = Vec::new();
        for (idx, e) in day.iter().enumerate() {
            let (top_min, timed, lane, lanes) = match e.start_minute {
                Some(sm) => (sm, true, lane_of[idx], lanes_of[idx].max(1)),
                None => (untimed_top.get(&e.id).copied().unwrap_or(0), false, 0, 1),
            };
            occ.push((top_min, top_min + e.minutes));
            let client = labels
                .clients
                .get(&e.project_id)
                .cloned()
                .unwrap_or_default();
            let time_label = match e.start_minute {
                Some(sm) => cal_time_label(sm, sm + e.minutes),
                None => String::new(),
            };
            evs.push(CalEvent {
                top: top_min * CAL_HOUR / 60,
                height: (e.minutes * CAL_HOUR / 60).max(20),
                timed,
                lane,
                lanes,
                project: labels
                    .projects
                    .get(&e.project_id)
                    .cloned()
                    .unwrap_or_else(|| "Untitled".into()),
                task: labels.tasks.get(&e.task_id).cloned().unwrap_or_default(),
                duration: format_hhmm(e.minutes.into()),
                time_label,
                client,
                entry: e.clone(),
            });
        }
        day_events.push(evs);
        occupied.push(occ);
    }
    // Show the full 24-hour day (the scroll container clips it); the grid scrolls
    // vertically and, on mount, jumps to the earliest block across the visible
    // days — or to the working hours (7am) when the range is empty.
    let max_hours = 24;
    let first_min = visible_days
        .iter()
        .flat_map(|&i| occupied.get(i).into_iter().flatten())
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
                    class: if cal_drag.read().is_some() { "ts-cal-grid dragging" } else { "ts-cal-grid" },
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
                            onmousemove: {
                                let occ = occupied[i].clone();
                                move |e: MouseEvent| {
                                    if !can_create { return; }
                                    let m = cal_y_to_min(e.element_coordinates().y);
                                    if cal_drag.read().is_some() {
                                        cal_drag.with_mut(|d| {
                                            if let Some(d) = d {
                                                d.cur_min = m;
                                                // Move follows the pointer across days;
                                                // Reorder tracks the column so a drop on
                                                // another day snaps back.
                                                if matches!(d.kind, DragKind::Move | DragKind::Reorder) {
                                                    d.day = i;
                                                }
                                            }
                                        });
                                    } else {
                                        // Track the free slot under the cursor for the
                                        // "+ Add time" hint; hide it over a block.
                                        let free = !occ.iter().any(|&(lo, hi)| m >= lo && m < hi);
                                        let next = free.then_some((i, m));
                                        if *add_hint.read() != next {
                                            add_hint.set(next);
                                        }
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
                            if let Some(d) = cal_drag.read().clone().filter(|d| d.day == i && d.kind == DragKind::Create) {
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
                            if let Some(entry) = cal_drag
                                .read()
                                .clone()
                                .filter(|d| d.day == i && d.kind == DragKind::Reorder)
                                .and_then(|d| d.entry.map(|e| (e, d.cur_min)))
                            {
                                {
                                    let (e, cur) = entry;
                                    let h = (e.minutes * CAL_HOUR / 60).max(20);
                                    let top = (cur * CAL_HOUR / 60 - h / 2).max(0);
                                    let name = labels
                                        .projects
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
                            if let Some(top) = cal_drag
                                .read()
                                .is_none()
                                .then(|| *add_hint.read())
                                .flatten()
                                .filter(|&(hd, _)| hd == i)
                                .map(|(_, m)| m * CAL_HOUR / 60)
                            {
                                div { class: "ts-cal-add-hint", style: "top: {top}px;", "+ Add time" }
                            }
                            for ev in day_events[i].iter() {
                                {
                                // Live preview: while this entry is being moved or
                                // resized in its own column, drive its box from the
                                // in-progress drag so you can see it grow/shrink and
                                // read its new time (Create has its own ghost above).
                                let live = cal_drag.read().as_ref().and_then(|d| {
                                    if d.entry.as_ref().map(|e| e.id) != Some(ev.entry.id) {
                                        return None;
                                    }
                                    match d.kind {
                                        DragKind::Resize => Some((d.start_min, d.resize_end())),
                                        DragKind::Move if d.day == i => {
                                            let s = d.move_start();
                                            Some((s, s + d.orig_dur))
                                        }
                                        _ => None,
                                    }
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
                                let reordering = cal_drag.read().as_ref().is_some_and(|d| {
                                    d.kind == DragKind::Reorder
                                        && d.entry.as_ref().map(|e| e.id) == Some(ev.entry.id)
                                });
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
                                let locked = busy || !tracking::editable(policy, tracking, &ev.entry);
                                let ev_class = if locked {
                                    format!("{base} locked")
                                } else {
                                    base.to_string()
                                };
                                let lock_title = match ev.entry.state {
                                    horae_core::types::EntryState::Submitted => {
                                        "Submitted for approval — can't be moved"
                                    }
                                    horae_core::types::EntryState::Approved => {
                                        "Approved — can't be moved"
                                    }
                                    horae_core::types::EntryState::Invoiced => {
                                        "Invoiced — can't be moved"
                                    }
                                    horae_core::types::EntryState::Open if locked => "Not currently editable",
                                    horae_core::types::EntryState::Open => "",
                                };
                                rsx! {
                                div {
                                    class: "{ev_class}",
                                    title: "{lock_title}",
                                    style: "top: {top_px}px; height: {height_px}px; left: calc(4px + {ev.lane} * (100% - 8px) / {ev.lanes}); width: calc((100% - 8px) / {ev.lanes} - 2px); right: auto;",
                                    // Over a block there's no free slot — clear the
                                    // "+ Add time" hint (the column's mousemove can't
                                    // fire while the block captures the pointer).
                                    onmouseenter: move |_| {
                                        if add_hint.read().is_some() {
                                            add_hint.set(None);
                                        }
                                    },
                                    // Pressing a timed entry starts a move drag (its
                                    // body); a plain click with no move opens it for
                                    // editing. Untimed entries just open for editing.
                                    onmousedown: {
                                        let entry = ev.entry.clone();
                                        let timed = ev.timed;
                                        let start = ev.entry.start_minute.unwrap_or(0);
                                        let dur = ev.entry.minutes;
                                        move |e: MouseEvent| {
                                            e.stop_propagation();
                                            if locked { return; }
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
                                                    entry: Some(entry.clone()),
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
                                                    entry: Some(entry.clone()),
                                                    orig_dur: dur,
                                                    orig_day: i,
                                                }));
                                            }
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
                                            onmousedown: {
                                                let entry = ev.entry.clone();
                                                let start = ev.entry.start_minute.unwrap_or(0);
                                                let dur = ev.entry.minutes;
                                                move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    cal_drag.set(Some(CalDrag {
                                                        kind: DragKind::Resize,
                                                        day: i,
                                                        start_min: start,
                                                        cur_min: start + dur,
                                                        grab_min: start + dur,
                                                        entry: Some(entry.clone()),
                                                        orig_dur: dur,
                                                        orig_day: i,
                                                    }));
                                                }
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

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, and the row actions"
)]
fn render_day_view(
    by_day: &[Vec<VisibleTimeEntry>; 7],
    daily_totals: &[i32],
    week_start: NaiveDate,
    selected_offset: i64,
    select_day: Callback<i64>,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
    open_edit: Callback<VisibleTimeEntry>,
    start_entry: Callback<VisibleTimeEntry>,
    stop_entry: Callback<VisibleTimeEntry>,
    tracking: &[crate::models::scoped_time::TimesheetTrackingOption],
    busy: bool,
    context: Option<TimesheetWriteContext>,
) -> Element {
    let offset = selected_offset.clamp(0, 6) as usize;
    let day_entries = &by_day[offset];
    let total = daily_totals[offset];

    rsx! {
        // Day strip: each day shows its own total and the viewed day is
        // underlined — Harvest presents the days this way here, not as tabs.
        div { class: "ts-daystrip",
            for i in 0i64..7 {
                {
                    let cls = if i == selected_offset { "ts-dayitem active" } else { "ts-dayitem" };
                    rsx! {
                        button {
                            class: "{cls}",
                            onclick: move |_| select_day.call(i),
                            span { class: "ts-dayitem-name", "{(week_start + Duration::days(i)).format(\"%a\")}" }
                            span { class: "ts-dayitem-total", "{format_hhmm(daily_totals[i as usize].into())}" }
                        }
                    }
                }
            }
            div { class: "ts-dayitem ts-weektotal",
                span { class: "ts-dayitem-name", "Week total" }
                span { class: "ts-dayitem-total", "{format_hhmm(daily_totals.iter().sum::<i32>().into())}" }
            }
        }

        div { class: "card",
            if day_entries.is_empty() {
                div { class: "ts-day-empty text-muted text-sm", "No entries for this day." }
            } else {
                div { class: "ts-day-list",
                    for entry in day_entries.iter() {
                        {
                            let proj = project_names.get(&entry.project_id).cloned().unwrap_or_else(|| entry.project_id.to_string());
                            let task = task_names.get(&entry.task_id).cloned().unwrap_or_else(|| "\u{2014}".into());
                            let note = entry.notes.clone().filter(|n| !n.trim().is_empty());
                            let running = entry.is_running;
                            let dur = format_hhmm(entry.minutes.into());
                            let e_start = entry.clone();
                            let e_edit = entry.clone();
                            let e_stop = entry.clone();
                            let can_edit = !busy && tracking::editable(context.map(|context| context.expected_policy), tracking, entry);
                            let can_start = !busy && tracking::eligible(tracking, entry.project_id, entry.task_id);
                            let can_stop = !busy && tracking::stoppable(tracking, entry, context.map(|context| context.expected_requester.user_id));
                            rsx! {
                                div { class: "ts-day-entry",
                                    div { class: "ts-day-entry-main",
                                        div { class: "ts-day-entry-project", "{proj}" }
                                        div { class: "ts-day-entry-task", "{task}" }
                                        if let Some(n) = note {
                                            div { class: "ts-day-entry-notes", "{n}" }
                                        }
                                    }
                                    div { class: "ts-day-entry-side",
                                        if running {
                                            span { class: "badge badge-success", "Running" }
                                            button { class: "ts-day-action primary", disabled: !can_stop, onclick: move |_| stop_entry.call(e_stop.clone()), "Stop" }
                                        } else {
                                            span { class: "ts-day-entry-dur text-mono", "{dur}" }
                                            button {
                                                class: "ts-day-action primary",
                                                disabled: !can_start,
                                                onclick: move |_| start_entry.call(e_start.clone()),
                                                "Start"
                                            }
                                        }
                                        button {
                                            class: "ts-day-action",
                                            disabled: !can_edit,
                                            onclick: move |_| open_edit.call(e_edit.clone()),
                                            "Edit"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "mt-4 text-right p-2",
                span { class: "text-muted text-sm", "Day total: " }
                span { class: "text-mono font-semibold text-primary",
                    "{format_hhmm(total.into())}"
                }
            }
        }
    }
}

/// A single week-grid cell edit, committed when the input loses focus.
#[derive(Clone)]
struct CellEdit {
    project_id: Uuid,
    task_id: Uuid,
    day: NaiveDate,
    /// The entry already in the cell (update/delete), or `None` to create one.
    existing: Option<Uuid>,
    input: String,
}

type CellKey = (Uuid, Uuid, NaiveDate);

fn discard_row_drafts(drafts: &mut HashMap<CellKey, String>, row: (Uuid, Uuid), start: NaiveDate) {
    drafts.retain(|&(project, task, day), _| {
        (project, task) != row || !(0..7).contains(&(day - start).num_days())
    });
}

/// The actions the editable week grid dispatches back to the page.
#[derive(Clone, Copy)]
struct WeekActions {
    commit: Callback<CellEdit>,
    remove_row: Callback<(Uuid, Uuid)>,
    removing_row: ReadSignal<bool>,
    add_row: Callback<()>,
    drafts: Signal<HashMap<CellKey, String>>,
    saving: ReadSignal<HashSet<CellKey>>,
    tracking: Memo<Vec<crate::models::scoped_time::TimesheetTrackingOption>>,
    busy: Memo<bool>,
    policy: Memo<Option<TimesheetPolicy>>,
}

/// A week grid row's per-day minutes and the entry ids behind each day, so a cell
/// can update its entry (one id), create a new one (none), or fall back to a
/// read-only total when a day holds several entries for the same project/task.
#[derive(Default)]
struct RowAgg {
    mins: [i32; 7],
    ids: [Vec<Uuid>; 7],
}

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, pending rows, and grid actions"
)]
fn render_week_view(
    entries: &[VisibleTimeEntry],
    daily_totals: &[i32],
    week_start: NaiveDate,
    today: NaiveDate,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
    pending: &[(Uuid, Uuid)],
    mut actions: WeekActions,
) -> Element {
    // Group by (project_id, task_id), tracking per-day minutes and entry ids,
    // preserving first-seen order. Rows added via "Add row" (no entries yet)
    // are appended so they render as empty, editable rows.
    let mut row_keys: Vec<(Uuid, Uuid)> = Vec::new();
    let mut row_map: HashMap<(Uuid, Uuid), RowAgg> = HashMap::new();
    for entry in entries {
        let offset = (entry.spent_date - week_start).num_days();
        if !(0..7).contains(&offset) {
            continue;
        }
        let row = row_map
            .entry((entry.project_id, entry.task_id))
            .or_insert_with(|| {
                row_keys.push((entry.project_id, entry.task_id));
                RowAgg::default()
            });
        row.mins[offset as usize] += entry.minutes;
        row.ids[offset as usize].push(entry.id);
    }
    for key in pending {
        if !row_map.contains_key(key) {
            row_keys.push(*key);
            row_map.insert(*key, RowAgg::default());
        }
    }
    for &(project, task, day) in actions.drafts.read().keys() {
        let key = (project, task);
        if (0..7).contains(&(day - week_start).num_days()) && !row_map.contains_key(&key) {
            row_keys.push(key);
            row_map.insert(key, RowAgg::default());
        }
    }

    let today_off = today_offset(today, week_start);
    let day_class = |i: usize, base: &str| {
        day_col_class(
            base,
            today_off,
            i,
            (week_start + Duration::days(i as i64)).weekday(),
        )
    };

    rsx! {
        div { class: "ts-grid-card",
            div { class: "ts-grid-scroll",
                // Header row
                div { class: "ts-row ts-head",
                    span {}
                    for i in 0..7 {
                        {
                            let d = week_start + Duration::days(i as i64);
                            rsx! {
                                span { class: "{day_class(i, \"ts-daycol\")}",
                                    span { class: "ts-dayname", "{d.format(\"%a\")}" }
                                    span { class: "ts-daynum", "{d.format(\"%d %b\")}" }
                                }
                            }
                        }
                    }
                    span { class: "ts-total-head", "Total" }
                    span {}
                }

                if row_keys.is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", "🗓" }
                        div { class: "empty-state-title", "No time this week" }
                        p { class: "text-muted text-sm", "Add an entry to start filling your timesheet." }
                    }
                }

                // Project rows
                for key in row_keys.iter() {
                    {
                        let (pid, tid) = *key;
                        let proj = project_names.get(&pid).cloned().unwrap_or_else(|| pid.to_string());
                        let task = task_names.get(&tid).cloned().unwrap_or_else(|| "\u{2014}".into());
                        let agg = &row_map[key];
                        let row_total: i32 = agg.mins.iter().sum();
                        let eligible = tracking::eligible(&actions.tracking.read(), pid, tid);
                        let row_editable = entries.iter().filter(|entry| entry.project_id == pid && entry.task_id == tid).all(|entry| tracking::editable((actions.policy)(), &actions.tracking.read(), entry));
                        rsx! {
                            div { class: "ts-row ts-body",
                                div { class: "ts-project",
                                    button { class: "ts-project-icon", "aria-label": "Task", "▤" }
                                    div {
                                        div { class: "ts-project-title", strong { "{proj}" } }
                                        div { class: "ts-project-task", "{task}" }
                                    }
                                }
                                for i in 0..7 {
                                    {
                                        let mins = agg.mins[i];
                                        // A cell is editable when it holds at most one entry: type a
                                        // duration to create/update/clear it. Days with several entries
                                        // show a read-only total (edit them in the Day view).
                                        if agg.ids[i].len() <= 1 {
                                            let day = week_start + Duration::days(i as i64);
                                            let existing = agg.ids[i].first().copied();
                                            let editable = match existing {
                                                None => eligible,
                                                Some(id) => entries.iter().find(|entry| entry.id == id).is_some_and(|entry| tracking::editable((actions.policy)(), &actions.tracking.read(), entry)),
                                            };
                                            let saved = if mins > 0 { format_hhmm(mins.into()) } else { String::new() };
                                            let cell_key = (pid, tid, day);
                                            let val = actions.drafts.read().get(&cell_key).cloned().unwrap_or_else(|| saved.clone());
                                            let icls = value_cell_class("ts-cell-input", mins, today_off, i);
                                            rsx! {
                                                div { class: "ts-cell",
                                                    input {
                                                        class: "{icls}",
                                                        r#type: "text",
                                                        value: "{val}",
                                                        disabled: !editable || (actions.removing_row)() || actions.saving.read().contains(&cell_key),
                                                        aria_label: "Hours for {proj}, {task}, {day}",
                                                        placeholder: "\u{2013}",
                                                        oninput: move |event| {
                                                            let input = event.value();
                                                            if input == saved {
                                                                actions.drafts.write().remove(&cell_key);
                                                            } else {
                                                                actions.drafts.write().insert(cell_key, input);
                                                            }
                                                        },
                                                        onblur: move |_| {
                                                            let input = actions.drafts.read().get(&cell_key).cloned();
                                                            if let Some(input) = input {
                                                                actions.commit.call(CellEdit { project_id: pid, task_id: tid, day, existing, input });
                                                            }
                                                        },
                                                    }
                                                }
                                            }
                                        } else {
                                            let cls = value_cell_class("ts-cell-box", mins, today_off, i);
                                            rsx! {
                                                div { class: "ts-cell",
                                                    div { class: "{cls}", title: "Multiple entries — edit in Day view", "{format_hhmm(mins.into())}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "ts-rowtotal", "{format_hhmm(row_total.into())}" }
                                div { class: "text-center",
                                    button {
                                        class: "ts-del",
                                        disabled: (actions.busy)() || !row_editable,
                                        "aria-label": "Remove row",
                                        onclick: move |_| actions.remove_row.call((pid, tid)),
                                        "\u{00d7}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Add a project/task row to fill in across the week.
                div { class: "ts-addrow-wrap",
                    button {
                        r#type: "button",
                        class: "ts-addrow",
                        disabled: (actions.busy)() || actions.tracking.read().is_empty(),
                        onclick: move |_| actions.add_row.call(()),
                        span { class: "plus", "\u{ff0b}" }
                        "Add row"
                    }
                }

                // Footer: column totals
                div { class: "ts-row ts-foot",
                    div {}
                    for i in 0..7 {
                        {
                            let t = daily_totals[i];
                            let cls = value_cell_class("ts-coltotal", t, today_off, i);
                            rsx! {
                                div { class: "{cls}",
                                    if t > 0 {
                                        "{format_hhmm(t.into())}"
                                    } else {
                                        "0"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "ts-grandtotal", "{format_hhmm(daily_totals.iter().sum::<i32>().into())}" }
                    div {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn today_offset_is_zero_on_the_monday() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(monday, monday), Some(0));
    }

    #[test]
    fn today_offset_is_six_on_the_sunday() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 19), monday), Some(6));
    }

    #[test]
    fn today_offset_is_none_before_the_week() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 12), monday), None);
    }

    #[test]
    fn today_offset_is_none_after_the_week() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 20), monday), None);
    }

    #[test]
    fn day_col_class_marks_today() {
        assert_eq!(day_col_class("c", Some(2), 2, Weekday::Wed), "c today");
    }

    #[test]
    fn day_col_class_marks_weekend() {
        assert_eq!(day_col_class("c", None, 5, Weekday::Sat), "c weekend");
    }

    #[test]
    fn day_col_class_today_wins_over_weekend() {
        assert_eq!(day_col_class("c", Some(6), 6, Weekday::Sun), "c today");
    }

    #[test]
    fn day_col_class_plain_weekday() {
        assert_eq!(day_col_class("c", None, 1, Weekday::Tue), "c");
    }

    #[test]
    fn value_cell_class_empty_wins_over_today() {
        assert_eq!(value_cell_class("v", 0, Some(2), 2), "v empty");
    }

    #[test]
    fn value_cell_class_marks_today_when_nonzero() {
        assert_eq!(value_cell_class("v", 30, Some(2), 2), "v today");
    }

    #[test]
    fn cal_span_url_roundtrips() {
        for span in [CalSpan::Week, CalSpan::WorkWeek, CalSpan::Day] {
            assert_eq!(span.to_string().parse::<CalSpan>().unwrap(), span);
        }
    }

    #[test]
    fn cal_span_defaults_to_week() {
        assert_eq!(CalSpan::default(), CalSpan::Week);
    }

    #[test]
    fn cal_span_unknown_falls_back_to_week() {
        assert_eq!("nonsense".parse::<CalSpan>().unwrap(), CalSpan::Week);
    }

    #[test]
    fn cal_span_day_shows_only_the_anchor_column() {
        assert_eq!(CalSpan::Day.visible_days(3, Weekday::Mon), vec![3]);
    }

    #[test]
    fn cal_span_work_week_shows_monday_to_friday() {
        assert_eq!(
            CalSpan::WorkWeek.visible_days(3, Weekday::Mon),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn cal_span_week_shows_all_seven_days() {
        assert_eq!(
            CalSpan::Week.visible_days(3, Weekday::Mon),
            vec![0, 1, 2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn cal_span_work_week_skips_sunday_when_it_is_the_first_column() {
        assert_eq!(
            CalSpan::WorkWeek.visible_days(0, Weekday::Sun),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn cal_span_work_week_preserves_date_order_across_weekends() {
        assert_eq!(
            CalSpan::WorkWeek.visible_days(0, Weekday::Wed),
            vec![0, 1, 2, 5, 6]
        );
    }

    #[test]
    fn weekend_style_follows_the_date_not_the_column_offset() {
        assert_eq!(day_col_class("c", None, 0, Weekday::Sun), "c weekend");
        assert_eq!(day_col_class("c", None, 5, Weekday::Fri), "c");
    }

    fn timed_entry(start_minute: i32, minutes: i32) -> VisibleTimeEntry {
        VisibleTimeEntry {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            user_name: "Person".into(),
            project_id: Uuid::nil(),
            project_name: "Project".into(),
            task_id: Uuid::nil(),
            task_name: "Task".into(),
            client_id: Uuid::nil(),
            client_name: "Client".into(),
            spent_date: ymd(2026, 9, 3),
            minutes,
            rounded_minutes: None,
            notes: None,
            billable: true,
            is_running: false,
            started_at: None,
            start_minute: Some(start_minute),
            sort_order: 0,
            state: horae_core::types::EntryState::Open,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn terminal_stop_recovery_is_owner_only_when_tracking_is_lost() {
        let mut entry = timed_entry(540, 60);
        entry.is_running = true;
        assert!(tracking::stoppable(&[], &entry, Some(entry.user_id)));
        assert!(!tracking::stoppable(&[], &entry, Some(Uuid::now_v7())));
        assert!(!tracking::editable(
            Some(TimesheetPolicy::Scoped),
            &[],
            &entry
        ));
    }

    #[test]
    fn delegated_stop_requires_the_exact_eligible_pair_and_keeps_state_locks() {
        let mut entry = timed_entry(540, 60);
        entry.is_running = true;
        let actor = Some(Uuid::now_v7());
        let mut options = vec![crate::models::scoped_time::TimesheetTrackingOption {
            project_id: entry.project_id,
            project_name: entry.project_name.clone(),
            task_id: Uuid::now_v7(),
            task_name: "Other task".into(),
            billable: true,
        }];
        assert!(!tracking::stoppable(&options, &entry, actor));
        options[0].task_id = entry.task_id;
        assert!(tracking::stoppable(&options, &entry, actor));
        assert!(!tracking::stoppable(&options, &entry, None));
        for state in [
            horae_core::types::EntryState::Submitted,
            horae_core::types::EntryState::Approved,
            horae_core::types::EntryState::Invoiced,
        ] {
            entry.state = state;
            assert!(!tracking::stoppable(&options, &entry, actor));
            assert!(!tracking::stoppable(&options, &entry, Some(entry.user_id)));
        }
    }

    #[test]
    fn visible_time_without_writable_tracking_does_not_enable_edit_controls() {
        let entry = timed_entry(540, 60);
        assert!(!tracking::editable(
            Some(TimesheetPolicy::Scoped),
            &[],
            &entry
        ));
        let options = vec![crate::models::scoped_time::TimesheetTrackingOption {
            project_id: entry.project_id,
            project_name: entry.project_name.clone(),
            task_id: entry.task_id,
            task_name: entry.task_name.clone(),
            billable: false,
        }];
        assert!(tracking::editable(
            Some(TimesheetPolicy::Scoped),
            &options,
            &entry
        ));
        assert!(!tracking::editable(None, &options, &entry));
    }

    #[test]
    fn legacy_historical_edits_do_not_create_new_tracking_eligibility() {
        let entry = timed_entry(540, 60);
        assert!(tracking::editable(
            Some(TimesheetPolicy::LegacyOwn),
            &[],
            &entry
        ));
        assert!(!tracking::eligible(&[], entry.project_id, entry.task_id));
        assert!(!tracking::editable(
            Some(TimesheetPolicy::Scoped),
            &[],
            &entry
        ));
    }

    #[test]
    fn read_only_day_keeps_facts_but_disables_edit_and_start_buttons() {
        let mut dom = VirtualDom::new(|| {
            let mut by_day: [Vec<VisibleTimeEntry>; 7] = Default::default();
            by_day[0].push(timed_entry(540, 60));
            render_day_view(
                &by_day,
                &[60, 0, 0, 0, 0, 0, 0],
                ymd(2026, 9, 3),
                0,
                use_callback(|_| {}),
                &HashMap::new(),
                &HashMap::new(),
                use_callback(|_| {}),
                use_callback(|_| {}),
                use_callback(|_| {}),
                &[],
                false,
                Some(TimesheetWriteContext {
                    expected_requester: PermissionRequester {
                        org_id: Uuid::nil(),
                        user_id: Uuid::now_v7(),
                    },
                    subject_id: Uuid::nil(),
                    expected_policy: TimesheetPolicy::Scoped,
                }),
            )
        });
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("1:00"));
        for label in ["Start", "Edit"] {
            let button = html
                .split("<button")
                .find(|part| part.contains(&format!(">{label}</button>")))
                .unwrap();
            assert!(
                button.split('>').next().unwrap().contains("disabled"),
                "{label} must be disabled: {button}"
            );
        }
    }

    #[test]
    fn invalid_duration_cannot_start_a_timer_save_or_clear_a_cell() {
        let entry = timed_entry(540, 60);
        for input in [
            "-1",
            "-0",
            "NaN",
            "inf",
            "71582789:00",
            "1:60",
            "oops",
            "24:01",
        ] {
            assert!(modal_minutes(input, true).is_err(), "timer: {input}");
            assert!(modal_minutes(input, false).is_err(), "save: {input}");
            assert!(cell_fields(input, Some(&entry)).is_err(), "cell: {input}");
        }
    }

    #[test]
    fn only_a_blank_duration_can_start_an_eligible_timer() {
        assert_eq!(modal_minutes("  ", true), Ok(None));
        assert!(modal_minutes("  ", false).is_err());
        for input in ["0", "0:00", "0.0001"] {
            assert!(modal_minutes(input, true).is_err());
        }
        assert_eq!(modal_minutes("1.5", true), Ok(Some(90)));
        assert_eq!(modal_minutes("24:00", false), Ok(Some(1440)));
    }

    #[test]
    fn weekly_duration_edit_preserves_nine_am_start_notes_and_billability() {
        let mut entry = timed_entry(540, 60);
        entry.notes = Some("Keep these notes".to_string());
        entry.billable = false;

        let fields = cell_fields("1:30", Some(&entry)).unwrap();

        assert_eq!(fields.minutes, 90);
        assert_eq!(fields.start_minute, Some(540));
        assert_eq!(fields.notes, entry.notes);
        assert!(!fields.billable);
    }

    #[test]
    fn weekly_new_entries_are_untimed_and_explicit_clears_still_work() {
        let fields = cell_fields("1.5", None).unwrap();
        assert_eq!(fields.minutes, 90);
        assert_eq!(fields.start_minute, None);
        assert_eq!(fields.notes, None);
        assert!(fields.billable);
        for input in ["", "  ", "0:00"] {
            assert_eq!(
                cell_fields(input, Some(&timed_entry(540, 60)))
                    .unwrap()
                    .minutes,
                0
            );
        }
    }

    /// Stopping a timer inside the minute it started leaves an entry of zero
    /// length, which does not begin before its own end. The cluster scan used to
    /// make no progress on one and spin forever, freezing the Calendar view.
    #[test]
    fn timed_lanes_terminates_on_a_zero_length_entry() {
        let day = vec![timed_entry(540, 0), timed_entry(600, 30)];
        let (lane_of, lanes_of) = timed_lanes(&day);
        assert_eq!(lane_of.len(), 2);
        assert_eq!(lanes_of, vec![1, 1]);
    }

    #[test]
    fn timed_lanes_still_widens_overlapping_entries() {
        let day = vec![timed_entry(540, 60), timed_entry(570, 60)];
        let (lane_of, lanes_of) = timed_lanes(&day);
        assert_eq!(lane_of, vec![0, 1]);
        assert_eq!(lanes_of, vec![2, 2]);
    }
}
