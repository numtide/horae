use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use horae_core::duration::format_hours2 as hours;

use crate::components::badge::Badge;
use crate::components::form::{FormGroup, Input};
use crate::components::table::DataTable;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportCursor, TimeReportQuery};
use crate::server_fns;

fn period(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate), &'static str> {
    let from = from.parse().map_err(|_| "Enter a valid start date.")?;
    let to = to.parse().map_err(|_| "Enter a valid end date.")?;
    if from > to {
        return Err("The end date must be on or after the start date.");
    }
    Ok((from, to))
}

#[component]
pub(super) fn ScopedReports(
    requester: PermissionRequester,
    on_check_access: EventHandler<()>,
) -> Element {
    let today = chrono::Utc::now().date_naive();
    let mut from = use_signal(move || today.with_day(1).unwrap_or(today).to_string());
    let mut to = use_signal(move || today.to_string());
    let mut cursors = use_signal(|| vec![None::<TimeReportCursor>]);
    let mut page = use_resource(move || async move {
        let key = (from(), to(), cursors.read().last().cloned().flatten());
        let result = async {
            let (date_from, date_to) = period(&key.0, &key.1)?;
            let loaded = server_fns::list_visible_time_report_entries(TimeReportQuery {
                date_from,
                date_to,
                client_ids: vec![],
                project_ids: vec![],
                user_ids: vec![],
                task_ids: vec![],
                tag_ids: vec![],
                after: key.2.clone(),
                expected_requester: Some(requester),
            })
            .await
            .map_err(|_| "Could not load the report. Check your access and retry.")?;
            if loaded.requester != requester {
                return Err("Your session changed. Reload this page before viewing reports.");
            }
            Ok(loaded)
        }
        .await;
        (key, result)
    });
    let key = (from(), to(), cursors.read().last().cloned().flatten());
    let dates = period(&key.0, &key.1);
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let loaded = current
        .filter(|_| ready && dates.is_ok())
        .and_then(|(_, result)| result.as_ref().ok());
    let next = loaded.and_then(|page| page.next_after.clone());
    let download_query = loaded.and_then(|_| dates.ok()).map(|(date_from, date_to)| {
        format!("from={date_from}&to={date_to}&expected_org_id={}&expected_user_id={}&expected_policy=scoped", requester.org_id, requester.user_id)
    });
    let date_error_id = dates.err().map(|_| "report-date-error".to_string());

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "Reports" }
            div { class: "page-actions",
                if let Some(query) = download_query {
                    a { class: "btn btn-secondary", href: "/api/reports/export/csv?{query}", "Export CSV" }
                    a { class: "btn btn-secondary", href: "/api/reports/export/xlsx?{query}", "Export XLSX" }
                }
                button { id: "report-refresh", r#type: "button", class: "btn btn-secondary", disabled: !ready,
                    onclick: move |_| { if ready { page.restart(); } }, "Refresh report"
                }
                button { id: "reports-retry-access", r#type: "button", class: "btn btn-secondary",
                    onclick: move |_| on_check_access.call(()), "Refresh access"
                }
            }
        }
        div { class: "flex flex-wrap items-end gap-4 mb-6",
            FormGroup { label: "From", id: "report-from",
                Input { kind: "date", id: "report-from", value: from(), error_id: date_error_id.clone(),
                    oninput: move |event: FormEvent| { cursors.set(vec![None]); from.set(event.value()); }
                }
            }
            FormGroup { label: "To", id: "report-to",
                Input { kind: "date", id: "report-to", value: to(), error_id: date_error_id,
                    oninput: move |event: FormEvent| { cursors.set(vec![None]); to.set(event.value()); }
                }
            }
        }
        h2 { class: "text-lg mb-4", "Detailed time" }
        if let Err(message) = dates {
            p { id: "report-date-error", class: "alert alert-danger", role: "alert", "{message}" }
        } else if !ready {
            p { role: "status", class: "text-secondary", "Loading detailed report…" }
        } else if let Some((_, Err(message))) = current {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        if let Some(loaded) = loaded {
            dl { class: "flex flex-wrap gap-6 mb-6", aria_label: "Full-period totals",
                div { dt { "Entries" } dd { class: "text-mono m-0", "{loaded.totals.entry_count}" } }
                div { dt { "Total hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.total_minutes)}" } }
                div { dt { "Rounded hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.rounded_minutes)}" } }
                div { dt { "Billable hours" } dd { class: "text-mono m-0", "{hours(loaded.totals.billable_minutes)}" } }
            }
            if loaded.entries.is_empty() {
                p { role: "status", class: "text-secondary", "No entries on this page. Try another date range or return to the previous page." }
            } else {
                DataTable {
                    table {
                        thead { tr {
                            th { scope: "col", "Date" }
                            th { scope: "col", "Project" }
                            th { scope: "col", "Task" }
                            th { scope: "col", "Teammate" }
                            th { scope: "col", class: "text-right", "Hours" }
                            th { scope: "col", class: "text-right", "Rounded" }
                            th { scope: "col", class: "text-center", "Billable" }
                            th { scope: "col", "Notes" }
                        } }
                        tbody {
                            for entry in &loaded.entries {
                                tr { key: "{entry.id}",
                                    td { class: "text-mono whitespace-nowrap", "{entry.spent_date}" }
                                    td { "{entry.project_name}" }
                                    td { "{entry.task_name}" }
                                    td { "{entry.user_name}" }
                                    td { class: "text-mono text-right", "{hours(i64::from(entry.minutes))}" }
                                    td { class: "text-mono text-right", "{hours(i64::from(entry.rounded_minutes))}" }
                                    td { class: "text-center",
                                        if entry.billable { Badge { variant: "success", "Yes" } }
                                        else { Badge { variant: "neutral", "No" } }
                                    }
                                    td { class: "wrap-anywhere", "{entry.notes.as_deref().unwrap_or(\"—\")}" }
                                }
                            }
                        }
                    }
                }
            }
        }
        nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "Report pages",
            button { id: "report-previous", r#type: "button", class: "btn btn-secondary", disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if ready && cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
            }
            button { id: "report-next", r#type: "button", class: "btn btn-secondary", disabled: !ready || next.is_none(),
                onclick: move |_| { if ready && let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
            }
        }
    }
}
