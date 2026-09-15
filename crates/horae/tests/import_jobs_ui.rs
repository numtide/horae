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
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/models/jobs.rs"]
mod models;
#[path = "../src/components/toast.rs"]
pub mod toast;

mod components {
    pub use crate::modal;
    pub use crate::toast;
}

type JobResponse = Result<Option<models::JobStatus>, ServerFnError>;
type ActionResponse = Result<models::JobStatus, ServerFnError>;
type ChangeResponse = oneshot::Receiver<Result<(), ServerFnError>>;
type ConnectionResponse = oneshot::Receiver<Result<ConnectionStatus, ServerFnError>>;
type ConnectResponse = oneshot::Receiver<Result<String, ServerFnError>>;
type ApiSubmission = (ImportMode, SyncScope, Option<i64>);

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
    csv_submissions: Rc<RefCell<Vec<(ImportMode, String)>>>,
    api_submissions: Rc<RefCell<Vec<ApiSubmission>>>,
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
    action.send(Ok(job(1, "queued"))).unwrap();
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
    let mut accepted = job(1, "running");
    accepted.phase = Some("cancelling".into());
    action.send(Ok(accepted)).unwrap();
    ui.settle();
    assert!(ui.html().contains("Cancellation requested"));
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
async fn submission_displays_its_acknowledged_state_before_status_polling_returns() {
    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-csv");
    ui.choose_file("import.csv");
    let _pending_status = probe.request();
    ui.click("preview-csv");
    assert!(ui.html().contains("Queued"));
    assert!(ui.html().contains("Following import"));
    assert_eq!(&*probe.requests.borrow(), &[Uuid::from_u128(11)]);
    assert_eq!(probe.csv_submissions.borrow().len(), 1);
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
    assert_eq!(ui.html().matches("btn btn-primary").count(), 1);
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
async fn result_copy_distinguishes_preview_zero_skipped_and_mixed_outcomes() {
    for mode in [ImportMode::DryRun, ImportMode::Commit] {
        for (created, updated, skipped, errored) in [(0, 0, 0, 0), (0, 0, 7, 0), (2, 3, 4, 1)] {
            let probe = Probe::default();
            let mut snapshot = job(1, "succeeded");
            let mut report = ImportReport::new(SourceKind::Csv, mode);
            report.summary.time_entries.created = created;
            report.summary.time_entries.updated = updated;
            report.summary.time_entries.skipped = skipped;
            for _ in 0..errored {
                report.record(
                    horae_core::importers::harvest::types::EntityType::TimeEntry,
                    &horae_core::importers::harvest::types::RowOutcome::Errored {
                        source_location: "row 2".into(),
                        reason: "Invalid date".into(),
                    },
                );
            }
            snapshot.report = Some(serde_json::to_value(report).unwrap());
            probe.history.borrow_mut().push(snapshot.clone());
            let mut ui = Ui::start(&probe);
            let response = probe.request();
            ui.click(&format!("select-{}", snapshot.id));
            response.send(Ok(Some(snapshot))).unwrap();
            ui.settle();
            let html = ui.html();
            assert!(
                html.contains(&format!(
                    "{created} created · {updated} updated · {skipped} skipped · {errored} errored"
                )),
                "{html}"
            );
            assert!(!html.contains("Every record was written"), "{html}");
            assert!(!html.contains("nothing was written"), "{html}");
            if mode == ImportMode::DryRun {
                assert!(html.contains("No business data was written"), "{html}");
                assert!(html.contains("Historical preview"), "{html}");
            }
        }
    }
}

#[test]
fn unselected_connected_importer_does_not_claim_an_empty_history() {
    let probe = Probe::default();
    *probe.connection.borrow_mut() = ConnectionStatus {
        configured: true,
        connected: true,
        account_id: Some("A".into()),
        ..ConnectionStatus::default()
    };
    probe.history.borrow_mut().push(job(1, "succeeded"));
    let mut ui = Ui::start(&probe);
    ui.click("choose-api");
    let html = ui.html();
    assert!(html.contains("No report selected"), "{html}");
    assert!(!html.contains("Nothing imported yet"), "{html}");
    assert!(!html.contains("without writing a single row"), "{html}");
    assert!(!html.contains("Every import is reversible"), "{html}");
}

#[tokio::test]
async fn history_labels_selection_and_unknown_metadata_are_readable() {
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
        snapshot.created_at = chrono::DateTime::parse_from_rfc3339("2026-09-15T10:30:00Z")
            .unwrap()
            .to_utc();
        snapshot.report = mode
            .map(|mode| serde_json::to_value(ImportReport::new(SourceKind::Csv, mode)).unwrap());
        probe.history.borrow_mut().push(snapshot.clone());
        let mut ui = Ui::start(&probe);
        let html = ui.html();
        assert!(html.contains(source) && html.contains(label), "{html}");
        assert!(html.contains("15 Sep 2026, 10:30 UTC"), "{html}");
        assert!(html.contains("datetime="), "{html}");
        let response = probe.request();
        ui.click(&format!("select-{}", snapshot.id));
        response.send(Ok(Some(snapshot))).unwrap();
        ui.settle();
        assert!(ui.html().contains("aria-pressed=\"true\""));
    }
}

