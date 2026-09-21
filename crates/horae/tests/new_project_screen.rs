#![cfg(feature = "server")]

//! Render the production page with controlled, authorized server responses.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/avatar.rs"]
pub mod avatar;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/date_picker.rs"]
pub mod date_picker;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/select_field.rs"]
pub mod select_field;
mod components {
    pub use super::{avatar, controls, date_picker, form, icons, modal, select_field};
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
    selected_tasks: Vec<project_creation::CreationTask>,
    selected_people: Vec<project_creation::CreationPerson>,
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
            selected_tasks: vec![],
            selected_people: vec![],
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
async fn billing_and_visibility_keep_native_radios_inside_designed_option_cards() {
    let html = render(Probe::default());
    assert_eq!(
        html.matches("data-project-type-icon=").count(),
        3,
        "Missing project type icons"
    );
    assert_eq!(
        html.matches("np-option ").count(),
        5,
        "Three rate options and two visibility options need card chrome"
    );
    for group in ["np-project-type", "np-rate-mode", "np-visibility"] {
        assert!(
            html.contains(&format!("name=\"{group}\"")),
            "Keep native keyboard radio groups: {group}"
        );
    }
}

#[test]
fn shared_input_without_options_keeps_its_original_class_and_external_label() {
    let mut dom = VirtualDom::new(|| rsx! { form::Input { id: "existing-field" } });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("class=\"form-input\""));
    assert!(!html.contains("aria-label="));
}

#[test]
fn compact_input_adds_utilities_and_accessible_name_without_losing_native_state() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            form::Input { id: "compact-field", class: "w-24 font-mono text-right", label: "Tax (%)", value: "1.50", disabled: true }
        }
    });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    for attribute in [
        "class=\"form-input w-24 font-mono text-right\"",
        "aria-label=\"Tax (%)\"",
        "value=\"1.50\"",
        "disabled",
    ] {
        assert!(
            html.contains(attribute),
            "Missing compact input attribute: {attribute}"
        );
    }
}

