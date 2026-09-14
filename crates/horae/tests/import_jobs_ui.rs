#![cfg(feature = "server")]

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use dioxus::core::{AttributeValue, ElementId, Mutation};
use dioxus::html::{FileData, SerializedFileData, SerializedFormObject};
use dioxus::prelude::*;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::importers::harvest::types::{
    ConnectionStatus, ImportMode, ImportReport, SourceKind, SyncScope,
};
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/pages/importers.rs"]
mod importers;
#[path = "../src/models/jobs.rs"]
mod models;
#[path = "../src/components/toast.rs"]
pub mod toast;

mod components {
    pub use crate::toast;
}

type JobResponse = Result<Option<models::JobStatus>, ServerFnError>;
type ActionResponse = Result<(), ServerFnError>;

#[derive(Clone, Default)]
struct Probe {
    history: Rc<RefCell<Vec<models::JobStatus>>>,
    requests: Rc<RefCell<Vec<Uuid>>>,
    responses: Rc<RefCell<VecDeque<oneshot::Receiver<JobResponse>>>>,
    history_error: Rc<RefCell<Option<String>>>,
    history_cursors: Rc<RefCell<Vec<Option<Uuid>>>>,
    retries: Rc<RefCell<Vec<Uuid>>>,
    cancellations: Rc<RefCell<Vec<Uuid>>>,
    csv_submissions: Rc<RefCell<Vec<(ImportMode, String)>>>,
    action_responses: Rc<RefCell<VecDeque<oneshot::Receiver<ActionResponse>>>>,
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

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { importers::HarvestImport {} }
}

struct Ui {
    dom: VirtualDom,
    targets: HashMap<String, ElementId>,
}

