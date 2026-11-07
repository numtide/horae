use dioxus::prelude::*;
use horae_core::importers::harvest::types::{EntityCounts, EntityType};

use super::entity_label;
use super::stat_cell::StatCell;

/// One entity's summary: a total and the created/updated/skipped/errored split.
#[component]
pub(super) fn StatTile(entity: EntityType, counts: EntityCounts) -> Element {
    let has_errors = counts.errored > 0;
    let tile_class = if has_errors {
        "counter-tile has-errors"
    } else {
        "counter-tile"
    };
    rsx! {
        div { class: "{tile_class}",
            div { class: "counter-head",
                span { class: "counter-head-name", "{entity_label(entity)}" }
                span { class: "counter-head-total", "{counts.processed()}" }
            }
            div { class: "counter-breakdown",
                StatCell { value: counts.created, label: "Created", tone: "created" }
                StatCell { value: counts.updated, label: "Updated", tone: "" }
                StatCell { value: counts.skipped, label: "Skipped", tone: "" }
                StatCell {
                    value: counts.errored,
                    label: "Errored",
                    tone: if has_errors { "errored" } else { "" },
                }
            }
        }
    }
}
