#![cfg(feature = "server")]

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use dioxus::core::{AttributeValue, ElementId, Mutation};
use dioxus::history::MemoryHistory;
use dioxus::html::{FileData, SerializedFileData, SerializedFormObject};
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::importers::harvest::client::{
    ClientSettings, ClientSource, DISCONNECT_UNCONFIRMED,
};
use horae_core::importers::harvest::types::{
    ConnectionStatus, EntityType, ImportMode, ImportReport, PersonErrorKind, RowOutcome,
    SourceKind, SyncScope,
};
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/pages/importers.rs"]
mod importers;
#[path = "../src/components/menu.rs"]
pub mod menu;
#[path = "../src/components/modal.rs"]
pub mod modal;

mod models {
    include!("../src/models/jobs.rs");

    pub mod user {
        #[derive(Debug, Clone, PartialEq)]
        pub struct NewPerson {
            pub email: String,
            pub name: String,
        }

        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum Activation {
            Active,
            Deactivated,
        }

        pub struct User {
            pub email: String,
        }

        pub struct AddedPeople {
            pub added: Vec<User>,
            pub present: Vec<String>,
        }
    }
}

mod components {
    pub use crate::{menu, modal};
}

/// The importer routes, as the app declares them.
mod route {
    use crate::importers::{
        ConnectOutcome, HarvestCsv, HarvestOAuth, HarvestOverview, HarvestRun, HarvestRuns,
        ImporterIndex, RunsCursor, SecretFocus,
    };
    use dioxus::prelude::*;
    use uuid::Uuid;

    #[component]
    fn ProjectList() -> Element {
        rsx! { "projects" }
    }

    #[component]
    fn AdminUsers() -> Element {
        rsx! { "team" }
    }

    #[derive(Routable, Clone, PartialEq)]
    pub enum Route {
        #[route("/projects")]
        ProjectList {},
        #[route("/admin/users")]
        AdminUsers {},
        #[route("/admin/importers")]
        ImporterIndex {},
        #[route("/admin/importers/harvest?:..outcome")]
        HarvestOverview { outcome: ConnectOutcome },
        #[route("/admin/importers/harvest/oauth?:..focus")]
        HarvestOAuth { focus: SecretFocus },
        #[route("/admin/importers/harvest/csv")]
        HarvestCsv {},
        #[route("/admin/importers/harvest/runs?:..before")]
        HarvestRuns { before: RunsCursor },
        #[route("/admin/importers/harvest/runs/:id")]
        HarvestRun { id: Uuid },
    }
}

type JobResponse = Result<Option<models::JobStatus>, ServerFnError>;
type ActionResponse = Result<models::JobStatus, ServerFnError>;
type ChangeResponse = oneshot::Receiver<Result<(), ServerFnError>>;
type ConnectionResponse = oneshot::Receiver<Result<ConnectionStatus, ServerFnError>>;
type ConnectResponse = oneshot::Receiver<Result<String, ServerFnError>>;
type ApiSubmission = (ImportMode, SyncScope, Option<i64>);
type ClientSave = (String, Option<String>, String, bool);
type Additions = Vec<(Vec<models::user::NewPerson>, models::user::Activation)>;

#[derive(Clone, Default)]
struct Probe {
    connection: Rc<RefCell<ConnectionStatus>>,
    connection_response: Rc<RefCell<Option<ConnectionResponse>>>,
    connects: Rc<RefCell<usize>>,
    connect_response: Rc<RefCell<Option<ConnectResponse>>>,
    disconnects: Rc<RefCell<usize>>,
    disconnect_response: Rc<RefCell<Option<ChangeResponse>>>,
    changes: Rc<RefCell<Vec<(String, i64, i64)>>>,
    change_responses: Rc<RefCell<VecDeque<ChangeResponse>>>,
    history: Rc<RefCell<Vec<models::JobStatus>>>,
    requests: Rc<RefCell<Vec<Uuid>>>,
    responses: Rc<RefCell<VecDeque<oneshot::Receiver<JobResponse>>>>,
    history_error: Rc<RefCell<Option<String>>>,
    history_cursors: Rc<RefCell<Vec<Option<Uuid>>>>,
    retries: Rc<RefCell<Vec<Uuid>>>,
    cancellations: Rc<RefCell<Vec<Uuid>>>,
    commits: Rc<RefCell<Vec<Uuid>>>,
    csv_submissions: Rc<RefCell<Vec<(ImportMode, String)>>>,
    api_submissions: Rc<RefCell<Vec<ApiSubmission>>>,
    action_responses: Rc<RefCell<VecDeque<oneshot::Receiver<ActionResponse>>>>,
    summary: Rc<RefCell<models::ErrorSummary>>,
    summaries: Rc<RefCell<Vec<Uuid>>>,
    additions: Rc<RefCell<Additions>>,
    addition_error: Rc<RefCell<Option<String>>>,
    /// Emails the server reports as already having an account.
    already_present: Rc<RefCell<Vec<String>>>,
    client: Rc<RefCell<ClientSettings>>,
    client_saves: Rc<RefCell<Vec<ClientSave>>>,
    client_removes: Rc<RefCell<usize>>,
}

impl Probe {
    fn request(&self) -> oneshot::Sender<JobResponse> {
        let (send, receive) = oneshot::channel();
        self.responses.borrow_mut().push_back(receive);
        send
    }

    fn action(&self) -> oneshot::Sender<ActionResponse> {
        let (send, receive) = oneshot::channel();
        self.action_responses.borrow_mut().push_back(receive);
        send
    }
}

#[derive(Props, Clone)]
struct AppProps {
    probe: Probe,
    history: Rc<MemoryHistory>,
}

impl PartialEq for AppProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn app(props: AppProps) -> Element {
    use_context_provider(|| props.probe.clone());
    let history = props.history.clone();
    rsx! {
        HistoryProvider {
            history: move |_| history.clone() as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

struct Ui {
    dom: VirtualDom,
    history: Rc<MemoryHistory>,
    targets: HashMap<String, ElementId>,
}

impl Ui {
    fn start(probe: &Probe) -> Self {
        Self::at(probe, "/admin/importers")
    }

    fn at(probe: &Probe, path: &str) -> Self {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let history = Rc::new(MemoryHistory::with_initial_path(path));
        let mut ui = Self {
            dom: VirtualDom::new_with_props(
                app,
                AppProps {
                    probe: probe.clone(),
                    history: history.clone(),
                },
            ),
            history,
            targets: HashMap::new(),
        };
        let mutations = ui.dom.rebuild_to_vec().edits;
        ui.record(mutations);
        ui.settle();
        ui
    }

    fn record(&mut self, mutations: Vec<Mutation>) {
        for mutation in mutations {
            if let Mutation::SetAttribute {
                name: attribute,
                value: AttributeValue::Text(name),
                id,
                ..
            } = mutation
                && matches!(attribute, "data-testid" | "id")
            {
                self.targets.insert(name, id);
            }
        }
    }

    fn settle(&mut self) {
        for _ in 0..30 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            let mutations = self.dom.render_immediate_to_vec().edits;
            self.record(mutations);
        }
        panic!("importer did not settle after controlled responses");
    }

    fn target(&self, name: &str) -> ElementId {
        *self
            .targets
            .get(name)
            .unwrap_or_else(|| panic!("missing target {name}: {}", self.html()))
    }

    fn click(&mut self, name: &str) {
        let id = self.target(name);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event("click", event, id);
        self.settle();
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }

    /// The router's current path and query.
    fn path(&self) -> String {
        self.history.current_route()
    }

    fn form_event(&mut self, event_name: &str, name: &str, value: &str) {
        let id = self.target(name);
        let data = SerializedFormData::new(value.into(), vec![]);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event(event_name, event, id);
        self.settle();
    }

    fn type_into(&mut self, name: &str, value: &str) {
        self.form_event("input", name, value);
    }

    fn check(&mut self, name: &str) {
        self.form_event("change", name, "true");
    }

    fn cancel_dialog(&mut self) {
        let id = self.targets["harvest-change-account"];
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedCancelData {}))) as Rc<dyn Any>,
            false,
        );
        self.dom.runtime().handle_event("cancel", event, id);
        self.settle();
    }

    fn choose_file(&mut self, name: &str) {
        let id = self.targets["csv-file"];
        let data = SerializedFormData::new(
            String::new(),
            vec![SerializedFormObject {
                key: "file".into(),
                text: None,
                file: Some(SerializedFileData {
                    path: name.into(),
                    size: 5,
                    last_modified: 0,
                    content_type: Some("text/csv".into()),
                    contents: Some("test\n".into()),
                }),
            }],
        );
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event("change", event, id);
        self.settle();
    }
}

fn job(id: u128, status: &str) -> models::JobStatus {
    models::JobStatus {
        id: Uuid::from_u128(id),
        kind: "harvest_csv_import".into(),
        status: status.into(),
        phase: Some("time_entries".into()),
        processed_count: 12,
        total_count: Some(30),
        report: None,
        last_error: None,
        created_at: chrono::Utc::now(),
        finished_at: None,
        retry_availability: models::RetryAvailability::Available,
        mode: Some(ImportMode::DryRun),
        source: Some(SourceKind::Csv),
        commit_availability: models::CommitAvailability::UnavailableState,
        committed_as: None,
        error_kind: None,
        sync: None,
    }
}

