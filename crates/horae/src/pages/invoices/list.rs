use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use horae_core::types::InvoiceStatus;

use super::recovery::RecoveryStorage;
use super::{INVOICES_HEADING, Shown, StatusBadge, date, money, prepares_invoices, settings};
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::MetricCard;
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Segmented;
use crate::components::filter_bar::{FilterBar, SearchField};
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, LoadError, Skeleton};
use crate::components::table::DataTable;
use crate::i18n::{Tr, use_tr};
use crate::models::invoice::{CurrencyAmount, InvoiceListItem, UninvoicedClient};
use crate::route::Route;
use crate::server_fns;

/// The status filter's values; Overdue is computed, not stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StatusFilter {
    Any,
    Is(Shown),
}

impl StatusFilter {
    fn keeps(self, item: &InvoiceListItem) -> bool {
        match self {
            Self::Any => true,
            Self::Is(shown) => Shown::of(item.invoice.status, item.overdue) == shown,
        }
    }
}

/// The period filter, over the issue date.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Period {
    Any,
    ThisMonth,
    LastMonth,
    ThisYear,
    LastYear,
}

impl Period {
    const ALL: [Self; 5] = [
        Self::Any,
        Self::ThisMonth,
        Self::LastMonth,
        Self::ThisYear,
        Self::LastYear,
    ];

    fn label(self, tr: Tr) -> String {
        match self {
            Self::Any => tr.invoices_period_any(),
            Self::ThisMonth => tr.invoices_period_this_month(),
            Self::LastMonth => tr.invoices_period_last_month(),
            Self::ThisYear => tr.invoices_period_this_year(),
            Self::LastYear => tr.invoices_period_last_year(),
        }
    }

    fn keeps(self, issued: NaiveDate, today: NaiveDate) -> bool {
        let month = |day: NaiveDate| (day.year(), day.month());
        let last_month = today
            .with_day(1)
            .and_then(|first| first.pred_opt())
            .unwrap_or(today);
        match self {
            Self::Any => true,
            Self::ThisMonth => month(issued) == month(today),
            Self::LastMonth => month(issued) == month(last_month),
            Self::ThisYear => issued.year() == today.year(),
            Self::LastYear => issued.year() == today.year() - 1,
        }
    }
}

/// Which of the list's two views shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Invoices,
    Uninvoiced,
}

/// Per-currency sums, in currency order, with how many invoices they hold.
fn totals<'a>(items: impl Iterator<Item = &'a InvoiceListItem>) -> (Vec<CurrencyAmount>, usize) {
    let mut count = 0;
    let mut sums = std::collections::BTreeMap::<String, i64>::new();
    for item in items {
        count += 1;
        let sum = sums
            .entry(item.invoice.currency.trim().to_owned())
            .or_default();
        *sum = sum.saturating_add(item.invoice.total_cents);
    }
    let sums = sums
        .into_iter()
        .map(|(currency, cents)| CurrencyAmount { currency, cents })
        .collect();
    (sums, count)
}

