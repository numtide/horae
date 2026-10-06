use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use horae_core::duration::format_hours2 as hours;

use crate::components::badge::Badge;
use crate::components::controls::Checkbox;
use crate::components::form::{FormGroup, Input, Select};
use crate::components::table::DataTable;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{
    TimeReportBillability, TimeReportCursor, TimeReportGroupCursor, TimeReportGroupQuery,
    TimeReportGrouping, TimeReportQuery,
};
use crate::server_fns;

#[path = "scoped/expanded.rs"]
mod expanded;
#[path = "scoped/grouped.rs"]
mod grouped;

#[derive(Clone, PartialEq)]
struct Selection {
    dimension: TimeReportGrouping,
    id: uuid::Uuid,
    name: String,
}

impl Selection {
    fn filter_key(&self) -> &'static str {
        match self.dimension {
            TimeReportGrouping::Client => "client_ids",
            TimeReportGrouping::Project => "project_ids",
            TimeReportGrouping::Task => "task_ids",
            TimeReportGrouping::Person => "user_ids",
        }
    }

    fn apply(&self, query: &mut TimeReportQuery) {
        let target = match self.dimension {
            TimeReportGrouping::Client => &mut query.client_ids,
            TimeReportGrouping::Project => &mut query.project_ids,
            TimeReportGrouping::Task => &mut query.task_ids,
            TimeReportGrouping::Person => &mut query.user_ids,
        };
        *target = vec![self.id];
    }

    fn apply_grouped(&self, query: &mut TimeReportGroupQuery) {
        let target = match self.dimension {
            TimeReportGrouping::Client => &mut query.client_ids,
            TimeReportGrouping::Project => &mut query.project_ids,
            TimeReportGrouping::Task => &mut query.task_ids,
            TimeReportGrouping::Person => &mut query.user_ids,
        };
        *target = vec![self.id];
    }

    fn initial_dimension(&self) -> TimeReportGrouping {
        match self.dimension {
            TimeReportGrouping::Project => TimeReportGrouping::Task,
            _ => TimeReportGrouping::Project,
        }
    }

    fn includes_tab(&self, dimension: TimeReportGrouping) -> bool {
        use TimeReportGrouping::*;
        matches!(
            (self.dimension, dimension),
            (Client, Project | Task | Person)
                | (Project, Task | Person)
                | (Task, Project | Person)
                | (Person, Project | Task)
        )
    }

    fn expansion(&self, dimension: TimeReportGrouping) -> Option<TimeReportGrouping> {
        use TimeReportGrouping::*;
        match (self.dimension, dimension) {
            (Client | Project, Task) => Some(Person),
            (Client, Person) => Some(Project),
            (Project, Person) => Some(Task),
            _ => None,
        }
    }
}

fn period(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate), &'static str> {
    let from = from.parse().map_err(|_| "Enter a valid start date.")?;
    let to = to.parse().map_err(|_| "Enter a valid end date.")?;
    if from > to {
        return Err("The end date must be on or after the start date.");
    }
    Ok((from, to))
}

fn billability_value(value: TimeReportBillability) -> &'static str {
    match value {
        TimeReportBillability::All => "all",
        TimeReportBillability::Billable => "billable",
        TimeReportBillability::NonBillable => "non_billable",
    }
}

