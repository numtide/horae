use dioxus::prelude::*;

use crate::route::Route;

/// Importers / Harvest / `here`. Without `here`, Harvest is the current page.
#[component]
pub(super) fn BackLinks(#[props(default)] here: Option<String>) -> Element {
    rsx! {
        nav { aria_label: "Breadcrumb", class: "text-sm text-secondary mb-4",
            Link { to: Route::ImporterIndex {}, "data-testid": "all-importers", "Importers" }
            span { aria_hidden: "true", class: "px-2", "/" }
            if let Some(here) = here {
                Link { to: Route::HarvestOverview { outcome: Default::default() }, "data-testid": "back-to-harvest", "Harvest" }
                span { aria_hidden: "true", class: "px-2", "/" }
                span { aria_current: "page", "{here}" }
            } else {
                span { aria_current: "page", "Harvest" }
            }
        }
    }
}
