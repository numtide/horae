use chrono::Datelike;
use dioxus::prelude::*;
// The report/export convention for hours (e.g. 90 → "1.50"), shared with the
// CSV exporter.
use horae_core::duration::format_hours2 as hours;
use horae_core::money::format_cents as money;

use super::{is_manager, loaded};
use crate::components::badge::Badge;
use crate::components::form::{FormGroup, Input};
use crate::components::table::DataTable;
use crate::server_fns;

/// Do not add monetary values until their currencies have been checked.
fn money_total(
    rows: &[crate::models::ReportRow],
    amount: fn(&crate::models::ReportRow) -> i64,
) -> String {
    let Some(first) = rows.first() else {
        return "\u{2014}".into();
    };
    if rows.iter().any(|r| r.currency != first.currency) {
        return "\u{2014}".into();
    }
    match rows
        .iter()
        .try_fold(0i64, |sum, row| sum.checked_add(amount(row)))
    {
        Some(total) => money(total, &first.currency),
        None => "Amount exceeds supported range".into(),
    }
}

/// `Some(v)` for a non-empty filter selection, `None` for "all".
fn opt(v: String) -> Option<String> {
    if v.is_empty() { None } else { Some(v) }
}

/// A labelled dropdown for one report filter: a leading "all" option followed by
/// `(value, name)` choices. Emits the picked value ("" for "all").
#[component]
fn FilterSelect(
    label: String,
    value: String,
    all_label: String,
    options: Vec<(String, String)>,
    onselect: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "form-group",
            label { class: "form-label", "{label}" }
            select {
                class: "form-select",
                value: "{value}",
                oninput: move |e| onselect.call(e.value()),
                option { value: "", "{all_label}" }
                for (v, name) in options.iter() {
                    option { value: "{v}", "{name}" }
                }
            }
        }
    }
}

