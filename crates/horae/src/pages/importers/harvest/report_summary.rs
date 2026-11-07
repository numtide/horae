use dioxus::prelude::*;
use horae_core::importers::harvest::types::{EntityType, ImportMode, ImportReport};

use super::presentation;
use super::stat_tile::StatTile;

/// A finished run's outcome: a status banner and one tile per entity.
#[component]
pub(super) fn ReportSummary(report: ImportReport, partial: bool) -> Element {
    let error_count = report.error_count();
    let is_dry = report.mode == ImportMode::DryRun;
    rsx! {
        div { class: "flex flex-col gap-4 mt-4",
            if partial {
                div { class: "banner banner-warning",
                    span { class: "banner-icon", "◔" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Partial report — confirmed batches only" }
                        div { class: "banner-detail",
                            if is_dry { "Preview stopped before completion. No business data was written. Its job and partial report are retained." }
                            else { "Only confirmed work is included. Unconfirmed work was rolled back." }
                        }
                    }
                }
            } else if is_dry {
                div { class: "banner banner-warning",
                    span { class: "banner-icon", "◔" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Preview complete — no business data changed" }
                        div { class: "banner-detail",
                            "No business data was written. The job and report are retained. Review the numbers and errors below before committing."
                        }
                    }
                }
            } else if error_count > 0 {
                div { class: "banner banner-danger",
                    span { class: "banner-icon", "▲" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Import complete with {error_count} errors" }
                        div { class: "banner-detail", "The records below could not be applied." }
                    }
                }
            } else {
                div { class: "banner banner-success",
                    span { class: "banner-icon", "✓" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Import complete" }
                        div { class: "banner-detail", "Review the created, updated and skipped counts below." }
                    }
                }
            }
            p { class: "text-sm text-secondary", "{presentation::result_summary(&report)}" }
            div { class: "himp-tiles",
                for e in EntityType::ALL {
                    StatTile { entity: e, counts: *report.summary.counts(e) }
                }
            }
        }
    }
}
