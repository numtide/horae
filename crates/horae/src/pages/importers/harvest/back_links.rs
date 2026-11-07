use dioxus::prelude::*;

use super::runs_cursor::RunsCursor;
use crate::route::Route;

/// The way back from a run: the Harvest overview and the full history.
#[component]
pub(super) fn BackLinks() -> Element {
    rsx! {
        div { class: "flex items-center gap-3 flex-wrap mb-6",
            Link { to: Route::HarvestOverview {}, class: "imp-back text-primary text-sm", "data-testid": "back-to-harvest", "← Harvest" }
            span { class: "text-faint text-sm", "·" }
            Link { to: Route::HarvestRuns { before: RunsCursor::default() }, class: "imp-back text-secondary text-sm", "data-testid": "all-runs", "All runs" }
        }
    }
}
