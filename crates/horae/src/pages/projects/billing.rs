use super::*;
use crate::models::project::ProjectInvoices;

#[derive(Clone, Default, PartialEq)]
pub(super) enum InvoiceHistoryState {
    #[default]
    Hidden,
    Loading,
    Ready(ProjectInvoices),
    Failed(String),
}

#[component]
pub(super) fn InvoiceSummary(
    state: InvoiceHistoryState,
    #[props(default)] on_retry: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        section { class: "card p-5 rounded-xl min-w-0 wrap-anywhere", aria_labelledby: "project-invoiced-title",
            h2 { id: "project-invoiced-title", class: "text-sm font-sans font-normal text-secondary m-0", "Invoiced" }
            match state {
                InvoiceHistoryState::Hidden => rsx! {
                    p { class: "font-mono text-3xl font-semibold text-faint mt-2 mb-3", "N/A" }
                    p { class: "text-xs text-muted m-0", "Not available with your permissions." }
                },
                InvoiceHistoryState::Loading => rsx! { p { class: "text-sm text-muted mt-2", role: "status", "Loading invoiced total…" } },
                InvoiceHistoryState::Failed(error) => rsx! {
                    p { class: "text-sm text-danger mt-2", role: "alert", "Could not load invoiced total: {error}" }
                    if let Some(retry) = on_retry {
                        button { r#type: "button", class: "btn btn-secondary btn-sm min-h-control mt-2", onclick: move |_| retry.call(()), "Retry invoiced total" }
                    }
                },
                InvoiceHistoryState::Ready(data) => {
                    let mut counts = HashMap::<&str, usize>::new();
                    for invoice in &data.invoices {
                        if invoice.status != horae_core::types::InvoiceStatus::Void {
                            *counts.entry(&invoice.currency).or_default() += 1;
                        }
                    }
                    rsx! {
                        if data.invoices.is_empty() {
                            p { class: "text-sm text-muted mt-2 mb-3", "No invoices for this project." }
                        } else if counts.is_empty() {
                            p { class: "text-sm text-muted mt-2 mb-3", "No non-void invoices." }
                        }
                        for (currency, total) in &data.totals {
                            if let Some(count) = counts.get(currency.as_str()) {
                                div { key: "{currency}",
                                    p { class: "font-mono text-3xl font-semibold text-strong mt-2 mb-3", "{format_cents(total.non_void_cents, currency)}" }
                                    p { class: "text-xs text-muted m-0",
                                        if *count == 1 { "Across 1 invoice" } else { "Across {count} invoices" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
pub(super) fn InvoiceHistory(data: ProjectInvoices) -> Element {
    rsx! {
        div { class: "mt-4",
            if data.invoices.is_empty() {
                p { class: "text-muted", "No invoices for this project" }
            } else {
                div { class: "table-container bg-secondary rounded-xl",
                    table { aria_label: "Project invoice history",
                        thead { class: "bg-cell-empty", tr {
                            th { scope: "col", class: "text-secondary", "Status" }
                            th { scope: "col", class: "text-secondary whitespace-nowrap", "Issue date" }
                            th { scope: "col", class: "text-secondary whitespace-nowrap", "Paid on" }
                            th { scope: "col", class: "text-secondary", "ID" }
                            th { scope: "col", class: "text-secondary", "Subject" }
                            th { scope: "col", class: "text-secondary text-right whitespace-nowrap", "Pre-tax amount" }
                        } }
                        tbody { for invoice in &data.invoices {
                            tr { key: "{invoice.id}",
                                td { class: "py-3", span { class: super::super::invoices::invoice_badge_class(invoice.status), "{invoice.status}" } }
                                td { class: "py-3 font-mono whitespace-nowrap", "{invoice.issued_on.format(\"%d %b %Y\")}" }
                                td { class: "py-3 text-muted font-mono whitespace-nowrap",
                                    if invoice.status == horae_core::types::InvoiceStatus::Paid { "Not recorded" }
                                    else { "—" }
                                }
                                td { class: "py-3", Link { to: Route::InvoiceDetail { id: invoice.id },
                                    class: "inline-flex items-center min-h-control font-mono whitespace-nowrap", "{invoice.number}"
                                } }
                                td { class: "py-3 text-muted whitespace-nowrap", "Not recorded" }
                                td { class: "py-3 font-mono text-right whitespace-nowrap", "{format_cents(invoice.net_before_tax_cents, &invoice.currency)}" }
                            }
                        } }
                        tfoot { for (currency, totals) in &data.totals {
                            tr { key: "{currency}", class: "bg-cell-empty",
                                th { scope: "row", colspan: "5", class: "px-5 py-4 text-right font-semibold",
                                    if data.invoices.iter().any(|invoice| invoice.currency == *currency && invoice.status == horae_core::types::InvoiceStatus::Void) {
                                        "Total (excl. void)"
                                    } else { "Total" }
                                }
                                td { class: "px-5 py-4 text-right font-mono font-semibold whitespace-nowrap", "{format_cents(totals.non_void_cents, currency)}" }
                            }
                        } }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::ProjectInvoice;
    use dioxus::history::MemoryHistory;
    use dioxus::router::components::HistoryProvider;
    use horae_core::types::InvoiceStatus;
    use std::rc::Rc;

    #[derive(Clone, PartialEq, Routable)]
    enum TestRoute {
        #[route("/")]
        Fixture {},
    }

    fn fixture() -> ProjectInvoices {
        let invoices = vec![
            ProjectInvoice {
                id: Uuid::from_u128(10),
                number: "INV <10>".into(),
                status: InvoiceStatus::Paid,
                issued_on: "2026-09-01".parse().unwrap(),
                currency: "USD".into(),
                net_before_tax_cents: 1234,
            },
            ProjectInvoice {
                id: Uuid::from_u128(11),
                number: "INV-11".into(),
                status: InvoiceStatus::Void,
                issued_on: "2026-08-01".parse().unwrap(),
                currency: "EUR".into(),
                net_before_tax_cents: 5678,
            },
        ];
        let totals = horae_core::invoice::invoice_ledger_totals(
            invoices
                .iter()
                .map(|row| (row.currency.as_str(), row.status, row.net_before_tax_cents)),
        )
        .unwrap();
        ProjectInvoices { invoices, totals }
    }

    #[component]
    fn Fixture() -> Element {
        rsx! { InvoiceHistory { data: fixture() } }
    }

    #[test]
    fn invoice_tile_separates_currencies_and_excludes_void_amounts_and_counts() {
        let html = dioxus::ssr::render_element(rsx! { InvoiceSummary {
            state: InvoiceHistoryState::Ready(fixture()),
        } });
        for expected in ["Invoiced", "USD 12.34", "Across 1 invoice"] {
            assert!(html.contains(expected), "missing {expected}: {html}");
        }
        assert!(!html.contains("EUR 56.78"), "{html}");
        assert!(!html.contains("EUR 0.00"), "{html}");
        assert!(!html.contains("Lifetime"), "{html}");
    }

    #[test]
    fn invoice_tile_distinguishes_void_only_history_from_a_real_zero_invoice() {
        for (status, expected) in [
            (InvoiceStatus::Void, "No non-void invoices."),
            (InvoiceStatus::Draft, "USD 0.00"),
        ] {
            let mut data = fixture();
            data.invoices.truncate(1);
            data.invoices[0].status = status;
            data.invoices[0].net_before_tax_cents = 0;
            data.totals = horae_core::invoice::invoice_ledger_totals(
                data.invoices
                    .iter()
                    .map(|row| (row.currency.as_str(), row.status, row.net_before_tax_cents)),
            )
            .unwrap();
            let html = dioxus::ssr::render_element(rsx! { InvoiceSummary {
                state: InvoiceHistoryState::Ready(data),
            } });
            assert!(html.contains(expected), "{html}");
            if status == InvoiceStatus::Void {
                assert!(!html.contains("0.00"), "{html}");
            } else {
                assert!(html.contains("Across 1 invoice"), "{html}");
            }
        }
    }

    #[test]
    fn invoice_tile_never_turns_pending_private_failed_or_empty_history_into_zero() {
        for (state, expected) in [
            (
                InvoiceHistoryState::Hidden,
                "Not available with your permissions",
            ),
            (InvoiceHistoryState::Loading, "Loading invoiced total"),
            (
                InvoiceHistoryState::Failed("History unavailable".into()),
                "History unavailable",
            ),
            (
                InvoiceHistoryState::Ready(ProjectInvoices {
                    invoices: vec![],
                    totals: Default::default(),
                }),
                "No invoices for this project",
            ),
        ] {
            let html = dioxus::ssr::render_element(rsx! { InvoiceSummary { state } });
            assert!(html.contains(expected), "{html}");
            assert!(!html.contains("0.00"), "{html}");
        }
    }

    #[test]
    fn invoice_history_labels_attribution_and_currencies_and_links_real_records() {
        let mut dom = VirtualDom::new(|| {
            rsx! {
                HistoryProvider { history: move |_| Rc::new(MemoryHistory::with_initial_path("/")) as Rc<dyn History>,
                    Router::<TestRoute> {}
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for expected in [
            "Project invoice history",
            "INV &#60;10&#62;",
            "/invoices/00000000-0000-0000-0000-00000000000a",
            "USD 12.34",
            "EUR 56.78",
            "Total (excl. void)",
            "Not recorded",
            "min-h-control font-mono whitespace-nowrap",
            "font-mono whitespace-nowrap",
        ] {
            assert!(html.contains(expected), "missing {expected}: {html}");
        }
        for removed in [
            "All invoice history",
            "Draft reservations",
            "Void history",
            "after discounts",
            "<dl",
        ] {
            assert!(!html.contains(removed), "unexpected {removed}: {html}");
        }
    }

    #[test]
    fn empty_invoice_history_is_not_a_fabricated_total() {
        let html = dioxus::ssr::render_element(rsx! { InvoiceHistory {
            data: ProjectInvoices {invoices:vec![],totals:Default::default()}
        } });
        assert!(html.contains("No invoices for this project"));
        assert!(!html.contains("0.00"));
    }
}
