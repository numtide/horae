use chrono::{Datelike, Months, Utc};
use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use uuid::Uuid;

use crate::components::form::{FormGroup, Input};
use crate::server_fns;

#[component]
pub(super) fn ProjectFeeBalances(project_id: Uuid) -> Element {
    let mut from = use_signal(|| {
        let today = Utc::now().date_naive();
        today.with_day(1).unwrap_or(today).to_string()
    });
    let mut to = use_signal(|| {
        let today = Utc::now().date_naive();
        today
            .with_day(1)
            .and_then(|first| first.checked_add_months(Months::new(1)))
            .and_then(|next| next.pred_opt())
            .unwrap_or(today)
            .to_string()
    });
    let mut balances = use_resource(move || async move {
        server_fns::get_project_fee_balances(project_id.to_string(), from(), to()).await
    });
    let loading = balances.state()() != UseResourceState::Ready;
    rsx! {
        section { class: "card mt-6 p-5 wrap-anywhere", aria_label: "Project fee balances",
            h2 { class: "page-title text-xl", "Fee balances" }
            p { class: "text-sm text-muted mb-4",
                "Before tax, after invoice discounts. Invoiced amounts include drafts and exclude void invoices. Each month has its own balance; single fees and milestones due by the end date are also shown. This view reserves nothing."
            }
            div { class: "grid md:grid-cols-2 gap-3",
                FormGroup { label: "Fee period from", id: "project-fee-from",
                    Input { id: "project-fee-from", kind: "date", value: from(), oninput: move |event: FormEvent| from.set(event.value()) }
                }
                FormGroup { label: "Fee period to", id: "project-fee-to",
                    Input { id: "project-fee-to", kind: "date", value: to(), oninput: move |event: FormEvent| to.set(event.value()) }
                }
            }
            button { r#type: "button", class: "btn btn-secondary btn-sm mb-4", disabled: loading,
                onclick: move |_| balances.restart(), "Refresh balances"
            }
            if loading {
                p { role: "status", "Loading fee balances…" }
            } else {
                match &*balances.read() {
                    Some(Ok(rows)) => rsx! {
                        if rows.is_empty() { p { class: "text-sm text-muted", "No scheduled fees in this period." } }
                        for row in rows {
                            div { key: "{row.period_key}:{row.currency}", class: "mb-4", role: "group", aria_label: "Fee balance: {row.description}",
                                h3 { class: "text-sm font-semibold", "{row.description}" }
                                p { class: "text-sm text-muted", "Occurrence: {row.period_key}" }
                                p { class: "text-sm font-mono", "Agreed: {row.currency} " {format_cents_plain(row.balance.agreed_cents)} }
                                p { class: "text-sm font-mono", "Invoiced (including drafts): {row.currency} " {format_cents_plain(row.balance.invoiced_cents)} }
                                p { class: "text-sm font-mono",
                                    if row.balance.remaining_cents < 0 { "Over-invoiced: " } else { "Remaining: " }
                                    "{row.currency} " {format_cents_plain(row.balance.remaining_cents)}
                                }
                            }
                        }
                    },
                    Some(Err(error)) => rsx! { p { class: "text-danger", role: "alert", "Could not load fee balances: {error}" } },
                    None => rsx! {},
                }
            }
        }
    }
}
