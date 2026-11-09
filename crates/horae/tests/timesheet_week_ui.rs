#![cfg(feature = "server")]

//! Render the production timesheet with controlled week responses, so a week
//! change can be observed while the next week is still loading.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, Weekday};
use dioxus::dioxus_core::{ElementId, Mutation, TemplateAttribute, TemplateNode};
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
mod real_menu;
/// The real menu, counting its renders. It takes children, so it renders
/// whenever the toolbar does, and so whenever the page does.
pub mod menu {
    use dioxus::prelude::*;
    use std::cell::Cell;

    pub use super::real_menu::MenuItem;

    thread_local! {
        pub static RENDERS: Cell<usize> = const { Cell::new(0) };
    }

    #[component]
    pub fn Menu(id: String, label: String, children: Element) -> Element {
        RENDERS.set(RENDERS.get() + 1);
        rsx! { super::real_menu::Menu { id, label, {children} } }
    }
}
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/no_tracking.rs"]
pub mod no_tracking;
#[path = "../src/components/select_field.rs"]
pub mod select_field;
#[path = "../src/components/today.rs"]
pub mod today;
mod components {
    pub use super::{controls, date_picker, menu, modal, no_tracking, select_field, today};

    /// The shell's week start and a viewer who cannot manage projects, without
    /// the rest of the session.
    pub mod session {
        use dioxus::prelude::*;

        pub struct Session;

        impl Session {
            pub fn can_create_projects(&self) -> bool {
                false
            }
        }

        pub fn use_session() -> Session {
            Session
        }

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

/// The routes the page links to.
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
        #[route("/projects")]
        ProjectList {},
    }

    #[component]
    fn ProjectList() -> Element {
        rsx! {}
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
    /// While set, commands are recorded but stay in flight.
    static HOLD_COMMANDS: Cell<bool> = const { Cell::new(false) };
    /// While set, the viewer has no projects to track.
    static NO_TRACKING: Cell<bool> = const { Cell::new(false) };
    /// While set, every command is refused.
    static FAIL_COMMANDS: Cell<bool> = const { Cell::new(false) };
    /// Seconds the selected person's zone is ahead of UTC.
    static SUBJECT_OFFSET: Cell<i32> = const { Cell::new(0) };
}

/// Answer the commands held in flight.
fn let_commands_through() {
    HOLD_COMMANDS.set(false);
    RELEASED.with(|released| released.notify_waiters());
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
        subject_utc_offset_seconds: SUBJECT_OFFSET.get(),
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
        if NO_TRACKING.get() {
            return Ok(Vec::new());
        }
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
    ) -> Result<Option<String>, ServerFnError> {
        COMMANDS.with_borrow_mut(|commands| commands.push(command));
        if FAIL_COMMANDS.get() {
            return Err(ServerFnError::new("refused"));
        }
        let released = RELEASED.with(Rc::clone);
        loop {
            let notified = released.notified();
            if !HOLD_COMMANDS.get() {
                return Ok(None);
            }
            notified.await;
        }
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
        let id = self.ids("click")[nth];
        self.fire("click", id, Box::new(SerializedMouseData::default()), true);
    }

    /// Every element listening for `name`, in the order they were created.
    fn ids(&self, name: &str) -> Vec<ElementId> {
        self.listeners
            .iter()
            .filter(|(listener, _)| listener == name)
            .map(|(_, id)| *id)
            .collect()
    }

    /// A mouse event `y` px down its target, as a calendar column reads it.
    fn mouse(&mut self, name: &str, id: ElementId, y: f64) {
        use dioxus::html::geometry::{
            ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint,
        };
        let at = Coordinates::new(
            ScreenPoint::zero(),
            ClientPoint::zero(),
            ElementPoint::new(0.0, y),
            PagePoint::zero(),
        );
        let data = SerializedMouseData::new(None, Default::default(), at, Default::default());
        self.fire(name, id, Box::new(data), true);
    }