/// Amounts in several currencies, never added together.
fn amounts(sums: &[CurrencyAmount]) -> String {
    sums.iter()
        .map(|sum| money(sum.cents, &sum.currency))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[component]
pub(super) fn InvoiceListContent() -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let session = crate::components::session::use_session();
    let configures = matches!(
        &*session.permissions.read(),
        Ok(own) if settings::edits_invoice_settings(own)
    );
    let prepares = prepares_invoices(session);
    let mut invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    // Only someone who may prepare invoices may see unbilled work priced.
    let mut uninvoiced = use_resource(move || async move {
        if prepares {
            server_fns::list_uninvoiced_clients().await.map(Some)
        } else {
            Ok(None)
        }
    });
    let retry_invoices = use_callback(move |()| invoices.restart());
    let retry_uninvoiced = use_callback(move |()| uninvoiced.restart());
    let mut view = use_signal(|| View::Invoices);
    let switch_view = use_callback(move |label: String| {
        view.set(if label == tr.invoices_view_uninvoiced() {
            View::Uninvoiced
        } else {
            View::Invoices
        });
    });

    let ready_clients = match &*uninvoiced.read() {
        Some(Ok(Some(clients))) => Some(clients.len()),
        _ => None,
    };

    rsx! {
        div {
            PageHeader { title: tr.invoices_title(), title_id: INVOICES_HEADING,
                actions: rsx! {
                    if configures {
                        Menu { id: "invoices-more", label: tr.invoices_more(), align_right: true, trigger_class: "text-sm py-3 px-5",
                            SettingsItem {}
                        }
                    }
                    if prepares && storage.ready() {
                        Link { id: "invoices-new", to: Route::NewInvoice {}, class: "btn btn-primary", {tr.invoices_new()} }
                    }
                },
            }
            if let Some(Ok(list)) = &*invoices.read() {
                Summary { list: list.clone(), ready_clients }
            }
            if prepares {
                div { class: "mb-5",
                    Segmented {
                        items: vec![tr.invoices_view_invoices(), tr.invoices_view_uninvoiced()],
                        active: if view() == View::Uninvoiced { tr.invoices_view_uninvoiced() } else { tr.invoices_view_invoices() },
                        onselect: switch_view,
                    }
                }
            }
            if view() == View::Uninvoiced {
                match &*uninvoiced.read() {
                    None => rsx! { Skeleton { rows: 4, columns: 5, label: tr.invoices_uninvoiced_loading() } },
                    Some(Err(error)) => rsx! {
                        LoadError { error: error.clone(), what: tr.invoices_uninvoiced_what(), retry: retry_uninvoiced,
                            pending: uninvoiced.state()() != UseResourceState::Ready }
                    },
                    Some(Ok(clients)) => rsx! {
                        UninvoicedTable { clients: clients.clone().unwrap_or_default(), can_prepare: storage.ready() }
                    },
                }
            } else {
                match &*invoices.read() {
                    None => rsx! { Skeleton { rows: 5, columns: 6, label: tr.invoices_loading() } },
                    Some(Err(error)) => rsx! {
                        LoadError { error: error.clone(), what: tr.invoices_what(), retry: retry_invoices,
                            pending: invoices.state()() != UseResourceState::Ready,
                            focus_on_recovery: INVOICES_HEADING }
                    },
                    Some(Ok(list)) => rsx! {
                        InvoicesTable { list: list.clone(), stale: invoices.state()() != UseResourceState::Ready,
                            can_prepare: prepares && storage.ready() }
                    },
                }
            }
        }
    }
}

/// The header's "More" item, a link to Invoice settings.
#[component]
fn SettingsItem() -> Element {
    let tr = use_tr();
    let navigator = use_navigator();
    let open = use_callback(move |_: MouseEvent| {
        navigator.push(Route::InvoiceSettings {});
    });
    rsx! { MenuItem { id: "invoices-settings", onclick: open, {tr.invoices_settings()} } }
}

/// What is outstanding, what is overdue, and how many clients have work to
/// invoice. Paid this month waits for payments, which bring a paid date.
#[component]
fn Summary(list: Vec<InvoiceListItem>, ready_clients: Option<usize>) -> Element {
    let tr = use_tr();
    let (outstanding, sent) = totals(
        list.iter()
            .filter(|item| item.invoice.status == InvoiceStatus::Sent),
    );
    let (overdue, late) = totals(list.iter().filter(|item| item.overdue));
    let zero = || "—".to_owned();
    rsx! {
        section { class: "grid md:grid-cols-3 gap-4 mb-6", aria_label: tr.invoices_summary_label(),
            MetricCard {
                label: tr.invoices_summary_outstanding(),
                value: if outstanding.is_empty() { zero() } else { amounts(&outstanding) },
                delta: tr.invoices_summary_count(sent as i64),
            }
            MetricCard {
                label: tr.invoices_summary_overdue(),
                value: if overdue.is_empty() { zero() } else { amounts(&overdue) },
                delta: tr.invoices_summary_count(late as i64),
                direction: if late > 0 { "down" } else { "" },
            }
            if let Some(clients) = ready_clients {
                MetricCard {
                    label: tr.invoices_summary_ready(),
                    value: "{clients}",
                    delta: tr.invoices_summary_ready_clients(clients as i64),
                }
            }
        }
    }
}

