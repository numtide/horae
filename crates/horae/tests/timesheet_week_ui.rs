#![cfg(feature = "server")]

//! Render the production timesheet with controlled week responses, so a week
//! change can be observed while the next week is still loading.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, Weekday};
use dioxus::dioxus_core::{ElementId, Mutation};
use dioxus::html::{
    PlatformEventData, SerializedFocusData, SerializedFormData, SerializedHtmlEventConverter,
    SerializedMouseData,
};
use dioxus::prelude::*;
use futures_util::FutureExt;
use tokio::sync::Notify;
use uuid::Uuid;

// Included at the root: `scoped_time` names its siblings through `super`.
#[path = "../src/models/people.rs"]
pub mod people;
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
#[path = "../src/models/scoped_time.rs"]
pub mod scoped_time;
mod models {
    pub use super::{people, permission_editor, scoped_time};
}
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/date_picker.rs"]
pub mod date_picker;
#[path = "../src/components/menu.rs"]
pub mod menu;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/select_field.rs"]
pub mod select_field;
mod components {
    pub use super::{controls, date_picker, menu, modal, select_field};

    /// The shell's week start, without the rest of the session.
    pub mod session {
        use dioxus::prelude::*;

        pub fn use_week_start() -> Resource<Result<chrono::Weekday, ServerFnError>> {
            use_resource(crate::server_fns::get_week_start)
        }
    }

    /// The shell's shared timer, without the rail that renders it.
    pub mod timer_widget {
        use dioxus::prelude::*;

        #[derive(Clone, Copy)]
        pub struct RunningTimer {
            changes: Signal<u64>,
        }

        impl RunningTimer {
            pub fn refresh(&mut self) {
                *self.changes.write() += 1;
            }

            pub fn changes(&self) -> u64 {
                *self.changes.read()
            }
        }

        pub fn use_running_timer() -> RunningTimer {
            use_context::<RunningTimer>()
        }

        pub fn use_running_timer_provider() {
            let changes = use_signal(|| 0);
            use_context_provider(|| RunningTimer { changes });
        }
    }
}
// A directory path keeps the page's own submodules under `pages/timesheet/`.
#[path = "../src/pages"]
mod pages {
    use dioxus::prelude::*;

    pub mod timesheet;

    pub(crate) fn loaded<T>(
        state: &Option<Result<T, ServerFnError>>,
        render: impl FnOnce(&T) -> Element,
    ) -> Element {
        match state {
            Some(Ok(v)) => render(v),
            Some(Err(e)) => rsx! { div { "{e}" } },
            None => rsx! { div { "Loading…" } },
        }
    }
}
use pages::timesheet;

/// The one route the page links to.
mod route {
    use super::timesheet::{Anchor, CalSpan, Timesheet, ViewMode};
    use dioxus::prelude::*;

    #[derive(Routable, Clone, PartialEq)]
    pub enum Route {
        #[route("/timesheet/:view/:date?:span&:user")]
        Timesheet {
            view: ViewMode,
            date: Anchor,
            span: CalSpan,
            user: String,
        },
    }
}

use models::permission_editor::PermissionRequester;
use models::scoped_time::{
    TimesheetCommand, TimesheetPage, TimesheetPerson, TimesheetPolicy, TimesheetTrackingOption,
    VisibleTimeEntry,
};
use timesheet::{Anchor, CalSpan, Timesheet, ViewMode};

const ME: Uuid = Uuid::from_u128(1);
const PROJECT: Uuid = Uuid::from_u128(2);
const TASK: Uuid = Uuid::from_u128(3);

thread_local! {
    /// Released weeks by their first day; a week not here keeps loading.
    static WEEKS: RefCell<HashMap<NaiveDate, TimesheetPage>> = RefCell::default();
    static RELEASED: Rc<Notify> = Rc::default();
    static COMMANDS: RefCell<Vec<TimesheetCommand>> = RefCell::default();
}

