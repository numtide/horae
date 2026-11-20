use dioxus::prelude::*;

use super::run_list::RunList;
use super::runs_cursor::RunsCursor;
use crate::components::back_link::BackLink;
use crate::components::page_header::PageHeader;
use crate::route::Route;

/// Every retained run, a page at a time; `before` is the last run of the
/// previous page.
#[component]
pub fn HarvestRuns(before: RunsCursor) -> Element {
    let cursor = before.0;
    rsx! {
        div { class: "himp-page min-w-0",
            BackLink { to: Route::HarvestOverview { outcome: Default::default() }, label: "Back to Harvest" }
            PageHeader { title: "Harvest runs", title_id: "harvest-runs-heading" }
            if cursor.is_some() {
                Link { to: Route::HarvestRuns { before: RunsCursor::default() }, class: "text-sm", "data-testid": "newer-history", "Newest runs" }
            }
            section { aria_label: "Import history",
                RunList { before: cursor, limit: 20, paged: true }
            }
        }
    }
}
