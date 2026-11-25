use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind};

use super::presentation;
use crate::components::table::DataTable;
use crate::i18n::Tr;
use crate::models::{CommitAvailability, JobStatus};
use crate::route::Route;

/// Runs newest first: when, preview or import, result and status. Each row
/// opens its run. `flush` inside the card that frames it.
#[component]
pub(super) fn HistoryTable(
    jobs: Vec<JobStatus>,
    #[props(default)] flush: bool,
    #[props(default)] busy: bool,
) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        DataTable { flush, compact: true, busy,
            table {
                thead {
                    tr {
                        th { scope: "col", "When" }
                        th { scope: "col", "Run" }
                        th { scope: "col", "Result" }
                        th { scope: "col", "Status" }
                    }
                }
                tbody {
                    for job in jobs.iter() {
                        tr { key: "{job.id}", class: "row-link",
                            td { class: "text-mono whitespace-nowrap",
                                Link { to: Route::HarvestRun { id: job.id }, class: "text-secondary", "data-testid": "run-{job.id}",
                                    time { datetime: job.created_at.to_rfc3339(), {job.created_at.format("%d %b %Y, %H:%M UTC").to_string()} }
                                }
                            }
                            td { class: "whitespace-nowrap",
                                span { class: if job.mode == Some(ImportMode::Commit) { "badge badge-pine badge-sm" } else { "badge badge-warning badge-sm" },
                                    "{presentation::run_label(job)}"
                                }
                                if job.source != Some(SourceKind::HarvestApi) {
                                    span { class: "text-xs text-faint ml-2", "{presentation::source_label(&job.kind)}" }
                                }
                            }
                            td { class: "whitespace-nowrap", "{result(job)}" }
                            td { class: "text-secondary whitespace-nowrap", {status(tr, job)} }
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

fn status(tr: Tr, job: &JobStatus) -> String {
    match job.commit_availability {
        // Its row is the way to review and import it.
        CommitAvailability::Available => tr.harvest_history_ready(),
        CommitAvailability::Committed => "Imported".into(),
        CommitAvailability::NewerImport => "Replaced by a newer run".into(),
        _ => presentation::state_label(&job.status).into(),
    }
}
