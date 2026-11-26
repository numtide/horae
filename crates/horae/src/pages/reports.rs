use chrono::Datelike;
use dioxus::prelude::*;
// The report/export convention for hours (e.g. 90 → "1.50"), shared with the
// CSV exporter.
use horae_core::duration::format_hours2 as hours;
use horae_core::money::format_cents as money;
use horae_core::money::{Withheld, sum_by_currency};

use super::{Failure, loaded};
use crate::components::badge::Badge;
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Segmented;
use crate::components::debounce::{SEARCH_DEBOUNCE, use_debounced};
use crate::components::filter_bar::FilterBar;
use crate::components::form::Input;
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, ErrorState, Loading};
use crate::components::table::DataTable;
use crate::i18n::Tr;
use crate::server_fns;

/// The Reports title, where focus lands once a failed report loads again.
const REPORTS_HEADING: &str = "reports-heading";

#[path = "reports/scoped.rs"]
mod scoped;

/// One line per currency; a single withheld amount hides them all.
fn money_totals(
    rows: &[crate::models::ReportRow],
    amount: fn(&crate::models::ReportRow) -> Option<i64>,
    currency: fn(&crate::models::ReportRow) -> &str,
) -> Vec<String> {
    if rows.is_empty() {
        return vec!["\u{2014}".into()];
    }
    match sum_by_currency(rows.iter().map(|row| (currency(row), amount(row)))) {
        Err(Withheld) => vec!["Restricted".into()],
        Ok(totals) => totals
            .into_iter()
            .map(|(code, total)| match total {
                Some(cents) => money(cents, code),
                None => format!("{code} amount exceeds supported range"),
            })
            .collect(),
    }
}

/// `Some(v)` for a non-empty filter selection, `None` for "all".
fn opt(v: String) -> Option<String> {
    if v.is_empty() { None } else { Some(v) }
}

/// `(value, name)` choices of a report filter, for its `Combobox`.
fn combo(options: Vec<(String, String)>) -> Vec<ComboOption> {
    options
        .into_iter()
        .map(|(value, label)| ComboOption {
            value,
            label,
            group: None,
        })
        .collect()
}

