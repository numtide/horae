use dioxus::prelude::*;

use super::run_list::RunList;
use super::runs_cursor::RunsCursor;
use crate::route::Route;

/// Every retained run, a page at a time; `before` is the last run of the
/// previous page.
#[component]
pub fn HarvestRuns(before: RunsCursor) -> Element {
    let cursor = before.0;
    rsx! {
        div { class: "himp-page min-w-0",
            div { class: "page-header",
                h1 { class: "page-title", "Harvest runs" }
            }
            div { class: "flex items-center gap-3 flex-wrap mb-6",
                Link { to: Route::HarvestOverview {}, class: "imp-back text-primary text-sm", "data-testid": "back-to-harvest", "← Back to Harvest" }
                if cursor.is_some() {
                    span { class: "text-faint text-sm", "·" }
                    Link { to: Route::HarvestRuns { before: RunsCursor::default() }, class: "imp-back text-secondary text-sm", "data-testid": "newer-history", "Newest runs" }
                }
            }
            section { class: "card", aria_label: "Import history",
                p { class: "text-xs text-faint", "Dry runs and commits, newest first. Open a run to see its report." }
                RunList { before: cursor, limit: 20, paged: true }
            }
        }
    }
}
