use chrono::{Datelike, NaiveDate};
use dioxus::prelude::*;
use horae_core::types::InvoiceStatus;

use super::filter::{InvoiceFilter, PeriodValue, StatusValue};
use super::recovery::RecoveryStorage;
use super::{INVOICES_HEADING, Shown, StatusBadge, date, money, prepares_invoices, settings};
use crate::components::badge::Badge;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::MetricCard;
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::Segmented;
use crate::components::debounce::{SEARCH_DEBOUNCE, use_debounced};
use crate::components::filter_bar::{FilterBar, SearchField};
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, LoadError};
use crate::components::table::{DataTable, NameCell, TableMessage, TableSkeleton};
use crate::i18n::{Tr, use_tr};
use crate::models::invoice::{CurrencyAmount, InvoiceListItem, UninvoicedClient};
use crate::route::Route;
use crate::server_fns;

type Invoices = Resource<Result<Vec<InvoiceListItem>, ServerFnError>>;
type Uninvoiced = Resource<Result<Option<Vec<UninvoicedClient>>, ServerFnError>>;

fn status_label(tr: Tr, status: StatusValue) -> String {
    match status {
        StatusValue::Draft => tr.invoices_status_draft(),
        StatusValue::Sent => tr.invoices_status_sent(),
        StatusValue::Overdue => tr.invoices_status_overdue(),
        StatusValue::Paid => tr.invoices_status_paid(),
        StatusValue::Void => tr.invoices_status_void(),
        StatusValue::Closed => tr.invoices_status_closed(),
    }
}

/// Whether `item` has the chosen status. Sent includes the overdue ones,
/// which are sent and unpaid; Overdue narrows to those.
fn keeps_status(status: Option<StatusValue>, item: &InvoiceListItem) -> bool {
    let stored = item.invoice.status;
    match status {
        None => true,
        Some(StatusValue::Draft) => stored == InvoiceStatus::Draft,
        Some(StatusValue::Sent) => stored == InvoiceStatus::Sent,
        Some(StatusValue::Overdue) => item.overdue,
        Some(StatusValue::Paid) => stored == InvoiceStatus::Paid,
        Some(StatusValue::Void) => stored == InvoiceStatus::Void,
        Some(StatusValue::Closed) => stored == InvoiceStatus::Closed,
    }
}

fn period_label(tr: Tr, period: Option<PeriodValue>) -> String {
    match period {
        None => tr.invoices_period_any(),
        Some(PeriodValue::ThisMonth) => tr.invoices_period_this_month(),
        Some(PeriodValue::LastMonth) => tr.invoices_period_last_month(),
        Some(PeriodValue::ThisYear) => tr.invoices_period_this_year(),
        Some(PeriodValue::LastYear) => tr.invoices_period_last_year(),
    }
}

/// Whether an invoice issued on `issued` falls in `period`, seen on `today`.
fn keeps_period(period: Option<PeriodValue>, issued: NaiveDate, today: NaiveDate) -> bool {
    let month = |day: NaiveDate| (day.year(), day.month());
    let last_month = today
        .with_day(1)
        .and_then(|first| first.pred_opt())
        .unwrap_or(today);
    match period {
        None => true,
        Some(PeriodValue::ThisMonth) => month(issued) == month(today),
        Some(PeriodValue::LastMonth) => month(issued) == month(last_month),
        Some(PeriodValue::ThisYear) => issued.year() == today.year(),
        Some(PeriodValue::LastYear) => issued.year() == today.year() - 1,
    }
}

/// Per-currency sums, in currency order, with how many invoices they hold.
/// `None` when a sum does not fit, so no card shows a capped figure.
fn totals<'a>(
    items: impl Iterator<Item = &'a InvoiceListItem>,
) -> Option<(Vec<CurrencyAmount>, usize)> {
    let mut count = 0;
    let mut sums = std::collections::BTreeMap::<String, i64>::new();
    for item in items {
        count += 1;
        let sum = sums
            .entry(item.invoice.currency.trim().to_owned())
            .or_default();
        *sum = sum.checked_add(item.due_cents())?;
    }
    let sums = sums
        .into_iter()
        .map(|(currency, cents)| CurrencyAmount { currency, cents })
        .collect();
    Some((sums, count))
}

