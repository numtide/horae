use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind, SyncScope};

use super::back_links::BackLinks;
use super::connection_card::ConnectionCard;
use super::history_table::HistoryTable;
use super::run_panel::RunPanel;
use super::runs_cursor::RunsCursor;
use super::{Conn, go_to_harvest, pause, presentation, use_connection};
use crate::models::{CommitAvailability, JobErrorKind, JobStatus, RetryAvailability};
use crate::route::Route;
use crate::server_fns;

/// The Harvest page: the connection, the two steps (Preview → Import) with
/// the current run's progress in place, and the latest runs.
#[component]
pub fn HarvestOverview() -> Element {
    let mut status = use_connection();
    let mut history = use_signal(|| None::<Result<Vec<JobStatus>, String>>);
    // Bumped to read the runs again after an action.
    let mut follow = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Follow the latest runs while one is active. When it ends, read the
    // connection again: an import changes what it allows.
    let _poll = use_resource(move || {
        follow();
        async move {
            let mut was_active = false;
            loop {
                let current = server_fns::list_harvest_import_jobs(None, Some(5))
                    .await
                    .map_err(|e| e.to_string());
                let active = matches!(&current, Ok(jobs) if jobs.iter().any(JobStatus::is_active));
                if history.peek().as_ref() != Some(&current) {
                    history.set(Some(current));
                }
                if !active {
                    if was_active {
                        status.restart();
                    }
                    return;
                }
                was_active = true;
                pause().await;
            }
        }
    });

    let changed = use_callback(move |_: ()| {
        status.restart();
        follow += 1;
    });
    let run_changed = use_callback(move |_: Option<uuid::Uuid>| follow += 1);

    let conn = Conn::from_resource(&status);
    let connection = conn.ready().cloned();
    let generation = connection.as_ref().map(|s| s.account_generation);
    let account = connection.as_ref().and_then(|s| s.account_id.clone());
    let start = move |mode: ImportMode, sync: SyncScope| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            error.set(None);
            if let Err(e) = server_fns::start_harvest_api_import(mode, sync, generation).await {
                error.set(Some(e.to_string()));
            }
            follow += 1;
            busy.set(false);
        });
    };
    let connect = move |_| async move {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        error.set(None);
        if let Err(e) = go_to_harvest().await {
            error.set(Some(format!(
                "Could not start Harvest connection. Try connecting again. {e}"
            )));
        }
        busy.set(false);
    };

    let jobs = match &*history.read() {
        Some(Ok(jobs)) => Some(jobs.clone()),
        _ => None,
    };
    let latest_api = jobs.as_ref().and_then(|jobs| {
        jobs.iter()
            .find(|job| job.source == Some(SourceKind::HarvestApi))
            .cloned()
    });
    let refused = latest_api.as_ref().is_some_and(|job| {
        job.status == "failed" && job.error_kind == Some(JobErrorKind::ReconnectRequired)
    });
    let step = Step::of(&conn, latest_api);
    let (first, second) = step.progress();
    let csv_link = rsx! {
        Link { to: Route::HarvestCsv {}, class: "text-sm", "data-testid": "use-csv", "Import a CSV export instead" }
    };

    rsx! {
        div { class: "himp-page min-w-0",
            BackLinks {}
            ConnectionCard { connection: conn.clone(), refused, on_changed: changed }
            section { class: "card flex flex-col gap-6", aria_label: "Import from Harvest",
                ol { class: "himp-steps", aria_label: "Import steps",
                    li { class: "himp-step", "data-state": first, span { class: "himp-step-dot", "1" } "Preview" }
                    li { class: "himp-step-line", aria_hidden: "true" }
                    li { class: "himp-step", "data-state": second, span { class: "himp-step-dot", "2" } "Import" }
                }
                match &step {
                    Step::Loading => rsx! { p { class: "text-sm text-secondary", role: "status", "Checking connection… CSV import remains available." } },
                    Step::Unavailable(detail) => rsx! {
                        div { class: "alert alert-danger", role: "alert",
                            p { "Could not check Harvest connection. Try checking again." }
                            details { summary { "Details" } p { "{detail}" } }
                        }
                        div { class: "flex gap-4 items-center flex-wrap",
                            button { r#type: "button", class: "btn btn-secondary", "data-testid": "recheck-connection".to_owned(),
                                onclick: move |_| status.restart(), "Check again"
                            }
                            {csv_link}
                        }
                    },
                    Step::Setup => rsx! {
                        div { class: "flex flex-col gap-4",
                            h2 { class: "himp-title", "Set up the Harvest OAuth app" }
                            p { class: "text-sm text-secondary", "Set up the Harvest OAuth app before connecting, or import a CSV export." }
                            div { class: "flex gap-4 items-center flex-wrap",
                                Link { to: Route::HarvestOAuth {}, class: "btn btn-primary", "data-testid": "setup-harvest-oauth", "Set up OAuth app" }
                                {csv_link}
                            }
                        }
                    },
                    Step::Connect { bound } => rsx! {
                        div { class: "flex flex-col gap-4",
                            h2 { class: "himp-title",
                                if *bound { "Reconnect your Harvest account" } else { "Connect your Harvest account" }
                            }
                            p { class: "text-sm text-secondary",
                                if *bound { "Imported data and the account binding are kept. Harvest asks you to approve read-only access again." }
                                else { "Harvest asks you to approve read-only access. Afterwards you come back here to preview the import." }
                            }
                            div { class: "flex gap-4 items-center flex-wrap",
                                button { r#type: "button", class: "btn btn-primary", "data-testid": "connect-harvest".to_owned(),
                                    disabled: busy(), onclick: connect,
                                    if busy() { "Connecting…" } else if *bound { "Reconnect Harvest" } else { "Connect Harvest" }
                                }
                                {csv_link}
                            }
                        }
                    },
                    Step::Ready => rsx! {
                        div { class: "flex flex-col gap-4",
                            h2 { class: "himp-title", "See what Harvest would bring in" }
                            p { class: "text-sm text-secondary",
                                "The preview reads clients, projects, tasks, people and time entries, and lists anything that can't be imported. Nothing in Horae changes until you import."
                            }
                            div { class: "flex gap-4 items-center flex-wrap",
                                button { r#type: "button", class: "btn btn-primary", "data-testid": "preview-api".to_owned(),
                                    disabled: busy(), onclick: move |_| start(ImportMode::DryRun, SyncScope::Full),
                                    "Start preview"
                                }
                                {csv_link}
                            }
                        }
                    },
                    Step::Run(job) => rsx! {
                        for job in [job.clone()] {
                            RunPanel { key: "{job.id}", job, account: account.clone(), generation, on_change: run_changed, summary: true }
                        }
                    },
                    Step::Imported(last) => rsx! {
                        div { class: "flex flex-col gap-4",
                            h2 { class: "himp-title", "Harvest is imported" }
                            p { class: "text-sm text-secondary",
                                if let Some(job) = last { "{imported_line(job)} " }
                                "To bring in what changed in Harvest since then, sync. Edits you made in Horae are kept."
                            }
                            div { class: "flex gap-4 items-center flex-wrap",
                                button { r#type: "button", class: "btn btn-primary", "data-testid": "resync".to_owned(),
                                    disabled: busy(), onclick: move |_| start(ImportMode::Commit, SyncScope::Incremental),
                                    "Sync changes"
                                }
                                if let Some(job) = last {
                                    Link { to: Route::HarvestRun { id: job.id }, class: "text-sm", "View the import" }
                                }
                            }
                        }
                    },
                }
                if let Some(message) = error() {
                    div { class: "alert alert-danger", role: "alert", "{message}" }
                }
            }
            section { class: "flex flex-col gap-3 mt-6", aria_labelledby: "history-title",
                div { class: "flex items-center justify-between gap-3",
                    h2 { id: "history-title", class: "himp-title", "History" }
                    Link { to: Route::HarvestRuns { before: RunsCursor::default() }, class: "text-sm", "data-testid": "all-runs", "All runs" }
                }
                match &*history.read() {
                    None => rsx! { p { class: "text-sm", "Loading runs…" } },
                    Some(Err(message)) => rsx! {
                        div { role: "alert",
                            p { "Could not load runs: {message}" }
                            button { r#type: "button", class: "btn btn-ghost btn-sm", "data-testid": "refresh-history".to_owned(),
                                onclick: move |_| follow += 1, "Try again"
                            }
                        }
                    },
                    Some(Ok(jobs)) if jobs.is_empty() => rsx! { p { class: "text-sm text-secondary", "No runs yet. Start a preview to create one." } },
                    Some(Ok(jobs)) => rsx! { HistoryTable { jobs: jobs.clone() } },
                }
            }
        }
    }
}