/// The time report's groupings: (the server's value, the label).
fn group_choices(tr: Tr) -> [(&'static str, String); 4] {
    [
        ("project", tr.reports_group_project()),
        ("client", tr.reports_group_client()),
        ("task", tr.reports_group_task()),
        ("person", tr.reports_group_person()),
    ]
}

#[component]
pub fn Reports() -> Element {
    use crate::models::time_report::TimeReportAccess;

    // Retain identity and view when refreshing access unmounts the result view.
    let mut binding = use_signal(|| None::<TimeReportAccess>);
    let mut access = use_resource(move || async move {
        let expected = *binding.peek();
        let result = server_fns::get_time_report_access(expected.map(|value| value.requester))
            .await
            .map_err(|_| "Report access is unavailable. Try again, or sign in again if it keeps failing.")?;
        if expected.is_some_and(|expected| expected != result) {
            return Err("Your session or report access changed. Reload this page to continue.");
        }
        binding.set(Some(result));
        Ok(result)
    });
    let retry_access = use_callback(move |_| access.restart());
    if access.state()() != UseResourceState::Ready {
        return rsx! { Loading { label: "Loading report access…" } };
    }
    match &*access.read() {
        Some(Ok(access)) if access.financial => rsx! { FinancialReports {} },
        Some(Ok(allowed)) => rsx! {
            scoped::ScopedReports {
                requester: allowed.requester,
                on_check_access: move |_| access.restart(),
            }
        },
        response => {
            let message = response
                .as_ref()
                .and_then(|result| result.as_ref().err())
                .copied()
                .unwrap_or("Report access could not be checked. Try again, or sign in again if it keeps failing.");
            rsx! {
                PageHeader { title: "Reports" }
                ErrorState {
                    title: "Could not check report access",
                    body: message,
                    retry_id: "reports-retry-access",
                    onretry: retry_access,
                }
            }
        }
    }
}

#[component]
fn FinancialReports() -> Element {
    let tr = crate::i18n::use_tr();
    let today = crate::components::today::today();
    let month_start = today.with_day(1).unwrap_or(today);

    // Typing a date edits one segment at a time, and each edit is a valid date,
    // so the reports load once typing pauses. Until then they show as loading:
    // a table for other dates must not sit beside the typed ones.
    let mut from_input = use_signal(move || month_start.to_string());
    let mut to_input = use_signal(move || today.to_string());
    let from_date = use_debounced(from_input, SEARCH_DEBOUNCE);
    let to_date = use_debounced(to_input, SEARCH_DEBOUNCE);
    let dates_settled = use_memo(move || from_date() == from_input() && to_date() == to_input());
    let mut group_by = use_signal(|| "project".to_string());
    let mut client_filter = use_signal(String::new);
    let mut project_filter = use_signal(String::new);
    let mut user_filter = use_signal(String::new);
    let mut tag_filter = use_signal(String::new);
    let mut active_tab = use_signal(|| "time".to_string());
    // A client's projects replace the list, so its project choice resets.
    let pick_client = use_callback(move |value: String| {
        client_filter.set(value);
        project_filter.set(String::new());
    });
    let pick_project = use_callback(move |value: String| project_filter.set(value));
    let pick_teammate = use_callback(move |value: String| user_filter.set(value));
    let pick_group = use_callback(move |picked: String| {
        if let Some((value, _)) = group_choices(tr)
            .into_iter()
            .find(|(_, label)| *label == picked)
        {
            group_by.set(value.to_string());
        }
    });

    // Dropdown sources. Projects narrow to the chosen client.
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let users = use_resource(|| async move { server_fns::list_users(false).await });
    let mut tags = use_resource(|| async move { server_fns::list_project_tags(None).await });
    let projects = use_resource(move || {
        let c = opt(client_filter.read().clone());
        async move { server_fns::list_projects(c, false).await }
    });

    let filters = FinancialFilters {
        from: from_date,
        to: to_date,
        client: client_filter,
        project: project_filter,
        user: user_filter,
        tag: tag_filter,
        dates_settled,
    };

    // The export must match what the tables show, so the active filters ride
    // along in the query string; unset ones are left out and mean "all".
    let export_query = {
        let mut q = format!("from={}&to={}", from_input.read(), to_input.read());
        for (name, value) in [
            ("client_id", client_filter.read().clone()),
            ("project_id", project_filter.read().clone()),
            ("user_id", user_filter.read().clone()),
            ("tag_id", tag_filter()),
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
    let mut seen_tags = std::collections::BTreeSet::new();
    let tag_options = tags
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .into_iter()
        .flatten()
        .filter(|tag| seen_tags.insert(tag.tag_id))
        .map(|tag| (tag.tag_id.to_string(), tag.name.clone()))
        .collect::<Vec<_>>();
    let tag_label = if tag_filter().is_empty() {
        tr.reports_tag_any()
    } else {
        tag_options
            .iter()
            .find(|(id, _)| *id == tag_filter())
            .map(|(_, name)| tr.reports_tag_named(name))
            .unwrap_or_else(|| tr.reports_tag_unavailable())
    };
    let groups = group_choices(tr);

    rsx! {
        div {
            PageHeader { title: "Reports", title_id: REPORTS_HEADING,
                actions: rsx! {
                    a { class: "btn btn-secondary", href: "{export_csv_url}", "Export CSV" }
                    a { class: "btn btn-secondary", href: "{export_xlsx_url}", "Export XLSX" }
                },
            }

            FilterBar {
                filters: rsx! {
                    div { class: "flex flex-wrap items-center gap-2 min-w-0 max-w-full",
                        label { class: "text-sm text-secondary", r#for: "report-filter-from", {tr.reports_from()} }
                        Input {
                            kind: "date",
                            id: "report-filter-from",
                            class: "w-auto min-w-0 max-w-full min-h-control",
                            value: "{from_input}",
                            oninput: move |e: FormEvent| from_input.set(e.value()),
                        }
                        label { class: "text-sm text-secondary", r#for: "report-filter-to", {tr.reports_to()} }
                        Input {
                            kind: "date",
                            id: "report-filter-to",
                            class: "w-auto min-w-0 max-w-full min-h-control",
                            value: "{to_input}",
                            oninput: move |e: FormEvent| to_input.set(e.value()),
                        }
                    }
                    Combobox {
                        id: "report-filter-client",
                        name: tr.reports_client(),
                        trigger_class: "text-sm px-4",
                        options: combo(client_opts),
                        value: client_filter(),
                        placeholder: tr.reports_client_any(),
                        all_label: tr.reports_client_any(),
                        onselect: pick_client,
                    }
                    Combobox {
                        id: "report-filter-project",
                        name: tr.reports_project(),
                        trigger_class: "text-sm px-4",
                        options: combo(project_opts),
                        value: project_filter(),
                        placeholder: tr.reports_project_any(),
                        all_label: tr.reports_project_any(),
                        onselect: pick_project,
                    }
                    Combobox {
                        id: "report-filter-teammate",
                        name: tr.reports_teammate(),
                        trigger_class: "text-sm px-4",
                        options: combo(user_opts),
                        value: user_filter(),
                        placeholder: tr.reports_teammate_any(),
                        all_label: tr.reports_teammate_any(),
                        onselect: pick_teammate,
                    }
                    if !tag_options.is_empty() || !tag_filter().is_empty() {
                        Menu { id: "report-tag", label: tag_label, trigger_class: "text-sm px-4",
                            MenuItem { selected: tag_filter().is_empty(), onclick: move |_| tag_filter.set(String::new()), {tr.reports_tag_any()} }
                            for (id, name) in tag_options {
                                MenuItem { key: "{id}", selected: tag_filter() == id,
                                    onclick: { let id = id.clone(); move |_| tag_filter.set(id.clone()) },
                                    "{name}"
                                }
                            }
                        }
                    }
                },
            }

            if matches!(&*tags.read(), Some(Err(_))) {
                div { class: "alert alert-danger", role: "alert", "Could not load project tags. "
                    button { class: "btn btn-secondary btn-sm", onclick: move |_| tags.restart(), "Try again" }
                }
            }

            div { class: "report-tabs flex flex-wrap items-center gap-6 mb-6",
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
                // Grouping switches the time report's view, so it is a Segmented.
                if tab == "time" {
                    div { class: "flex-1" }
                    div { class: "flex items-center gap-3",
                        span { id: "report-group-label", class: "text-sm text-secondary", {tr.reports_group_by()} }
                        Segmented { label_id: Some("report-group-label".to_string()), items: groups.iter().map(|(_, label)| label.clone()).collect(),
                            active: groups.iter().find(|(value, _)| *value == group_by()).map(|(_, label)| label.clone()).unwrap_or_default(),
                            onselect: pick_group,
                        }
                    }
                }
            }

            // Only the visible tab is mounted, so a filter change loads one report.
            if tab == "time" {
                FinancialTimeReport { filters, group_by }
            } else {
                FinancialDetailedReport { filters }
            }
        }
    }
}

/// The filters both financial report tabs share; "" means all.
#[derive(Clone, Copy, PartialEq)]
struct FinancialFilters {
    from: ReadSignal<String>,
    to: ReadSignal<String>,
    client: Signal<String>,
    project: Signal<String>,
    user: Signal<String>,
    tag: Signal<String>,
    /// False while the typed dates are ahead of `from`/`to`.
    dates_settled: Memo<bool>,
}

impl FinancialFilters {
    /// (from, to, client, project, user, tag), read so a change re-loads.
    #[allow(clippy::type_complexity)]
    fn read(
        &self,
    ) -> (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            self.from.read().clone(),
            self.to.read().clone(),
            opt(self.client.read().clone()),
            opt(self.project.read().clone()),
            opt(self.user.read().clone()),
            opt(self.tag.read().clone()),
        )
    }
}

#[component]
fn FinancialTimeReport(filters: FinancialFilters, group_by: Signal<String>) -> Element {
    let tr = crate::i18n::use_tr();
    let mut summary = use_resource(move || {
        let (f, t, cl, pr, us, tag) = filters.read();
        let g = group_by.read().clone();
        async move { server_fns::report_time(f, t, g, cl, pr, us, tag).await }
    });
    let retry = use_callback(move |_| summary.restart());
    let pending = summary.state()() != UseResourceState::Ready;
    // A failed report stays up while its retry runs, so focus keeps its button.
    let failed = matches!(&*summary.read(), Some(Err(_)));
    rsx! {
        if !failed && (pending || !(filters.dates_settled)()) {
            Loading { label: "Loading report…" }
        } else {
            {loaded(&*summary.read(), Failure { what: "the report", retry, pending, inset: false, focus: Some(REPORTS_HEADING) }, |rows| {
                if rows.is_empty() {
                    return rsx! {
                        EmptyState { icon: "reports", title: "No time in this range", body: "Pick other dates or clear a filter to see tracked time." }
                    };
                }
                let grand_total: i64 = rows.iter().map(|r| r.total_minutes).sum();
                let grand_rounded: i64 = rows.iter().map(|r| r.rounded_minutes).sum();
                let grand_billable: i64 = rows.iter().map(|r| r.billable_minutes).sum();
                let grand_bill_amount = money_totals(rows, |r| r.billable_cents, |r| &r.currency);
                let grand_cost_amount = money_totals(rows, |r| r.cost_cents, |r| &r.cost_currency);
                rsx! {
                    DataTable { label: tr.reports_time_label(),
                        table { class: "name-table", aria_label: tr.reports_time_label(),
                            thead {
                                tr {
                                    th { scope: "col", "Group" }
                                    th { scope: "col", class: "text-right", "Total hours" }
                                    th { scope: "col", class: "text-right", "Rounded" }
                                    th { scope: "col", class: "text-right", "Billable hrs" }
                                    th { scope: "col", class: "text-right", "Billable amount" }
                                    th { scope: "col", class: "text-right", "Cost" }
                                }
                            }
                            tbody {
                                for row in rows.iter() {
                                    tr { key: "{row.group_id}:{row.currency}",
                                        td { "{row.label}" }
                                        td { class: "text-mono text-right", "{hours(row.total_minutes)}" }
                                        td { class: "text-mono text-right", "{hours(row.rounded_minutes)}" }
                                        td { class: "text-mono text-right", "{hours(row.billable_minutes)}" }
                                        td { class: "text-mono text-right",
                                            {row.billable_cents.map(|cents| money(cents, &row.currency)).unwrap_or_else(|| "Restricted".into())}
                                        }
                                        td { class: "text-mono text-right",
                                            {row.cost_cents.map(|cents| money(cents, &row.cost_currency)).unwrap_or_else(|| "Restricted".into())}
                                        }
                                    }
                                }
                            }
                            tfoot {
                                tr { class: "font-semibold",
                                    th { scope: "row", "Total" }
                                    td { class: "text-mono text-right", "{hours(grand_total)}" }
                                    td { class: "text-mono text-right", "{hours(grand_rounded)}" }
                                    td { class: "text-mono text-right", "{hours(grand_billable)}" }
                                    td { class: "text-mono text-right",
                                        for line in grand_bill_amount { div { "{line}" } }
                                    }
                                    td { class: "text-mono text-right",
                                        for line in grand_cost_amount { div { "{line}" } }
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

#[component]
fn FinancialDetailedReport(filters: FinancialFilters) -> Element {
    let tr = crate::i18n::use_tr();
    let mut detailed = use_resource(move || {
        let (f, t, cl, pr, us, tag) = filters.read();
        async move { server_fns::report_detailed(f, t, cl, pr, us, tag).await }
    });
    let retry = use_callback(move |_| detailed.restart());
    let pending = detailed.state()() != UseResourceState::Ready;
    let failed = matches!(&*detailed.read(), Some(Err(_)));
    rsx! {
        if !failed && (pending || !(filters.dates_settled)()) {
            Loading { label: "Loading detailed report…" }
        } else {
            {loaded(&*detailed.read(), Failure { what: "the detailed report", retry, pending, inset: false, focus: Some(REPORTS_HEADING) }, |entries| {
                if entries.is_empty() {
                    return rsx! {
                        EmptyState { icon: "reports", title: "No entries in this range", body: "Pick other dates or clear a filter to see time entries." }
                    };
                }
                rsx! {
                    DataTable { label: tr.reports_detailed_label(),
                        table { aria_label: tr.reports_detailed_label(),
                            thead {
                                tr {
                                    th { scope: "col", "Date" }
                                    th { scope: "col", "Project" }
                                    th { scope: "col", "Task" }
                                    th { scope: "col", "Teammate" }
                                    th { scope: "col", class: "text-right", "Hours" }
                                    th { scope: "col", class: "text-right", "Rounded" }
                                    th { scope: "col", class: "text-center", "Billable" }
                                    th { scope: "col", "Notes" }
                                }
                            }
                            tbody {
                                // The rows carry no entry ID and are replaced
                                // wholesale on every load; they hold no state.
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
            billable_cents: Some(cents),
            cost_cents: Some(0),
            currency: currency.into(),
            cost_currency: "EUR".into(),
        }
    }

    fn billable(rows: &[ReportRow]) -> Vec<String> {
        money_totals(rows, |r| r.billable_cents, |r| &r.currency)
    }

    fn cost(rows: &[ReportRow]) -> Vec<String> {
        money_totals(rows, |r| r.cost_cents, |r| &r.cost_currency)
    }

    #[test]
    fn redacted_costs_do_not_become_zero_or_partial_totals() {
        let mut payload = serde_json::to_value(row("EUR", 100)).unwrap();
        payload.as_object_mut().unwrap().remove("cost_cents");
        let restricted: ReportRow = serde_json::from_value(payload).unwrap();
        let rows = [row("EUR", 250), restricted];
        assert_eq!(cost(&rows), ["Restricted"]);
    }

    #[test]
    fn a_withheld_cost_hides_every_currency_and_out_of_range_total() {
        let mut rows = [row("EUR", 1), row("USD", 2), row("CHF", 3), row("CHF", 4)];
        // Two currencies, one already out of range, before the withheld row.
        rows[0].cost_currency = "USD".into();
        rows[1].cost_cents = Some(i64::MAX);
        rows[2].cost_cents = Some(i64::MAX);
        rows[3].cost_cents = None;
        assert_eq!(cost(&rows), ["Restricted"]);
    }

    #[test]
    fn mixed_currencies_are_totalled_separately_even_beyond_the_range_of_one_sum() {
        let rows = [row("USD", i64::MAX), row("EUR", i64::MAX)];
        assert_eq!(
            billable(&rows),
            [money(i64::MAX, "EUR"), money(i64::MAX, "USD")]
        );
    }

    #[test]
    fn monetary_total_overflow_is_visible_instead_of_panicking() {
        let rows = [row("EUR", i64::MAX), row("EUR", 1)];
        assert_eq!(billable(&rows), ["EUR amount exceeds supported range"]);
        let rows = [row("EUR", i64::MAX), row("USD", 500), row("EUR", 1)];
        assert_eq!(
            billable(&rows),
            [
                "EUR amount exceeds supported range".to_string(),
                money(500, "USD")
            ]
        );
    }

    #[test]
    fn single_currency_totals_are_formatted_and_empty_reports_have_no_amount() {
        let rows = [row("EUR", 100), row("EUR", 250)];
        assert_eq!(billable(&rows), [money(350, "EUR")]);
        assert_eq!(billable(&[]), ["\u{2014}"]);
    }

    #[test]
    fn zero_and_negative_currency_totals_are_still_shown() {
        let rows = [row("USD", 0), row("EUR", 500), row("EUR", -550)];
        assert_eq!(billable(&rows), ["EUR -0.50", "USD 0.00"]);
    }

    #[test]
    fn padded_codes_stay_separate_groups_even_though_labels_are_trimmed() {
        let rows = [row("EU ", 200), row(" EU", 100), row("EU ", 5)];
        assert_eq!(billable(&rows), ["EU 1.00", "EU 2.05"]);
    }

    #[test]
    fn cost_total_uses_organization_currency_across_billing_currencies() {
        let mut rows = [row("USD", 100), row("CHF", 250)];
        rows[0].cost_cents = Some(50);
        rows[1].cost_cents = Some(75);
        assert_eq!(cost(&rows), [money(125, "EUR")]);
    }

    #[test]
    fn a_page_re_render_leaves_the_report_tab_alone() {
        thread_local! {
            static TAB_RENDERS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        // Takes the props FinancialTimeReport takes, without its server calls.
        #[component]
        fn Tab(filters: FinancialFilters, group_by: Signal<String>) -> Element {
            TAB_RENDERS.with(|renders| renders.set(renders.get() + 1));
            rsx! {}
        }
        // The filter wiring of FinancialReports.
        fn page() -> Element {
            let from_input = use_signal(|| "2026-09-01".to_string());
            let to_input = use_signal(|| "2026-09-30".to_string());
            let from_date = use_debounced(from_input, SEARCH_DEBOUNCE);
            let to_date = use_debounced(to_input, SEARCH_DEBOUNCE);
            let filters = FinancialFilters {
                from: from_date,
                to: to_date,
                client: use_signal(String::new),
                project: use_signal(String::new),
                user: use_signal(String::new),
                tag: use_signal(String::new),
                dates_settled: use_memo(move || {
                    from_date() == from_input() && to_date() == to_input()
                }),
            };
            let group_by = use_signal(|| "project".to_string());
            rsx! { Tab { filters, group_by } }
        }

        let mut dom = VirtualDom::new(page);
        dom.rebuild_in_place();
        assert_eq!(TAB_RENDERS.get(), 1);
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate_to_vec();
        assert_eq!(
            TAB_RENDERS.get(),
            1,
            "equal props must not re-render the tab"
        );
    }
}
