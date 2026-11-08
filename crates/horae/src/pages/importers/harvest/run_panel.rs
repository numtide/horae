use dioxus::prelude::*;
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, ImportReport, SourceKind, SyncScope,
};
use uuid::Uuid;

use super::{entity_label, go_to_harvest, presentation};
use crate::models::{CommitAvailability, JobErrorKind, JobStatus, RetryAvailability};
use crate::route::Route;
use crate::server_fns;

/// A run's state and the one thing to do next: its progress while it runs,
/// and how it ended. The Harvest page and the run page show the same panel.
///
/// `on_change` gets the id of a run an action started, or `None` when the
/// action changed this run.
#[component]
pub(super) fn RunPanel(
    job: JobStatus,
    /// The bound Harvest account and its generation, for reconnect advice
    /// and for starting a new preview.
    account: Option<String>,
    generation: Option<i64>,
    on_change: EventHandler<Option<Uuid>>,
    /// The Harvest page adds a per-entity count and a link to the run.
    #[props(default)]
    summary: bool,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let id = job.id;
    let preview = job.mode != Some(ImportMode::Commit);
    let noun = if preview { "preview" } else { "import" };

    // One way to run every action: refuse a second click, keep the error here.
    let act = move |what: Action| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            error.set(None);
            let result = match what {
                Action::Cancel => server_fns::cancel_harvest_import_job(id)
                    .await
                    .map(|_| None),
                Action::Retry => server_fns::retry_harvest_import_job(id).await.map(|_| None),
                Action::Commit => server_fns::commit_harvest_import_preview(id)
                    .await
                    .map(|run| Some(run.id)),
                Action::Start(mode, sync) => {
                    server_fns::start_harvest_api_import(mode, sync, generation)
                        .await
                        .map(|run| Some(run.id))
                }
                Action::Reconnect => go_to_harvest().await.map(|()| None),
            };
            match result {
                Ok(started) => on_change.call(started),
                Err(e) => {
                    let failed = match what {
                        Action::Cancel => format!("Could not cancel the {noun}"),
                        Action::Retry => format!("Could not continue the {noun}"),
                        Action::Commit => "Could not import this preview".to_owned(),
                        Action::Start(..) => "Could not start the run".to_owned(),
                        Action::Reconnect => "Could not start Harvest connection".to_owned(),
                    };
                    error.set(Some(format!("{failed}: {e}")));
                    // A refusal usually means the run moved on; read it again.
                    on_change.call(None);
                }
            }
            busy.set(false);
        });
    };

    let report = job
        .report
        .clone()
        .and_then(|value| serde_json::from_value::<ImportReport>(value).ok());
    let api = job.source == Some(SourceKind::HarvestApi);
    // "Sync changes" is an incremental import.
    let sync = !preview && job.sync == Some(SyncScope::Incremental);
    let (ready, skipped) = report
        .as_ref()
        .map_or((0, 0), presentation::ready_and_skipped);
    let entries = report
        .as_ref()
        .map_or(0, |report| report.summary.time_entries.processed());
    let written = (!preview).then(|| {
        format!(
            "{} records were written and stay in Horae. The batch in progress was undone.",
            presentation::count(ready)
        )
    });
    let (badge, badge_text) = match job.status.as_str() {
        "queued" | "running" if preview => ("badge-warning", "Nothing in Horae changes"),
        "queued" | "running" => ("badge-pine", "Writing to Horae"),
        "succeeded" if preview && job.committed_as.is_some() => ("badge-success", "Imported"),
        "succeeded" if preview => ("badge-warning", "Nothing imported yet"),
        "succeeded" if sync => ("badge-success", "Synced"),
        "succeeded" => ("badge-success", "Imported"),
        "cancelled" => ("badge-neutral", "Cancelled"),
        "failed" => ("badge-danger", "Stopped"),
        _ => ("badge-neutral", "Status unavailable"),
    };
    let retry_label = if preview {
        "Continue preview"
    } else {
        "Finish the import"
    };
    let can_retry = job.retry_availability == RetryAvailability::Available;
    let access_error = matches!(
        job.error_kind,
        Some(JobErrorKind::ReconnectRequired | JobErrorKind::Forbidden)
    );
    let run_link = rsx! {
        if summary {
            Link { to: Route::HarvestRun { id }, class: "btn btn-ghost", "data-testid": "open-run", "Open the run" }
        }
    };

    rsx! {
        section { class: "himp-panel flex flex-col gap-4", "data-testid": "run-panel", aria_busy: busy(),
            div { class: "flex items-center gap-3 flex-wrap",
                h2 { class: "himp-title", if preview { "Preview" } else if sync { "Sync" } else { "Import" } }
                span { class: "badge {badge}", "data-testid": "run-badge", "{badge_text}" }
            }
            if job.is_active() {
                RunProgress { job: job.clone(), report: report.clone() }
                // Only a queued job is waiting to be tried again; a running one is
                // that attempt, and its last failure no longer applies.
                if job.status == "queued" && job.last_error.is_some() {
                    div { class: "alert alert-warning",
                        if job.error_kind == Some(JobErrorKind::Unavailable) {
                            "Harvest didn't respond in time. Horae will try again in a moment and continue from where it stopped; you don't need to do anything."
                        } else {
                            "The last attempt failed. Horae will try again in a moment and continue from where it stopped."
                        }
                    }
                }
                p { class: "text-sm text-secondary",
                    if preview { "You can leave this page; the preview keeps running." }
                    else { "If you cancel, what is already written stays and the batch in progress is undone. You can leave this page." }
                }
                div { class: "flex gap-3 flex-wrap",
                    {
                        let cancelling = job.phase.as_deref() == Some("cancelling");
                        rsx! {
                            button { r#type: "button", class: "btn btn-secondary", "data-testid": "cancel-import".to_owned(),
                                disabled: busy() || cancelling,
                                onclick: move |_| act(Action::Cancel),
                                if cancelling { "Cancelling…" } else if preview { "Cancel preview" } else { "Cancel import" }
                            }
                        }
                    }
                    {run_link}
                }
            } else {
                match job.status.as_str() {
                    "succeeded" if preview => rsx! {
                        div { class: "flex flex-col gap-2",
                            h3 { class: "himp-title", "{presentation::count(ready)} records are ready" }
                            p { class: "text-sm text-secondary",
                                if skipped > 0 { "{presentation::count(skipped)} will be skipped. " }
                                if api { "Importing reads Harvest again, so anything changed since this preview comes across too." }
                                else { "Importing reads the same file again." }
                            }
                        }
                        if summary && let Some(report) = &report {
                            Counts { report: report.clone() }
                        }
                        div { class: "flex gap-3 flex-wrap items-center",
                            match (job.committed_as, job.commit_availability) {
                                (Some(commit), _) => rsx! {
                                    p { class: "text-sm", "data-testid": "committed-as",
                                        "This preview was imported. "
                                        Link { to: Route::HarvestRun { id: commit }, "Open the import" }
                                    }
                                },
                                (None, CommitAvailability::Available) => rsx! {
                                    button { r#type: "button", class: "btn btn-primary", "data-testid": "commit-preview".to_owned(),
                                        disabled: busy(), onclick: move |_| act(Action::Commit), "Import into Horae"
                                    }
                                },
                                (None, availability) => rsx! {
                                    p { class: "text-sm text-faint", "data-testid": "commit-unavailable", "{presentation::commit_reason(availability)}" }
                                },
                            }
                            if api {
                                button { r#type: "button", class: "btn btn-secondary", "data-testid": "new-preview".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Start(ImportMode::DryRun, SyncScope::Full)),
                                    "Preview again"
                                }
                            } else {
                                Link { to: Route::HarvestCsv {}, class: "btn btn-secondary", "Choose another file" }
                            }
                            if summary {
                                Link { to: Route::HarvestRun { id }, class: "btn btn-ghost", "data-testid": "open-run", "Review the preview" }
                            }
                        }
                    },
                    "succeeded" => rsx! {
                        if sync && let Some(report) = &report {
                            p { class: "text-sm",
                                "{presentation::count(report.summary.time_entries.created)} new and {presentation::count(report.summary.time_entries.updated)} updated entries from Harvest."
                                if skipped > 0 { " {presentation::count(skipped)} not synced; see below." }
                            }
                        } else {
                            p { class: "text-sm",
                                "{presentation::count(ready)} records are in Horae."
                                if skipped > 0 { " {presentation::count(skipped)} were skipped; the run lists why." }
                            }
                        }
                        if report.as_ref().is_some_and(|report| report.summary.people.created > 0) {
                            p { class: "text-sm text-secondary",
                                "New people are deactivated members; activate the ones who should sign in on Team."
                            }
                        }
                        div { class: "flex gap-3 flex-wrap",
                            Link { to: Route::ProjectList {}, class: "btn btn-primary", "Go to projects" }
                            if report.as_ref().is_some_and(|report| report.summary.people.created > 0) {
                                Link { to: Route::AdminUsers {}, class: "btn btn-secondary", "Review new people" }
                            }
                            if api {
                                button { r#type: "button", class: "btn btn-secondary", "data-testid": "resync".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Start(ImportMode::Commit, SyncScope::Incremental)),
                                    "Sync changes"
                                }
                            }
                            {run_link}
                        }
                    },
                    "cancelled" => rsx! {
                        p { class: "text-sm",
                            if let Some(written) = &written { "{written}" }
                            else { "Nothing in Horae changed. It had checked {presentation::count(entries)} time entries; the report below covers only those." }
                        }
                        div { class: "flex gap-3 flex-wrap items-center",
                            if can_retry {
                                button { r#type: "button", class: "btn btn-primary", "data-testid": "retry-run".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Retry), "{retry_label}"
                                }
                                if !preview { span { class: "text-sm text-secondary", "Records already written aren't duplicated." } }
                            } else {
                                p { class: "text-sm text-faint", "{presentation::retry_reason(job.retry_availability)}" }
                            }
                            if preview && api {
                                button { r#type: "button", class: "btn btn-secondary", "data-testid": "new-preview".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Start(ImportMode::DryRun, SyncScope::Full)),
                                    "Start a new preview"
                                }
                            }
                            {run_link}
                        }
                    },
                    _ => rsx! {
                        div { class: "alert alert-danger", role: "alert", "data-testid": "run-error",
                            match job.error_kind {
                                Some(JobErrorKind::ReconnectRequired) => rsx! { "Harvest no longer accepts Horae's access, so the {noun} stopped." },
                                Some(JobErrorKind::Forbidden) => rsx! { "The Harvest user who connected Horae can't read everything the import needs." },
                                Some(JobErrorKind::Unavailable) => rsx! { "Harvest didn't respond after several tries. It is usually temporary." },
                                None => rsx! {
                                    "The {noun} stopped. "
                                    if let Some(reason) = &job.last_error { "{reason}" }
                                },
                            }
                            if preview { " Nothing in Horae changed." }
                        }
                        if let Some(written) = &written { p { class: "text-sm", "{written}" } }
                        if job.error_kind == Some(JobErrorKind::Forbidden) {
                            p { class: "text-sm text-secondary",
                                if let Some(account) = &account { "Connected to Harvest account {account}. " }
                                "Ask a Harvest administrator to reconnect, or give this user administrator rights in Harvest."
                            }
                        }
                        div { class: "flex gap-3 flex-wrap items-center",
                            match job.error_kind {
                                Some(JobErrorKind::ReconnectRequired) => rsx! {
                                    button { r#type: "button", class: "btn btn-primary", "data-testid": "reconnect".to_owned(),
                                        disabled: busy(), onclick: move |_| act(Action::Reconnect), "Reconnect Harvest"
                                    }
                                },
                                Some(JobErrorKind::Forbidden) => rsx! {
                                    button { r#type: "button", class: "btn btn-primary", "data-testid": "reconnect".to_owned(),
                                        disabled: busy(), onclick: move |_| act(Action::Reconnect), "Reconnect as another user"
                                    }
                                },
                                _ => rsx! {},
                            }
                            // After an access error, reconnecting comes first; continuing
                            // is still offered, since a reconnect to the same account
                            // leaves this run as it was.
                            if can_retry {
                                button { r#type: "button",
                                    class: if access_error { "btn btn-secondary" } else { "btn btn-primary" },
                                    "data-testid": "retry-run".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Retry), "{retry_label}"
                                }
                                if entries > 0 && !access_error {
                                    span { class: "text-sm text-secondary", "Picks up from {presentation::count(entries)} time entries." }
                                }
                                if job.error_kind == Some(JobErrorKind::ReconnectRequired) {
                                    span { class: "text-sm text-secondary",
                                        "Reconnect first, then continue; it picks up where it stopped."
                                    }
                                }
                            } else {
                                p { class: "text-sm text-faint", "{presentation::retry_reason(job.retry_availability)}" }
                            }
                            if preview && api {
                                button { r#type: "button", class: "btn btn-secondary", "data-testid": "new-preview".to_owned(),
                                    disabled: busy(), onclick: move |_| act(Action::Start(ImportMode::DryRun, SyncScope::Full)),
                                    "Start a new preview"
                                }
                            }
                            {run_link}
                        }
                    },
                }
            }
            if let Some(message) = error() {
                div { class: "alert alert-danger", role: "alert", "data-testid": "action-error", "{message}" }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Cancel,
    Retry,
    Commit,
    Start(ImportMode, SyncScope),
    Reconnect,
}

/// What the running job is doing, "x of N" with a bar when the phase's size
/// is known, and the count alone otherwise.
#[component]
fn RunProgress(job: JobStatus, report: Option<ImportReport>) -> Element {
    let progress = presentation::progress(&job, report.as_ref());
    let label = progress.label;
    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center justify-between gap-3 flex-wrap text-sm text-secondary",
                // Announce the phase, not the count: the page polls every second,
                // and the progress bar already carries the numbers.
                span { role: "status", aria_live: "polite", "{label}" }
                match (progress.done, progress.total) {
                    (Some(done), Some(total)) => rsx! {
                        span { class: "text-mono", "data-testid": "run-count",
                            "{presentation::count(done.min(total))} of {presentation::count(total)}"
                        }
                    },
                    (Some(done), None) => rsx! {
                        span { class: "text-mono", "data-testid": "run-count", "{presentation::count(done)}" }
                    },
                    _ => rsx! {},
                }
            }
            match (progress.done, progress.total) {
                (Some(done), Some(total)) if total > 0 => rsx! {
                    progress { class: "himp-bar", aria_label: "{label}", max: "{total}", value: "{done.min(total)}" }
                },
                _ => rsx! { progress { class: "himp-bar", aria_label: "{label}" } },
            }
        }
    }
}

/// Per-entity totals of a finished preview: what it would write, and what
/// it would skip.
#[component]
fn Counts(report: ImportReport) -> Element {
    rsx! {
        div { class: "himp-counts",
            for entity in EntityType::ALL {
                {
                    let counts = report.summary.counts(entity);
                    let value = counts.created + counts.updated;
                    rsx! {
                        div { key: "{entity.as_str()}",
                            span { class: "text-sm text-secondary", "{entity_label(entity)}" }
                            span { class: "text-mono text-lg", "{presentation::count(value)}" }
                            span { class: "text-xs text-secondary",
                                "{presentation::count(counts.created)} new"
                                if counts.updated > 0 { " · {presentation::count(counts.updated)} updated" }
                                if counts.errored > 0 { " · {presentation::count(counts.errored)} skipped" }
                            }
                        }
                    }
                }
            }
        }
    }
}