    /// How often the calendar has rendered, found as the scope whose root is
    /// `div.ts-cal`.
    fn calendar_renders(&self) -> usize {
        let calendar = (0..1000).map(ScopeId).find(|&id| {
            self.dom
                .get_scope(id)
                .and_then(|scope| scope.try_root_node())
                .is_some_and(|node| {
                    matches!(
                        node.template.roots.first(),
                        Some(TemplateNode::Element { attrs, .. }) if attrs.iter().any(|attr| matches!(
                            attr,
                            TemplateAttribute::Static { name: "class", value: "ts-cal", .. }
                        ))
                    )
                })
        });
        let calendar = calendar.expect("the calendar is mounted");
        self.dom.in_scope(calendar, dioxus::dioxus_core::generation)
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
    find_target(setup, "click", |page, nth| page.click(nth), shows)
}

/// The same search for any listener, with `act` doing what a target needs.
fn find_target(
    setup: impl Fn() -> Page,
    name: &str,
    act: impl Fn(&mut Page, usize),
    shows: impl Fn(&str) -> bool,
) -> usize {
    let count = setup().ids(name).len();
    let found = (0..count).find(|&nth| {
        let mut page = setup();
        // Some targets navigate, which needs a router this harness lacks.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            act(&mut page, nth);
            shows(&page.html())
        }))
        .unwrap_or(false)
    });
    COMMANDS.with_borrow_mut(Vec::clear);
    found.unwrap_or_else(|| panic!("no {name} target shows it"))
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
    page.fire("input", sunday.0, input("1:07"), true);
    page.fire("blur", sunday.1, Box::new(SerializedFocusData {}), false);
    assert_eq!(COMMANDS.with_borrow(Vec::len), 1);
    let html = page.html();
    assert!(!html.contains("Loading timesheet"), "{html}");
    assert!(html.contains("Old week project"), "{html}");
    assert!(html.contains("this week"), "{html}");
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    // The saved value stays until the reload brings it, and the page waits
    // for that reload rather than reading as unsaved.
    assert!(html.contains("value=\"1:07\""), "{html}");
    assert!(html.contains("data-editor-state=\"pending\""), "{html}");
    // The reload makes Saturday inert, which blurs it: the draft waits.
    page.fire("blur", saturday.1, Box::new(SerializedFocusData {}), false);
    assert_eq!(COMMANDS.with_borrow(Vec::len), 1);
    assert!(!page.html().contains("role=\"alert\""));

    // The server stores what it rounds the input to, not the input itself.
    let sunday_entry = entry(monday() + chrono::Duration::days(6), 75, "Old week project");
    release(
        monday(),
        vec![entry(monday(), 60, "Old week project"), sunday_entry],
    );
    page.settle();
    let html = page.html();
    assert!(html.contains("2:15 this week"), "{html}");
    // The stored value replaces the saved draft, which would otherwise still read 1:07.
    assert!(html.contains("value=\"1:15\""), "{html}");
    assert!(!html.contains("value=\"1:07\""), "{html}");
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

/// A calendar with one block, Monday 3:00–4:00, and which press grabs it.
fn calendar_with_a_block() -> (Page, usize, VisibleTimeEntry) {
    let timed = VisibleTimeEntry {
        start_minute: Some(180),
        ..entry(monday(), 60, "Old week project")
    };
    let loaded = || {
        let mut page = Page::open_in(ViewMode::Calendar);
        release(monday(), vec![timed.clone()]);
        page.settle();
        page
    };
    // Pressing the block and letting go without moving opens it for editing.
    let block = find_target(
        loaded,
        "mousedown",
        |page, nth| {
            let (down, up) = (page.ids("mousedown")[nth], page.ids("mouseup")[0]);
            page.mouse("mousedown", down, 0.0);
            page.mouse("mouseup", up, 0.0);
        },
        |html| html.contains("Edit time entry"),
    );
    let page = loaded();
    assert!(page.html().contains("3:00–4:00"));
    (page, block, timed)
}

