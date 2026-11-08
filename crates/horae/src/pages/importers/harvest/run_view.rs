use dioxus::prelude::*;
use horae_core::importers::harvest::types::{
    ImportReport, PersonOutcome, ReportPerson, SourceKind,
};
use uuid::Uuid;

use super::back_links::BackLinks;
use super::entity_table::EntityTable;
use super::error_groups::ErrorGroups;
use super::missing_people::MissingPeople;
use super::run_panel::RunPanel;
use super::{Conn, pause, presentation, use_connection};
use crate::models::JobStatus;
use crate::route::Route;
use crate::server_fns;

/// A run's state, followed until it ends, then what it changes in Horae and
/// what needs the administrator's attention.
#[component]
pub(super) fn RunView(id: Uuid) -> Element {
    let navigator = use_navigator();
    let status = use_connection();
    let mut job = use_signal(|| None::<Result<JobStatus, String>>);
    // Bumped to read the run again: after any action, a failed status read,
    // or the page coming back into view. A finished run is read once each
    // time, so what it offers (import, retry) never goes stale.
    let mut follow = use_signal(|| 0_u64);

    use_future(move || async move {
        let mut shown = document::eval(&WATCH_VISIBILITY.replace("RUN_ID", &id.to_string()));
        while shown.recv::<bool>().await.is_ok() {
            follow += 1;
        }
    });

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
    let on_change = use_callback(move |started: Option<Uuid>| match started {
        Some(run) => {
            navigator.push(Route::HarvestRun { id: run });
        }
        None => follow += 1,
    });

    let snapshot = job.read();
    let run = match &*snapshot {
        None => {
            return rsx! {
                div { class: "himp-page min-w-0", "data-run": "{id}",
                    BackLinks { here: "Run".to_owned() }
                    p { role: "status", "Loading run…" }
                }
            };
        }
        Some(Err(error)) => {
            return rsx! {
                div { class: "himp-page min-w-0", "data-run": "{id}",
                    BackLinks { here: "Run".to_owned() }
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
    let title = presentation::run_label(&run);
    let connection = Conn::from_resource(&status).ready().cloned();
    let new_people: Vec<ReportPerson> = match &*report.read() {
        Some(Ok(report)) => report
            .people
            .iter()
            .filter(|person| person.outcome == PersonOutcome::Created)
            .cloned()
            .collect(),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "himp-page min-w-0", "data-run": "{id}",
            BackLinks { here: title.to_owned() }
            div { class: "page-header",
                h1 { class: "page-title flex items-center gap-3 flex-wrap",
                    "{title} of "
                    time { class: "text-mono", datetime: run.created_at.to_rfc3339(),
                        {run.created_at.format("%d %b %Y, %H:%M UTC").to_string()}
                    }
                    if run.source != Some(SourceKind::HarvestApi) {
                        span { class: "text-secondary", "· {presentation::source_label(&run.kind)}" }
                    }
                }
            }
            div { class: "card",
                RunPanel {
                    job: run.clone(),
                    account: connection.as_ref().and_then(|s| s.account_id.clone()),
                    generation: connection.as_ref().map(|s| s.account_generation),
                    on_change,
                }
            }
            match &*report.read() {
                Some(Ok(report)) if !run.is_active() => rsx! {
                    EntityTable { report: report.clone(), partial }
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
                    ErrorGroups { job_id: id, groups: summary.groups.clone(), omitted: summary.omitted_errors, new_people: new_people.clone() }
                },
                Some(Some(Err(error))) => rsx! {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Could not group this run's errors. The full list can still be downloaded." }
                        details { summary { "Details" } p { "{error}" } }
                        a { class: "btn btn-secondary btn-sm", href: format!("/api/import/harvest/jobs/{id}/errors"), "Download all errors (NDJSON)" }
                    }
                },
                Some(None) | None if finished_with_errors() => rsx! { p { class: "text-sm mt-4", role: "status", "Grouping errors…" } },
                _ if !run.is_active() => rsx! {
                    ErrorGroups { job_id: id, groups: Vec::new(), omitted: 0, new_people: new_people.clone() }
                },
                _ => rsx! {},
            }
        }
    }
}

/// Tells the run page when the tab or window is shown again. The listeners
/// remove themselves once this run's page is gone, since dropping the eval
/// does not reach the browser.
const WATCH_VISIBILITY: &str = r#"
const shown = () => {
  if (!document.querySelector('[data-run="RUN_ID"]')) {
    window.removeEventListener('focus', shown);
    document.removeEventListener('visibilitychange', shown);
    return;
  }
  if (document.visibilityState === 'visible') dioxus.send(true);
};
window.addEventListener('focus', shown);
document.addEventListener('visibilitychange', shown);
await new Promise(() => {});
"#;
