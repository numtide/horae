#![cfg(feature = "server")]

//! Render the production timesheet with controlled week responses, so a week
//! change can be observed while the next week is still loading.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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
    /// Weeks whose next load fails once.
    static FAILING: RefCell<HashSet<NaiveDate>> = RefCell::default();
}

/// Keep the week's next load waiting until it is released again.
fn withhold(start: NaiveDate) {
    WEEKS.with_borrow_mut(|weeks| weeks.remove(&start));
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
        if FAILING.with_borrow_mut(|failing| failing.remove(&query.date_from)) {
            return Err(ServerFnError::new("week unavailable"));
        }
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
        Err(ServerFnError::new("submission refused"))
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

#[derive(Clone)]
struct AppProps {
    view: ViewMode,
    slot: DateSlot,
}

fn app(AppProps { view, slot }: AppProps) -> Element {
    components::timer_widget::use_running_timer_provider();
    let date = use_signal(|| Anchor(monday()));
    *slot.borrow_mut() = Some(date);
    rsx! { Timesheet { view, date: date(), span: CalSpan::Week, user: String::new() } }
}

struct Page {
    dom: VirtualDom,
    date: Signal<Anchor>,
    listeners: Vec<(String, ElementId)>,
}

impl Page {
    fn open() -> Self {
        Self::open_in(ViewMode::Week)
    }

    fn open_in(view: ViewMode) -> Self {
        dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter));
        let slot = DateSlot::default();
        let mut dom = VirtualDom::new_with_props(
            app,
            AppProps {
                view,
                slot: slot.clone(),
            },
        );
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

    fn click(&mut self, nth: usize) {
        let id = self.clicks()[nth];
        self.fire("click", id, Box::new(SerializedMouseData::default()), true);
    }

    fn clicks(&self) -> Vec<ElementId> {
        self.listeners
            .iter()
            .filter(|(name, _)| name == "click")
            .map(|(_, id)| *id)
            .collect()
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

/// The click target that makes `shows` true, found by clicking each one on a
/// fresh page built by `setup`. Mutations name listeners by element id only,
/// and the same setup always creates them in the same order.
fn find_click(setup: impl Fn() -> Page, shows: impl Fn(&str) -> bool) -> usize {
    let count = setup().clicks().len();
    let found = (0..count).find(|&nth| {
        let mut page = setup();
        // Some targets navigate, which needs a router this harness lacks.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            page.click(nth);
            shows(&page.html())
        }))
        .unwrap_or(false)
    });
    COMMANDS.with_borrow_mut(Vec::clear);
    found.expect("no click target shows it")
}

fn week_a(view: ViewMode) -> Page {
    let mut page = Page::open_in(view);
    release(monday(), vec![entry(monday(), 60, "Old week project")]);
    page.settle();
    page
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
    // Focus leaving an inert cell blurs it: that must neither keep a draft nor
    // ask for a save the held week cannot make.
    page.type_into_last_cell("2:00");
    let html = page.html();
    assert!(!html.contains("value=\"2:00\""), "{html}");
    assert!(!html.contains("role=\"alert\""), "{html}");
    release(next_week(), Vec::new());
    page.settle();
    let html = page.html();
    assert!(html.contains("data-editor-state=\"clean\""), "{html}");
    assert!(!html.contains("role=\"alert\""), "{html}");
}

#[tokio::test]
async fn returning_to_a_week_still_loading_keeps_it_on_screen() {
    let mut page = week_a(ViewMode::Week);
    page.go_to(next_week());
    withhold(monday());
    page.go_to(monday());
    let html = page.html();
    assert!(!html.contains("Loading timesheet"), "{html}");
    assert!(html.contains("Old week project"), "{html}");
    assert!(html.contains("inert"), "{html}");

    release(monday(), vec![entry(monday(), 60, "Old week project")]);
    page.settle();
    let html = page.html();
    assert!(html.contains("Old week project"), "{html}");
    assert!(!html.contains("inert"), "{html}");
}

#[tokio::test]
async fn a_reload_after_a_save_keeps_the_sheet_and_the_saved_value() {
    let mut page = week_a(ViewMode::Week);
    let (saturday, sunday) = {
        let mut cells = page
            .listeners
            .iter()
            .rev()
            .filter(|(name, _)| name == "input" || name == "blur")
            .map(|(_, id)| *id);
        // Each cell listens for blur, then input; newest first.
        let mut next = || (cells.next().unwrap(), cells.next().unwrap());
        let (sunday_input, sunday_blur) = next();
        let (saturday_input, saturday_blur) = next();
        ((saturday_input, saturday_blur), (sunday_input, sunday_blur))
    };
    let input = |value: &str| Box::new(SerializedFormData::new(value.into(), vec![]));
    // A draft on Saturday, then Sunday saved, which reloads the week.
    page.fire("input", saturday.0, input("0:45"), true);
    withhold(monday());
    page.fire("input", sunday.0, input("2:00"), true);
    page.fire("blur", sunday.1, Box::new(SerializedFocusData {}), false);
    assert_eq!(COMMANDS.with_borrow(Vec::len), 1);
    let html = page.html();
    assert!(!html.contains("Loading timesheet"), "{html}");
    assert!(html.contains("Old week project"), "{html}");
    assert!(html.contains("this week"), "{html}");
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    // The saved value stays until the reload brings it, and the page waits
    // for that reload rather than reading as unsaved.
    assert!(html.contains("value=\"2:00\""), "{html}");
    assert!(html.contains("data-editor-state=\"pending\""), "{html}");
    // The reload makes Saturday inert, which blurs it: the draft waits.
    page.fire("blur", saturday.1, Box::new(SerializedFocusData {}), false);
    assert_eq!(COMMANDS.with_borrow(Vec::len), 1);
    assert!(!page.html().contains("role=\"alert\""));

    let sunday_entry = entry(
        monday() + chrono::Duration::days(6),
        120,
        "Old week project",
    );
    release(
        monday(),
        vec![entry(monday(), 60, "Old week project"), sunday_entry],
    );
    page.settle();
    let html = page.html();
    assert!(html.contains("3:00 this week"), "{html}");
    assert!(html.contains("value=\"2:00\""), "{html}");
    assert!(html.contains("value=\"0:45\""), "{html}");
    assert!(html.contains("data-editor-state=\"dirty\""), "{html}");
    assert!(!html.contains("aria-busy=\"true\""), "{html}");
}