/// A finished run of `mode` with its report.
fn finished(id: u128, mode: ImportMode, report: ImportReport) -> models::JobStatus {
    let mut run = job(id, "succeeded");
    run.mode = Some(mode);
    run.commit_availability = match mode {
        ImportMode::DryRun => models::CommitAvailability::Available,
        ImportMode::Commit => models::CommitAvailability::NotPreview,
    };
    run.report = Some(serde_json::to_value(report).unwrap());
    run
}

fn run_path(id: u128) -> String {
    format!("/admin/importers/harvest/runs/{}", Uuid::from_u128(id))
}

/// Open a run's page and answer its first status read.
fn open_run(probe: &Probe, run: models::JobStatus) -> Ui {
    let response = probe.request();
    let path = format!("/admin/importers/harvest/runs/{}", run.id);
    let mut ui = Ui::at(probe, &path);
    response.send(Ok(Some(run))).unwrap();
    ui.settle();
    ui
}

fn errored_report(mode: ImportMode, errors: usize) -> ImportReport {
    let mut report = ImportReport::new(SourceKind::Csv, mode);
    for line in 1..=errors {
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: format!("record {line}"),
                reason: "invalid date".into(),
                person: None,
                entry_kind: None,
            },
        );
    }
    report
}

#[test]
fn the_index_lists_only_real_sources() {
    let probe = Probe::default();
    let ui = Ui::start(&probe);
    let html = ui.html();
    assert!(
        html.contains("Harvest") && html.contains("CSV file"),
        "{html}"
    );
    // An empty whole-query argument still renders its `?`.
    assert!(
        html.contains("href=\"/admin/importers/harvest?\""),
        "{html}"
    );
    assert!(
        html.contains("href=\"/admin/importers/harvest/csv\""),
        "{html}"
    );
    for planned in ["Toggl Track", "Clockify", "Planned"] {
        assert!(!html.contains(planned), "{html}");
    }
}

#[test]
fn the_index_badge_has_the_harvest_pages_tone() {
    for (connected, expired, tone) in [
        (true, false, "badge-success"),
        (true, true, "badge-warning"),
        (false, false, "badge-neutral"),
    ] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            connected,
            token_expired: expired,
            account_id: Some("A".into()),
            ..ConnectionStatus::default()
        };
        let html = Ui::start(&probe).html();
        assert!(
            html.contains(&format!(
                "class=\"badge {tone} badge-sm\" data-testid=\"index-connection-status\""
            )),
            "{html}"
        );
    }
}

#[tokio::test]
async fn a_run_page_follows_its_job_from_the_url() {
    for state in ["queued", "running"] {
        let probe = Probe::default();
        let mut snapshot = job(1, state);
        snapshot.last_error = Some("Temporary source failure".into());
        let ui = open_run(&probe, snapshot);
        let html = ui.html();
        assert!(html.contains("12 of 30"), "{html}");
        // Only the phase is announced; the count is left to the progress bar.
        assert!(
            html.contains("aria-live=\"polite\">Checking time entries</span>"),
            "{html}"
        );
        // A queued job waits for its next attempt; a running one is that
        // attempt, so the last failure is no longer news.
        assert_eq!(
            html.contains("The last attempt failed. Horae will try again"),
            state == "queued",
            "{state}: {html}"
        );
        assert!(
            html.contains("You can leave this page; the preview keeps running"),
            "{html}"
        );
        assert!(html.contains("Nothing in Horae changes"), "{html}");
        assert_eq!(&*probe.requests.borrow(), &[Uuid::from_u128(1)]);
    }
}

#[tokio::test]
async fn progress_uses_observed_counts_and_readable_phase_without_an_eta() {
    let probe = Probe::default();
    let mut snapshot = job(1, "running");
    snapshot.total_count = None;
    let ui = open_run(&probe, snapshot);
    let html = ui.html();
    assert!(html.contains("Checking time entries"), "{html}");
    assert!(html.contains("data-testid=\"run-count\">12<"), "{html}");
    assert!(!html.contains(" of 30"));
    assert!(!html.contains("time_entries"));
}

#[tokio::test]
async fn an_import_writes_to_horae_and_says_what_cancel_keeps() {
    let probe = Probe::default();
    let mut snapshot = job(1, "running");
    snapshot.mode = Some(ImportMode::Commit);
    snapshot.phase = Some("catalog".into());
    snapshot.processed_count = 180;
    snapshot.total_count = Some(229);
    let ui = open_run(&probe, snapshot);
    let html = ui.html();
    assert!(html.contains("Writing to Horae"), "{html}");
    assert!(
        html.contains("Writing clients, projects, tasks and people"),
        "{html}"
    );
    assert!(html.contains("180 of 229"), "{html}");
    assert!(html.contains("max=\"229\""), "{html}");
    assert!(html.contains("what is already written stays"), "{html}");
    assert!(html.contains("Cancel import"), "{html}");
}

#[tokio::test]
async fn a_stopped_run_says_what_to_do_for_each_error_kind() {
    for (kind, message, action) in [
        (
            models::JobErrorKind::ReconnectRequired,
            "no longer accepts Horae",
            "Reconnect Harvest",
        ),
        (
            models::JobErrorKind::Forbidden,
            "Ask a Harvest administrator",
            "Reconnect as another user",
        ),
        (
            models::JobErrorKind::Unavailable,
            "respond after several tries",
            "Continue preview",
        ),
    ] {
        let probe = Probe::default();
        probe.connection.borrow_mut().account_id = Some("1000001".into());
        let mut snapshot = job(1, "failed");
        snapshot.kind = "harvest_api_import".into();
        snapshot.source = Some(SourceKind::HarvestApi);
        snapshot.error_kind = Some(kind);
        snapshot.last_error = Some("raw provider text".into());
        let ui = open_run(&probe, snapshot);
        let html = ui.html();
        assert!(html.contains(message), "{html}");
        assert!(html.contains(action), "{html}");
        assert!(html.contains("Nothing in Horae changed"), "{html}");
        assert!(!html.contains("raw provider text"), "{html}");
        // A stopped preview can always be replaced by a new one.
        assert!(html.contains("data-testid=\"new-preview\""), "{html}");
        if kind == models::JobErrorKind::Forbidden {
            assert!(html.contains("Harvest account 1000001"), "{html}");
        }
    }
}

