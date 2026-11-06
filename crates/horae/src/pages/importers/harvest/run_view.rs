use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind, SyncScope};
use uuid::Uuid;

use super::back_links::BackLinks;
use super::error_groups::ErrorGroups;
use super::missing_people::MissingPeople;
use super::report_summary::ReportSummary;
use super::run_header::RunHeader;
use super::{pause, presentation};
use crate::models::JobStatus;
use crate::route::Route;
use crate::server_fns;

/// A run's state, followed until it ends, then its report, grouped errors and
/// the people it could not match.
#[component]
pub(super) fn RunView(id: Uuid) -> Element {
    let navigator = use_navigator();
    let mut job = use_signal(|| None::<Result<JobStatus, String>>);
    // Bumped to follow the run again after a retry or a failed status read.
    let mut follow = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut action_error = use_signal(|| None::<String>);

    let _poll = use_resource(move || {
        follow();
        async move {
            loop {
                let current = server_fns::get_harvest_import_job(id)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|found| {
                        found.ok_or_else(|| "Import job not found or no longer available".into())
                    });
                let active = matches!(&current, Ok(run) if run.is_active());
                if job.peek().as_ref() != Some(&current) {
                    job.set(Some(current));
                }
                if !active {
                    return;
                }
                pause().await;
            }
        }
    });
    let report = use_memo(move || match &*job.read() {
        Some(Ok(run)) => run
            .report
            .clone()
            .map(|value| serde_json::from_value::<ImportReport>(value).map_err(|e| e.to_string())),
        _ => None,
    });
    // A finished report never changes, so its errors are grouped once.
    let finished_with_errors = use_memo(move || {
        matches!(&*job.read(), Some(Ok(run)) if !run.is_active())
            && matches!(&*report.read(), Some(Ok(report)) if report.error_count() > 0)
    });
    let summary = use_resource(move || async move {
        if !finished_with_errors() {
            return None;
        }
        Some(
            server_fns::harvest_import_error_summary(id)
                .await
                .map_err(|e| e.to_string()),
        )
    });

    let cancel = use_callback(move |_: ()| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            action_error.set(None);
            match server_fns::cancel_harvest_import_job(id).await {
                Ok(run) => job.set(Some(Ok(run))),
                Err(error) => action_error.set(Some(format!("Could not cancel import: {error}"))),
            }
            busy.set(false);
        });
    });
    let retry = use_callback(move |_: ()| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            action_error.set(None);
            match server_fns::retry_harvest_import_job(id).await {
                Ok(run) => {
                    job.set(Some(Ok(run)));
                    follow += 1;
                }
                Err(error) => action_error.set(Some(format!("Could not retry import: {error}. Reload the run to check its state before trying again."))),
            }
            busy.set(false);
        });
    });
    let commit = use_callback(move |_: ()| {
        spawn(async move {
            if *busy.peek() {
                return;
            }
            busy.set(true);
            action_error.set(None);
            match server_fns::commit_harvest_import_preview(id).await {
                Ok(run) => {
                    navigator.push(Route::HarvestRun { id: run.id });
                }
                Err(error) => {
                    action_error.set(Some(format!("Could not commit this preview: {error}")))
                }
            }
            busy.set(false);
        });
    });
    let resync = move |_| async move {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        action_error.set(None);
        let started = match server_fns::harvest_connection_status().await {
            Ok(status) => server_fns::start_harvest_api_import(
                ImportMode::Commit,
                SyncScope::Incremental,
                Some(status.account_generation),
            )
            .await
            .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        match started {
            Ok(run) => {
                navigator.push(Route::HarvestRun { id: run.id });
            }
            Err(error) => action_error.set(Some(format!("Could not start the re-sync: {error}"))),
        }
        busy.set(false);
    };

    let snapshot = job.read();
    let run = match &*snapshot {
        None => {
            return rsx! {
                div { class: "himp-page min-w-0",
                    BackLinks {}
                    p { role: "status", "Loading run…" }
                }
            };
        }
        Some(Err(error)) => {
            return rsx! {
                div { class: "himp-page min-w-0",
                    BackLinks {}
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Status unavailable. The import may still be running." }
                        details { summary { "Details" } p { "{error}" } }
                        button {
                            r#type: "button", class: "btn btn-secondary btn-sm",
                            "data-testid": "resume-monitoring".to_owned(),
                            onclick: move |_| follow += 1,
                            "Check again"
                        }
                    }
                }
            };
        }
        Some(Ok(run)) => run.clone(),
    };
    drop(snapshot);
    let partial = presentation::partial_report(&run.status);
    let committed_api = run.source == Some(SourceKind::HarvestApi)
        && run.mode == Some(ImportMode::Commit)
        && run.status == "succeeded";

    rsx! {
        div { class: "himp-page min-w-0",
            BackLinks {}
            RunHeader { job: run.clone(), busy, on_cancel: cancel, on_retry: retry, on_commit: commit }
            if let Some(error) = action_error() {
                div { class: "alert alert-danger mt-4", role: "alert", "{error}" }
            }
            if run.is_active() {
                div { class: "card mt-4 flex items-center gap-3 flex-wrap",
                    span { class: "himp-spinner" }
                    div { class: "text-sm font-semibold",
                        div { role: "status", aria_live: "polite", aria_atomic: "true",
                            "{presentation::state_label(&run.status)} · {presentation::phase_label(run.phase.as_deref())}"
                            if run.status == "queued" { p { "Waiting to start." } }
                            if run.phase.as_deref() == Some("cancelling") { p { "Cancellation requested; waiting for the current batch to finish." } }
                        }
                        div { class: "text-xs text-faint",
                            "{run.processed_count} processed"
                            if let Some(total) = run.total_count { " of {total}" }
                        }
                        if let Some(error) = &run.last_error {
                            div { class: "text-xs text-warning", "Last attempt: {error}" }
                        }
                        p { class: "text-xs text-faint", "Leaving this page does not stop the import." }
                    }
                }
            }
            if partial {
                div { class: "alert alert-danger mt-4", role: "alert",
                    p { if run.status == "cancelled" { "Import cancelled" } else { "Import failed" } }
                    if let Some(error) = &run.last_error { p { "{error}" } }
                    if run.report.is_some() { p { "{run.processed_count} processed in confirmed batches" } }
                }
            }
            match &*report.read() {
                Some(Ok(report)) if !run.is_active() => rsx! {
                    ReportSummary { report: report.clone(), partial }
                },
                Some(Err(error)) => rsx! {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Could not show the report. Download the errors or reload the run." }
                        details { summary { "Details" } p { "{error}" } }
                    }
                },
                _ => rsx! {},
            }
            match &*summary.read() {
                Some(Some(Ok(summary))) => rsx! {
                    MissingPeople { people: summary.people.clone(), omitted: summary.omitted_people, source: run.source }
                    ErrorGroups { job_id: id, groups: summary.groups.clone(), omitted: summary.omitted_errors }
                },
                Some(Some(Err(error))) => rsx! {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Could not group this run's errors. The full list can still be downloaded." }
                        details { summary { "Details" } p { "{error}" } }
                        a { class: "btn btn-secondary btn-sm", href: format!("/api/import/harvest/jobs/{id}/errors"), "Download all errors (NDJSON)" }
                    }
                },
                Some(None) | None if finished_with_errors() => rsx! { p { class: "text-sm mt-4", role: "status", "Grouping errors…" } },
                _ => rsx! {},
            }
            if committed_api {
                div { class: "flex items-center gap-3 flex-wrap mt-4",
                    button {
                        r#type: "button",
                        class: "btn btn-secondary",
                        "data-testid": "resync".to_owned(),
                        disabled: busy(),
                        onclick: resync,
                        "Re-sync changes"
                    }
                    span { class: "text-faint text-sm",
                        "Incremental: only what changed since the last sync. Local edits are never overwritten."
                    }
                }
            }
        }
    }
}