#[tokio::test]
async fn an_added_row_survives_the_reload_after_its_first_save() {
    let loaded = || {
        let mut page = Page::open();
        // Another task than the tracking choice, so adding that choice adds a row.
        let other = VisibleTimeEntry {
            task_id: Uuid::from_u128(4),
            ..entry(monday(), 60, "Old week project")
        };
        release(monday(), vec![other]);
        page.settle();
        page
    };
    let open_add_row = find_click(loaded, |html| html.contains("Add a row"));
    let adding = || {
        let mut page = loaded();
        page.click(open_add_row);
        page
    };
    let cells = |html: &str| html.matches("ts-cell-input").count();
    let rows = cells(&adding().html());
    let add_row = find_click(adding, |html| cells(html) > rows);
    let mut page = adding();
    page.click(add_row);
    let added = cells(&page.html());
    withhold(monday());
    // The added row's cells are the newest.
    page.type_into_last_cell("2:00");
    assert_eq!(COMMANDS.with_borrow(Vec::len), 1);
    let html = page.html();
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    assert_eq!(cells(&html), added, "{html}");
    assert!(html.contains("value=\"2:00\""), "{html}");
}

#[tokio::test]
async fn day_and_calendar_views_hold_the_last_week_too() {
    for view in [ViewMode::Day, ViewMode::Calendar] {
        let mut page = week_a(view);
        // Another weekday: the held view must keep its own day, Monday.
        page.go_to(next_week() + chrono::Duration::days(2));
        let html = page.html();
        assert!(!html.contains("Loading timesheet"), "{view}: {html}");
        assert!(html.contains("Old week project"), "{view}: {html}");
        assert!(html.contains("inert"), "{view}: {html}");
    }
}

#[tokio::test]
async fn retrying_a_failed_week_does_not_bring_back_the_last_one() {
    let failed = || {
        let mut page = week_a(ViewMode::Week);
        FAILING.with_borrow_mut(|failing| failing.insert(next_week()));
        page.go_to(next_week());
        page
    };
    let html = failed().html();
    assert!(html.contains("week unavailable"), "{html}");
    assert!(!html.contains("Old week project"), "{html}");

    let retry = find_click(failed, |html| !html.contains("week unavailable"));
    let mut page = failed();
    page.click(retry);
    let html = page.html();
    assert!(html.contains("Loading timesheet"), "{html}");
    assert!(!html.contains("Old week project"), "{html}");
    assert!(!html.contains("inert"), "{html}");
}

#[tokio::test]
async fn a_week_change_closes_the_add_row_dialog_picker_and_submit_error() {
    let loaded = || {
        let mut page = Page::open();
        // Another task than the tracking choice, so adding that choice adds a row.
        let other = VisibleTimeEntry {
            task_id: Uuid::from_u128(4),
            ..entry(monday(), 60, "Old week project")
        };
        release(monday(), vec![other]);
        page.settle();
        page
    };
    let submit = find_click(loaded, |html| html.contains("submission refused"));
    let submitted = || {
        let mut page = loaded();
        page.click(submit);
        page
    };
    let open_add_row = find_click(submitted, |html| html.contains("Add a row"));
    let adding = || {
        let mut page = submitted();
        page.click(open_add_row);
        page
    };
    let cells = |html: &str| html.matches("ts-cell-input").count();
    let rows = cells(&adding().html());
    let add_row = find_click(adding, |html| cells(html) > rows);
    let added = || {
        let mut page = adding();
        page.click(add_row);
        page.click(open_add_row);
        page
    };
    let picker = find_click(added, |html| html.contains("dp-pop"));
    let mut page = added();
    page.click(picker);
    let html = page.html();
    for open in ["submission refused", "Add a row", "dp-pop"] {
        assert!(html.contains(open), "{open} is not open: {html}");
    }
    assert!(cells(&html) > rows, "{html}");

    page.go_to(next_week());
    release(next_week(), Vec::new());
    page.settle();
    let html = page.html();
    for stale in ["submission refused", "Add a row", "dp-pop", "ts-cell-input"] {
        assert!(!html.contains(stale), "{stale} survived: {html}");
    }
    assert!(html.contains("data-editor-state=\"clean\""), "{html}");
}