#[tokio::test]
async fn a_failed_status_read_can_be_checked_again() {
    let probe = Probe::default();
    let response = probe.request();
    let mut ui = Ui::at(&probe, &run_path(1));
    response
        .send(Err(ServerFnError::new("Network unavailable")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Network unavailable"));
    assert!(ui.html().contains("Status unavailable"));

    let response = probe.request();
    ui.click("resume-monitoring");
    response.send(Ok(Some(job(1, "cancelled")))).unwrap();
    ui.settle();
    assert!(
        ui.html().contains("Nothing in Horae changed"),
        "{}",
        ui.html()
    );
    assert!(!ui.html().contains("Network unavailable"));
    assert_eq!(
        &*probe.requests.borrow(),
        &[Uuid::from_u128(1), Uuid::from_u128(1)]
    );
}

#[tokio::test]
async fn a_missing_job_explains_that_status_is_unavailable() {
    let probe = Probe::default();
    let response = probe.request();
    let mut ui = Ui::at(&probe, &run_path(1));
    response.send(Ok(None)).unwrap();
    ui.settle();
    assert!(ui.html().contains("Import job not found"));
}

#[tokio::test]
async fn a_finished_preview_commits_once_and_opens_the_commit_run() {
    let probe = Probe::default();
    let preview = finished(
        1,
        ImportMode::DryRun,
        ImportReport::new(SourceKind::Csv, ImportMode::DryRun),
    );
    let mut ui = open_run(&probe, preview);
    assert!(ui.html().contains("records are ready"), "{}", ui.html());
    let action = probe.action();
    ui.click("commit-preview");
    ui.click("commit-preview");
    assert_eq!(&*probe.commits.borrow(), &[Uuid::from_u128(1)]);

    let next = probe.request();
    let mut commit = job(2, "queued");
    commit.mode = Some(ImportMode::Commit);
    commit.phase = None;
    action.send(Ok(commit.clone())).unwrap();
    ui.settle();
    next.send(Ok(Some(commit))).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Waiting to start"), "{html}");
    assert!(!html.contains("records are ready"), "{html}");
    assert_eq!(
        &*probe.requests.borrow(),
        &[Uuid::from_u128(1), Uuid::from_u128(2)]
    );
}

#[tokio::test]
async fn a_refused_commit_is_explained() {
    let probe = Probe::default();
    let preview = finished(
        1,
        ImportMode::DryRun,
        ImportReport::new(SourceKind::Csv, ImportMode::DryRun),
    );
    let mut ui = open_run(&probe, preview.clone());
    let action = probe.action();
    ui.click("commit-preview");
    // The refusal re-reads the run, which now says why.
    let reread = probe.request();
    action
        .send(Err(ServerFnError::new(
            "A newer import replaced this preview",
        )))
        .unwrap();
    ui.settle();
    let mut replaced = preview;
    replaced.commit_availability = models::CommitAvailability::NewerImport;
    reread.send(Ok(Some(replaced))).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Could not import this preview"), "{html}");
    assert!(
        html.contains("A newer import from this source replaced this preview"),
        "{html}"
    );
    assert!(!html.contains("data-testid=\"commit-preview\""), "{html}");
    assert_eq!(probe.requests.borrow().len(), 2);
}

#[tokio::test]
async fn a_committed_preview_links_to_its_commit() {
    let probe = Probe::default();
    let mut preview = finished(
        1,
        ImportMode::DryRun,
        ImportReport::new(SourceKind::Csv, ImportMode::DryRun),
    );
    preview.commit_availability = models::CommitAvailability::Committed;
    preview.committed_as = Some(Uuid::from_u128(2));
    let ui = open_run(&probe, preview);
    let html = ui.html();
    assert!(html.contains("This preview was imported"), "{html}");
    assert!(
        html.contains(&format!("href=\"{}\"", run_path(2))),
        "{html}"
    );
    assert!(!html.contains("Import into Horae"), "{html}");
    assert!(!html.contains("newer import"), "{html}");
}

#[tokio::test]
async fn a_preview_that_cannot_be_committed_says_why() {
    for (availability, reason) in [
        (models::CommitAvailability::NewerImport, "newer import"),
        (
            models::CommitAvailability::PreviousAccount,
            "previous Harvest account",
        ),
        (
            models::CommitAvailability::MissingUpload,
            "no longer retained",
        ),
        (
            models::CommitAvailability::Unknown,
            "Commit availability is unknown",
        ),
    ] {
        let probe = Probe::default();
        let mut preview = finished(
            1,
            ImportMode::DryRun,
            ImportReport::new(SourceKind::Csv, ImportMode::DryRun),
        );
        preview.commit_availability = availability;
        let ui = open_run(&probe, preview);
        let html = ui.html();
        assert!(html.contains(reason), "{html}");
        assert!(!html.contains("Import into Horae"), "{html}");
    }
}

#[tokio::test]
async fn retry_errors_are_visible_and_success_follows_the_run_again() {
    let probe = Probe::default();
    let mut ui = open_run(&probe, job(1, "failed"));
    let action = probe.action();
    ui.click("retry-run");
    ui.click("retry-run");
    assert_eq!(
        probe.retries.borrow().len(),
        1,
        "pending retry must not be submitted twice"
    );
    let reread = probe.request();
    action
        .send(Err(ServerFnError::new("Retry rejected")))
        .unwrap();
    ui.settle();
    reread.send(Ok(Some(job(1, "failed")))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not continue the preview"));
    let action = probe.action();
    let response = probe.request();
    ui.click("retry-run");
    action.send(Ok(job(1, "queued"))).unwrap();
    ui.settle();
    response.send(Ok(Some(job(1, "queued")))).unwrap();
    ui.settle();
    assert!(!ui.html().contains("Could not continue the preview"));
    let response = probe.request();
    response.send(Ok(Some(job(1, "running")))).unwrap();
    ui.settle();
    assert_eq!(
        &*probe.retries.borrow(),
        &[Uuid::from_u128(1), Uuid::from_u128(1)]
    );
    assert_eq!(probe.requests.borrow().len(), 3);
}

#[tokio::test]
async fn cancellation_errors_are_visible_and_acceptance_does_not_acknowledge_completion() {
    let probe = Probe::default();
    let mut ui = open_run(&probe, job(1, "running"));
    let action = probe.action();
    ui.click("cancel-import");
    let reread = probe.request();
    action
        .send(Err(ServerFnError::new("Cancellation rejected")))
        .unwrap();
    ui.settle();
    reread.send(Ok(Some(job(1, "running")))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not cancel the preview"));
    let action = probe.action();
    ui.click("cancel-import");
    let mut accepted = job(1, "running");
    accepted.phase = Some("cancelling".into());
    let reread = probe.request();
    action.send(Ok(accepted.clone())).unwrap();
    ui.settle();
    reread.send(Ok(Some(accepted))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Stopping after the current batch"));
    assert!(ui.html().contains("Cancelling…"));
    assert!(!ui.html().contains("Nothing in Horae changed"));
    assert_eq!(probe.cancellations.borrow().len(), 2);
}

#[tokio::test]
async fn blocked_retry_keeps_partial_reports_and_explains_the_missing_prerequisite() {
    for (availability, reason) in [
        (
            models::RetryAvailability::PreviousAccount,
            "previous Harvest account",
        ),
        (
            models::RetryAvailability::MissingUpload,
            "Upload is no longer retained",
        ),
        (
            models::RetryAvailability::Unknown,
            "Retry availability is unknown",
        ),
    ] {
        let probe = Probe::default();
        let mut snapshot = job(1, "failed");
        snapshot.mode = Some(ImportMode::Commit);
        snapshot.retry_availability = availability;
        snapshot.report = Some(
            serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::Commit)).unwrap(),
        );
        let ui = open_run(&probe, snapshot);
        let html = ui.html();
        assert!(html.contains(reason), "{html}");
        assert!(!html.contains("data-testid=\"retry-run\""), "{html}");
        assert!(html.contains("What it covered before it stopped"), "{html}");
        assert!(!html.contains("Go to projects"), "{html}");
        assert!(probe.retries.borrow().is_empty());
    }
}

#[tokio::test]
async fn failed_and_cancelled_runs_show_partial_outcomes_without_success_actions() {
    for state in ["failed", "cancelled"] {
        for mode in [ImportMode::Commit, ImportMode::DryRun] {
            let probe = Probe::default();
            let mut snapshot = job(1, state);
            snapshot.kind = "harvest_api_import".into();
            snapshot.source = Some(SourceKind::HarvestApi);
            snapshot.mode = Some(mode);
            let mut report = ImportReport::new(SourceKind::HarvestApi, mode);
            report.summary.time_entries.created = 499;
            snapshot.report = Some(serde_json::to_value(report).unwrap());
            snapshot.last_error = (state == "failed").then(|| "Source connection lost".to_string());
            let ui = open_run(&probe, snapshot);
            let html = ui.html();
            assert!(html.contains("What it covered before it stopped"), "{html}");
            assert!(html.contains("499"), "{html}");
            assert!(
                html.contains(if state == "failed" {
                    "Source connection lost"
                } else {
                    "Cancelled"
                }),
                "{html}"
            );
            if mode == ImportMode::Commit {
                assert!(html.contains("499 records were written and stay"), "{html}");
            }
            assert!(!html.contains("Go to projects"), "{html}");
            assert!(!html.contains("Import into Horae"), "{html}");
            assert!(!html.contains("Sync changes"), "{html}");
        }
    }
}

#[tokio::test]
async fn result_copy_distinguishes_preview_zero_skipped_and_mixed_outcomes() {
    for mode in [ImportMode::DryRun, ImportMode::Commit] {
        for (created, updated, skipped, errored) in [(0, 0, 0, 0), (0, 0, 7, 0), (2, 3, 4, 1)] {
            let probe = Probe::default();
            let mut report = errored_report(mode, errored);
            report.summary.time_entries.created = created;
            report.summary.time_entries.updated = updated;
            report.summary.time_entries.skipped = skipped;
            let ui = open_run(&probe, finished(1, mode, report));
            let html = ui.html();
            let ready = created + updated;
            assert!(
                html.contains(&format!(
                    "{ready} records {}",
                    if mode == ImportMode::DryRun {
                        "are ready"
                    } else {
                        "are in Horae"
                    }
                )),
                "{html}"
            );
            // Records already in Horae are neither ready nor skipped.
            assert_eq!(
                html.contains("will be skipped"),
                mode == ImportMode::DryRun && errored > 0,
                "{html}"
            );
            assert_eq!(
                html.contains("were skipped"),
                mode == ImportMode::Commit && errored > 0,
                "{html}"
            );
            assert!(html.contains("Already in Horae"), "{html}");
            if mode == ImportMode::DryRun {
                assert!(html.contains("Nothing imported yet"), "{html}");
            }
        }
    }
}

#[tokio::test]
async fn errors_are_grouped_on_the_server_with_a_job_scoped_download() {
    for state in ["succeeded", "failed"] {
        let probe = Probe::default();
        *probe.summary.borrow_mut() = models::ErrorSummary {
            groups: vec![models::ErrorGroup {
                entity: EntityType::TimeEntry,
                kind: None,
                entry_kind: None,
                reason: "invalid date".into(),
                count: 2,
                samples: ["record 1", "record 2"]
                    .map(|location| models::ErrorSample {
                        location: location.into(),
                        reason: "invalid date".into(),
                    })
                    .into(),
            }],
            omitted_errors: 4,
            ..models::ErrorSummary::default()
        };
        let mut report = errored_report(ImportMode::Commit, 2);
        report.archive_errors(1).unwrap();
        let mut snapshot = finished(17, ImportMode::Commit, report);
        snapshot.status = state.into();
        let ui = open_run(&probe, snapshot);
        let html = ui.html();
        assert_eq!(&*probe.summaries.borrow(), &[Uuid::from_u128(17)]);
        assert!(html.contains("invalid date"), "{html}");
        assert!(
            html.contains(">record 1<") && html.contains(">record 2<"),
            "{html}"
        );
        assert!(html.contains("4 more issues"), "{html}");
        assert!(
            html.contains(&format!(
                "/api/import/harvest/jobs/{}/errors",
                Uuid::from_u128(17)
            )),
            "{html}"
        );
        if state == "succeeded" {
            assert!(html.contains("2 were skipped"), "{html}");
        } else {
            assert!(html.contains("What it covered before it stopped"), "{html}");
        }
    }
}

#[tokio::test]
async fn a_run_without_errors_never_asks_for_a_summary() {
    let probe = Probe::default();
    let report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
    let _ui = open_run(&probe, finished(1, ImportMode::Commit, report));
    assert!(probe.summaries.borrow().is_empty());
}

fn missing(
    email: Option<&str>,
    name: &str,
    kind: PersonErrorKind,
    entries: u64,
) -> models::MissingPerson {
    models::MissingPerson {
        email: email.map(Into::into),
        name: Some(name.into()),
        kind,
        entries,
    }
}

fn run_with_missing_people(probe: &Probe) -> Ui {
    *probe.summary.borrow_mut() = models::ErrorSummary {
        people: vec![
            missing(
                Some("dana@x.com"),
                "Dana",
                PersonErrorKind::UnmatchedEmail,
                3,
            ),
            missing(None, "No Mail", PersonErrorKind::MissingEmail, 2),
            missing(None, "Sam Name", PersonErrorKind::UnmatchedName, 1),
        ],
        ..models::ErrorSummary::default()
    };
    open_run(
        probe,
        finished(1, ImportMode::DryRun, errored_report(ImportMode::DryRun, 6)),
    )
}

#[tokio::test]
async fn missing_people_say_what_to_do_and_can_all_be_added_deactivated() {
    let probe = Probe::default();
    let mut ui = run_with_missing_people(&probe);
    let html = ui.html();
    assert!(html.contains("People not imported"), "{html}");
    assert!(html.contains("No Horae account has this email"), "{html}");
    assert!(
        html.contains("Add an email to this person in Harvest"),
        "{html}"
    );
    assert!(html.contains("Add all as deactivated (1)"), "{html}");
    assert!(
        !html.contains("data-testid=\"add-person-No Mail\""),
        "{html}"
    );

    ui.click("add-all-people");
    assert_eq!(
        &*probe.additions.borrow(),
        &[(
            vec![models::user::NewPerson {
                email: "dana@x.com".into(),
                name: "Dana".into(),
            }],
            models::user::Activation::Deactivated,
        )]
    );
    let html = ui.html();
    assert!(html.contains("Added 1 people"), "{html}");
    assert!(html.contains("Start a new preview"), "{html}");
    assert!(!html.contains("Add all as deactivated"), "{html}");
}

#[tokio::test]
async fn adding_everyone_reports_who_already_had_an_account() {
    let probe = Probe::default();
    // Added meanwhile from another tab, or before a reload.
    probe.already_present.borrow_mut().push("dana@x.com".into());
    *probe.summary.borrow_mut() = models::ErrorSummary {
        people: vec![
            missing(
                Some("dana@x.com"),
                "Dana",
                PersonErrorKind::UnmatchedEmail,
                3,
            ),
            missing(Some("lee@x.com"), "Lee", PersonErrorKind::UnmatchedEmail, 1),
        ],
        ..models::ErrorSummary::default()
    };
    let mut ui = open_run(
        &probe,
        finished(1, ImportMode::DryRun, errored_report(ImportMode::DryRun, 4)),
    );
    assert!(ui.html().contains("Add all as deactivated (2)"));
    ui.click("add-all-people");
    let html = ui.html();
    assert!(html.contains("Added 1 people."), "{html}");
    assert!(html.contains("1 already had an account"), "{html}");
    assert!(html.contains("Already in Horae"), "{html}");
    assert!(!html.contains("Add all as deactivated"), "{html}");
    assert_eq!(probe.additions.borrow().len(), 1);
}

#[tokio::test]
async fn a_person_is_added_in_place_and_can_be_made_active() {
    let probe = Probe::default();
    let mut ui = run_with_missing_people(&probe);
    // A CSV row known only by name: the administrator supplies the email.
    ui.click("add-person-Sam Name");
    ui.type_into("person-email-Sam Name", "sam@x.com");
    ui.check("person-active-Sam Name");
    ui.click("confirm-person-Sam Name");
    assert_eq!(
        &*probe.additions.borrow(),
        &[(
            vec![models::user::NewPerson {
                email: "sam@x.com".into(),
                name: "Sam Name".into(),
            }],
            models::user::Activation::Active,
        )]
    );
    assert!(ui.html().contains("Added"), "{}", ui.html());
}

#[tokio::test]
async fn a_refused_addition_stays_on_its_row() {
    let probe = Probe::default();
    *probe.addition_error.borrow_mut() =
        Some("A user with the email dana@x.com already exists".into());
    let mut ui = run_with_missing_people(&probe);
    ui.click("add-person-dana@x.com");
    ui.click("confirm-person-dana@x.com");
    let html = ui.html();
    assert!(html.contains("already exists"), "{html}");
    assert!(
        html.contains("data-testid=\"confirm-person-dana@x.com\""),
        "{html}"
    );
}

#[tokio::test]
async fn a_csv_preview_opens_its_run() {
    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-csv");
    ui.choose_file("import.csv");
    let _pending_status = probe.request();
    ui.click("preview-csv");
    ui.click("preview-csv");
    assert_eq!(&*probe.requests.borrow(), &[Uuid::from_u128(11)]);
    assert_eq!(
        &*probe.csv_submissions.borrow(),
        &[(ImportMode::DryRun, "import.csv".into())]
    );
}

#[tokio::test]
async fn an_api_preview_is_bound_to_the_current_account_and_followed_in_place() {
    let probe = Probe::default();
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        account_generation: 2,
        ..ConnectionStatus::default()
    };
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    assert!(ui.html().contains("See what Harvest would bring in"));
    ui.click("preview-api");
    assert_eq!(
        &*probe.api_submissions.borrow(),
        &[(ImportMode::DryRun, SyncScope::Full, Some(2))]
    );
    // The Harvest page reads its runs again instead of leaving for the run.
    assert!(probe.requests.borrow().is_empty());
    assert_eq!(probe.history_cursors.borrow().len(), 2);
}

#[tokio::test]
async fn a_refused_connection_turns_the_connection_line_red() {
    let probe = Probe::default();
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    let mut run = job(1, "failed");
    run.kind = "harvest_api_import".into();
    run.source = Some(SourceKind::HarvestApi);
    run.error_kind = Some(models::JobErrorKind::ReconnectRequired);
    probe.history.borrow_mut().push(run);
    let ui = Ui::at(&probe, "/admin/importers/harvest");
    let html = ui.html();
    assert!(
        html.contains("badge-danger badge-sm\" data-testid=\"connection-status\">Access refused"),
        "{html}"
    );
    assert!(html.contains("Reconnect Harvest"), "{html}");
}

#[tokio::test]
async fn a_run_stopped_on_a_401_can_be_continued_after_reconnecting() {
    let probe = Probe::default();
    // A 401 leaves the credentials stored, so the page still reads connected,
    // as it does again after reconnecting to the same account.
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    let mut run = job(1, "failed");
    run.kind = "harvest_api_import".into();
    run.source = Some(SourceKind::HarvestApi);
    run.error_kind = Some(models::JobErrorKind::ReconnectRequired);
    probe.history.borrow_mut().push(run);
    let mut ui = Ui::at(&probe, "/admin/importers/harvest");
    let html = ui.html();
    assert!(html.contains("data-testid=\"reconnect\""), "{html}");
    assert!(html.contains("Continue preview"), "{html}");
    let _accepted = probe.action();
    ui.click("retry-run");
    assert_eq!(&*probe.retries.borrow(), &[Uuid::from_u128(1)]);
}

#[tokio::test]
async fn an_incremental_import_reads_as_a_sync() {
    let probe = Probe::default();
    let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit);
    report.summary.time_entries.created = 214;
    report.summary.time_entries.updated = 38;
    let mut run = finished(1, ImportMode::Commit, report);
    run.kind = "harvest_api_import".into();
    run.source = Some(SourceKind::HarvestApi);
    run.sync = Some(SyncScope::Incremental);
    let html = open_run(&probe, run).html();
    assert!(html.contains("Synced"), "{html}");
    assert!(
        html.contains("214 new and 38 updated entries from Harvest"),
        "{html}"
    );
}

#[tokio::test]
async fn the_harvest_page_shows_each_step_from_the_latest_api_run() {
    let connected = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    let api = |mut run: models::JobStatus| {
        run.kind = "harvest_api_import".into();
        run.source = Some(SourceKind::HarvestApi);
        run
    };
    let mut preview = api(finished(
        1,
        ImportMode::DryRun,
        ImportReport::new(SourceKind::HarvestApi, ImportMode::DryRun),
    ));
    preview.report = Some(
        serde_json::to_value({
            let mut report = errored_report(ImportMode::DryRun, 7);
            report.summary.time_entries.created = 21_293;
            report
        })
        .unwrap(),
    );
    let mut importing = api(job(2, "running"));
    importing.mode = Some(ImportMode::Commit);
    importing.total_count = Some(21_529);
    let mut imported = api(finished(
        3,
        ImportMode::Commit,
        ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit),
    ));
    imported.status = "succeeded".into();
    for (run, text, steps) in [
        (preview, "21,293 records are ready", ["done", "current"]),
        (importing, "Writing to Horae", ["done", "current"]),
        (imported, "Harvest is imported", ["done", "done"]),
    ] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = connected.clone();
        probe.history.borrow_mut().push(run);
        let ui = Ui::at(&probe, "/admin/importers/harvest");
        let html = ui.html();
        assert!(html.contains(text), "{html}");
        // The state is told to assistive tech too, not by the dot's colour alone.
        for (name, state) in ["Preview", "Import"].into_iter().zip(steps) {
            let expected = match state {
                "current" => "data-state=\"current\" aria-current=\"step\"".to_owned(),
                _ => format!("data-state=\"done\" aria-label=\"{name}, done\""),
            };
            assert!(html.contains(&expected), "{name} {state}: {html}");
        }
    }
}

