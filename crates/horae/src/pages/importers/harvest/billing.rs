use dioxus::prelude::*;
use horae_core::importers::harvest::types::{
    EntryScan, ImportMode, ImportReport, LinkConflictKind, SourceKind,
};

use super::presentation;
use crate::i18n::use_tr;

/// What a run did to the billing of its time entries (spec 025): entries
/// linked to imported invoices, reopened, billed with no invoice, and the
/// conflicts it left as Horae had them.
#[component]
pub(super) fn BillingTable(report: ImportReport) -> Element {
    let tr = use_tr();
    let links = report.summary.links;
    let rows = [
        (tr.harvest_billing_linked(), links.linked, None),
        (tr.harvest_billing_unlinked(), links.unlinked, None),
        (
            tr.harvest_billing_without_invoice(),
            links.billed_without_invoice,
            Some(tr.harvest_billing_without_invoice_hint()),
        ),
        (tr.harvest_billing_conflicts(), links.conflicts, None),
    ];
    let quiet = report.source == SourceKind::Csv && links == Default::default();
    if quiet {
        return rsx! {};
    }
    let title = match report.mode {
        ImportMode::DryRun => tr.harvest_billing_preview_title(),
        ImportMode::Commit => tr.harvest_billing_title(),
    };
    rsx! {
        section { class: "flex flex-col gap-3 mt-6", aria_labelledby: "billing-title",
            h2 { id: "billing-title", class: "himp-title", "{title}" }
            if report.invoice_scan == Some(EntryScan::Full) && report.source == SourceKind::HarvestApi {
                p { class: "text-sm text-secondary", {tr.harvest_billing_full_scan()} }
            }
            if report.watermarks_held() {
                p { class: "alert alert-warning text-sm", role: "status", {tr.harvest_billing_watermarks_held()} }
            }
            div { class: "card p-0 overflow-x-auto",
                table { class: "table",
                    tbody {
                        for (index, (label, value, hint)) in rows.into_iter().enumerate() {
                            tr { key: "{index}",
                                td {
                                    div { "{label}" }
                                    if let Some(hint) = hint {
                                        div { class: "text-xs text-secondary", "{hint}" }
                                    }
                                }
                                td { class: "text-right text-mono", "{presentation::count(value)}" }
                            }
                        }
                    }
                }
            }
            if report.invoices_skipped_drafts > 0 {
                p { class: "text-sm text-secondary",
                    {tr.harvest_billing_drafts(report.invoices_skipped_drafts as i64)}
                }
            }
            if report.invoices_no_longer_listed > 0 {
                p { class: "text-sm text-secondary",
                    {tr.harvest_billing_no_longer_listed(report.invoices_no_longer_listed as i64)}
                }
            }
            if report.invoices_voided_drafts > 0 {
                p { class: "text-sm text-secondary",
                    {tr.harvest_billing_voided_drafts(report.invoices_voided_drafts as i64)}
                }
            }
            if !report.invoices_removed.is_empty() {
                p { class: "text-sm text-secondary",
                    {tr.harvest_billing_removed(report.invoices_removed.len() as i64, report.invoices_removed.join(", "))}
                }
            }
            if !report.link_conflicts.is_empty() {
                ul { class: "text-sm",
                    for conflict in report.link_conflicts.iter() {
                        li { key: "{conflict.source_location}",
                            {match conflict.kind {
                                LinkConflictKind::BilledInHorae => tr.harvest_billing_conflict_billed(
                                    conflict.source_location.clone(),
                                    conflict.horae_invoice_number.clone().unwrap_or_default(),
                                ),
                                LinkConflictKind::SubmittedInHorae => {
                                    tr.harvest_billing_conflict_submitted(conflict.source_location.clone())
                                }
                            }}
                        }
                    }
                    if report.omitted_link_conflicts > 0 {
                        li { {tr.harvest_billing_conflicts_more(report.omitted_link_conflicts as i64)} }
                    }
                }
            }
        }
    }
}