/// Where the Harvest page stands. A run is shown while it is active, when it
/// stopped, or when it is a preview that can still be imported.
#[derive(Clone, PartialEq)]
enum Step {
    Loading,
    Unavailable(String),
    Setup,
    Connect { bound: bool },
    Ready,
    Run(JobStatus),
    Imported(Option<JobStatus>),
}

impl Step {
    fn of(conn: &Conn, latest: Option<JobStatus>) -> Self {
        let status = match conn {
            Conn::Loading => return Self::Loading,
            Conn::Err(detail) => return Self::Unavailable(detail.clone()),
            Conn::Ready(status) => status,
        };
        if !status.configured {
            return Self::Setup;
        }
        if !status.connected {
            return Self::Connect {
                bound: status.account_id.is_some(),
            };
        }
        match latest {
            Some(job)
                if job.is_active()
                    || (job.can_retry()
                        && job.retry_availability == RetryAvailability::Available)
                    || job.commit_availability == CommitAvailability::Available =>
            {
                Self::Run(job)
            }
            Some(job) if job.mode == Some(ImportMode::Commit) && job.status == "succeeded" => {
                Self::Imported(Some(job))
            }
            _ if status.has_provenance => Self::Imported(None),
            _ => Self::Ready,
        }
    }

    /// The Preview and Import steps' states: "current", "done" or "".
    fn progress(&self) -> (&'static str, &'static str) {
        match self {
            Self::Ready => ("current", ""),
            Self::Run(job) if job.mode == Some(ImportMode::Commit) => ("done", "current"),
            Self::Run(job) if job.status == "succeeded" => ("done", "current"),
            Self::Run(_) => ("current", ""),
            Self::Imported(_) => ("done", "done"),
            _ => ("", ""),
        }
    }
}

fn imported_line(job: &JobStatus) -> String {
    let Some(report) = job
        .report
        .clone()
        .and_then(|value| serde_json::from_value::<ImportReport>(value).ok())
    else {
        return String::new();
    };
    let (ready, skipped) = presentation::ready_and_skipped(&report);
    let when = job
        .finished_at
        .unwrap_or(job.created_at)
        .format("%d %b %Y, %H:%M UTC");
    format!(
        "{} records imported on {when}; {} skipped.",
        presentation::count(ready),
        presentation::count(skipped)
    )
}