#[tokio::test]
async fn a_committed_api_run_can_resync_incrementally() {
    let probe = Probe::default();
    probe.connection.borrow_mut().account_generation = 4;
    let mut run = finished(
        1,
        ImportMode::Commit,
        ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit),
    );
    run.kind = "harvest_api_import".into();
    run.source = Some(SourceKind::HarvestApi);
    let mut ui = open_run(&probe, run);
    let _pending_status = probe.request();
    ui.click("resync");
    assert_eq!(
        &*probe.api_submissions.borrow(),
        &[(ImportMode::Commit, SyncScope::Incremental, Some(4))]
    );
    assert_eq!(probe.requests.borrow().len(), 2);
}

#[tokio::test]
async fn history_labels_mode_source_and_date() {
    for (kind, mode, source, label) in [
        (
            "harvest_api_import",
            Some(ImportMode::DryRun),
            "Harvest API",
            "Preview",
        ),
        (
            "harvest_csv_import",
            Some(ImportMode::Commit),
            "CSV file",
            "Import",
        ),
        ("future_import", None, "Unknown source", "Mode unavailable"),
    ] {
        let probe = Probe::default();
        let mut snapshot = job(1, "succeeded");
        snapshot.kind = kind.into();
        snapshot.mode = mode;
        snapshot.created_at = chrono::DateTime::parse_from_rfc3339("2026-09-15T10:30:00Z")
            .unwrap()
            .to_utc();
        probe.history.borrow_mut().push(snapshot.clone());
        let ui = Ui::at(&probe, "/admin/importers/harvest");
        let html = ui.html();
        assert!(html.contains(source) && html.contains(label), "{html}");
        assert!(html.contains("15 Sep 2026, 10:30 UTC"), "{html}");
        assert!(
            html.contains(&format!("href=\"{}\"", run_path(1))),
            "{html}"
        );
    }
}