#[tokio::test]
async fn progress_uses_observed_counts_and_readable_phase_without_an_eta() {
    let probe = Probe::default();
    let mut snapshot = job(1, "running");
    snapshot.total_count = None;
    probe.history.borrow_mut().push(snapshot.clone());
    let response = probe.request();
    let mut ui = Ui::start(&probe);
    response.send(Ok(Some(snapshot))).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Time entries"), "{html}");
    assert!(
        html.contains("Leaving this page does not stop the import"),
        "{html}"
    );
    assert!(html.contains("12 processed"), "{html}");
    assert!(!html.contains(" of 30"));
    assert!(!html.contains("time_entries"));
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
        snapshot.retry_availability = availability;
        snapshot.report = Some(
            serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::Commit)).unwrap(),
        );
        probe.history.borrow_mut().push(snapshot.clone());
        let mut ui = Ui::start(&probe);
        let html = ui.html();
        assert!(html.contains(reason), "{html}");
        assert!(!html.contains(&format!("data-testid=\"retry-{}\"", snapshot.id)));
        let response = probe.request();
        ui.click(&format!("select-{}", snapshot.id));
        response.send(Ok(Some(snapshot))).unwrap();
        ui.settle();
        assert!(ui.html().contains("Partial report"));
        assert!(!ui.html().contains("Import complete"));
        assert!(probe.retries.borrow().is_empty());
    }
}

#[tokio::test]
async fn navigating_away_invalidates_a_pending_preview_confirmation() {
    let probe = Probe::default();
    let mut ui = Ui::start(&probe);
    ui.click("choose-csv");
    ui.choose_file("original.csv");
    let response = probe.request();
    ui.click("preview-csv");
    ui.click("all-importers");
    let mut preview = job(11, "succeeded");
    preview.report =
        Some(serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::DryRun)).unwrap());
    response.send(Ok(Some(preview))).unwrap();
    ui.settle();
    assert!(!ui.html().contains("Commit this import"));
    assert!(ui.html().contains("Historical preview"));
    assert_eq!(probe.csv_submissions.borrow().len(), 1);
}