fn release(start: NaiveDate, entries: Vec<VisibleTimeEntry>) {
    let page = TimesheetPage {
        requester: PermissionRequester {
            org_id: Uuid::nil(),
            user_id: ME,
        },
        subject: TimesheetPerson {
            id: ME,
            name: "Me".into(),
        },
        policy: TimesheetPolicy::LegacyOwn,
        entries,
        next_after: None,
    };
    WEEKS.with_borrow_mut(|weeks| weeks.insert(start, page));
    RELEASED.with(|released| released.notify_waiters());
}

mod server_fns {
    use super::*;
    use models::scoped_time::{
        TimesheetPeoplePage, TimesheetPeopleQuery, TimesheetQuery, TimesheetWriteContext,
    };

    pub async fn get_week_start() -> Result<Weekday, ServerFnError> {
        Ok(Weekday::Mon)
    }

    pub async fn load_timesheet_page(
        query: TimesheetQuery,
    ) -> Result<TimesheetPage, ServerFnError> {
        let released = RELEASED.with(Rc::clone);
        loop {
            let notified = released.notified();
            if let Some(page) = WEEKS.with_borrow(|weeks| weeks.get(&query.date_from).cloned()) {
                return Ok(page);
            }
            notified.await;
        }
    }

    pub async fn load_timesheet_tracking(
        _: TimesheetWriteContext,
    ) -> Result<Vec<TimesheetTrackingOption>, ServerFnError> {
        Ok(vec![TimesheetTrackingOption {
            project_id: PROJECT,
            project_name: "Project".into(),
            task_id: TASK,
            task_name: "Task".into(),
            billable: true,
        }])
    }

    pub async fn apply_timesheet_command(
        _: TimesheetWriteContext,
        command: TimesheetCommand,
    ) -> Result<(), ServerFnError> {
        COMMANDS.with_borrow_mut(|commands| commands.push(command));
        Ok(())
    }

    pub async fn submit_week(_: String, _: TimesheetWriteContext) -> Result<(), ServerFnError> {
        panic!("this test must not submit a week")
    }

    pub async fn list_timesheet_people(
        _: TimesheetPeopleQuery,
    ) -> Result<TimesheetPeoplePage, ServerFnError> {
        panic!("a legacy own sheet has no person picker")
    }
}

fn entry(day: NaiveDate, minutes: i32, project_name: &str) -> VisibleTimeEntry {
    VisibleTimeEntry {
        id: Uuid::now_v7(),
        user_id: ME,
        user_name: "Me".into(),
        project_id: PROJECT,
        project_name: project_name.into(),
        task_id: TASK,
        task_name: "Task".into(),
        client_id: Uuid::nil(),
        client_name: "Client".into(),
        spent_date: day,
        minutes,
        rounded_minutes: None,
        notes: None,
        billable: true,
        is_running: false,
        started_at: None,
        start_minute: None,
        sort_order: 0,
        state: horae_core::types::EntryState::Open,
        created_at: chrono::Utc::now(),
    }
}

/// A past week, so neither week under test contains today.
fn monday() -> NaiveDate {
    "2026-09-07".parse().unwrap()
}

type DateSlot = Rc<RefCell<Option<Signal<Anchor>>>>;

fn app(slot: DateSlot) -> Element {
    components::timer_widget::use_running_timer_provider();
    let date = use_signal(|| Anchor(monday()));
    *slot.borrow_mut() = Some(date);
    rsx! { Timesheet { view: ViewMode::Week, date: date(), span: CalSpan::Week, user: String::new() } }
}

struct Page {
    dom: VirtualDom,
    date: Signal<Anchor>,
    listeners: Vec<(String, ElementId)>,
}