#[tokio::test]
async fn the_overview_shows_the_latest_five_runs() {
    let probe = Probe::default();
    probe
        .history
        .borrow_mut()
        .extend((1..=8).map(|id| job(id, "succeeded")));
    let ui = Ui::at(&probe, "/admin/importers/harvest");
    let html = ui.html();
    assert!(html.contains(&run_path(5)), "{html}");
    assert!(!html.contains(&run_path(6)), "{html}");
    assert!(
        html.contains("href=\"/admin/importers/harvest/runs?\""),
        "{html}"
    );
}

#[tokio::test]
async fn run_history_pages_back_and_returns_to_the_newest_runs() {
    let probe = Probe::default();
    probe
        .history
        .borrow_mut()
        .extend((1..=24).map(|id| job(id, "succeeded")));
    let mut ui = Ui::at(&probe, "/admin/importers/harvest/runs");
    assert!(!ui.html().contains(&run_path(24)));
    ui.click("older-history");
    assert!(ui.html().contains(&run_path(24)), "{}", ui.html());
    assert!(!ui.html().contains(&run_path(1)));
    ui.click("newer-history");
    assert!(ui.html().contains(&run_path(1)));
    assert_eq!(
        &*probe.history_cursors.borrow(),
        &[None, Some(Uuid::from_u128(20)), None]
    );
}

#[tokio::test]
async fn a_history_failure_can_be_tried_again() {
    let probe = Probe::default();
    *probe.history_error.borrow_mut() = Some("History unavailable".into());
    let mut ui = Ui::at(&probe, "/admin/importers/harvest/runs");
    assert!(ui.html().contains("Could not load runs"));
    *probe.history_error.borrow_mut() = None;
    probe.history.borrow_mut().push(job(1, "running"));
    ui.click("refresh-history");
    assert!(!ui.html().contains("Could not load runs"));
    assert!(ui.html().contains(&run_path(1)));
}

#[test]
fn account_change_confirmation_can_cancel_without_mutation_in_both_binding_states() {
    for connected in [false, true] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            connected,
            account_id: Some("account-A".into()),
            account_generation: 2,
            connection_revision: 4,
            ..ConnectionStatus::default()
        };
        let mut ui = Ui::start(&probe);
        ui.click("choose-api");
        if !connected {
            assert!(ui.html().contains("Reconnect Harvest"));
        }
        ui.click("change-account");
        let html = ui.html();
        assert!(
            html.contains("aria-labelledby=\"harvest-change-account-title\""),
            "{html}"
        );
        assert!(html.contains("account-A"));
        assert!(html.contains("Business data and retained reports are kept"));
        ui.click("cancel-change-account");
        assert!(!ui.html().contains("Change account and connect"));
        assert!(probe.changes.borrow().is_empty());
        ui.click("change-account");
        ui.cancel_dialog();
        assert!(!ui.html().contains("Change account and connect"));
        assert!(probe.changes.borrow().is_empty());
    }
}

#[test]
fn account_change_explains_every_blocker() {
    for (provenance, active, reason) in [(true, 0, "migration"), (false, 1, "cancel")] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            account_id: Some("A".into()),
            has_provenance: provenance,
            active_imports: active,
            ..ConnectionStatus::default()
        };
        let mut ui = Ui::start(&probe);
        ui.click("choose-api");
        let html = ui.html();
        assert!(html.contains(reason), "{html}");
        // Still focusable, so the arrow keys reach it and its reason is read.
        assert!(
            html.contains("data-testid=\"change-account\" aria-disabled=true>"),
            "{html}"
        );
        ui.click("change-account");
        assert!(!ui.html().contains("Change account and connect"));
        assert!(probe.changes.borrow().is_empty());
    }
}

#[tokio::test]
async fn disconnect_waits_for_a_run_started_from_the_page() {
    let probe = Probe::default();
    // Read once at load, before any run: no active imports yet.
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    let mut ui = Ui::at(&probe, "/admin/importers/harvest");
    assert!(!ui.html().contains("before disconnecting"), "{}", ui.html());
    ui.click("preview-api");
    let html = ui.html();
    assert!(
        html.contains("data-testid=\"disconnect-harvest\" aria-disabled=true>"),
        "{html}"
    );
    assert!(html.contains("before disconnecting"), "{html}");
    ui.click("disconnect-harvest");
    assert!(
        !ui.html()
            .contains("Clients, projects and time already imported stay"),
        "{}",
        ui.html()
    );
}

#[test]
fn disconnect_waits_for_queued_and_running_imports() {
    for active in [0, 1] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            connected: true,
            account_id: Some("A".into()),
            active_imports: active,
            ..ConnectionStatus::default()
        };
        let mut ui = Ui::start(&probe);
        ui.click("choose-api");
        assert_eq!(
            ui.html().contains("before disconnecting"),
            active > 0,
            "{}",
            ui.html()
        );
        ui.click("disconnect-harvest");
        assert_eq!(
            ui.html()
                .contains("Clients, projects and time already imported stay"),
            active == 0,
            "{}",
            ui.html()
        );
    }
}

