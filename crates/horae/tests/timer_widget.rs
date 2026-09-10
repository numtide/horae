#![cfg(feature = "server")]

//! Exercise the production widget with controlled resource responses. Count
//! name copies at the lookup boundary instead of relying on timing assertions.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::core::NoOpMutations;
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

#[derive(Clone, Default)]
struct Probe {
    sources: Rc<RefCell<Option<Sources>>>,
    project_copies: Rc<Cell<usize>>,
    task_copies: Rc<Cell<usize>>,
    renders: Rc<Cell<usize>>,
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

#[tokio::test]
async fn timer_names_reuse_lookups_and_follow_resource_changes() {
    let probe = Probe::default();
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
        Ok(Some(models::TimeEntry {
            id: Uuid::from_u128(3),
            project_id: PROJECT,
            task_id: TASK,
            minutes: 0,
            started_at: Some(chrono::Utc::now()),
        }))
    }

    pub async fn list_projects(
        _: Option<String>,
        _: bool,
    ) -> Result<Vec<NamedRecord>, ServerFnError> {
        consume_context::<Sources>()
            .projects
            .read()
            .clone()
            .ok_or_else(|| ServerFnError::new("unavailable"))
    }

    pub async fn list_tasks() -> Result<Vec<NamedRecord>, ServerFnError> {
        consume_context::<Sources>()
            .tasks
            .read()
            .clone()
            .ok_or_else(|| ServerFnError::new("unavailable"))
    }

    pub async fn start_timer(_: String, _: String, _: Option<String>) -> Result<(), ServerFnError> {
        panic!("this test must not start a timer")
    }

    pub async fn stop_timer(_: String) -> Result<(), ServerFnError> {
        panic!("this test must not stop a timer")
    }
}

mod components {
    pub mod project_task_picker {
        use super::super::*;

        #[component]
        pub fn ProjectTaskPicker(
            project: Signal<String>,
            task: Signal<String>,
            projects: Resource<Result<Vec<NamedRecord>, ServerFnError>>,
            tasks: Resource<Result<Vec<NamedRecord>, ServerFnError>>,
        ) -> Element {
            rsx! {}
        }
    }
}