#[tokio::test]
async fn api_preview_confirmation_is_bound_to_the_original_account_generation() {
    for account_changed in [false, true] {
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
        let response = probe.request();
        ui.click("preview-api");
        let mut preview = job(11, "succeeded");
        preview.kind = "harvest_api_import".into();
        preview.report = Some(
            serde_json::to_value(ImportReport::new(
                SourceKind::HarvestApi,
                ImportMode::DryRun,
            ))
            .unwrap(),
        );
        response.send(Ok(Some(preview))).unwrap();
        ui.settle();
        assert_eq!(ui.html().matches("btn btn-primary").count(), 1);
        if account_changed {
            probe.connection.borrow_mut().account_generation = 3;
            probe.connection.borrow_mut().account_id = Some("B".into());
        }
        ui.click("refresh-connection");
        if account_changed {
            assert!(
                !ui.html().contains("Commit this import"),
                "A preview from account A cannot confirm against B"
            );
            assert_eq!(probe.api_submissions.borrow().len(), 1);
        } else {
            let _response = probe.request();
            ui.click("commit-preview");
            ui.click("commit-preview");
            assert_eq!(
                &*probe.api_submissions.borrow(),
                &[
                    (ImportMode::DryRun, SyncScope::Full, Some(2)),
                    (ImportMode::Commit, SyncScope::Full, Some(2))
                ]
            );
        }
    }
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

#[tokio::test]
async fn archived_report_shows_total_errors_and_a_job_scoped_download() {
    use horae_core::importers::harvest::types::{EntityType, RowOutcome};

    for state in ["succeeded", "failed"] {
        let probe = Probe::default();
        let mut snapshot = job(17, state);
        let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
        for line in 1..=2 {
            report.record(
                EntityType::TimeEntry,
                &RowOutcome::Errored {
                    source_location: format!("record {line}"),
                    reason: "invalid date".into(),
                },
            );
        }
        report.archive_errors(1).unwrap();
        snapshot.report = Some(serde_json::to_value(report).unwrap());
        probe.history.borrow_mut().push(snapshot.clone());
        let mut ui = Ui::start(&probe);
        let response = probe.request();
        ui.click(&format!("select-{}", snapshot.id));
        response.send(Ok(Some(snapshot.clone()))).unwrap();
        ui.settle();
        let html = ui.html();
        assert!(html.contains("Showing 0 inline errors of 2."), "{html}");
        assert!(html.contains("Download all errors"), "{html}");
        assert!(
            html.contains(&format!("/api/import/harvest/jobs/{}/errors", snapshot.id)),
            "{html}"
        );
        assert!(!html.contains("Every record was written"), "{html}");
        if state == "succeeded" {
            assert!(html.contains("Import complete with 2 errors"), "{html}");
        } else {
            assert!(html.contains("Partial report"), "{html}");
        }
    }
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
            assert!(ui.html().contains("Reconnect original account"));
        }
        ui.click("manage-connection");
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
        ui.click("manage-connection");
        let html = ui.html();
        assert!(html.contains(reason), "{html}");
        assert!(html.contains("disabled"));
        assert!(probe.changes.borrow().is_empty());
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
    ui.click("manage-connection");
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
    assert!(!html.contains("Reconnect original account"));
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
    ui.click("manage-connection");
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

    pub async fn cancel_harvest_import_job(id: Uuid) -> ActionResponse {
        let probe = consume_context::<Probe>();
        probe.cancellations.borrow_mut().push(id);
        let response = probe
            .action_responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected cancellation");
        response.await.unwrap()
    }
    pub async fn retry_harvest_import_job(id: Uuid) -> ActionResponse {
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
    ui.click("choose-api");
    assert!(ui.html().contains("Could not check Harvest connection"));
    ui.click("refresh-connection");
    assert!(ui.html().contains("Deployment setup required"));
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
        assert!(ui.html().contains("aria-expanded=\"false\""));
        ui.click("manage-connection");
        let html = ui.html();
        assert!(html.contains("aria-expanded=\"true\""));
        assert!(html.contains("aria-controls=\"harvest-connection-management\""));
        assert!(html.contains("Change account"));
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
            ui.click("manage-connection");
            let (send, receive) = oneshot::channel();
            *probe.disconnect_response.borrow_mut() = Some(receive);
            ui.click("disconnect-harvest");
            ui.click("disconnect-harvest");
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
    assert!(ui.html().contains("Read-only"));
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
    assert!(ui.html().contains("<summary>Details</summary>"));
}