#[test]
fn account_change_is_busy_then_reports_recoverable_authorization_failure() {
    let probe = Probe::default();
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        account_generation: 2,
        connection_revision: 4,
        ..ConnectionStatus::default()
    };
    let (send, receive) = oneshot::channel();
    probe.change_responses.borrow_mut().push_back(receive);
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("change-account");
    ui.click("confirm-change-account");
    ui.click("confirm-change-account");
    assert!(ui.html().contains("Changing account…"));
    ui.cancel_dialog();
    assert!(ui.html().contains("Changing account…"));
    assert_eq!(probe.changes.borrow().as_slice(), &[("A".into(), 2, 4)]);
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        account_generation: 3,
        connection_revision: 5,
        ..ConnectionStatus::default()
    };
    send.send(Ok(())).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(
        html.contains("Account released. Use Connect Harvest"),
        "{html}"
    );
    assert!(!html.contains("Reconnect Harvest"));
}

#[test]
fn stale_confirmation_keeps_the_dialog_and_shows_the_error() {
    let probe = Probe::default();
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    let (send, receive) = oneshot::channel();
    probe.change_responses.borrow_mut().push_back(receive);
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("change-account");
    ui.click("confirm-change-account");
    send.send(Err(ServerFnError::new("Reload the importer")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Reload the importer"));
    assert!(ui.html().contains("Change account and connect"));
}

mod server_fns {
    use super::*;

    pub async fn harvest_client_settings() -> Result<ClientSettings, ServerFnError> {
        Ok(consume_context::<Probe>().client.borrow().clone())
    }

    /// The server refuses an unconfirmed change that would delete tokens.
    fn unconfirmed() -> ServerFnError {
        ServerFnError::ServerError {
            message: DISCONNECT_UNCONFIRMED.into(),
            code: axum::http::StatusCode::CONFLICT.as_u16(),
            details: None,
        }
    }

    pub async fn save_harvest_client(
        client_id: String,
        client_secret: Option<String>,
        redirect_url: String,
        confirm_disconnect: bool,
    ) -> Result<ClientSettings, ServerFnError> {
        let probe = consume_context::<Probe>();
        let mut client = probe.client.borrow_mut();
        let disconnects = client.client_id.as_ref().is_some_and(|id| *id != client_id)
            && probe.connection.borrow().connected;
        if disconnects && !confirm_disconnect {
            return Err(unconfirmed());
        }
        if disconnects {
            probe.connection.borrow_mut().connected = false;
        }
        client.source = ClientSource::Stored;
        client.client_id = Some(client_id.clone());
        client.secret_set = true;
        client.secret_readable = true;
        client.redirect_url = Some(redirect_url.clone());
        probe.client_saves.borrow_mut().push((
            client_id,
            client_secret,
            redirect_url,
            confirm_disconnect,
        ));
        Ok(client.clone())
    }

    pub async fn remove_harvest_client(
        confirm_disconnect: bool,
    ) -> Result<ClientSettings, ServerFnError> {
        let probe = consume_context::<Probe>();
        if probe.connection.borrow().connected && !confirm_disconnect {
            return Err(unconfirmed());
        }
        probe.connection.borrow_mut().connected = false;
        *probe.client_removes.borrow_mut() += 1;
        let mut client = probe.client.borrow_mut();
        *client = ClientSettings {
            encryption_key_set: client.encryption_key_set,
            ..ClientSettings::default()
        };
        Ok(client.clone())
    }

    pub async fn harvest_connection_status() -> Result<ConnectionStatus, ServerFnError> {
        let probe = consume_context::<Probe>();
        let response = probe.connection_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        Ok(probe.connection.borrow().clone())
    }

    pub async fn list_harvest_import_jobs(
        before: Option<Uuid>,
        limit: Option<i64>,
    ) -> Result<Vec<models::JobStatus>, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.history_cursors.borrow_mut().push(before);
        if let Some(error) = probe.history_error.borrow().as_ref() {
            return Err(ServerFnError::new(error));
        }
        let history = probe.history.borrow();
        let start = before.map_or(0, |id| {
            history
                .iter()
                .position(|job| job.id == id)
                .expect("unknown test cursor")
                + 1
        });
        Ok(history
            .iter()
            .skip(start)
            .take(limit.unwrap_or(20).clamp(1, 100) as usize)
            .cloned()
            .collect())
    }

    pub async fn get_harvest_import_job(id: Uuid) -> JobResponse {
        let probe = consume_context::<Probe>();
        probe.requests.borrow_mut().push(id);
        let response = probe
            .responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected status request");
        response
            .await
            .expect("test must resolve the status request")
    }

    pub async fn start_harvest_api_import(
        mode: ImportMode,
        scope: SyncScope,
        generation: Option<i64>,
    ) -> Result<models::JobStatus, ServerFnError> {
        let probe = consume_context::<Probe>();
        let mut submitted = probe.api_submissions.borrow_mut();
        submitted.push((mode, scope, generation));
        let mut snapshot = job(10 + submitted.len() as u128, "queued");
        snapshot.kind = "harvest_api_import".into();
        // As on the server, the new run is the newest in the history.
        probe.history.borrow_mut().insert(0, snapshot.clone());
        Ok(snapshot)
    }

    pub async fn harvest_change_account(
        account: String,
        generation: i64,
        revision: i64,
    ) -> Result<(), ServerFnError> {
        let probe = consume_context::<Probe>();
        probe
            .changes
            .borrow_mut()
            .push((account, generation, revision));
        let response = probe
            .change_responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected account change");
        response.await.unwrap()
    }

    pub struct CsvUpload(FileData);

    impl From<FileData> for CsvUpload {
        fn from(file: FileData) -> Self {
            Self(file)
        }
    }

    pub async fn start_harvest_csv_import(
        mode: ImportMode,
        file: CsvUpload,
    ) -> Result<models::JobStatus, ServerFnError> {
        let probe = consume_context::<Probe>();
        let mut submitted = probe.csv_submissions.borrow_mut();
        submitted.push((mode, file.0.name()));
        Ok(job(10 + submitted.len() as u128, "queued"))
    }

    fn next_action(probe: &Probe, what: &str) -> oneshot::Receiver<ActionResponse> {
        probe
            .action_responses
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected {what}"))
    }

    pub async fn cancel_harvest_import_job(id: Uuid) -> ActionResponse {
        let probe = consume_context::<Probe>();
        probe.cancellations.borrow_mut().push(id);
        next_action(&probe, "cancellation").await.unwrap()
    }

    pub async fn retry_harvest_import_job(id: Uuid) -> ActionResponse {
        let probe = consume_context::<Probe>();
        probe.retries.borrow_mut().push(id);
        next_action(&probe, "retry").await.unwrap()
    }

    pub async fn commit_harvest_import_preview(id: Uuid) -> ActionResponse {
        let probe = consume_context::<Probe>();
        probe.commits.borrow_mut().push(id);
        next_action(&probe, "commit").await.unwrap()
    }

    pub async fn harvest_import_error_summary(
        id: Uuid,
    ) -> Result<models::ErrorSummary, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.summaries.borrow_mut().push(id);
        Ok(probe.summary.borrow().clone())
    }

    pub async fn add_harvest_people(
        people: Vec<models::user::NewPerson>,
        activation: models::user::Activation,
    ) -> Result<models::user::AddedPeople, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe
            .additions
            .borrow_mut()
            .push((people.clone(), activation));
        if let Some(error) = probe.addition_error.borrow().as_ref() {
            return Err(ServerFnError::new(error));
        }
        let present = probe.already_present.borrow();
        let (present, added): (Vec<_>, Vec<_>) = people
            .into_iter()
            .partition(|person| present.contains(&person.email));
        Ok(models::user::AddedPeople {
            added: added
                .into_iter()
                .map(|person| models::user::User {
                    email: person.email,
                })
                .collect(),
            present: present.into_iter().map(|person| person.email).collect(),
        })
    }

    pub async fn harvest_connect_start() -> Result<String, ServerFnError> {
        let probe = consume_context::<Probe>();
        *probe.connects.borrow_mut() += 1;
        let response = probe.connect_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        Err(ServerFnError::new("authorization unavailable"))
    }

    pub async fn harvest_disconnect() -> Result<(), ServerFnError> {
        let probe = consume_context::<Probe>();
        *probe.disconnects.borrow_mut() += 1;
        let response = probe
            .disconnect_response
            .borrow_mut()
            .take()
            .expect("unexpected disconnect");
        response.await.unwrap()
    }
}

#[test]
fn connection_picker_distinguishes_loading_unavailable_and_unconfigured() {
    let probe = Probe::default();
    let (send, receive) = oneshot::channel();
    *probe.connection_response.borrow_mut() = Some(receive);
    let mut ui = Ui::start(&probe);
    assert!(ui.html().contains("Checking connection"));
    assert!(!ui.html().contains("Connected"));
    send.send(Err(ServerFnError::new("check failed"))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Status unavailable"));
    let (send, receive) = oneshot::channel();
    *probe.connection_response.borrow_mut() = Some(receive);
    ui.click("choose-api");
    send.send(Err(ServerFnError::new("check failed"))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not check Harvest connection"));
    ui.click("recheck-connection");
    assert!(ui.html().contains("Set up the Harvest OAuth app"));
    assert!(!ui.html().contains("Connect Harvest</button>"));
}

#[test]
fn connection_management_groups_actions_and_never_promises_automatic_repair() {
    for (connected, expired, label) in [
        (true, false, "Connected"),
        (true, true, "Token expired"),
        (false, false, "Disconnected"),
    ] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            connected,
            token_expired: expired,
            account_id: Some("account-A".into()),
            ..ConnectionStatus::default()
        };
        let mut ui = Ui::start(&probe);
        ui.click("choose-api");
        assert!(ui.html().contains(label));
        assert!(ui.html().contains("account-A"));
        let html = ui.html();
        // One Connection menu holds every connection action.
        assert!(
            html.contains("aria-controls=\"harvest-connection\""),
            "{html}"
        );
        for item in [
            "Check connection",
            "OAuth app credentials",
            "Change account",
        ] {
            assert!(html.contains(item), "{item}: {html}");
        }
        assert_eq!(
            html.contains("data-testid=\"disconnect-harvest\""),
            connected
        );
        assert!(!html.contains("within a minute") && !html.contains("refreshes automatically"));
        assert!(probe.changes.borrow().is_empty());
    }
}