impl Ui {
    fn start(probe: &Probe) -> Self {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut ui = Self {
            dom: VirtualDom::new_with_props(app, probe.clone()),
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
                name: "data-testid",
                value: AttributeValue::Text(name),
                id,
                ..
            } = mutation
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

    fn click(&mut self, name: &str) {
        let id = *self
            .targets
            .get(name)
            .unwrap_or_else(|| panic!("missing button {name}: {}", self.html()));
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
    }
}

#[tokio::test]
async fn reopening_importer_restores_queued_and_running_jobs_from_history() {
    for state in ["queued", "running"] {
        let probe = Probe::default();
        let mut snapshot = job(1, state);
        snapshot.last_error = Some("Temporary source failure".into());
        probe.history.borrow_mut().push(snapshot.clone());
        let response = probe.request();
        let mut ui = Ui::start(&probe);
        response.send(Ok(Some(snapshot))).unwrap();
        ui.settle();
        let html = ui.html();
        assert!(html.contains("Import history"), "{html}");
        assert!(
            html.contains("12 processed") && html.contains(" of 30"),
            "{html}"
        );
        assert!(
            html.contains("Last attempt: Temporary source failure"),
            "{html}"
        );
        assert_eq!(&*probe.requests.borrow(), &[Uuid::from_u128(1)]);
    }
}

#[tokio::test]
async fn failed_poll_can_resume_without_replaying_its_old_error() {
    let probe = Probe::default();
    probe.history.borrow_mut().push(job(1, "running"));
    let response = probe.request();
    let mut ui = Ui::start(&probe);
    response
        .send(Err(ServerFnError::new("Network unavailable")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Network unavailable"));
    assert!(!ui.html().contains("Following import"));

    let response = probe.request();
    ui.click("resume-monitoring");
    assert!(ui.html().contains("Following import"));
    assert!(!ui.html().contains("Network unavailable"));
    response.send(Ok(Some(job(1, "cancelled")))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Import cancelled"));
    assert!(!ui.html().contains("Following import"));
    assert_eq!(
        &*probe.requests.borrow(),
        &[Uuid::from_u128(1), Uuid::from_u128(1)]
    );
}

#[tokio::test]
async fn missing_job_stops_polling_and_explains_that_status_is_unavailable() {
    let probe = Probe::default();
    probe.history.borrow_mut().push(job(1, "running"));
    let response = probe.request();
    let mut ui = Ui::start(&probe);
    response.send(Ok(None)).unwrap();
    ui.settle();
    assert!(ui.html().contains("Import job not found"));
    assert!(!ui.html().contains("Following import"));
}

#[tokio::test]
async fn historical_preview_is_read_only_and_selecting_another_job_clears_it() {
    let probe = Probe::default();
    let mut preview = job(1, "succeeded");
    preview.report =
        Some(serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::DryRun)).unwrap());
    probe
        .history
        .borrow_mut()
        .extend([preview.clone(), job(2, "failed")]);
    let mut ui = Ui::start(&probe);
    let response = probe.request();
    ui.click(&format!("select-{}", preview.id));
    response.send(Ok(Some(preview))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Historical preview"));
    assert!(!ui.html().contains("Commit this import"));
    let response = probe.request();
    ui.click(&format!("select-{}", Uuid::from_u128(2)));
    assert!(!ui.html().contains("Historical preview"));
    response.send(Ok(Some(job(2, "failed")))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Import failed"));
}

#[tokio::test]
async fn retry_errors_are_visible_and_success_restarts_monitoring() {
    let probe = Probe::default();
    probe.history.borrow_mut().push(job(1, "failed"));
    let mut ui = Ui::start(&probe);
    let action = probe.action();
    let target = format!("retry-{}", Uuid::from_u128(1));
    ui.click(&target);
    ui.click(&target);
    assert_eq!(
        probe.retries.borrow().len(),
        1,
        "pending retry must not be submitted twice"
    );
    action
        .send(Err(ServerFnError::new("Retry rejected")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not retry import"));
    let action = probe.action();
    let response = probe.request();
    ui.click(&target);
    action.send(Ok(())).unwrap();
    ui.settle();
    assert!(ui.html().contains("Following import"));
    assert!(!ui.html().contains("Could not retry import"));
    response.send(Ok(Some(job(1, "running")))).unwrap();
    ui.settle();
    assert_eq!(
        &*probe.retries.borrow(),
        &[Uuid::from_u128(1), Uuid::from_u128(1)]
    );
}

#[tokio::test]
async fn cancellation_errors_are_visible_and_acceptance_does_not_acknowledge_completion() {
    let probe = Probe::default();
    probe.history.borrow_mut().push(job(1, "running"));
    let response = probe.request();
    let mut ui = Ui::start(&probe);
    response.send(Ok(Some(job(1, "running")))).unwrap();
    ui.settle();
    let action = probe.action();
    ui.click("cancel-import");
    action
        .send(Err(ServerFnError::new("Cancellation rejected")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not cancel import"));
    assert!(ui.html().contains("Following import"));
    let action = probe.action();
    ui.click("cancel-import");
    action.send(Ok(())).unwrap();
    ui.settle();
    assert!(ui.html().contains("cancelling"));
    assert!(ui.html().contains("Following import"));
    assert!(!ui.html().contains("Import cancelled"));
    assert_eq!(probe.cancellations.borrow().len(), 2);
}

#[tokio::test]
async fn history_failure_can_be_refreshed_and_restores_an_active_job() {
    let probe = Probe::default();
    *probe.history_error.borrow_mut() = Some("History unavailable".into());
    let mut ui = Ui::start(&probe);
    assert!(ui.html().contains("Could not load import history"));
    *probe.history_error.borrow_mut() = None;
    probe.history.borrow_mut().push(job(1, "running"));
    let _response = probe.request();
    ui.click("refresh-history");
    assert!(!ui.html().contains("Could not load import history"));
    assert_eq!(&*probe.requests.borrow(), &[Uuid::from_u128(1)]);
}

#[tokio::test]
async fn history_can_page_back_and_return_to_the_latest_imports() {
    let probe = Probe::default();
    probe
        .history
        .borrow_mut()
        .extend((1..=24).map(|id| job(id, "succeeded")));
    let mut ui = Ui::start(&probe);
    assert!(
        !ui.html()
            .contains(&format!("select-{}", Uuid::from_u128(24)))
    );
    ui.click("older-history");
    assert!(
        ui.html()
            .contains(&format!("select-{}", Uuid::from_u128(24)))
    );
    assert!(
        !ui.html()
            .contains(&format!("select-{}", Uuid::from_u128(1)))
    );
    ui.click("newer-history");
    assert!(
        ui.html()
            .contains(&format!("select-{}", Uuid::from_u128(1)))
    );
    ui.click("older-history");
    ui.click("refresh-history");
    assert!(
        ui.html()
            .contains(&format!("select-{}", Uuid::from_u128(1)))
    );
    assert_eq!(
        &*probe.history_cursors.borrow(),
        &[
            None,
            Some(Uuid::from_u128(20)),
            None,
            Some(Uuid::from_u128(20)),
            None,
        ]
    );
}

#[tokio::test]
async fn replacing_a_file_during_preview_does_not_change_the_committed_source() {
    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-csv");
    ui.choose_file("original.csv");
    let preview_response = probe.request();
    ui.click("preview-csv");
    ui.click("preview-csv");
    ui.choose_file("replacement.csv");
    let mut preview = job(11, "succeeded");
    preview.report =
        Some(serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::DryRun)).unwrap());
    preview_response.send(Ok(Some(preview))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Report source: original.csv"));
    assert!(ui.html().contains("replacement.csv"));
    let _commit_response = probe.request();
    ui.click("commit-preview");
    assert_eq!(
        &*probe.csv_submissions.borrow(),
        &[
            (ImportMode::DryRun, "original.csv".into()),
            (ImportMode::Commit, "original.csv".into()),
        ]
    );
}

#[tokio::test]
async fn selecting_another_job_drops_the_previous_pending_status_request() {
    let probe = Probe::default();
    probe
        .history
        .borrow_mut()
        .extend([job(1, "running"), job(2, "succeeded")]);
    let old_response = probe.request();
    let mut ui = Ui::start(&probe);
    let response = probe.request();
    ui.click(&format!("select-{}", Uuid::from_u128(2)));
    assert!(old_response.send(Ok(Some(job(1, "failed")))).is_err());
    let mut completed = job(2, "succeeded");
    completed.report =
        Some(serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::Commit)).unwrap());
    response.send(Ok(Some(completed))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Import complete"));
    assert!(!ui.html().contains("Import failed"));
}

#[tokio::test]
async fn failed_and_cancelled_history_shows_partial_outcomes_without_success_actions() {
    for state in ["failed", "cancelled"] {
        for mode in [ImportMode::Commit, ImportMode::DryRun] {
            let probe = Probe::default();
            let mut snapshot = job(1, state);
            let mut report = ImportReport::new(SourceKind::Csv, mode);
            report.summary.time_entries.created = 499;
            snapshot.report = Some(serde_json::to_value(report).unwrap());
            snapshot.last_error = (state == "failed").then(|| "Source connection lost".to_string());
            probe.history.borrow_mut().push(snapshot.clone());
            let mut ui = Ui::start(&probe);
            let response = probe.request();
            ui.click(&format!("select-{}", snapshot.id));
            response.send(Ok(Some(snapshot))).unwrap();
            ui.settle();
            let html = ui.html();
            assert!(html.contains("Partial report"), "{html}");
            assert!(html.contains("499"), "{html}");
            assert!(
                html.contains(if state == "failed" {
                    "Source connection lost"
                } else {
                    "Import cancelled"
                }),
                "{html}"
            );
            assert!(!html.contains("Import complete"), "{html}");
            assert!(!html.contains("Commit this import"), "{html}");
            assert!(!html.contains("Re-sync changes"), "{html}");
        }
    }
}

#[tokio::test]
async fn newly_submitted_failed_preview_cannot_be_confirmed() {
    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-csv");
    ui.choose_file("partial.csv");
    let response = probe.request();
    ui.click("preview-csv");
    let mut snapshot = job(11, "failed");
    snapshot.report =
        Some(serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::DryRun)).unwrap());
    snapshot.last_error = Some("Upload unavailable".into());
    response.send(Ok(Some(snapshot))).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Partial report"), "{html}");
    assert!(html.contains("Upload unavailable"), "{html}");
    assert!(!html.contains("Commit this import"), "{html}");
    ui.choose_file("another.csv");
    assert!(!ui.html().contains("Partial report"));
    assert!(!ui.html().contains("Upload unavailable"));
}

mod server_fns {
    use super::*;

    pub async fn harvest_connection_status() -> Result<ConnectionStatus, ServerFnError> {
        Ok(ConnectionStatus {
            configured: false,
            connected: false,
            account_id: None,
            token_expired: false,
        })
    }

    pub async fn list_harvest_import_jobs(
        before: Option<Uuid>,
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
        Ok(history.iter().skip(start).take(20).cloned().collect())
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
        _: ImportMode,
        _: SyncScope,
    ) -> Result<Uuid, ServerFnError> {
        panic!("unexpected API import")
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
    ) -> Result<Uuid, ServerFnError> {
        let probe = consume_context::<Probe>();
        let mut submitted = probe.csv_submissions.borrow_mut();
        submitted.push((mode, file.0.name()));
        Ok(Uuid::from_u128(10 + submitted.len() as u128))
    }

    pub async fn cancel_harvest_import_job(id: Uuid) -> Result<(), ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.cancellations.borrow_mut().push(id);
        let response = probe
            .action_responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected cancellation");
        response.await.unwrap()
    }
    pub async fn retry_harvest_import_job(id: Uuid) -> Result<(), ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.retries.borrow_mut().push(id);
        let response = probe
            .action_responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected retry");
        response.await.unwrap()
    }
    pub async fn harvest_connect_start() -> Result<String, ServerFnError> {
        panic!("unexpected connect")
    }
    pub async fn harvest_disconnect() -> Result<(), ServerFnError> {
        panic!("unexpected disconnect")
    }
}