/// Grab the block at its top and drop it on Tuesday at 5:00.
fn drop_on_tuesday_at_five(page: &mut Page, block: usize) {
    let tuesday = (page.ids("mousemove")[1], page.ids("mouseup")[1]);
    page.mouse("mousedown", page.ids("mousedown")[block], 0.0);
    page.mouse("mousemove", tuesday.0, f64::from(5 * 48));
    page.mouse("mouseup", tuesday.1, f64::from(5 * 48));
}

#[tokio::test]
async fn a_dropped_block_stays_put_while_its_move_is_sent() {
    let (mut page, block, timed) = calendar_with_a_block();
    HOLD_COMMANDS.set(true);
    drop_on_tuesday_at_five(&mut page, block);
    let command = COMMANDS.with_borrow(|commands| commands.last().cloned());
    assert_eq!(
        command,
        Some(TimesheetCommand::Reschedule {
            entry_id: timed.id,
            spent_date: monday() + chrono::Duration::days(1),
            start_minute: 300,
            minutes: 60,
        })
    );
    let html = page.html();
    assert!(html.contains("5:00–6:00"), "{html}");
    assert!(!html.contains("3:00–4:00"), "{html}");

    // Its reload brings the stored move; the block does not jump back on the way.
    withhold(monday());
    let_commands_through();
    page.settle();
    let html = page.html();
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    assert!(html.contains("5:00–6:00"), "{html}");
    release(
        monday(),
        vec![VisibleTimeEntry {
            spent_date: monday() + chrono::Duration::days(1),
            start_minute: Some(300),
            ..timed.clone()
        }],
    );
    page.settle();
    let html = page.html();
    assert!(html.contains("5:00–6:00"), "{html}");
    assert!(!html.contains("aria-busy=\"true\""), "{html}");
}

#[tokio::test]
async fn a_refused_move_puts_the_block_back() {
    let (mut page, block, _) = calendar_with_a_block();
    FAIL_COMMANDS.set(true);
    withhold(monday());
    drop_on_tuesday_at_five(&mut page, block);
    // The refusal reloads the sheet; while that is held, the block is home.
    let html = page.html();
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    assert!(html.contains("Could not change entry"), "{html}");
    assert!(html.contains("3:00–4:00"), "{html}");
    assert!(!html.contains("5:00–6:00"), "{html}");
}

#[tokio::test]
async fn an_unrelated_reload_during_a_move_keeps_the_block_put() {
    let (mut page, block, _) = calendar_with_a_block();
    HOLD_COMMANDS.set(true);
    drop_on_tuesday_at_five(&mut page, block);
    // The rail's timer changes while the move is still being sent: that
    // reload lands with the sheet from before the move.
    let mut timer = page
        .dom
        .in_scope(ScopeId::APP, components::timer_widget::use_running_timer);
    page.dom.in_scope(ScopeId::APP, || timer.refresh());
    page.settle();
    let html = page.html();
    assert!(!html.contains("aria-busy=\"true\""), "{html}");
    assert!(html.contains("5:00–6:00"), "{html}");
    assert!(!html.contains("3:00–4:00"), "{html}");
}

#[tokio::test]
async fn hovering_and_dragging_in_the_calendar_leave_the_page_alone() {
    let (mut page, block, _) = calendar_with_a_block();
    let page_renders = || menu::RENDERS.get();
    let monday_column = page.ids("mousemove")[0];

    // Each snap step over a free slot moves the "+ Add time" hint.
    let before = page_renders();
    for hour in [8, 9, 10] {
        page.mouse("mousemove", monday_column, f64::from(hour * 48));
    }
    let html = page.html();
    assert!(html.contains("+ Add time"), "{html}");
    assert_eq!(page_renders(), before, "hovering re-rendered the page");

    // Starting a drag marks the page dirty once; its steps do not.
    page.mouse("mousedown", page.ids("mousedown")[block], 0.0);
    assert!(page.html().contains("data-editor-state=\"dirty\""));
    let before = page_renders();
    for hour in [5, 6, 7] {
        page.mouse("mousemove", monday_column, f64::from(hour * 48));
    }
    assert!(page.html().contains("7:00–8:00"), "{}", page.html());
    assert_eq!(page_renders(), before, "dragging re-rendered the page");
}

