use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind};

use super::presentation;
use crate::models::{CommitAvailability, JobStatus};
use crate::route::Route;

/// Runs newest first: when, preview or import, result, status, and what to
/// do with each next.
#[component]
pub(super) fn HistoryTable(jobs: Vec<JobStatus>) -> Element {
    rsx! {
        div { class: "card p-0 overflow-x-auto",
            table { class: "table",
                thead {
                    tr {
                        th { scope: "col", "When" }
                        th { scope: "col", "Run" }
                        th { scope: "col", "Result" }
                        th { scope: "col", "Status" }
                        th { scope: "col", aria_label: "Action" }
                    }
                }
                tbody {
                    for job in jobs.iter() {
                        tr { key: "{job.id}",
                            td { class: "text-mono text-sm whitespace-nowrap",
                                time { datetime: job.created_at.to_rfc3339(), {job.created_at.format("%d %b %Y, %H:%M UTC").to_string()} }
                            }
                            td { class: "whitespace-nowrap",
                                span { class: if job.mode == Some(ImportMode::Commit) { "badge badge-pine badge-sm" } else { "badge badge-warning badge-sm" },
                                    "{presentation::run_label(job)}"
                                }
                                if job.source != Some(SourceKind::HarvestApi) {
                                    span { class: "text-xs text-faint ml-2", "{presentation::source_label(&job.kind)}" }
                                }
                            }
                            td { class: "text-sm whitespace-nowrap", "{result(job)}" }
                            td { class: "text-sm text-secondary whitespace-nowrap", "{status(job)}" }
                            td { class: "text-right whitespace-nowrap",
                                Link { to: Route::HarvestRun { id: job.id }, "data-testid": "run-{job.id}",
                                    if job.commit_availability == CommitAvailability::Available { "Review and import" } else { "Open" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn result(job: &JobStatus) -> String {
    if job.is_active() {
        return "In progress".into();
    }
    let Some(report) = job
        .report
        .clone()
        .and_then(|value| serde_json::from_value::<ImportReport>(value).ok())
    else {
        return "No report".into();
    };
    let (ready, skipped) = presentation::ready_and_skipped(&report);
    let verb = match (job.mode, job.status.as_str()) {
        (Some(ImportMode::Commit), _) => "imported",
        (_, "succeeded") => "ready",
        _ => "checked",
    };
    format!(
        "{} {verb} · {} skipped",
        presentation::count(ready),
        presentation::count(skipped)
    )
}

fn status(job: &JobStatus) -> &'static str {
    match job.commit_availability {
        CommitAvailability::Committed => "Imported",
        CommitAvailability::NewerImport => "Replaced by a newer run",
        _ => presentation::state_label(&job.status),
    }
}
