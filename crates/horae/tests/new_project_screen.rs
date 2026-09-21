#![cfg(feature = "server")]

//! Render the production page with controlled, authorized server responses.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/modal.rs"]
pub mod modal;
mod components {
    pub use super::{form, modal};
}
#[path = "../src/models/project_creation.rs"]
pub mod project_creation;
mod models {
    pub use super::project_creation;
}
#[path = "../src/pages/new_project.rs"]
mod new_project;

use project_creation::{
    CreationClient, CreationOptions, CreationSearch, DraftSaved, ProjectDraft, ProjectForm,
};

#[derive(Clone)]
struct Probe {
    options: CreationOptions,
    draft: Option<ProjectDraft>,
    selected_client: Option<CreationClient>,
    writes: Rc<Cell<usize>>,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            options: CreationOptions {
                organization_currency: "EUR".into(),
                can_edit_private_settings: false,
                email_available: false,
                clients: vec![],
                tasks: vec![],
                people: vec![],
                more_clients: false,
                more_tasks: false,
                more_people: false,
                previous_code: None,
                suggested_code: None,
            },
            draft: None,
            selected_client: None,
            writes: Rc::new(Cell::new(0)),
        }
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! {
        HistoryProvider { history: |_| Rc::new(MemoryHistory::with_initial_path("/projects/new")) as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

mod route {
    use super::*;
    use new_project::NewProject;

    #[derive(Clone, PartialEq, Routable)]
    pub enum Route {
        #[route("/projects")]
        ProjectList {},
        #[route("/projects/new")]
        NewProject {},
        #[route("/projects/:id")]
        ProjectDetail { id: Uuid },
    }
    #[component]
    fn ProjectList() -> Element {
        rsx! { h1 { "Projects" } }
    }
    #[component]
    fn ProjectDetail(id: Uuid) -> Element {
        rsx! { h1 { "Project {id}" } }
    }
}

fn render(probe: Probe) -> String {
    let mut dom = VirtualDom::new_with_props(app, probe);
    dom.rebuild_in_place();
    for _ in 0..30 {
        if dom.wait_for_work().now_or_never().is_none() {
            return dioxus::ssr::render(&dom);
        }
        dom.render_immediate_to_vec();
    }
    panic!("new project render did not settle");
}

#[tokio::test]
async fn empty_form_has_labelled_fields_no_demo_data_and_no_spurious_save() {
    let probe = Probe::default();
    let html = render(probe.clone());
    assert!(html.contains("New project"), "{html}");
    assert!(html.contains("No draft saved yet"));
    for id in [
        "np-client",
        "np-name",
        "np-code",
        "np-start",
        "np-end",
        "np-tags",
        "np-currency",
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
        assert!(
            html.contains(&format!("for=\"{id}\"")),
            "unlabelled {id}: {html}"
        );
    }
    assert!(!html.contains("np-notes"));
    assert!(!html.contains("Meridian"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn recovered_client_outside_the_catalog_page_keeps_its_currency_and_name() {
    let client = CreationClient {
        id: Uuid::now_v7(),
        name: "Outside the first page".into(),
        currency: "CHF".into(),
        active: false,
        default_rate_cents: None,
    };
    let mut probe = Probe {
        selected_client: Some(client.clone()),
        ..Default::default()
    };
    probe.draft = Some(ProjectDraft {
        id: Uuid::now_v7(),
        revision: 12,
        saved_at: chrono::Utc::now(),
        form: ProjectForm {
            client_id: Some(client.id),
            name: "Recovered project".into(),
            code: "12-".into(),
            ..Default::default()
        },
    });
    let html = render(probe.clone());
    assert!(html.contains("Outside the first page"), "{html}");
    assert!(html.contains("(archived)"));
    assert!(html.contains("Client default (CHF)"));
    assert!(html.contains("value=\"Recovered project\""));
    assert!(html.contains("value=\"12-\""));
    assert!(html.contains("Draft saved at"));
    let save_button = html
        .split("Save project")
        .next()
        .unwrap()
        .rsplit("<button")
        .next()
        .unwrap();
    assert!(
        save_button.contains("disabled"),
        "An archived client cannot be used to create a project: {html}"
    );
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn administrative_notes_only_render_for_an_authorized_editor() {
    let mut probe = Probe::default();
    probe.options.can_edit_private_settings = true;
    assert!(render(probe).contains("id=\"np-notes\""));
}

mod server_fns {
    use super::*;

    pub async fn project_creation_options(
        _: CreationSearch,
    ) -> Result<CreationOptions, ServerFnError> {
        Ok(use_context::<Probe>().options)
    }
    pub async fn project_creation_client(_: Uuid) -> Result<Option<CreationClient>, ServerFnError> {
        Ok(use_context::<Probe>().selected_client)
    }
    pub async fn load_project_draft() -> Result<Option<ProjectDraft>, ServerFnError> {
        Ok(use_context::<Probe>().draft)
    }
    pub async fn save_project_draft(
        _: Uuid,
        _: i64,
        _: ProjectForm,
    ) -> Result<DraftSaved, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.writes.set(probe.writes.get() + 1);
        panic!("loading a saved draft must not write it");
    }
    pub async fn finalize_project_draft(
        _: Uuid,
        _: i64,
        _: ProjectForm,
    ) -> Result<Uuid, ServerFnError> {
        panic!("unexpected create");
    }
    pub async fn discard_project_draft(_: Uuid, _: i64) -> Result<(), ServerFnError> {
        panic!("unexpected discard");
    }
    pub async fn create_project_client(
        _: String,
        _: String,
        _: String,
    ) -> Result<CreationClient, ServerFnError> {
        panic!("unexpected client creation");
    }
}
