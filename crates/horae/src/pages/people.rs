//! People: everyone in the workspace with their week's hours, a main screen
//! like the Timesheet (`DESIGN.md`, *People*).

use chrono::{NaiveDate, Weekday};
use dioxus::prelude::*;
use horae_core::permissions::catalog::{
    PERMISSION_CATALOG_VERSION,
    Permission::{
        BillableRateReadAll, BillableRateReadManaged, CostRateReadAll, PeopleReadAll,
        PeopleReadManaged, PeopleWriteAll,
    },
};
use uuid::Uuid;

use crate::components::page_header::PageHeader;
use crate::components::screen_state::{ErrorState, Skeleton};
use crate::components::session::{use_session, use_week_start};
use crate::components::today::today;
use crate::route::Route;
use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};

mod add_person;
mod directory;

// The directory names its dialogs as siblings, so it also builds in the UI
// tests, which mount it without the router.
use super::admin::{permission_editor, rates};

#[component]
pub fn People() -> Element {
    let session = use_session();
    let own = session.permissions;
    let week_start = use_week_start();
    let retry = use_callback(move |_| session.refresh());
    let on_saved = use_callback(move |_changed: bool| {
        // The save may have changed the user's own access, which the rail renders from.
        session.refresh();
    });
    let navigator = use_navigator();
    let view_timesheet = use_callback(move |(user, week): (Uuid, NaiveDate)| {
        navigator.push(Route::Timesheet {
            view: ViewMode::Week,
            date: Anchor(week),
            span: CalSpan::default(),
            user: user.to_string(),
        });
    });
    let view_history = use_callback(move |()| {
        navigator.push(Route::PermissionAudit {});
    });
    // shortcut: an unreadable first weekday falls back to Monday rather than
    // blocking the list; the Timesheet shows that error.
    let first_day = match &*week_start.read() {
        Some(Ok(day)) => Some(*day),
        Some(Err(_)) => Some(Weekday::Mon),
        None => None,
    };
    // The permissions are shared with the rail, which refreshes them when its
    // account menu opens; keep a resolved answer meanwhile so the directory does
    // not remount and lose its filters and week.
    let resolved = matches!(&*own.read(), Some(Ok(_)));
    if (!resolved && own.state()() != UseResourceState::Ready) || first_day.is_none() {
        return rsx! {
            PageHeader { title: "People" }
            Skeleton { columns: 5, label: "Loading people…" }
        };
    }
    match &*own.read() {
        Some(Ok(access))
            if access.catalog_version == PERMISSION_CATALOG_VERSION
                && (access.grants.contains(&PeopleReadAll)
                    || access.grants.contains(&PeopleReadManaged)) =>
        {
            // Per-person access is decided when the dialog loads; with a
            // managed-only grant, other people's rows show a refusal there.
            let can_view_rates = [BillableRateReadManaged, BillableRateReadAll, CostRateReadAll]
                .iter()
                .any(|grant| access.grants.contains(grant));
            rsx! {
                directory::PeopleDirectory {
                    can_edit_permissions: access.is_administrator,
                    can_manage_people: access.grants.contains(&PeopleWriteAll),
                    can_view_rates,
                    first_day: first_day.unwrap_or(Weekday::Mon),
                    today: today(),
                    on_saved,
                    on_view_timesheet: view_timesheet,
                    // The audit log is open to administrators only.
                    on_view_history: access.is_administrator.then_some(view_history),
                }
            }
        }
        Some(Ok(_)) => rsx! {
            PageHeader { title: "People" }
            ErrorState {
                title: "Access unavailable",
                body: "Your account cannot view people. Ask an Administrator if you need it.",
            }
            super::admin::PermissionRecovery { on_saved }
        },
        _ => rsx! {
            PageHeader { title: "People" }
            ErrorState {
                title: "Could not verify your access",
                body: "Try again, or sign in again if it keeps failing.",
                onretry: retry,
            }
            super::admin::PermissionRecovery { on_saved }
        },
    }
}
