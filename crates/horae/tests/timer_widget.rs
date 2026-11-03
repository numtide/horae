#![cfg(feature = "server")]

//! Exercise the production widget with controlled resource responses. Count
//! name copies at the lookup boundary instead of relying on timing assertions.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::core::{ElementId, Mutation, NoOpMutations};
use dioxus::prelude::*;
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/timer_widget.rs"]
mod timer_widget;

const PROJECT: Uuid = Uuid::from_u128(1);
const TASK: Uuid = Uuid::from_u128(2);

#[derive(Clone)]
struct CountedName {
    text: String,
    copies: Rc<Cell<usize>>,
}

impl CountedName {
    // The widget copies a name into String-valued lookups. Cloning a fixture
    // record uses the derived Clone trait instead, so resource fetches do not
    // count as lookup rebuilds.
    fn clone(&self) -> String {
        self.copies.set(self.copies.get() + 1);
        self.text.clone()
    }
}

#[derive(Clone)]
struct NamedRecord {
    id: Uuid,
    name: CountedName,
}

#[derive(Clone, Copy)]
struct Sources {
    projects: Signal<Option<Vec<NamedRecord>>>,
    tasks: Signal<Option<Vec<NamedRecord>>>,
}

/// What the server reports and how often each read reaches it.
#[derive(Clone, Default)]
struct Server {
    running: Rc<Cell<bool>>,
    project_reads: Rc<Cell<usize>>,
    task_reads: Rc<Cell<usize>>,
    starts: Rc<Cell<usize>>,
    picker_saw_empty: Rc<Cell<bool>>,
    /// Catalog reads never answer.
    catalogs_hang: Rc<Cell<bool>>,
}

impl Server {
    fn catalog_reads(&self) -> (usize, usize) {
        (self.project_reads.get(), self.task_reads.get())
    }
}

#[derive(Clone, Default)]
struct Probe {
    sources: Rc<RefCell<Option<Sources>>>,
    server: Server,
    project_copies: Rc<Cell<usize>>,
    task_copies: Rc<Cell<usize>>,
    renders: Rc<Cell<usize>>,
}

impl Probe {
    fn running() -> Self {
        let probe = Self::default();
        probe.server.running.set(true);
        probe
    }
}

fn record(id: Uuid, text: &str, copies: Rc<Cell<usize>>) -> NamedRecord {
    NamedRecord {
        id,
        name: CountedName {
            text: text.into(),
            copies,
        },
    }
}

fn app(probe: Probe) -> Element {
    probe.renders.set(probe.renders.get() + 1);
    let projects = use_signal(|| {
        Some(vec![record(
            PROJECT,
            "Project",
            probe.project_copies.clone(),
        )])
    });
    let tasks = use_signal(|| Some(vec![record(TASK, "Task", probe.task_copies.clone())]));
    let sources = use_context_provider(|| Sources { projects, tasks });
    *probe.sources.borrow_mut() = Some(sources);
    let server = probe.server.clone();
    use_context_provider(|| server);
    timer_widget::use_running_timer_provider();
    // Call in this scope so mark_dirty drives the exact same hook/render path
    // as the widget's tick, without sleeping or changing its clock code.
    timer_widget::TimerWidget()
}

fn settle(dom: &mut VirtualDom) {
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate(&mut NoOpMutations);
    }
    panic!("widget did not settle after immediate resource responses");
}

fn click_listeners(edits: Vec<Mutation>) -> Vec<ElementId> {
    edits
        .into_iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "click" => Some(id),
            _ => None,
        })
        .collect()
}

/// Click and return the listeners the response rendered.
fn click(dom: &mut VirtualDom, id: ElementId) -> Vec<ElementId> {
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<
            dioxus_html::SerializedMouseData,
        >::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, id);
    let mut edits = Vec::new();
    for _ in 0..20 {
        edits.extend(dom.render_immediate_to_vec().edits);
        if dom.wait_for_work().now_or_never().is_none() {
            return click_listeners(edits);
        }
    }
    panic!("widget did not settle after a click");
}

#[tokio::test]
async fn an_idle_timer_loads_the_catalogs_only_when_the_picker_opens() {
    dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe::default();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let edits = dom.rebuild_to_vec().edits;
    settle(&mut dom);
    assert_eq!(probe.server.catalog_reads(), (0, 0));

    let [open] = click_listeners(edits)[..] else {
        panic!("the idle widget has one control");
    };
    let picker = click(&mut dom, open);
    assert_eq!(probe.server.catalog_reads(), (1, 1));

    // Closing and reopening re-reads, so names created meanwhile appear.
    let [_overlay, close, ..] = picker[..] else {
        panic!("the picker can be closed");
    };
    click(&mut dom, close);
    click(&mut dom, open);
    assert_eq!(probe.server.catalog_reads(), (2, 2));
}

#[tokio::test]
async fn a_first_open_shows_loading_until_the_catalogs_answer() {
    dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe::default();
    probe.server.catalogs_hang.set(true);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let edits = dom.rebuild_to_vec().edits;
    settle(&mut dom);
    click(&mut dom, click_listeners(edits)[0]);
    assert_eq!(probe.server.catalog_reads(), (1, 1));
    assert!(
        !probe.server.picker_saw_empty.get(),
        "an unanswered catalog must not look like an empty one"
    );
}

