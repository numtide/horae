use dioxus::prelude::*;

use super::run_list::RunList;
use crate::components::back_link::BackLink;
use crate::components::page_header::PageHeader;
use crate::route::Route;

/// Every retained run, newest first, a page at a time with "Load more".
#[component]
pub fn HarvestRuns() -> Element {
    rsx! {
        div { class: "himp-page min-w-0",
            BackLink { to: Route::HarvestOverview { outcome: Default::default() }, label: "Back to Harvest" }
            PageHeader { title: "Harvest runs", title_id: "harvest-runs-heading" }
            section { aria_label: "Import history",
                RunList { limit: 20 }
            }
        }
    }
}