/// Amounts in several currencies, never added together.
fn amounts(sums: &[CurrencyAmount]) -> String {
    sums.iter()
        .map(|sum| money(sum.cents, &sum.currency))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[component]
pub(super) fn InvoiceListContent(filter: InvoiceFilter) -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let navigator = use_navigator();
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
    let shown = filter.clone();
    let switch_view = use_callback(move |label: String| {
        navigator.push(Route::InvoiceList {
            filter: InvoiceFilter {
                uninvoiced: label == tr.invoices_view_uninvoiced(),
                ..shown.clone()
            },
        });
    });
    let uninvoiced_view = prepares && filter.uninvoiced;

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
            Summary { invoices, uninvoiced }
            if prepares {
                div { class: "mb-5",
                    Segmented {
                        items: vec![tr.invoices_view_invoices(), tr.invoices_view_uninvoiced()],
                        active: if uninvoiced_view { tr.invoices_view_uninvoiced() } else { tr.invoices_view_invoices() },
                        onselect: switch_view,
                    }
                }
            }
            if uninvoiced_view {
                UninvoicedTable { uninvoiced, can_prepare: storage.ready(), retry: retry_uninvoiced }
            } else {
                InvoicesTable { invoices, filter: filter.clone(), retry: retry_invoices }
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
fn Summary(invoices: Invoices, uninvoiced: Uninvoiced) -> Element {
    let tr = use_tr();
    let Some(Ok(list)) = &*invoices.read() else {
        return rsx! {};
    };
    let ready_clients = match &*uninvoiced.read() {
        Some(Ok(Some(clients))) => Some(clients.len()),
        _ => None,
    };
    let figure = |totals: Option<(Vec<CurrencyAmount>, usize)>| match totals {
        Some((sums, count)) if !sums.is_empty() => (amounts(&sums), count),
        Some((_, count)) => ("—".to_owned(), count),
        None => ("—".to_owned(), 0),
    };
    let (outstanding, sent) = figure(totals(
        list.iter()
            .filter(|item| item.invoice.status == InvoiceStatus::Sent),
    ));
    let (overdue, late) = figure(totals(list.iter().filter(|item| item.overdue)));
    rsx! {
        section { class: "grid md:grid-cols-3 gap-4 mb-6", aria_label: tr.invoices_summary_label(),
            MetricCard {
                label: tr.invoices_summary_outstanding(),
                value: outstanding,
                delta: tr.invoices_summary_count(sent as i64),
            }
            MetricCard {
                label: tr.invoices_summary_overdue(),
                value: overdue,
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

/// The filters and the list. Loading, an error and no match show inside the
/// table's frame, under its header, so the page does not jump.
#[component]
fn InvoicesTable(invoices: Invoices, filter: InvoiceFilter, retry: Callback<()>) -> Element {
    let tr = use_tr();
    let navigator = use_navigator();
    let today = crate::components::today::today();
    // The search box answers at once; the URL follows when typing pauses.
    let mut query = use_signal(|| filter.search.clone());
    let settled = use_debounced(query, SEARCH_DEBOUNCE);
    // Only a settled search moves the URL. The filter is read from the
    // router at that moment, so Back to another search is never undone.
    use_effect(move || {
        let search = settled();
        if let Route::InvoiceList { filter: current } = dioxus::router::router().current::<Route>()
            && search != current.search
        {
            navigator.replace(Route::InvoiceList {
                filter: InvoiceFilter { search, ..current },
            });
        }
    });
    // Back, or a shared link, brings another search: show it in the box.
    let external = filter.search.clone();
    use_effect(use_reactive!(|external| {
        if external != *settled.peek() {
            query.set(external);
        }
    }));
    let go = {
        let current = filter.clone();
        move |next: InvoiceFilter| {
            if next != current {
                navigator.push(Route::InvoiceList { filter: next });
            }
        }
    };
    let clear = {
        let go = go.clone();
        let current = filter.clone();
        use_callback(move |_: MouseEvent| {
            query.set(String::new());
            go(current.cleared());
        })
    };
    let pick_client = {
        let go = go.clone();
        let current = filter.clone();
        use_callback(move |value: String| {
            go(InvoiceFilter {
                client: value.parse().ok(),
                ..current.clone()
            })
        })
    };
    let pick_status = {
        let go = go.clone();
        let current = filter.clone();
        use_callback(move |status: Option<StatusValue>| {
            go(InvoiceFilter {
                status,
                ..current.clone()
            })
        })
    };
    let pick_period = {
        let current = filter.clone();
        use_callback(move |period: Option<PeriodValue>| {
            go(InvoiceFilter {
                period,
                ..current.clone()
            })
        })
    };

    let pending = invoices.state()() != UseResourceState::Ready;
    let read = invoices.read();
    let list: &[InvoiceListItem] = match &*read {
        // No invoice yet: the page's empty state rather than an empty table.
        // The header already holds "New invoice", the screen's one Primary.
        Some(Ok(list)) if list.is_empty() => {
            return rsx! {
                EmptyState { icon: "invoices", title: tr.invoices_empty_title(), body: tr.invoices_empty_body() }
            };
        }
        Some(Ok(list)) => list,
        _ => &[],
    };

    let needle = query().trim().to_lowercase();
    let visible: Vec<&InvoiceListItem> = list
        .iter()
        .filter(|item| keeps_status(filter.status, item))
        .filter(|item| {
            filter
                .client
                .is_none_or(|client| item.invoice.client_id == client)
        })
        .filter(|item| keeps_period(filter.period, item.invoice.issued_on, today))
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
    let status_shown = filter.status.map_or_else(
        || tr.invoices_status_any(),
        |status| status_label(tr, status),
    );

    rsx! {
        FilterBar {
            filters: rsx! {
                Menu { id: "invoices-status-filter", label: status_shown, trigger_class: "text-sm px-4",
                    StatusOption { value: None, current: filter.status, onpick: pick_status }
                    for status in StatusValue::ALL {
                        StatusOption { key: "{status:?}", value: Some(status), current: filter.status, onpick: pick_status }
                    }
                }
                Combobox { options: clients, value: filter.client.map(|client| client.to_string()).unwrap_or_default(),
                    placeholder: tr.invoices_client_any(), all_label: tr.invoices_client_any(),
                    trigger_class: "text-sm px-4", onselect: pick_client }
                Menu { id: "invoices-period-filter", label: period_label(tr, filter.period), trigger_class: "text-sm px-4",
                    PeriodOption { value: None, current: filter.period, onpick: pick_period }
                    for period in PeriodValue::ALL {
                        PeriodOption { key: "{period:?}", value: Some(period), current: filter.period, onpick: pick_period }
                    }
                }
            },
            search: rsx! { SearchField { id: "invoices-search", label: tr.invoices_search_label(), query } },
        }
        DataTable { label: tr.invoices_view_invoices(), sticky_header: true,
            busy: pending && matches!(&*read, Some(Ok(_))),
            table { aria_label: tr.invoices_view_invoices(),
                thead { tr {
                    th { scope: "col", {tr.invoices_column_number()} }
                    th { scope: "col", {tr.invoices_column_client()} }
                    th { scope: "col", {tr.invoices_column_issued()} }
                    th { scope: "col", {tr.invoices_column_due()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_amount()} }
                    th { scope: "col", {tr.invoices_column_status()} }
                } }
                tbody {
                    match &*read {
                        None => rsx! { TableSkeleton { columns: 6, rows: 5, label: tr.invoices_loading() } },
                        Some(Err(error)) => rsx! {
                            TableMessage { columns: 6,
                                LoadError { error: error.clone(), what: tr.invoices_what(), retry, pending, inset: true,
                                    focus_on_recovery: INVOICES_HEADING }
                            }
                        },
                        Some(Ok(_)) if visible.is_empty() => rsx! {
                            TableMessage { columns: 6,
                                EmptyState { icon: "invoices", inset: true, title: tr.invoices_none_match(),
                                    actions: rsx! { Button { variant: ButtonVariant::Secondary, small: true, label: tr.invoices_clear_filters(), onclick: clear } },
                                }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            for item in visible {
                                InvoiceRow { key: "{item.invoice.id}", item: item.clone() }
                            }
                        },
                    }
                }
            }
        }
    }
}

/// One invoice; its number's link covers the whole row. Rows are small and
/// compare equal when unchanged, so they take their item by value.
#[component]
fn InvoiceRow(item: InvoiceListItem) -> Element {
    let tr = use_tr();
    let invoice = &item.invoice;
    let shown = Shown::of(invoice.status, item.overdue);
    let void = invoice.status == InvoiceStatus::Void;
    rsx! {
        tr { class: "row-link",
            td { class: "text-mono",
                Link { to: Route::InvoiceDetail { id: invoice.id }, class: "font-semibold text-strong", "{invoice.number}" }
            }
            td { class: "wrap-anywhere", "{item.client_name}" }
            td { class: "text-mono text-secondary", {date(invoice.issued_on)} }
            td { class: if item.overdue { "text-mono text-danger" } else { "text-mono text-secondary" }, {date(invoice.due_on)} }
            td { class: if void { "text-mono text-right inv-void-amount text-muted" } else { "text-mono text-right" },
                {money(invoice.total_cents, &invoice.currency)}
            }
            td {
                span { class: "flex flex-wrap gap-2 items-center",
                    StatusBadge { shown }
                    if item.imported.is_some() {
                        Badge { variant: "neutral", {tr.invoices_imported()} }
                    }
                }
            }
        }
    }
}

#[component]
fn StatusOption(
    value: Option<StatusValue>,
    current: Option<StatusValue>,
    onpick: Callback<Option<StatusValue>>,
) -> Element {
    let tr = use_tr();
    let choose = use_callback(move |_: MouseEvent| onpick.call(value));
    let label = value.map_or_else(
        || tr.invoices_status_any(),
        |status| status_label(tr, status),
    );
    rsx! { MenuItem { selected: current == value, onclick: choose, "{label}" } }
}

#[component]
fn PeriodOption(
    value: Option<PeriodValue>,
    current: Option<PeriodValue>,
    onpick: Callback<Option<PeriodValue>>,
) -> Element {
    let tr = use_tr();
    let choose = use_callback(move |_: MouseEvent| onpick.call(value));
    rsx! { MenuItem { selected: current == value, onclick: choose, {period_label(tr, value)} } }
}

/// Per-currency sums of several lists of amounts, in currency order.
/// `None` when a sum does not fit, so no total shows a capped figure.
fn add_amounts<'a>(
    lists: impl Iterator<Item = &'a [CurrencyAmount]>,
) -> Option<Vec<CurrencyAmount>> {
    let mut sums = std::collections::BTreeMap::<&str, i64>::new();
    for amount in lists.flatten() {
        let sum = sums.entry(amount.currency.as_str()).or_default();
        *sum = sum.checked_add(amount.cents)?;
    }
    Some(
        sums.into_iter()
            .map(|(currency, cents)| CurrencyAmount {
                currency: currency.to_owned(),
                cents,
            })
            .collect(),
    )
}

/// Amounts in several currencies, or a dash when there are none.
fn amounts_or_dash(sums: Option<&[CurrencyAmount]>) -> String {
    match sums {
        Some(sums) if !sums.is_empty() => amounts(sums),
        _ => "—".into(),
    }
}

/// Clients with unbilled time or fees due. Each row opens a new invoice for
/// its client, and the footer adds them up per currency.
#[component]
fn UninvoicedTable(uninvoiced: Uninvoiced, can_prepare: bool, retry: Callback<()>) -> Element {
    let tr = use_tr();
    let pending = uninvoiced.state()() != UseResourceState::Ready;
    let read = uninvoiced.read();
    let clients: &[UninvoicedClient] = match &*read {
        Some(Ok(Some(clients))) => clients,
        _ => &[],
    };
    let minutes: i64 = clients.iter().map(|client| client.unbilled_minutes).sum();
    let time = add_amounts(clients.iter().map(|client| client.time.as_slice()));
    let fees = add_amounts(clients.iter().map(|client| client.fees.as_slice()));
    rsx! {
        DataTable { label: tr.invoices_view_uninvoiced(), busy: pending && !clients.is_empty(),
            table { aria_label: tr.invoices_view_uninvoiced(),
                thead { tr {
                    th { scope: "col", {tr.invoices_column_client()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_unbilled_time()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_unbilled_amount()} }
                    th { scope: "col", class: "text-right", {tr.invoices_column_fees_due()} }
                    th { scope: "col", {tr.invoices_column_last_invoice()} }
                } }
                tbody {
                    match &*read {
                        None => rsx! { TableSkeleton { columns: 5, rows: 4, label: tr.invoices_uninvoiced_loading() } },
                        Some(Err(error)) => rsx! {
                            TableMessage { columns: 5,
                                LoadError { error: error.clone(), what: tr.invoices_uninvoiced_what(), retry, pending, inset: true }
                            }
                        },
                        Some(Ok(_)) if clients.is_empty() => rsx! {
                            TableMessage { columns: 5,
                                EmptyState { icon: "invoices", success: true, inset: true, title: tr.invoices_uninvoiced_empty_title(),
                                    body: tr.invoices_uninvoiced_empty_body() }
                            }
                        },
                        Some(Ok(_)) => rsx! {
                            for client in clients {
                                UninvoicedRow { key: "{client.client_id}", client: client.clone(), can_prepare }
                            }
                        },
                    }
                }
                if !clients.is_empty() {
                    tfoot { tr {
                        th { scope: "row", {tr.invoices_total()} }
                        td { class: "text-mono text-right", {horae_core::duration::format_hhmm(minutes)} }
                        td { class: "text-mono text-right", {amounts_or_dash(time.as_deref())} }
                        td { class: "text-mono text-right", {amounts_or_dash(fees.as_deref())} }
                        td {}
                    } }
                }
            }
        }
    }
}

/// One client with work to invoice. Its name opens a new invoice for it and
/// covers the row; the last invoice keeps a link of its own above that.
#[component]
fn UninvoicedRow(client: UninvoicedClient, can_prepare: bool) -> Element {
    let tr = use_tr();
    rsx! {
        tr { class: if can_prepare { "row-link" },
            NameCell {
                if can_prepare {
                    Link {
                        to: Route::NewInvoiceForClient { client: client.client_id.to_string() },
                        class: "wrap-anywhere text-strong",
                        aria_label: tr.invoices_new_for(client.client_name.clone()),
                        "{client.client_name}"
                    }
                } else {
                    span { class: "wrap-anywhere", "{client.client_name}" }
                }
            }
            td { class: "text-mono text-right", {horae_core::duration::format_hhmm(client.unbilled_minutes)} }
            td { class: "text-mono text-right", {amounts_or_dash(Some(client.time.as_slice()))} }
            td { class: "text-mono text-right", {amounts_or_dash(Some(client.fees.as_slice()))} }
            td {
                match &client.last_invoice {
                    Some(last) => rsx! {
                        Link { to: Route::InvoiceDetail { id: last.id }, class: "text-mono", "{last.number}" }
                        span { class: "text-sm text-secondary", " " {tr.invoices_last_invoice_date(date(last.issued_on))} }
                    },
                    None => rsx! { span { class: "text-muted", {tr.invoices_never_invoiced()} } },
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
        assert!(keeps_period(
            Some(PeriodValue::ThisMonth),
            day("2026-01-01"),
            today
        ));
        assert!(keeps_period(
            Some(PeriodValue::LastMonth),
            day("2025-12-31"),
            today
        ));
        assert!(!keeps_period(
            Some(PeriodValue::LastMonth),
            day("2026-01-01"),
            today
        ));
        assert!(keeps_period(
            Some(PeriodValue::LastYear),
            day("2025-03-01"),
            today
        ));
        assert!(!keeps_period(
            Some(PeriodValue::ThisYear),
            day("2025-03-01"),
            today
        ));
        assert!(keeps_period(None, day("1999-03-01"), today));
    }

    fn item(currency: &str, cents: i64, status: InvoiceStatus, overdue: bool) -> InvoiceListItem {
        InvoiceListItem {
            invoice: crate::models::invoice::Invoice {
                currency: currency.into(),
                total_cents: cents,
                status,
                ..sample()
            },
            client_name: "Acme".into(),
            overdue,
            imported: None,
        }
    }

    #[test]
    fn an_imported_invoice_counts_for_what_is_still_due() {
        let mut partly_paid = item("EUR", 1000, InvoiceStatus::Sent, true);
        partly_paid.imported = Some(crate::models::invoice::ImportedInvoice {
            due_cents: 400,
            paid_on: None,
            discount_rate: None,
            tax1_rate: None,
            tax2_rate: None,
            period_start: None,
            period_end: None,
        });
        let list = [partly_paid, item("EUR", 100, InvoiceStatus::Sent, false)];
        let (sums, count) = totals(list.iter()).unwrap();
        assert_eq!((amounts(&sums).as_str(), count), ("EUR 5.00", 2));
    }

    #[test]
    fn totals_keep_currencies_apart_and_refuse_to_cap() {
        let list = [
            item("EUR", 100, InvoiceStatus::Sent, false),
            item("USD", 50, InvoiceStatus::Sent, false),
            item("EUR", 25, InvoiceStatus::Sent, false),
        ];
        let (sums, count) = totals(list.iter()).unwrap();
        assert_eq!(count, 3);
        assert_eq!(amounts(&sums), "EUR 1.25 · USD 0.50");
        let huge = [
            item("EUR", i64::MAX, InvoiceStatus::Sent, false),
            item("EUR", 1, InvoiceStatus::Sent, false),
        ];
        assert_eq!(totals(huge.iter()), None);
    }

    #[test]
    fn uninvoiced_totals_add_up_per_currency_and_refuse_to_cap() {
        let amount = |currency: &str, cents| CurrencyAmount {
            currency: currency.into(),
            cents,
        };
        let acme = [amount("EUR", 100), amount("USD", 50)];
        let globex = [amount("EUR", 25)];
        let sums = add_amounts([acme.as_slice(), &globex, &[]].into_iter()).unwrap();
        assert_eq!(amounts(&sums), "EUR 1.25 · USD 0.50");
        assert_eq!(amounts_or_dash(Some(&[])), "—");
        let huge = [amount("EUR", i64::MAX)];
        assert_eq!(add_amounts([huge.as_slice(), &globex].into_iter()), None);
    }

    #[test]
    fn sent_includes_overdue_and_overdue_narrows_to_it() {
        let late = item("EUR", 1, InvoiceStatus::Sent, true);
        let on_time = item("EUR", 1, InvoiceStatus::Sent, false);
        assert!(keeps_status(Some(StatusValue::Sent), &late));
        assert!(keeps_status(Some(StatusValue::Sent), &on_time));
        assert!(keeps_status(Some(StatusValue::Overdue), &late));
        assert!(!keeps_status(Some(StatusValue::Overdue), &on_time));
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