#[tokio::test]
async fn the_calendar_stays_put_while_the_page_re_renders_around_it() {
    let (mut page, _, _) = calendar_with_a_block();
    let (page_before, calendar_before) = (menu::RENDERS.get(), page.calendar_renders());
    // Opening the entry dialog re-renders the page, not the calendar's props.
    // The "+" button is the first click target the page creates.
    page.click(0);
    assert!(page.html().contains("New time entry"));
    assert!(
        menu::RENDERS.get() > page_before,
        "the page did not re-render"
    );
    assert_eq!(
        page.calendar_renders(),
        calendar_before,
        "the calendar re-rendered with its parent"
    );
}

#[tokio::test]
async fn the_toolbar_keeps_the_last_choices_while_the_sheet_reloads() {
    // The add button's attribute names, so `disabled` and `aria-disabled`
    // can be told apart.
    let add_button = |html: &str| -> Vec<String> {
        let start = html.find("class=\"ts-add\"").expect("the add button");
        let tag = html[start..].split('>').next().unwrap_or_default();
        tag.split_whitespace()
            .map(|attribute| attribute.split('=').next().unwrap_or_default().to_string())
            .collect()
    };
    let has = |html: &str, name: &str| add_button(html).iter().any(|a| a == name);

    // With choices, the add button keeps its look and focus through a week
    // change, but says it is unavailable until the new week is admitted.
    let mut page = week_a(ViewMode::Week);
    let html = page.html();
    assert!(
        !has(&html, "disabled") && !has(&html, "aria-disabled"),
        "{html}"
    );
    page.go_to(next_week());
    let html = page.html();
    assert!(html.contains("inert"), "{html}");
    assert!(!has(&html, "disabled"), "{html}");
    assert!(html.contains("aria-disabled=\"true\""), "{html}");
    release(next_week(), Vec::new());
    page.settle();
    let html = page.html();
    assert!(
        !has(&html, "disabled") && !has(&html, "aria-disabled"),
        "{html}"
    );

    // Without, so does the reason beside the disabled button.
    NO_TRACKING.set(true);
    withhold(next_week());
    let hint = "No projects are assigned to you yet.";
    let mut page = week_a(ViewMode::Week);
    let html = page.html();
    assert!(html.contains(hint), "{html}");
    assert!(has(&html, "disabled"), "{html}");
    page.go_to(next_week());
    let html = page.html();
    assert!(html.contains("inert"), "{html}");
    assert!(html.contains(hint), "{html}");
    assert!(has(&html, "disabled"), "{html}");
}

#[tokio::test]
async fn today_is_the_selected_persons_day_not_the_viewers() {
    use chrono::Timelike;
    // UTC+14 or UTC−12, whichever is on another date than UTC right now.
    let hours: i32 = if chrono::Utc::now().hour() >= 11 {
        14
    } else {
        -12
    };
    SUBJECT_OFFSET.set(hours * 3600);
    let theirs = (chrono::Utc::now() + chrono::Duration::hours(hours.into())).date_naive();
    let mut page = Page::open_in(ViewMode::Day);
    page.go_to(theirs);
    let their_monday =
        theirs - chrono::Duration::days(theirs.weekday().num_days_from_monday().into());
    release(their_monday, Vec::new());
    page.settle();
    let html = page.html();
    assert!(html.contains("<span class=\"cur\">Today</span>"), "{html}");
}