#[tokio::test]
async fn assignment_controls_reuse_searchable_picker_and_compact_task_entry() {
    let html = render(Probe::default());
    for attribute in [
        "id=\"np-add-person\"",
        "aria-label=\"Choose teammate\"",
        "aria-label=\"Search teammate\"",
        "aria-label=\"Find or create a task\"",
        "placeholder=\"Add a task and press Enter\"",
        "input-group flex-1 basis-assignment-picker min-w-0 h-10",
    ] {
        assert!(
            html.contains(attribute),
            "Missing assignment control: {attribute}"
        );
    }
    assert!(!html.contains("Search teammates</label>"));
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
    let archived_label = html.rfind("Outside the first page (archived)").unwrap();
    let archived_option = html[..archived_label].rsplit("<button").next().unwrap();
    assert!(
        archived_option.contains("role=\"option\"") && archived_option.contains("disabled=true")
    );
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

fn with_form(form: ProjectForm) -> Probe {
    Probe {
        draft: Some(ProjectDraft {
            id: Uuid::now_v7(),
            revision: 1,
            saved_at: chrono::Utc::now(),
            form,
        }),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_real_code_suggestion_does_not_replace_recovered_input() {
    let mut probe = with_form(ProjectForm {
        code: "12-".into(),
        ..Default::default()
    });
    probe.options.previous_code = Some("0009".into());
    probe.options.suggested_code = Some("0010".into());
    let html = render(probe.clone());
    assert!(html.contains("value=\"12-\""));
    assert!(html.contains("Last code: ") && html.contains("0009"));
    assert!(html.contains("Use 0010"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn client_and_currency_use_labelled_form_dropdowns() {
    let html = render(Probe::default());
    assert!(
        html.contains("popovertarget=\"np-client-options\""),
        "{html}"
    );
    assert!(html.contains("aria-label=\"Choose Client\""));
    assert!(html.contains("popovertarget=\"np-currency-options\""));
    assert!(html.contains("aria-label=\"Choose Currency\""));
}

#[tokio::test]
async fn recovered_tags_use_the_shared_inline_chip_field_without_saving() {
    let probe = with_form(ProjectForm {
        tags: vec!["Q3".into(), "platform".into()],
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(
        html.contains("chip-input chip-input-neutral np-tags"),
        "{html}"
    );
    assert!(html.contains("class=\"chip-input-field\""));
    assert!(html.contains("aria-label=\"Remove tag platform\""));
    assert!(html.contains("aria-describedby=\"np-tags-hint\""));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn planning_dates_restore_readable_shared_calendar_fields_without_saving() {
    let probe = with_form(ProjectForm {
        starts_on: "2026-09-01".into(),
        ends_on: "2026-10-15".into(),
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(html.contains("01 Sep 2026"), "{html}");
    assert!(html.contains("15 Oct 2026"));
    assert!(html.contains("popovertarget=\"np-start-calendar\""));
    assert!(html.contains("aria-label=\"Choose Start date\""));
    assert!(html.contains("aria-label=\"Clear Start date\""));
    assert!(!html.contains("type=\"date\""));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn time_and_materials_restores_project_rate_and_budget_without_enabling_unconfigured_email() {
    use horae_core::project::{BudgetMode, RateMode};
    let probe = with_form(ProjectForm {
        rate_mode: RateMode::Project,
        project_rate: "0".into(),
        budget_mode: BudgetMode::TotalHours,
        budget_value: "125:30".into(),
        ..Default::default()
    });
    let html = render(probe);
    let text = html.replace("&#38;", "&").replace("&amp;", "&");
    for label in ["Time & Materials", "Fixed Fee", "Non-Billable"] {
        assert!(text.contains(label), "missing {label}: {html}");
    }
    for id in ["np-project-rate", "np-budget-mode", "np-budget-value"] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
    }
    assert!(html.contains("value=\"125:30\""));
    assert!(html.contains("Email delivery is not configured"));
    assert!(html.contains("Invoice defaults"));
}

#[tokio::test]
async fn nonbillable_budget_picker_omits_fee_modes() {
    let html = render(with_form(ProjectForm {
        project_type: horae_core::types::ProjectType::NonBillable,
        ..Default::default()
    }));
    assert!(html.contains("aria-label=\"Choose Budget\""));
    assert!(html.contains("Hours per task"));
    assert!(!html.contains("Total project fees"));
    assert!(!html.contains("Fees per task"));
}

#[tokio::test]
async fn fixed_fee_restores_each_schedule_and_never_shows_hourly_rate_controls() {
    use horae_core::types::ProjectType;
    use project_creation::{FeeMode, MilestoneInput};
    for mode in [FeeMode::Single, FeeMode::Milestones, FeeMode::Monthly] {
        let probe = with_form(ProjectForm {
            project_type: ProjectType::FixedFee,
            fee_mode: mode,
            fee_amount: "800.25".into(),
            milestones: vec![MilestoneInput {
                id: Uuid::now_v7(),
                name: "Research delivery".into(),
                due_on: "2026-11-30".into(),
                amount: "400.25".into(),
            }],
            ..Default::default()
        });
        let html = render(probe);
        assert!(html.contains("Project fee"), "{html}");
        assert!(!html.contains("Billable rates"));
        match mode {
            FeeMode::Single => assert!(html.contains("value=\"800.25\"")),
            FeeMode::Milestones => {
                assert!(html.contains("Research delivery"));
                assert!(html.contains("value=\"2026-11-30\""));
                assert!(html.contains("Add milestone"));
            }
            FeeMode::Monthly => assert!(html.contains("id=\"np-monthly-day\"")),
        }
        assert!(html.contains("Invoice defaults"));
    }
}

#[tokio::test]
async fn nonbillable_hides_fee_rate_and_invoice_controls_without_erasing_the_draft() {
    let probe = with_form(ProjectForm {
        project_type: horae_core::types::ProjectType::NonBillable,
        fee_amount: "800.25".into(),
        project_rate: "95".into(),
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(html.contains("id=\"np-budget-mode\""), "{html}");
    for hidden in [
        "id=\"np-project-rate\"",
        "id=\"np-fee-amount\"",
        "Invoice defaults",
    ] {
        assert!(!html.contains(hidden));
    }
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn tasks_and_team_restore_scoped_settings_without_exposing_costs_to_managers() {
    use horae_core::project::{BudgetMode, RateMode};
    use project_creation::{
        CreationPerson, CreationTask, ProjectMemberInput, ProjectTaskInput, TaskAccess, TaskSource,
    };
    let user_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();
    let mut probe = with_form(ProjectForm {
        rate_mode: RateMode::Task,
        budget_mode: BudgetMode::HoursPerTask,
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing { task_id },
            billable: true,
            rate: "0".into(),
            budget: "12:30".into(),
            access: TaskAccess::Restricted {
                user_ids: vec![user_id],
            },
        }],
        team: vec![ProjectMemberInput {
            user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    });
    probe.options.tasks.push(CreationTask {
        id: task_id,
        name: "Release preparation".into(),
        billable: true,
        default_rate_cents: Some(8000),
    });
    probe.options.people.push(CreationPerson {
        id: user_id,
        name: "Project teammate".into(),
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    let html = render(probe.clone());
    for text in [
        "Tasks",
        "Team",
        "Release preparation",
        "Project teammate",
        "Project manager",
        "avatar-project",
        "Hourly rate for Release preparation",
        "Budget hours for Release preparation",
        "Restricted (1)",
        "Add everyone",
        "Report visibility",
    ] {
        assert!(html.contains(text), "missing {text}: {html}");
    }
    assert!(html.contains("value=\"12:30\""));
    assert!(!html.contains("np-cost-rate"));
    assert_eq!(probe.writes.get(), 0);
    probe.options.can_edit_private_settings = true;
    probe.options.people[0].cost_rate_cents = Some(6234);
    let html = render(probe);
    assert!(html.contains("np-cost-rate"));
    assert!(html.contains("placeholder=\"62.34\""));
    assert!(html.contains("Cost rate for Project teammate"));
}

#[tokio::test]
async fn recovered_tasks_and_people_outside_the_catalog_page_keep_their_identity() {
    use project_creation::{
        CreationPerson, CreationTask, ProjectMemberInput, ProjectTaskInput, TaskAccess, TaskSource,
    };
    let task_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let mut probe = with_form(ProjectForm {
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing { task_id },
            billable: true,
            rate: String::new(),
            budget: String::new(),
            access: TaskAccess::Restricted {
                user_ids: vec![user_id],
            },
        }],
        team: vec![ProjectMemberInput {
            user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    });
    probe.selected_tasks.push(CreationTask {
        id: task_id,
        name: "Selected remote task".into(),
        billable: true,
        default_rate_cents: None,
    });
    probe.selected_people.push(CreationPerson {
        id: user_id,
        name: "Selected remote teammate".into(),
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    let html = render(probe.clone());
    assert!(html.contains("Selected remote task"), "{html}");
    assert!(html.contains("Selected remote teammate"), "{html}");
    assert!(!html.contains("Unavailable task"));
    assert!(!html.contains("Unavailable teammate"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn invoice_defaults_restore_custom_terms_and_named_second_tax() {
    use project_creation::{InvoiceDefaultsInput, SecondTaxInput};
    let probe = with_form(ProjectForm {
        invoice_defaults: InvoiceDefaultsInput {
            terms_days: "37".into(),
            po_number: "PO-actual".into(),
            tax: "7.25".into(),
            second_tax: Some(SecondTaxInput {
                name: "Local tax".into(),
                percentage: "1.50".into(),
            }),
            discount: "2.75".into(),
        },
        ..Default::default()
    });
    let html = render(probe);
    for (id, value) in [
        ("np-terms-days", "37"),
        ("np-po-number", "PO-actual"),
        ("np-tax", "7.25"),
        ("np-second-tax-name", "Local tax"),
        ("np-second-tax", "1.50"),
        ("np-discount", "2.75"),
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
        assert!(
            html.contains(&format!("value=\"{value}\"")),
            "missing {value}: {html}"
        );
    }
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
    pub async fn project_creation_selection(
        task_ids: Vec<Uuid>,
        user_ids: Vec<Uuid>,
    ) -> Result<project_creation::CreationSelection, ServerFnError> {
        let probe = use_context::<Probe>();
        Ok(project_creation::CreationSelection {
            tasks: probe
                .selected_tasks
                .into_iter()
                .filter(|task| task_ids.contains(&task.id))
                .collect(),
            people: probe
                .selected_people
                .into_iter()
                .filter(|person| user_ids.contains(&person.id))
                .collect(),
        })
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