#[test]
fn connection_actions_prevent_duplicate_submissions_and_keep_errors_local() {
    for connected in [false, true] {
        let probe = Probe::default();
        *probe.connection.borrow_mut() = ConnectionStatus {
            configured: true,
            connected,
            account_id: Some("account-A".into()),
            ..ConnectionStatus::default()
        };
        let mut ui = Ui::start(&probe);
        ui.click("choose-api");
        if connected {
            let (send, receive) = oneshot::channel();
            *probe.disconnect_response.borrow_mut() = Some(receive);
            ui.click("disconnect-harvest");
            // Disconnect asks first and states what stays.
            assert!(
                ui.html()
                    .contains("Clients, projects and time already imported stay")
            );
            assert_eq!(*probe.disconnects.borrow(), 0);
            ui.click("confirm-disconnect");
            ui.click("confirm-disconnect");
            assert_eq!(*probe.disconnects.borrow(), 1);
            send.send(Err(ServerFnError::new("disconnect unavailable")))
                .unwrap();
        } else {
            let (send, receive) = oneshot::channel();
            *probe.connect_response.borrow_mut() = Some(receive);
            ui.click("connect-harvest");
            ui.click("connect-harvest");
            assert_eq!(*probe.connects.borrow(), 1);
            send.send(Err(ServerFnError::new("authorization unavailable")))
                .unwrap();
        }
        ui.settle();
        let html = ui.html();
        assert!(
            html.contains(if connected {
                "Could not disconnect Harvest"
            } else {
                "Could not start Harvest connection"
            }),
            "{html}"
        );
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("account-A"));
    }
}

#[test]
fn configured_unbound_connection_explains_read_only_access() {
    let probe = Probe::default();
    probe.connection.borrow_mut().configured = true;
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    assert!(ui.html().contains("Not connected"));
    assert!(ui.html().contains("Connect Harvest"));
    assert!(
        ui.html()
            .contains("read-only, Horae never writes to Harvest")
    );
    assert!(!ui.html().contains("data-testid=\"change-account\""));
}

#[test]
fn pending_connect_is_single_submission_and_failure_keeps_a_recovery_action() {
    let probe = Probe::default();
    probe.connection.borrow_mut().configured = true;
    let (send, receive) = oneshot::channel();
    *probe.connect_response.borrow_mut() = Some(receive);
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("connect-harvest");
    ui.click("connect-harvest");
    assert_eq!(*probe.connects.borrow(), 1);
    assert!(ui.html().contains("Connecting…"));
    send.send(Err(ServerFnError::new("authorization unavailable")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not start Harvest connection"));
    assert!(ui.html().contains("Connect Harvest"));
    assert!(ui.html().contains("authorization unavailable"));
}

fn key_only() -> ClientSettings {
    ClientSettings {
        encryption_key_set: true,
        ..ClientSettings::default()
    }
}

#[test]
fn unconfigured_importer_saves_an_oauth_app_and_never_shows_its_secret() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = key_only();
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    let html = ui.html();
    assert!(
        html.contains("Harvest OAuth app") && html.contains("Not configured"),
        "{html}"
    );
    assert!(html.contains("/auth/harvest/callback"), "{html}");
    assert!(!html.contains("Latest runs"));

    ui.type_into("harvest-client-id", " 1234.pt.app ");
    ui.type_into("harvest-client-secret", "very-secret");
    probe.connection.borrow_mut().configured = true;
    ui.click("save-harvest-client");

    assert_eq!(
        &*probe.client_saves.borrow(),
        &[(
            "1234.pt.app".to_owned(),
            Some("very-secret".to_owned()),
            "/auth/harvest/callback".to_owned(),
            false
        )]
    );
    let html = ui.html();
    assert!(
        html.contains("Configured") && html.contains("Replace secret"),
        "{html}"
    );
    assert!(!html.contains("very-secret"), "{html}");
    ui.click("harvest-oauth-back");
    assert!(ui.html().contains("Connect Harvest"), "{}", ui.html());
}

#[test]
fn a_blank_secret_keeps_the_stored_one() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Stored,
        client_id: Some("app".into()),
        secret_set: true,
        secret_readable: true,
        redirect_url: Some("https://horae.example/auth/harvest/callback".into()),
        ..key_only()
    };
    probe.connection.borrow_mut().configured = true;
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("harvest-oauth-app");
    ui.click("save-harvest-client");
    assert_eq!(
        &*probe.client_saves.borrow(),
        &[(
            "app".to_owned(),
            None,
            "https://horae.example/auth/harvest/callback".to_owned(),
            false
        )]
    );
}

#[test]
fn changes_that_disconnect_harvest_ask_first() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Stored,
        client_id: Some("first".into()),
        secret_set: true,
        secret_readable: true,
        redirect_url: Some("https://horae.example/auth/harvest/callback".into()),
        ..key_only()
    };
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("account-A".into()),
        ..ConnectionStatus::default()
    };
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("harvest-oauth-app");

    ui.type_into("harvest-client-id", "second");
    ui.click("replace-harvest-secret");
    ui.type_into("harvest-client-secret", "s2");
    ui.click("save-harvest-client");
    assert!(probe.client_saves.borrow().is_empty());
    assert!(ui.html().contains("disconnects Harvest"), "{}", ui.html());
    ui.click("confirm-harvest-client-change");
    assert_eq!(probe.client_saves.borrow().len(), 1);
    assert!(
        probe.client_saves.borrow()[0].3,
        "the save carries the confirmation"
    );

    // Harvest is reconnected elsewhere while this page still believes it is
    // disconnected: the server refuses, and the page asks before retrying.
    probe.connection.borrow_mut().connected = true;
    ui.click("remove-harvest-client");
    assert_eq!(*probe.client_removes.borrow(), 0);
    assert!(
        ui.html()
            .contains("Removing the OAuth app disconnects Harvest"),
        "{}",
        ui.html()
    );
    ui.click("confirm-harvest-client-change");
    assert_eq!(*probe.client_removes.borrow(), 1);
    assert!(ui.html().contains("Not configured"));
}

#[test]
fn rendered_page_never_contains_the_client_secret() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = key_only();
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    ui.type_into("harvest-client-id", "app");
    ui.type_into("harvest-client-secret", "rendered-secret");
    // The typed secret is in the form, so the checks below can fail.
    assert!(ui.html().contains("rendered-secret"), "{}", ui.html());
    probe.connection.borrow_mut().configured = true;
    ui.click("save-harvest-client");
    assert_eq!(
        probe.client_saves.borrow()[0].1.as_deref(),
        Some("rendered-secret")
    );
    assert!(!ui.html().contains("rendered-secret"), "{}", ui.html());

    // Reopening the view, and replacing the secret, start from an empty field.
    ui.click("harvest-oauth-back");
    ui.click("harvest-oauth-app");
    assert!(!ui.html().contains("rendered-secret"), "{}", ui.html());
    ui.click("replace-harvest-secret");
    assert!(!ui.html().contains("rendered-secret"), "{}", ui.html());
}

#[test]
fn a_stored_client_without_a_redirect_url_asks_to_be_saved_again() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Stored,
        client_id: Some("app".into()),
        secret_set: true,
        secret_readable: true,
        ..key_only()
    };
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    let html = ui.html();
    assert!(html.contains("Redirect URL needed"), "{html}");
    ui.click("save-harvest-client");
    assert_eq!(probe.client_saves.borrow()[0].2, "/auth/harvest/callback");
    assert!(!ui.html().contains("Redirect URL needed"));
}

#[test]
fn a_client_can_be_removed_without_the_encryption_key() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Stored,
        client_id: Some("app".into()),
        secret_set: true,
        redirect_url: Some("https://horae.example/auth/harvest/callback".into()),
        ..ClientSettings::default()
    };
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    let html = ui.html();
    assert!(
        !html.contains("data-testid=\"save-harvest-client\""),
        "{html}"
    );
    ui.click("remove-harvest-client");
    assert_eq!(*probe.client_removes.borrow(), 1);
}