#[component]
pub fn Reports() -> Element {
    let today = chrono::Utc::now().date_naive();
    let month_start = today.with_day(1).unwrap_or(today);

    let mut from_date = use_signal(move || month_start.to_string());
    let mut to_date = use_signal(move || today.to_string());
    let mut group_by = use_signal(|| "project".to_string());
    let mut client_filter = use_signal(String::new);
    let mut project_filter = use_signal(String::new);
    let mut user_filter = use_signal(String::new);
    let mut active_tab = use_signal(|| "time".to_string());

    let me = use_resource(|| async move { server_fns::get_me().await });

    // Dropdown sources. Projects narrow to the chosen client.
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let users = use_resource(|| async move { server_fns::list_users(false).await });
    let projects = use_resource(move || {
        let c = opt(client_filter.read().clone());
        async move { server_fns::list_projects(c, false).await }
    });

    // Read the signals inside each resource so a filter change re-loads.
    let summary = use_resource(move || {
        let (f, t, g) = (
            from_date.read().clone(),
            to_date.read().clone(),
            group_by.read().clone(),
        );
        let (cl, pr, us) = (
            opt(client_filter.read().clone()),
            opt(project_filter.read().clone()),
            opt(user_filter.read().clone()),
        );
        async move { server_fns::report_time(f, t, g, cl, pr, us).await }
    });
    let detailed = use_resource(move || {
        let (f, t) = (from_date.read().clone(), to_date.read().clone());
        let (cl, pr, us) = (
            opt(client_filter.read().clone()),
            opt(project_filter.read().clone()),
            opt(user_filter.read().clone()),
        );
        async move { server_fns::report_detailed(f, t, cl, pr, us).await }
    });

    // Reports cover every user's time and money, so the endpoints are
    // manager-only (SPEC §6). Mirror the Approvals page: keep the rail link for
    // everyone and show a notice here instead of a wall of errors.
    let is_manager = is_manager(&me);
    if !is_manager {
        return rsx! {
            div {
                div { class: "page-header",
                    h1 { class: "page-title", "Reports" }
                }
                div { class: "card p-8 text-center",
                    p { class: "text-muted", "Manager or admin access is required to view reports." }
                }
            }
        };
    }

    // The export must match what the tables show, so the active filters ride
    // along in the query string; unset ones are left out and mean "all".
    let export_query = {
        let mut q = format!("from={}&to={}", from_date.read(), to_date.read());
        for (name, value) in [
            ("client_id", client_filter.read().clone()),
            ("project_id", project_filter.read().clone()),
            ("user_id", user_filter.read().clone()),
        ] {
            if !value.is_empty() {
                q.push_str(&format!("&{name}={value}"));
            }
        }
        q
    };
    let export_csv_url = format!("/api/reports/export/csv?{export_query}");
    let export_xlsx_url = format!("/api/reports/export/xlsx?{export_query}");

    let client_opts: Vec<(String, String)> = clients
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|l| {
            l.iter()
                .map(|c| (c.id.to_string(), c.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    let project_opts: Vec<(String, String)> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|l| {
            l.iter()
                .map(|p| (p.id.to_string(), p.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    let user_opts: Vec<(String, String)> = users
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|l| {
            l.iter()
                .map(|u| (u.id.to_string(), u.name.clone()))
                .collect()
        })
        .unwrap_or_default();

    let tab = active_tab.read().clone();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Reports" }
                div { class: "page-actions",
                    a { class: "btn btn-secondary", href: "{export_csv_url}", "Export CSV" }
                    a { class: "btn btn-secondary", href: "{export_xlsx_url}", "Export XLSX" }
                }
            }

            div { class: "card mb-6",
                div { class: "flex gap-4 items-end flex-wrap",
                    FormGroup { label: "From",
                        Input {
                            kind: "date",
                            value: "{from_date}",
                            oninput: move |e: FormEvent| from_date.set(e.value()),
                        }
                    }
                    FormGroup { label: "To",
                        Input {
                            kind: "date",
                            value: "{to_date}",
                            oninput: move |e: FormEvent| to_date.set(e.value()),
                        }
                    }
                    div { class: "form-group",
                        label { class: "form-label", "Group by" }
                        select {
                            class: "form-select",
                            value: "{group_by}",
                            oninput: move |e| group_by.set(e.value()),
                            option { value: "project", "Project" }
                            option { value: "client", "Client" }
                            option { value: "task", "Task" }
                            option { value: "person", "Person" }
                        }
                    }
                    FilterSelect {
                        label: "Client",
                        value: client_filter(),
                        all_label: "All clients",
                        options: client_opts,
                        onselect: move |v| {
                            client_filter.set(v);
                            project_filter.set(String::new());
                        },
                    }
                    FilterSelect {
                        label: "Project",
                        value: project_filter(),
                        all_label: "All projects",
                        options: project_opts,
                        onselect: move |v| project_filter.set(v),
                    }
                    FilterSelect {
                        label: "Teammate",
                        value: user_filter(),
                        all_label: "All teammates",
                        options: user_opts,
                        onselect: move |v| user_filter.set(v),
                    }
                }
            }

            div { class: "report-tabs flex items-center gap-6 mb-6",
                button {
                    class: if tab == "time" { "report-tab active" } else { "report-tab" },
                    onclick: move |_| active_tab.set("time".into()),
                    "Time"
                }
                button {
                    class: if tab == "detailed" { "report-tab active" } else { "report-tab" },
                    onclick: move |_| active_tab.set("detailed".into()),
                    "Detailed time"
                }
            }

            if tab == "time" {
                {loaded(&*summary.read(), |rows| {
                        if rows.is_empty() {
                            return rsx! {
                                div { class: "card p-8 text-center",
                                    p { class: "text-muted", "No time tracked in this range." }
                                }
                            };
                        }
                        let grand_total: i64 = rows.iter().map(|r| r.total_minutes).sum();
                        let grand_rounded: i64 = rows.iter().map(|r| r.rounded_minutes).sum();
                        let grand_billable: i64 = rows.iter().map(|r| r.billable_minutes).sum();
                        let grand_bill_amount = money_total(rows, |r| r.billable_cents);
                        let grand_cost_amount = money_total(rows, |r| r.cost_cents);
                        rsx! {
                            DataTable {
                                table {
                                    thead {
                                        tr {
                                            th { "Group" }
                                            th { class: "text-right", "Total hours" }
                                            th { class: "text-right", "Rounded" }
                                            th { class: "text-right", "Billable hrs" }
                                            th { class: "text-right", "Billable amount" }
                                            th { class: "text-right", "Cost" }
                                        }
                                    }
                                    tbody {
                                        for row in rows.iter() {
                                            tr { key: "{row.group_id}:{row.currency}",
                                                td { "{row.label}" }
                                                td { class: "text-mono text-right", "{hours(row.total_minutes)}" }
                                                td { class: "text-mono text-right", "{hours(row.rounded_minutes)}" }
                                                td { class: "text-mono text-right", "{hours(row.billable_minutes)}" }
                                                td { class: "text-mono text-right", "{money(row.billable_cents, &row.currency)}" }
                                                td { class: "text-mono text-right", "{money(row.cost_cents, &row.currency)}" }
                                            }
                                        }
                                        tr { class: "report-total-row",
                                            td { "Total" }
                                            td { class: "text-mono text-right", "{hours(grand_total)}" }
                                            td { class: "text-mono text-right", "{hours(grand_rounded)}" }
                                            td { class: "text-mono text-right", "{hours(grand_billable)}" }
                                            td { class: "text-mono text-right", "{grand_bill_amount}" }
                                            td { class: "text-mono text-right", "{grand_cost_amount}" }
                                        }
                                    }
                                }
                            }
                        }
                })}
            }

            if tab == "detailed" {
                {loaded(&*detailed.read(), |entries| {
                    if entries.is_empty() {
                        return rsx! {
                            div { class: "card p-8 text-center",
                                p { class: "text-muted", "No entries in this range." }
                            }
                        };
                    }
                    rsx! {
                        DataTable {
                            table {
                                thead {
                                    tr {
                                        th { "Date" }
                                        th { "Project" }
                                        th { "Task" }
                                        th { "Teammate" }
                                        th { class: "text-right", "Hours" }
                                        th { class: "text-right", "Rounded" }
                                        th { class: "text-center", "Billable" }
                                        th { "Notes" }
                                    }
                                }
                                tbody {
                                    for (i, e) in entries.iter().enumerate() {
                                        tr { key: "{i}",
                                            td { class: "text-mono", "{e.spent_date}" }
                                            td { "{e.project_name}" }
                                            td { "{e.task_name}" }
                                            td { "{e.user_name}" }
                                            td { class: "text-mono text-right", "{hours(e.minutes as i64)}" }
                                            td { class: "text-mono text-right",
                                                "{hours(e.rounded_minutes.unwrap_or(e.minutes) as i64)}"
                                            }
                                            td { class: "text-center",
                                                if e.billable {
                                                    Badge { variant: "success", "Yes" }
                                                } else {
                                                    Badge { variant: "neutral", "No" }
                                                }
                                            }
                                            td { "{e.notes.as_deref().unwrap_or(\"\u{2014}\")}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                })}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReportRow;

    fn row(currency: &str, cents: i64) -> ReportRow {
        ReportRow {
            group_id: uuid::Uuid::nil(),
            label: "Same label".into(),
            total_minutes: 60,
            rounded_minutes: 60,
            billable_minutes: 60,
            billable_cents: cents,
            cost_cents: 0,
            currency: currency.into(),
        }
    }

    #[test]
    fn mixed_currencies_are_not_added_even_when_the_numbers_would_overflow() {
        let rows = [row("EUR", i64::MAX), row("USD", i64::MAX)];
        assert_eq!(money_total(&rows, |r| r.billable_cents), "\u{2014}");
    }

    #[test]
    fn monetary_total_overflow_is_visible_instead_of_panicking() {
        let rows = [row("EUR", i64::MAX), row("EUR", 1)];
        assert_eq!(
            money_total(&rows, |r| r.billable_cents),
            "Amount exceeds supported range"
        );
    }

    #[test]
    fn single_currency_totals_are_formatted_and_empty_reports_have_no_amount() {
        let rows = [row("EUR", 100), row("EUR", 250)];
        assert_eq!(money_total(&rows, |r| r.billable_cents), money(350, "EUR"));
        assert_eq!(money_total(&[], |r| r.billable_cents), "\u{2014}");
    }
}
