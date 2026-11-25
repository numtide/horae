use dioxus::prelude::*;
use horae_core::importers::harvest::types::{EntityType, ImportMode, ImportReport};

use super::{entity_label, presentation};
use crate::components::card::Card;
use crate::components::table::DataTable;

/// Per entity, what a run creates, updates, finds already in Horae and skips.
#[component]
pub(super) fn EntityTable(report: ImportReport, partial: bool) -> Element {
    let title = match (report.mode, partial) {
        (_, true) => "What it covered before it stopped",
        (ImportMode::DryRun, false) => "What will change in Horae",
        (ImportMode::Commit, false) => "What changed in Horae",
    };
    rsx! {
        Card { title, id: "entities-title", flush: true, class: "mt-6",
            DataTable { flush: true, compact: true,
                table {
                    thead {
                        tr {
                            th { scope: "col", "Records" }
                            th { scope: "col", class: "text-right", "New" }
                            th { scope: "col", class: "text-right", "Updated" }
                            th { scope: "col", class: "text-right", "Already in Horae" }
                            th { scope: "col", class: "text-right", "Skipped" }
                        }
                    }
                    tbody {
                        for entity in EntityType::ALL {
                            {
                                let counts = report.summary.counts(entity);
                                rsx! {
                                    tr { key: "{entity.as_str()}",
                                        th { scope: "row", class: "font-normal", "{entity_label(entity)}" }
                                        td { class: "text-right text-mono", "{presentation::count(counts.created)}" }
                                        td { class: "text-right text-mono", "{presentation::count(counts.updated)}" }
                                        td { class: "text-right text-mono text-secondary", "{presentation::count(counts.skipped)}" }
                                        td { class: if counts.errored > 0 { "text-right text-mono text-warning" } else { "text-right text-mono text-secondary" },
                                            "{presentation::count(counts.errored)}"
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
}
