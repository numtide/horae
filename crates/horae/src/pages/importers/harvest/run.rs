use dioxus::prelude::*;
use uuid::Uuid;

use super::run_view::RunView;

/// One run's own page. Keyed by the id, so moving to another run (a commit, a
/// link) starts its state and polling afresh.
#[component]
pub fn HarvestRun(id: Uuid) -> Element {
    rsx! {
        for id in [id] {
            RunView { key: "{id}", id }
        }
    }
}