impl Page {
    fn open() -> Self {
        dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter));
        let slot = DateSlot::default();
        let mut dom = VirtualDom::new_with_props(app, slot.clone());
        let edits = dom.rebuild_to_vec().edits;
        let date = slot.borrow().unwrap();
        let mut page = Self {
            dom,
            date,
            listeners: Vec::new(),
        };
        page.record(edits);
        page.settle();
        page
    }

    fn record(&mut self, edits: Vec<Mutation>) {
        self.listeners
            .extend(edits.into_iter().filter_map(|edit| match edit {
                Mutation::NewEventListener { name, id } => Some((name, id)),
                _ => None,
            }));
    }

    fn settle(&mut self) {
        for _ in 0..50 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            let edits = self.dom.render_immediate_to_vec();
            self.record(edits.edits);
        }
        panic!("the timesheet did not settle");
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }

    fn go_to(&mut self, day: NaiveDate) {
        let mut date = self.date;
        self.dom.in_scope(ScopeId::APP, || date.set(Anchor(day)));
        self.settle();
    }

    /// The most recently created element listening for `name`.
    fn last(&self, name: &str) -> ElementId {
        self.listeners
            .iter()
            .rev()
            .find(|(listener, _)| listener == name)
            .map(|(_, id)| *id)
            .unwrap()
    }

    fn fire(&mut self, name: &str, id: ElementId, data: Box<dyn Any>, bubbles: bool) {
        let event = Event::new(
            Rc::new(PlatformEventData::new(data)) as Rc<dyn Any>,
            bubbles,
        );
        self.dom.runtime().handle_event(name, event, id);
        self.settle();
    }

    fn type_into_last_cell(&mut self, value: &str) {
        let (input, blur) = (self.last("input"), self.last("blur"));
        self.fire(
            "input",
            input,
            Box::new(SerializedFormData::new(value.into(), vec![])),
            true,
        );
        self.fire("blur", blur, Box::new(SerializedFocusData {}), false);
    }
}

fn next_week() -> NaiveDate {
    monday() + chrono::Duration::days(7)
}

#[tokio::test]
async fn a_week_change_keeps_the_last_week_on_screen_and_inert() {
    let mut page = Page::open();
    assert!(page.html().contains("Loading timesheet"));
    release(monday(), vec![entry(monday(), 60, "Old week project")]);
    page.settle();
    assert!(page.html().contains("Old week project"));
    assert!(!page.html().contains("inert"));

    page.go_to(next_week());
    let html = page.html();
    assert!(!html.contains("Loading timesheet"), "{html}");
    assert!(html.contains("Old week project"), "{html}");
    assert!(html.contains("inert"), "{html}");
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    // The pager already names the requested week.
    assert!(html.contains("14 Sep – 20 Sep 2026"), "{html}");
}

#[tokio::test]
async fn the_new_week_replaces_old_rows_drafts_dialogs_and_errors() {
    let mut page = Page::open();
    release(monday(), vec![entry(monday(), 60, "Old week project")]);
    page.settle();
    // An invalid draft leaves both the draft and an error behind.
    page.type_into_last_cell("not a duration");
    let html = page.html();
    assert!(html.contains("value=\"not a duration\""), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    // The "+" button is the first click target the page creates.
    let add = page
        .listeners
        .iter()
        .find(|(name, _)| name == "click")
        .map(|(_, id)| *id)
        .unwrap();
    page.fire("click", add, Box::new(SerializedMouseData::default()), true);
    assert!(page.html().contains("New time entry"));

    page.go_to(next_week());
    release(next_week(), Vec::new());
    page.settle();
    let html = page.html();
    assert!(html.contains("No time this week"), "{html}");
    for stale in [
        "Old week project",
        "not a duration",
        "role=\"alert\"",
        "New time entry",
        "inert",
        "aria-busy=\"true\"",
    ] {
        assert!(!html.contains(stale), "{stale} survived: {html}");
    }
    assert!(html.contains("data-editor-state=\"clean\""), "{html}");
    assert_eq!(next_week().weekday(), Weekday::Mon);
}

#[tokio::test]
async fn the_inert_old_week_cannot_commit() {
    let mut page = Page::open();
    release(monday(), vec![entry(monday(), 60, "Old week project")]);
    page.settle();
    page.go_to(next_week());
    assert!(page.html().contains("inert"));
    page.type_into_last_cell("2:00");
    assert!(
        COMMANDS.with_borrow(Vec::is_empty),
        "the old week must not be saved"
    );
    // Nor may the attempt leave a draft or an error for the new week.
    release(next_week(), Vec::new());
    page.settle();
    let html = page.html();
    assert!(html.contains("data-editor-state=\"clean\""), "{html}");
    assert!(!html.contains("role=\"alert\""), "{html}");
}
