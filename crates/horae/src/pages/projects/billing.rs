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
pub(super) fn InvoiceHistory(data: ProjectInvoices) -> Element {
    rsx! {
        div { class: "mt-6",
            h2 { class: "text-2xl font-semibold text-strong m-0", "All invoice history" }
            p { class: "text-xs text-muted mt-2 mb-4",
                "Project amounts after discounts, before tax. Includes all issue dates, independently of the chart period."
            }
            if data.invoices.is_empty() {
                p { class: "text-muted", "No invoices for this project" }
            } else {
                div { class: "bg-secondary rounded-xl",
                    DataTable { table { aria_label: "Project invoice history",
                        thead { tr {
                            th { scope: "col", "Status" }
                            th { scope: "col", class: "whitespace-nowrap", "Issue date" }
                            th { scope: "col", class: "whitespace-nowrap", "Paid on" }
                            th { scope: "col", "ID" }
                            th { scope: "col", "Subject" }
                            th { scope: "col", class: "text-right whitespace-nowrap", "Pre-tax amount" }
                        } }
                        tbody { for invoice in &data.invoices {
                            tr { key: "{invoice.id}",
                                td { span { class: super::super::invoices::invoice_badge_class(invoice.status), "{invoice.status}" } }
                                td { class: "whitespace-nowrap", "{invoice.issued_on.format(\"%d %b %Y\")}" }
                                td { class: "text-muted whitespace-nowrap",
                                    if invoice.status == horae_core::types::InvoiceStatus::Paid { "Not recorded" }
                                    else { "—" }
                                }
                                td { Link { to: Route::InvoiceDetail { id: invoice.id },
                                    class: "inline-flex items-center min-h-control font-mono", "{invoice.number}"
                                } }
                                td { class: "text-muted", "Not recorded" }
                                td { class: "font-mono text-right whitespace-nowrap", "{format_cents(invoice.net_before_tax_cents, &invoice.currency)}" }
                            }
                        } }
                        tfoot { for (currency, totals) in &data.totals {
                            tr { key: "{currency}",
                                th { scope: "row", colspan: "5", class: "px-5 py-4 text-right font-semibold", "Total excluding void ({currency})" }
                                td { class: "px-5 py-4 text-right font-mono whitespace-nowrap", "{format_cents(totals.non_void_cents, currency)}" }
                            }
                        } }
                    } }
                }
                div { class: "flex flex-wrap gap-6 mt-4",
                    for (currency, totals) in &data.totals {
                        dl { key: "{currency}", class: "text-sm m-0",
                            for (label, amount) in [
                                ("Draft reservations", totals.draft_cents),
                                ("Sent", totals.sent_cents),
                                ("Paid", totals.paid_cents),
                                ("Void history", totals.void_cents),
                            ] {
                                div { class: "flex justify-between gap-4",
                                    dt { class: "text-muted", "{label}" }
                                    dd { class: "font-mono m-0", "{format_cents(amount, currency)}" }
                                }
                            }
                        }
                    }
                }
                p { class: "text-xs text-muted mt-4", "Totals include draft reservations; void invoices remain in history but are excluded from totals." }
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

    #[component]
    fn Fixture() -> Element {
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
        rsx! { InvoiceHistory { data: ProjectInvoices { invoices, totals } } }
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
            "All invoice history",
            "INV &#60;10&#62;",
            "/invoices/00000000-0000-0000-0000-00000000000a",
            "USD 12.34",
            "EUR 56.78",
            "Void history",
            "Draft reservations",
            "Not recorded",
            "after discounts",
        ] {
            assert!(html.contains(expected), "missing {expected}: {html}");
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