#[component]
pub(super) fn ScopedReports(
    requester: PermissionRequester,
    on_check_access: EventHandler<()>,
) -> Element {
    let today = chrono::Utc::now().date_naive();
    let mut from = use_signal(move || today.with_day(1).unwrap_or(today).to_string());
    let mut to = use_signal(move || today.to_string());
    let mut show_groups = use_signal(|| false);
    let mut active_projects_only = use_signal(|| false);
    let mut billability = use_signal(|| TimeReportBillability::All);
    let mut group_dimension = use_signal(|| TimeReportGrouping::Client);
    let mut group_context = use_signal(|| None::<Selection>);
    let mut group_cursors = use_signal(|| vec![None::<TimeReportGroupCursor>]);
    let mut selection = use_signal(Vec::<Selection>::new);
    let mut cursors = use_signal(|| vec![None::<TimeReportCursor>]);
    let mut page = use_resource(move || async move {
        let key = (
            from(),
            to(),
            cursors.read().last().cloned().flatten(),
            selection(),
            show_groups(),
            active_projects_only(),
            billability(),
        );
        let result = async {
            if key.4 {
                return Ok(None);
            }
            let (date_from, date_to) = period(&key.0, &key.1)?;
            let mut query = TimeReportQuery {
                date_from,
                date_to,
                active_projects_only: key.5,
                billability: key.6,
                client_ids: vec![],
                project_ids: vec![],
                user_ids: vec![],
                task_ids: vec![],
                tag_ids: vec![],
                after: key.2.clone(),
                expected_requester: Some(requester),
            };
            for selected in &key.3 {
                selected.apply(&mut query);
            }
            let loaded = server_fns::list_visible_time_report_entries(query)
                .await
                .map_err(|_| "Could not load the report. Check your access and retry.")?;
            if loaded.requester != requester {
                return Err("Your session changed. Reload this page before viewing reports.");
            }
            Ok(Some(loaded))
        }
        .await;
        (key, result)
    });
    let key = (
        from(),
        to(),
        cursors.read().last().cloned().flatten(),
        selection(),
        show_groups(),
        active_projects_only(),
        billability(),
    );
    let dates = period(&key.0, &key.1);
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let loaded = current
        .filter(|_| ready && dates.is_ok() && !show_groups())
        .and_then(|(_, result)| result.as_ref().ok())
        .and_then(Option::as_ref);
    let next = loaded.and_then(|page| page.next_after.clone());
    let download_query = loaded.and_then(|_| dates.ok()).map(|(date_from, date_to)| {
        let filter: String = key.3.iter().map(|selected| format!("&{}={}", selected.filter_key(), selected.id)).collect();
        format!("from={date_from}&to={date_to}&active_projects_only={}&billability={}&expected_org_id={}&expected_user_id={}&expected_policy=scoped{filter}", key.5, billability_value(key.6), requester.org_id, requester.user_id)
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
                if !show_groups() {
                    button { id: "report-refresh", r#type: "button", class: "btn btn-secondary", disabled: !ready,
                        onclick: move |_| { if ready { page.restart(); } }, "Refresh report"
                    }
                }
                button { id: "reports-retry-access", r#type: "button", class: "btn btn-secondary",
                    onclick: move |_| on_check_access.call(()), "Refresh access"
                }
            }
        }
        div { class: "segmented flex-wrap mb-6", role: "group", aria_label: "Report views",
            button { id: "report-view-time", r#type: "button",
                class: if show_groups() { "segmented-item active" } else { "segmented-item" }, aria_pressed: show_groups(),
                onclick: move |_| {
                    selection.set(vec![]); cursors.set(vec![None]);
                    if group_context().is_some() { group_dimension.set(TimeReportGrouping::Client); }
                    group_context.set(None); group_cursors.set(vec![None]); show_groups.set(true);
                }, "Time"
            }
            button { id: "report-view-detailed", r#type: "button",
                class: if !show_groups() { "segmented-item active" } else { "segmented-item" }, aria_pressed: !show_groups(),
                onclick: move |_| { selection.set(vec![]); cursors.set(vec![None]); show_groups.set(false); }, "Detailed time"
            }
        }
        div { class: "flex flex-wrap items-end gap-4 mb-6",
            FormGroup { label: "From", id: "report-from",
                Input { kind: "date", id: "report-from", value: from(), error_id: date_error_id.clone(),
                    oninput: move |event: FormEvent| { cursors.set(vec![None]); group_cursors.set(vec![None]); from.set(event.value()); }
                }
            }
            FormGroup { label: "To", id: "report-to",
                Input { kind: "date", id: "report-to", value: to(), error_id: date_error_id,
                    oninput: move |event: FormEvent| { cursors.set(vec![None]); group_cursors.set(vec![None]); to.set(event.value()); }
                }
            }
            FormGroup { label: "Show", id: "report-billability",
                Select {
                    id: "report-billability",
                    selected: billability_value(billability()),
                    options: vec![
                        ("all".into(), "All hours".into()),
                        ("billable".into(), "Billable hours".into()),
                        ("non_billable".into(), "Non-billable hours".into()),
                    ],
                    onchange: move |event: FormEvent| {
                        let next = match event.value().as_str() {
                            "all" => TimeReportBillability::All,
                            "billable" => TimeReportBillability::Billable,
                            "non_billable" => TimeReportBillability::NonBillable,
                            _ => return,
                        };
                        cursors.set(vec![None]); group_cursors.set(vec![None]); billability.set(next);
                    }
                }
            }
        }
        div { class: "mb-4",
            Checkbox { id: "report-active-projects-only", label: "Active projects only", checked: active_projects_only(),
                onclick: move |_| {
                    let next = !active_projects_only();
                    cursors.set(vec![None]); group_cursors.set(vec![None]); active_projects_only.set(next);
                }
            }
        }
        if !show_groups() { h2 { class: "text-lg mb-4", "Detailed time" } }
        if let Err(message) = dates {
            p { id: "report-date-error", class: "alert alert-danger", role: "alert", "{message}" }
        } else if show_groups() {
            grouped::GroupedTimeReport {
                requester, from, to, active_projects_only, billability, dimension: group_dimension, cursors: group_cursors, context: group_context,
                on_detail: move |selected| { selection.set(selected); cursors.set(vec![None]); show_groups.set(false); }
            }
        } else if !ready {
            p { role: "status", class: "text-secondary", "Loading detailed report…" }
        } else if let Some((_, Err(message))) = current {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        if let Some(loaded) = loaded {
            if !key.3.is_empty() {
                div { class: "flex flex-wrap items-center gap-3 mb-4",
                    for selected in &key.3 { p { class: "m-0 wrap-anywhere", "{selected.name}" } }
                    button { id: "report-clear-selection", r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| { selection.set(vec![]); cursors.set(vec![None]); }, "Clear selection"
                    }
                }
            }
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
        if !show_groups() { nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "Report pages",
            button { id: "report-previous", r#type: "button", class: "btn btn-secondary", disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if ready && cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
            }
            button { id: "report-next", r#type: "button", class: "btn btn-secondary", disabled: !ready || next.is_none(),
                onclick: move |_| { if ready && let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
            }
        } }
    }
}