#[component]
fn InvoicesTable(list: Vec<InvoiceListItem>, stale: bool, can_prepare: bool) -> Element {
    let tr = use_tr();
    let mut status = use_signal(|| StatusFilter::Any);
    let mut client = use_signal(String::new);
    let mut period = use_signal(|| Period::Any);
    let mut query = use_signal(String::new);
    let today = crate::components::today::today();
    let clear = use_callback(move |_: MouseEvent| {
        status.set(StatusFilter::Any);
        client.set(String::new());
        period.set(Period::Any);
        query.set(String::new());
    });
    let pick_client = use_callback(move |value: String| client.set(value));

    if list.is_empty() {
        return rsx! {
            EmptyState { icon: "invoices", title: tr.invoices_empty_title(), body: tr.invoices_empty_body(),
                actions: rsx! {
                    if can_prepare {
                        Link { to: Route::NewInvoice {}, class: "btn btn-primary", {tr.invoices_new()} }
                    }
                },
            }
        };
    }

    let needle = query().trim().to_lowercase();
    let visible: Vec<&InvoiceListItem> = list
        .iter()
        .filter(|item| status().keeps(item))
        .filter(|item| client().is_empty() || item.invoice.client_id.to_string() == client())
        .filter(|item| period().keeps(item.invoice.issued_on, today))
        .filter(|item| {
            needle.is_empty()
                || item.invoice.number.to_lowercase().contains(&needle)
                || item.client_name.to_lowercase().contains(&needle)
        })
        .collect();
    let mut clients: Vec<ComboOption> = list
        .iter()
        .map(|item| ComboOption {
            value: item.invoice.client_id.to_string(),
            label: item.client_name.clone(),
            group: None,
        })
        .collect();
    clients.sort_by(|a, b| a.label.cmp(&b.label).then(a.value.cmp(&b.value)));
    clients.dedup_by(|a, b| a.value == b.value);
    let statuses = [
        Shown::Status(InvoiceStatus::Draft),
        Shown::Status(InvoiceStatus::Sent),
        Shown::Overdue,
        Shown::Status(InvoiceStatus::Paid),
        Shown::Status(InvoiceStatus::Void),
    ];
    let status_label = match status() {
        StatusFilter::Any => tr.invoices_status_any(),
        StatusFilter::Is(shown) => shown.label(tr),
    };

    rsx! {
        FilterBar {
            filters: rsx! {
                Menu { id: "invoices-status-filter", label: status_label, trigger_class: "text-sm px-4",
                    StatusOption { value: StatusFilter::Any, label: tr.invoices_status_any(), status }
                    for shown in statuses {
                        StatusOption { key: "{shown:?}", value: StatusFilter::Is(shown), label: shown.label(tr), status }
                    }
                }
                Combobox { options: clients, value: client(), placeholder: tr.invoices_client_any(),
                    all_label: tr.invoices_client_any(), trigger_class: "text-sm px-4", onselect: pick_client }
                Menu { id: "invoices-period-filter", label: period().label(tr), trigger_class: "text-sm px-4",
                    for value in Period::ALL {
                        PeriodOption { key: "{value:?}", value, period }
                    }
                }
            },
            search: rsx! { SearchField { id: "invoices-search", label: tr.invoices_search_label(), query } },
        }
        if visible.is_empty() {
            EmptyState { icon: "invoices", title: tr.invoices_none_match(),
                actions: rsx! { Button { variant: ButtonVariant::Secondary, label: tr.invoices_clear_filters(), onclick: clear } },
            }
        } else {
            div { class: "inv-content", aria_busy: stale,
                DataTable {
                    table {
                        thead { tr {
                            th { scope: "col", {tr.invoices_column_number()} }
                            th { scope: "col", {tr.invoices_column_client()} }
                            th { scope: "col", {tr.invoices_column_issued()} }
                            th { scope: "col", {tr.invoices_column_due()} }
                            th { scope: "col", class: "text-right", {tr.invoices_column_amount()} }
                            th { scope: "col", {tr.invoices_column_status()} }
                        } }
                        tbody {
                            for item in visible {
                                InvoiceRow { key: "{item.invoice.id}", item: item.clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One invoice; its number's link covers the whole row.
#[component]
fn InvoiceRow(item: InvoiceListItem) -> Element {
    let invoice = &item.invoice;
    let shown = Shown::of(invoice.status, item.overdue);
    let void = invoice.status == InvoiceStatus::Void;
    rsx! {
        tr { class: "row-link",
            td { class: "text-mono",
                Link { to: Route::InvoiceDetail { id: invoice.id }, class: "font-semibold text-strong", "{invoice.number}" }
            }
            td { class: "wrap-anywhere", "{item.client_name}" }
            td { class: "text-mono", {date(invoice.issued_on)} }
            td { class: if item.overdue { "text-mono text-danger" } else { "text-mono" }, {date(invoice.due_on)} }
            td { class: if void { "text-mono text-right inv-void-amount text-muted" } else { "text-mono text-right" },
                {money(invoice.total_cents, &invoice.currency)}
            }
            td { StatusBadge { shown } }
        }
    }
}

#[component]
fn StatusOption(value: StatusFilter, label: String, status: Signal<StatusFilter>) -> Element {
    let mut status = status;
    let choose = use_callback(move |_: MouseEvent| status.set(value));
    rsx! { MenuItem { selected: status() == value, onclick: choose, "{label}" } }
}

#[component]
fn PeriodOption(value: Period, period: Signal<Period>) -> Element {
    let tr = use_tr();
    let mut period = period;
    let choose = use_callback(move |_: MouseEvent| period.set(value));
    rsx! { MenuItem { selected: period() == value, onclick: choose, {value.label(tr)} } }
}

/// Clients with unbilled time or fees due, each with its own New invoice.
#[component]
fn UninvoicedTable(clients: Vec<UninvoicedClient>, can_prepare: bool) -> Element {
    let tr = use_tr();
    if clients.is_empty() {
        return rsx! {
            EmptyState { icon: "invoices", success: true, title: tr.invoices_uninvoiced_empty_title(),
                body: tr.invoices_uninvoiced_empty_body() }
        };
    }
    rsx! {
        DataTable {
            table {
                thead { tr {
                    th { scope: "col", {tr.invoices_column_client()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_unbilled_time()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_unbilled_amount()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_fees_due()} }
                    th { scope: "col", aria_label: tr.action_menu() }
                } }
                tbody {
                    for client in clients {
                        tr { key: "{client.client_id}",
                            td { class: "wrap-anywhere font-semibold", "{client.client_name}" }
                            td { class: "text-mono text-right", {horae_core::duration::format_hhmm(client.unbilled_minutes)} }
                            td { class: "text-mono text-right", {if client.time.is_empty() { "—".into() } else { amounts(&client.time) }} }
                            td { class: "text-mono text-right", {if client.fees.is_empty() { "—".into() } else { amounts(&client.fees) }} }
                            td { class: "text-right",
                                if can_prepare {
                                    Link {
                                        to: Route::NewInvoiceForClient { client: client.client_id.to_string() },
                                        class: "btn btn-secondary btn-sm",
                                        aria_label: tr.invoices_new_for(client.client_name.clone()),
                                        {tr.invoices_new()}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    #[test]
    fn periods_follow_the_issue_date() {
        let today = day("2026-01-15");
        assert!(Period::ThisMonth.keeps(day("2026-01-01"), today));
        assert!(Period::LastMonth.keeps(day("2025-12-31"), today));
        assert!(!Period::LastMonth.keeps(day("2026-01-01"), today));
        assert!(Period::LastYear.keeps(day("2025-03-01"), today));
        assert!(!Period::ThisYear.keeps(day("2025-03-01"), today));
        assert!(Period::Any.keeps(day("1999-03-01"), today));
    }

    #[test]
    fn totals_keep_currencies_apart() {
        let item = |currency: &str, cents| InvoiceListItem {
            invoice: crate::models::invoice::Invoice {
                currency: currency.into(),
                total_cents: cents,
                ..sample()
            },
            client_name: "Acme".into(),
            overdue: false,
        };
        let list = [item("EUR", 100), item("USD", 50), item("EUR", 25)];
        let (sums, count) = totals(list.iter());
        assert_eq!(count, 3);
        assert_eq!(amounts(&sums), "EUR 1.25 · USD 0.50");
    }

    fn sample() -> crate::models::invoice::Invoice {
        crate::models::invoice::Invoice {
            id: uuid::Uuid::nil(),
            org_id: uuid::Uuid::nil(),
            client_id: uuid::Uuid::nil(),
            number: "INV-1".into(),
            status: InvoiceStatus::Sent,
            issued_on: day("2026-01-01"),
            due_on: day("2026-01-31"),
            currency: "EUR".into(),
            total_cents: 0,
            terms_days: 30,
            po_number: String::new(),
            discount_bps: 0,
            tax1_bps: 0,
            tax2_name: None,
            tax2_bps: None,
            subtotal_cents: 0,
            discount_cents: 0,
            tax1_cents: 0,
            tax2_cents: 0,
            notes: None,
            subject: String::new(),
            created_at: chrono::DateTime::UNIX_EPOCH,
        }
    }
}
