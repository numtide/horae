//! People: everyone in the workspace with their week's hours, a main screen
//! like the Timesheet (`DESIGN.md`, *People*).

use chrono::NaiveDate;
use dioxus::prelude::*;
use horae_core::permissions::catalog::{
    PERMISSION_CATALOG_VERSION,
    Permission::{PeopleReadAll, PeopleReadManaged, PeopleWriteAll},
};
use uuid::Uuid;

use crate::components::page_header::PageHeader;
use crate::components::screen_state::{ErrorState, NoAccess};
use crate::components::session::{use_session, use_week_start};
use crate::components::today::today;
use crate::i18n::use_tr;
use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;

// Explicit paths let the UI tests include this file with `#[path]`.
#[path = "people/add_person.rs"]
mod add_person;
#[path = "people/directory.rs"]
mod directory;

// The directory names its dialogs as siblings, so it also builds in the UI
// tests, which mount it without the router.
use super::admin::{permission_editor, rates};

#[component]
pub fn People() -> Element {
    let tr = use_tr();
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
    // The permissions are shared with the rail, which refreshes them when its
    // account menu opens; the last answer stays meanwhile, so the directory
    // does not remount and lose its filters and week.
    match &*own.read() {
        Ok(access)
            if access.catalog_version == PERMISSION_CATALOG_VERSION
                && (access.grants.contains(&PeopleReadAll)
                    || access.grants.contains(&PeopleReadManaged)) =>
        {
            rsx! {
                directory::PeopleDirectory {
                    can_edit_permissions: access.is_administrator,
                    can_manage_people: access.grants.contains(&PeopleWriteAll),
                    first_day: week_start,
                    today: today(),
                    on_saved,
                    on_view_timesheet: view_timesheet,
                    // The audit log is open to administrators only.
                    on_view_history: access.is_administrator.then_some(view_history),
                }
            }
        }
        Ok(_) => rsx! {
            PageHeader { title: tr.people_title() }
            NoAccess {}
            super::admin::PermissionRecovery { on_saved }
        },
        _ => rsx! {
            PageHeader { title: tr.people_title() }
            ErrorState {
                title: tr.people_access_check_failed(),
                body: tr.people_access_check_failed_hint(),
                onretry: retry,
            }
            super::admin::PermissionRecovery { on_saved }
        },
    }
}