#[test]
fn environment_client_and_missing_key_read_as_locked() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Environment,
        client_id: Some("env-app".into()),
        secret_set: true,
        secret_readable: true,
        redirect_url: Some("https://env.example/auth/harvest/callback".into()),
        redirect_from_environment: true,
        stored_ignored: true,
        ..key_only()
    };
    probe.connection.borrow_mut().configured = true;
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("harvest-oauth-app");
    let html = ui.html();
    assert!(
        html.contains("Configured by the environment") && html.contains("env-app"),
        "{html}"
    );
    assert!(html.contains("ignored while they are set"), "{html}");
    assert!(!html.contains("data-testid=\"save-harvest-client\""));
    assert!(!html.contains("data-testid=\"replace-harvest-secret\""));
    assert!(!html.contains("also needs HORAE_HARVEST_REDIRECT_URL"));

    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Environment,
        client_id: Some("env-app".into()),
        secret_set: true,
        secret_readable: true,
        ..key_only()
    };
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    assert!(ui.html().contains("also needs HORAE_HARVEST_REDIRECT_URL"));

    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    ui.click("setup-harvest-oauth");
    let html = ui.html();
    assert!(html.contains("HORAE_HARVEST_ENC_KEY"), "{html}");
    assert!(!html.contains("data-testid=\"save-harvest-client\""));
}

const STORED_REDIRECT: &str = "https://horae.example/auth/harvest/callback";

fn stored_client() -> ClientSettings {
    ClientSettings {
        source: ClientSource::Stored,
        client_id: Some("app".into()),
        secret_set: true,
        secret_readable: true,
        redirect_url: Some(STORED_REDIRECT.into()),
        ..key_only()
    }
}

/// The Harvest page as the OAuth callback leaves it after `query`.
fn after_callback(query: &str) -> (Probe, Ui) {
    let probe = Probe::default();
    probe.connection.borrow_mut().configured = true;
    *probe.client.borrow_mut() = stored_client();
    let ui = Ui::at(&probe, &format!("/admin/importers/harvest?{query}"));
    (probe, ui)
}

#[test]
fn each_connect_failure_says_what_happened_and_offers_its_fix() {
    for (query, sentence, action) in [
        (
            "connect_error=invalid_client",
            "Harvest didn&#39;t accept Horae&#39;s OAuth app credentials.",
            "fix-oauth-secret",
        ),
        (
            "connect_error=redirect_mismatch",
            "The redirect URL doesn&#39;t match the one registered in your Harvest app.",
            "harvest-developers",
        ),
        (
            "connect_error=code_expired",
            "The authorization expired before Horae could use it.",
            "banner-connect",
        ),
        (
            "connect_error=denied",
            "You cancelled the connection in Harvest.",
            "banner-connect",
        ),
        (
            "connect_error=state",
            "The connection couldn&#39;t be verified. Start it again from this page.",
            "banner-connect",
        ),
        (
            "connect_error=other&status=400",
            "Harvest refused the connection (HTTP 400).",
            "banner-connect",
        ),
        (
            "connect_error=other",
            "Horae couldn&#39;t finish connecting to Harvest.",
            "banner-connect",
        ),
    ] {
        let (_, ui) = after_callback(query);
        let html = ui.html();
        assert!(html.contains(sentence), "{query}: {html}");
        assert!(
            html.contains(&format!("data-testid=\"{action}\"")),
            "{query}: {html}"
        );
        let role = if query == "connect_error=denied" {
            "role=\"status\""
        } else {
            "role=\"alert\""
        };
        assert!(
            opening_tag(&html, "data-testid=\"connect-banner\"").contains(role),
            "{query}: only real errors are alerts: {html}"
        );
    }
}

/// The opening tag of the element carrying `attribute`.
fn opening_tag<'a>(html: &'a str, attribute: &str) -> &'a str {
    let at = html
        .find(attribute)
        .unwrap_or_else(|| panic!("missing {attribute}: {html}"));
    let start = html[..at].rfind('<').unwrap();
    let end = at + html[at..].find('>').unwrap();
    &html[start..=end]
}

#[test]
fn an_unavailable_harvest_is_not_called_a_refusal() {
    for status in [503, 429] {
        let (_, ui) = after_callback(&format!("connect_error=other&status={status}"));
        let html = ui.html();
        assert!(
            html.contains(&format!(
                "Harvest is unavailable right now (HTTP {status})."
            )),
            "{html}"
        );
        assert!(html.contains("Try again in a few minutes."), "{html}");
        assert!(!html.contains("refused"), "{html}");
    }
}

#[test]
fn no_banner_without_a_connect_failure() {
    for query in ["connected=1", "error=1"] {
        let (_, ui) = after_callback(query);
        assert!(!ui.html().contains("connect-banner"), "{query}");
    }
}

#[test]
fn a_dismissed_banner_does_not_come_back() {
    let (_, mut ui) = after_callback("connect_error=state");
    ui.click("dismiss-connect-banner");
    assert_eq!(
        ui.path(),
        route::Route::HarvestOverview {
            outcome: Default::default()
        }
        .to_string()
    );
    let html = ui.html();
    assert!(!html.contains("couldn&#39;t be verified"), "{html}");
    assert!(html.contains("Connect Harvest"), "{html}");
}

#[test]
fn the_banner_reconnects_in_place() {
    let (probe, mut ui) = after_callback("connect_error=code_expired");
    ui.click("banner-connect");
    assert_eq!(*probe.connects.borrow(), 1);
}

#[test]
fn refused_credentials_open_the_oauth_app_ready_for_a_new_secret() {
    let (_, mut ui) = after_callback("connect_error=invalid_client");
    assert!(
        ui.html().contains("A blank secret keeps the stored one"),
        "{}",
        ui.html()
    );
    ui.click("fix-oauth-secret");
    let html = ui.html();
    assert!(html.contains("Harvest OAuth2 app"), "{html}");
    // The stored secret is not shown: the field to paste a new one is.
    assert!(html.contains("id=\"harvest-client-secret\""), "{html}");
    assert!(!html.contains("data-testid=\"replace-harvest-secret\""));
}

#[test]
fn refused_environment_credentials_are_fixed_by_the_operator() {
    let probe = Probe::default();
    probe.connection.borrow_mut().configured = true;
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Environment,
        client_id: Some("env-app".into()),
        secret_set: true,
        secret_readable: true,
        ..key_only()
    };
    let ui = Ui::at(
        &probe,
        "/admin/importers/harvest?connect_error=invalid_client",
    );
    let html = ui.html();
    assert!(html.contains("HORAE_HARVEST_CLIENT_SECRET"), "{html}");
    assert!(
        !html.contains("A blank secret keeps the stored one"),
        "{html}"
    );
}

#[test]
fn a_saved_secret_leaves_the_focus_query_behind() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = stored_client();
    let mut ui = Ui::at(&probe, "/admin/importers/harvest/oauth?focus=secret");
    ui.type_into("harvest-client-secret", "new-secret");
    ui.click("save-harvest-client");
    assert_eq!(probe.client_saves.borrow().len(), 1);
    assert_eq!(
        ui.path(),
        route::Route::HarvestOAuth {
            focus: Default::default()
        }
        .to_string()
    );
}

#[test]
fn a_redirect_mismatch_shows_the_url_to_register() {
    let (_, ui) = after_callback("connect_error=redirect_mismatch");
    let html = ui.html();
    assert!(html.contains(STORED_REDIRECT), "{html}");
    assert!(html.contains("data-testid=\"copy-redirect-url\""), "{html}");
    // The banner has no <label>, so the field names itself.
    assert!(
        opening_tag(&html, "id=\"harvest-redirect-url\"").contains("aria-label=\"Redirect URL\""),
        "{html}"
    );
    assert!(
        html.contains("https://id.getharvest.com/developers"),
        "{html}"
    );
}

#[test]
fn an_expired_code_also_points_at_the_redirect_url() {
    let (_, ui) = after_callback("connect_error=code_expired");
    assert!(ui.html().contains("redirect URL"), "{}", ui.html());
}

#[test]
fn the_oauth_app_always_shows_the_redirect_url_it_will_use() {
    let probe = Probe::default();
    probe.connection.borrow_mut().configured = true;
    *probe.client.borrow_mut() = stored_client();
    let ui = Ui::at(&probe, "/admin/importers/harvest/oauth");
    let html = ui.html();
    assert!(html.contains(STORED_REDIRECT), "{html}");
    assert!(html.contains("data-testid=\"copy-redirect-url\""), "{html}");
    assert!(html.contains("must match exactly"), "{html}");
}

#[test]
fn an_environment_secret_stays_locked_when_opened_for_a_new_secret() {
    let probe = Probe::default();
    *probe.client.borrow_mut() = ClientSettings {
        source: ClientSource::Environment,
        client_id: Some("env-app".into()),
        secret_set: true,
        secret_readable: true,
        ..key_only()
    };
    let ui = Ui::at(&probe, "/admin/importers/harvest/oauth?focus=secret");
    let html = ui.html();
    assert!(html.contains("set in the environment"), "{html}");
    assert!(!html.contains("id=\"harvest-client-secret\""), "{html}");
}