#[tokio::test]
async fn starting_from_the_picker_refreshes_the_timer_and_its_followers() {
    dioxus_html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe::default();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let edits = dom.rebuild_to_vec().edits;
    settle(&mut dom);
    let picker = click(&mut dom, click_listeners(edits)[0]);
    let [_overlay, _close, start, _cancel] = picker[..] else {
        panic!("the picker has close, start and cancel controls: {picker:?}");
    };
    click(&mut dom, start);

    assert_eq!(probe.server.starts.get(), 1);
    assert!(dioxus::ssr::render(&dom).contains("Project · Task"));
    let changes = dom.in_scope(ScopeId::APP, || {
        consume_context::<timer_widget::RunningTimer>().changes()
    });
    assert_eq!(changes, 1, "pages following the timer must refresh");
    assert_eq!(probe.server.catalog_reads(), (1, 1));
}

#[tokio::test]
async fn timer_names_reuse_lookups_and_follow_resource_changes() {
    let probe = Probe::running();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Project · Task"));
    let initial = (probe.project_copies.get(), probe.task_copies.get());
    assert_eq!(initial, (1, 1));
    let renders = probe.renders.get();

    for _ in 0..100 {
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
        settle(&mut dom);
    }
    assert_eq!(probe.renders.get() - renders, 100);
    assert_eq!(
        (probe.project_copies.get(), probe.task_copies.get()),
        initial
    );

    let mut sources = probe.sources.borrow().unwrap();
    dom.in_scope(ScopeId::APP, || {
        sources.projects.set(Some(vec![record(
            PROJECT,
            "Renamed",
            probe.project_copies.clone(),
        )]));
    });
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Renamed · Task"));
    assert_eq!(
        (probe.project_copies.get(), probe.task_copies.get()),
        (2, 1)
    );

    dom.in_scope(ScopeId::APP, || sources.tasks.set(None));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Renamed"));
    assert!(!html.contains("Renamed · Task"));

    dom.in_scope(ScopeId::APP, || {
        sources.tasks.set(Some(vec![record(
            TASK,
            "Recovered",
            probe.task_copies.clone(),
        )]));
    });
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Renamed · Recovered"));
    assert_eq!(
        (probe.project_copies.get(), probe.task_copies.get()),
        (2, 2)
    );
    let timer_changes = dom.in_scope(ScopeId::APP, || {
        consume_context::<timer_widget::RunningTimer>().changes()
    });
    assert_eq!(
        timer_changes, 0,
        "name changes must not refresh time entries"
    );
}

mod models {
    #[derive(Clone)]
    pub struct TimeEntry {
        pub id: uuid::Uuid,
        pub project_id: uuid::Uuid,
        pub task_id: uuid::Uuid,
        pub minutes: i32,
        pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    }
}

mod server_fns {
    use super::*;

    pub async fn get_current_timer() -> Result<Option<models::TimeEntry>, ServerFnError> {
        let running = consume_context::<Server>().running.get();
        Ok(running.then(|| models::TimeEntry {
            id: Uuid::from_u128(3),
            project_id: PROJECT,
            task_id: TASK,
            minutes: 0,
            started_at: Some(chrono::Utc::now()),
        }))
    }

    pub async fn list_tracking_projects() -> Result<Vec<NamedRecord>, ServerFnError> {
        let server = consume_context::<Server>();
        server.project_reads.set(server.project_reads.get() + 1);
        if server.catalogs_hang.get() {
            std::future::pending::<()>().await;
        }
        consume_context::<Sources>()
            .projects
            .read()
            .clone()
            .ok_or_else(|| ServerFnError::new("unavailable"))
    }

    pub async fn list_tracking_tasks() -> Result<Vec<NamedRecord>, ServerFnError> {
        let server = consume_context::<Server>();
        server.task_reads.set(server.task_reads.get() + 1);
        if server.catalogs_hang.get() {
            std::future::pending::<()>().await;
        }
        consume_context::<Sources>()
            .tasks
            .read()
            .clone()
            .ok_or_else(|| ServerFnError::new("unavailable"))
    }

    pub async fn start_timer(
        project: String,
        task: String,
        _: Option<String>,
    ) -> Result<(), ServerFnError> {
        assert_eq!((project, task), (PROJECT.to_string(), TASK.to_string()));
        let server = consume_context::<Server>();
        server.starts.set(server.starts.get() + 1);
        server.running.set(true);
        Ok(())
    }

    pub async fn stop_timer(_: String) -> Result<(), ServerFnError> {
        panic!("this test must not stop a timer")
    }
}

mod components {
    pub mod no_tracking {
        use super::super::*;

        #[component]
        pub fn NoTrackingHint(
            id: String,
            #[props(default)] assignee: Option<String>,
            can_manage: bool,
            #[props(default)] on_follow: EventHandler<()>,
        ) -> Element {
            rsx! {}
        }
    }

    pub mod session {
        #[derive(Clone, Copy)]
        pub struct Session;

        impl Session {
            pub fn can_create_projects(&self) -> bool {
                false
            }
        }

        pub fn use_session() -> Session {
            Session
        }
    }

    pub mod project_task_picker {
        use super::super::*;

        /// Picks the fixture project and task, as a user would.
        #[component]
        pub fn ProjectTaskPicker(
            mut project: Signal<String>,
            mut task: Signal<String>,
            projects: Resource<Result<Vec<NamedRecord>, ServerFnError>>,
            tasks: Resource<Result<Vec<NamedRecord>, ServerFnError>>,
        ) -> Element {
            use_hook(|| {
                project.set(PROJECT.to_string());
                task.set(TASK.to_string());
            });
            // The fixtures are never empty, so an empty list is a placeholder
            // shown as if it were the catalog.
            let empty = |list: Option<&Result<Vec<NamedRecord>, _>>| matches!(list, Some(Ok(list)) if list.is_empty());
            if empty(projects.read().as_ref()) || empty(tasks.read().as_ref()) {
                consume_context::<Server>().picker_saw_empty.set(true);
            }
            rsx! {}
        }
    }
}
